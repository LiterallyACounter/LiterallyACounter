//! Logique pure des raccourcis : à partir des appuis et relâchements de touches,
//! décide quelles actions déclencher.
//!
//! Règles (voir F5 dans les specs) :
//! 1. Un raccourci se déclenche quand sa touche `trigger` est PRESSÉE alors que toutes ses
//!    touches `held` sont DÉJÀ tenues. Les autres touches enfoncées (Z pour avancer…) sont ignorées.
//! 2. Si plusieurs raccourcis correspondent, seuls les PLUS PRÉCIS (le plus de touches) se déclenchent.
//! 3. Un appui = une action : la répétition automatique d'une touche maintenue est ignorée.

// `HashSet` : un ensemble sans doublons, avec une recherche très rapide (« cette touche est-elle dedans ? »).
use std::collections::HashSet;

use super::{Key, Shortcut};
use crate::counter::CounterAction;

pub struct Matcher {
    shortcuts: Vec<Shortcut>,
    /// Les touches actuellement enfoncées.
    pressed: HashSet<Key>,
}

impl Matcher {
    /// Crée un matcher avec la liste des raccourcis à surveiller.
    /// `Self` = le type en cours de définition (ici `Matcher`).
    pub fn new(shortcuts: Vec<Shortcut>) -> Self {
        Self {
            shortcuts,
            pressed: HashSet::new(),
        }
    }

    /// À appeler quand une touche est enfoncée. Renvoie les actions à déclencher (souvent aucune).
    pub fn key_down(&mut self, key: Key) -> Vec<CounterAction> {
        // `insert` renvoie `false` si la touche était DÉJÀ dans l'ensemble :
        // c'est la répétition automatique d'une touche maintenue → on ignore (règle 3).
        if !self.pressed.insert(key) {
            return Vec::new();
        }

        // On garde les raccourcis dont la touche déclencheuse est celle-ci
        // ET dont toutes les touches à tenir sont enfoncées (règle 1).
        // `.iter()` parcourt la liste, `.filter(...)` garde ce qui vérifie la condition,
        // `.collect()` rassemble le résultat dans un `Vec`.
        // `&Shortcut` : on garde des RÉFÉRENCES vers les raccourcis, sans les copier.
        let matching: Vec<&Shortcut> = self
            .shortcuts
            .iter()
            .filter(|s| s.trigger == key && s.held.iter().all(|k| self.pressed.contains(k)))
            .collect();

        // Le plus grand nombre de touches tenues parmi les raccourcis trouvés (règle 2).
        // `max()` renvoie une `Option` : `Some(n)` s'il y a au moins un élément, `None` sinon.
        // Rust n'a pas de « null » : l'absence de valeur est explicite et le compilateur
        // oblige à traiter les deux cas.
        let Some(most_precise) = matching.iter().map(|s| s.held.len()).max() else {
            return Vec::new(); // aucun raccourci ne correspond
        };

        matching
            .into_iter()
            .filter(|s| s.held.len() == most_precise)
            .map(|s| s.action)
            .collect()
    }

    /// À appeler quand une touche est relâchée.
    pub fn key_up(&mut self, key: Key) {
        self.pressed.remove(&key);
    }

    /// Oublie les touches que le système dit relâchées.
    ///
    /// Sécurité : si un relâchement nous a échappé (par ex. verrouillage avec Win+L, ou une
    /// fenêtre d'administration Windows qui capte le clavier), la touche resterait « enfoncée »
    /// pour toujours et ses appuis suivants seraient pris pour des répétitions.
    ///
    /// `is_down` est une fonction passée en paramètre (`impl Fn(Key) -> bool`) :
    /// le matcher ne sait pas comment interroger le système, c'est l'appelant qui le fournit.
    /// Dans les tests, on peut ainsi simuler le système très simplement.
    pub fn forget_released(&mut self, is_down: impl Fn(Key) -> bool) {
        // `retain` ne garde que les éléments pour lesquels la fonction renvoie `true`.
        self.pressed.retain(|k| is_down(*k));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const Z: Key = Key(0x5A); // touche « Z » (avancer dans un jeu en AZERTY)

    /// Les raccourcis utilisés dans les tests :
    /// « Num + » → +1, « Num - » → -1, « Ctrl + Num + » → +5.
    fn test_matcher() -> Matcher {
        Matcher::new(vec![
            Shortcut {
                held: vec![],
                trigger: Key::NUMPAD_ADD,
                action: CounterAction::Add(1),
            },
            Shortcut {
                held: vec![],
                trigger: Key::NUMPAD_SUBTRACT,
                action: CounterAction::Subtract(1),
            },
            Shortcut {
                held: vec![Key::CONTROL],
                trigger: Key::NUMPAD_ADD,
                action: CounterAction::Add(5),
            },
        ])
    }

    #[test]
    fn single_key_triggers_its_action() {
        let mut m = test_matcher();
        assert_eq!(m.key_down(Key::NUMPAD_ADD), vec![CounterAction::Add(1)]);
    }

    #[test]
    fn unrelated_key_triggers_nothing() {
        let mut m = test_matcher();
        assert_eq!(m.key_down(Z), vec![]);
    }

    #[test]
    fn other_held_keys_are_ignored() {
        // On avance (Z tenu) et on appuie sur Num + : +1 quand même.
        let mut m = test_matcher();
        m.key_down(Z);
        assert_eq!(m.key_down(Key::NUMPAD_ADD), vec![CounterAction::Add(1)]);
    }

    #[test]
    fn combination_triggers_only_the_most_precise() {
        // Ctrl tenu puis Num + : +5 seulement, PAS +1 en plus.
        let mut m = test_matcher();
        m.key_down(Key::CONTROL);
        assert_eq!(m.key_down(Key::NUMPAD_ADD), vec![CounterAction::Add(5)]);
    }

    #[test]
    fn combination_in_wrong_order_triggers_nothing() {
        // Num + tenu (déclenche +1), PUIS Ctrl : Ctrl n'est le déclencheur d'aucun raccourci.
        let mut m = test_matcher();
        m.key_down(Key::NUMPAD_ADD);
        assert_eq!(m.key_down(Key::CONTROL), vec![]);
    }

    #[test]
    fn held_key_repeat_is_ignored() {
        // Windows répète l'appui tant que la touche est maintenue : une seule action.
        let mut m = test_matcher();
        assert_eq!(m.key_down(Key::NUMPAD_ADD), vec![CounterAction::Add(1)]);
        assert_eq!(m.key_down(Key::NUMPAD_ADD), vec![]);
        assert_eq!(m.key_down(Key::NUMPAD_ADD), vec![]);
    }

    #[test]
    fn release_then_press_triggers_again() {
        let mut m = test_matcher();
        m.key_down(Key::NUMPAD_ADD);
        m.key_up(Key::NUMPAD_ADD);
        assert_eq!(m.key_down(Key::NUMPAD_ADD), vec![CounterAction::Add(1)]);
    }

    #[test]
    fn after_releasing_ctrl_single_key_works_again() {
        let mut m = test_matcher();
        m.key_down(Key::CONTROL);
        m.key_up(Key::CONTROL);
        assert_eq!(m.key_down(Key::NUMPAD_ADD), vec![CounterAction::Add(1)]);
    }

    #[test]
    fn missed_release_is_recovered() {
        // Le relâchement de Num + nous a échappé : sans correction, l'appui suivant
        // serait pris pour une répétition. Le système dit qu'aucune touche n'est enfoncée.
        let mut m = test_matcher();
        m.key_down(Key::NUMPAD_ADD);
        m.forget_released(|_| false); // `|_| false` : fonction qui répond « relâchée » pour toute touche
        assert_eq!(m.key_down(Key::NUMPAD_ADD), vec![CounterAction::Add(1)]);
    }
}
