//! Raccourcis clavier globaux : détecter des touches ou combinaisons de touches
//! même quand une autre application (le jeu) a le focus, SANS bloquer ces touches.
//!
//! Organisation :
//! - ce fichier (`mod.rs`) : les types communs (`Key`, `Shortcut`) et la fonction `start` ;
//! - `matcher.rs` : la logique PURE « quelles touches sont enfoncées → quelle action »,
//!   indépendante du système, donc testable automatiquement ;
//! - `mouse.rs` : la logique PURE de l'interrogation des boutons de souris ;
//! - `windows.rs` : le branchement sur Windows (hook clavier + interrogation de la souris),
//!   compilé uniquement sous Windows.

mod matcher;
mod mouse;

// `#[cfg(windows)]` : ce module n'est compilé que sous Windows.
// Pour Linux/Mac, on ajoutera plus tard d'autres modules derrière la même fonction `start`.
#[cfg(windows)]
mod windows;

// `pub use` : on RÉ-EXPORTE ces éléments pour que le reste du programme écrive
// `hotkeys::start` au lieu de `hotkeys::windows::start`.
#[cfg(windows)]
pub use windows::start;

use crate::command::Command;

/// Une touche du clavier, identifiée par son code « virtual-key » Windows.
///
/// C'est une « newtype » : une struct qui enveloppe un seul nombre (`u16`).
/// Avantage : le compilateur empêche de confondre une touche avec un nombre quelconque.
/// `Hash` permet de ranger des `Key` dans un ensemble (`HashSet`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Key(pub u16);

// Constantes associées au type `Key` : on écrit `Key::NUMPAD_ADD` plutôt qu'un nombre magique.
// Les valeurs sont les codes virtual-key de Windows.
impl Key {
    pub const SHIFT: Key = Key(0x10);
    pub const CONTROL: Key = Key(0x11);
    pub const ALT: Key = Key(0x12);
    pub const ENTER: Key = Key(0x0D);
    pub const NUMPAD_0: Key = Key(0x60);
    pub const NUMPAD_ADD: Key = Key(0x6B);
    pub const NUMPAD_SUBTRACT: Key = Key(0x6D);
    pub const NUMPAD_DECIMAL: Key = Key(0x6E);
    /// Entrée du pavé numérique. Windows lui donne le MÊME code que l'Entrée principale (0x0D) ;
    /// on la distingue grâce à l'indicateur « touche étendue » et on lui attribue un code
    /// à nous, hors de la plage des codes Windows (qui s'arrête à 0xFF).
    pub const NUMPAD_ENTER: Key = Key(0x100 | 0x0D);

    // Boutons de la souris : Windows leur donne aussi des codes virtual-key.
    pub const MOUSE_LEFT: Key = Key(0x01);
    pub const MOUSE_RIGHT: Key = Key(0x02);
    /// Bouton du milieu (clic molette).
    pub const MOUSE_MIDDLE: Key = Key(0x04);
    /// Bouton latéral « arrière » (souvent « page précédente »).
    pub const MOUSE_X1: Key = Key(0x05);
    /// Bouton latéral « avant ».
    pub const MOUSE_X2: Key = Key(0x06);

    /// Est-ce un bouton de souris ?
    pub fn is_mouse(self) -> bool {
        matches!(
            self,
            Key::MOUSE_LEFT | Key::MOUSE_RIGHT | Key::MOUSE_MIDDLE | Key::MOUSE_X1 | Key::MOUSE_X2
        )
    }

    /// Est-ce une touche de modification (Ctrl, Maj, Alt) ?
    pub fn is_modifier(self) -> bool {
        matches!(self, Key::CONTROL | Key::SHIFT | Key::ALT)
    }
}

/// Un raccourci : des touches à tenir (`held`), puis une touche déclencheuse (`trigger`).
///
/// Exemples :
/// - « Num + » seul : `held` vide, `trigger` = `Key::NUMPAD_ADD` ;
/// - « Ctrl + Num + » : `held` = `[Key::CONTROL]`, `trigger` = `Key::NUMPAD_ADD`.
#[derive(Debug, Clone)]
pub struct Shortcut {
    /// Touches qui doivent déjà être enfoncées. `Vec` = un tableau de taille variable.
    pub held: Vec<Key>,
    /// Touche dont l'appui déclenche le raccourci.
    pub trigger: Key,
    /// La commande déclenchée par le raccourci.
    pub command: Command,
}

impl Shortcut {
    /// Le raccourci est-il autorisé ? Le clic gauche et le clic droit ne peuvent déclencher
    /// un raccourci qu'avec au moins une touche de modification (Ctrl, Maj, Alt) tenue :
    /// seuls, ils se déclencheraient à chaque tir ou visée en jeu.
    pub fn is_allowed(&self) -> bool {
        let is_main_click = matches!(self.trigger, Key::MOUSE_LEFT | Key::MOUSE_RIGHT);
        // `.any(...)` : vrai si AU MOINS UN élément vérifie la condition.
        !is_main_click || self.held.iter().any(|key| key.is_modifier())
    }
}

/// Les boutons de souris utilisés par au moins un raccourci, sans doublon.
/// Liste vide → l'appli n'interrogera pas du tout la souris.
pub fn mouse_keys_used(shortcuts: &[Shortcut]) -> Vec<Key> {
    let mut keys: Vec<Key> = shortcuts
        .iter()
        // `flat_map` : pour chaque raccourci, produit PLUSIEURS éléments (ses touches),
        // et met le tout bout à bout en une seule suite.
        .flat_map(|s| s.held.iter().copied().chain(std::iter::once(s.trigger)))
        .filter(|key| key.is_mouse())
        .collect();
    keys.sort_by_key(|key| key.0);
    keys.dedup(); // retire les doublons consécutifs (d'où le tri juste avant)
    keys
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::counter::CounterAction;
    use crate::tracker::Targets;

    fn shortcut(held: &[Key], trigger: Key) -> Shortcut {
        Shortcut {
            held: held.to_vec(),
            trigger,
            command: Command::Count(Targets::BOTH, CounterAction::Add(1)),
        }
    }

    #[test]
    fn main_clicks_need_a_modifier() {
        assert!(!shortcut(&[], Key::MOUSE_LEFT).is_allowed());
        assert!(!shortcut(&[], Key::MOUSE_RIGHT).is_allowed());
        // Une touche non modificatrice ne suffit pas.
        assert!(!shortcut(&[Key::NUMPAD_0], Key::MOUSE_LEFT).is_allowed());
        assert!(shortcut(&[Key::CONTROL], Key::MOUSE_LEFT).is_allowed());
        assert!(shortcut(&[Key::SHIFT], Key::MOUSE_RIGHT).is_allowed());
    }

    #[test]
    fn other_buttons_and_keys_are_always_allowed() {
        assert!(shortcut(&[], Key::MOUSE_MIDDLE).is_allowed());
        assert!(shortcut(&[], Key::MOUSE_X1).is_allowed());
        assert!(shortcut(&[], Key::MOUSE_X2).is_allowed());
        assert!(shortcut(&[], Key::NUMPAD_ADD).is_allowed());
    }

    #[test]
    fn mouse_keys_used_lists_each_button_once() {
        let shortcuts = vec![
            shortcut(&[Key::CONTROL], Key::MOUSE_MIDDLE),
            shortcut(&[], Key::MOUSE_X2),
            shortcut(&[Key::MOUSE_X2], Key::NUMPAD_ADD), // bouton tenu, touche déclencheuse
            shortcut(&[], Key::NUMPAD_ADD),
        ];
        assert_eq!(
            mouse_keys_used(&shortcuts),
            vec![Key::MOUSE_MIDDLE, Key::MOUSE_X2]
        );
    }

    #[test]
    fn no_mouse_shortcut_means_no_mouse_key() {
        let shortcuts = vec![shortcut(&[Key::CONTROL], Key::NUMPAD_ADD)];
        assert!(mouse_keys_used(&shortcuts).is_empty());
    }
}

/// Version « vide » pour les systèmes pas encore pris en charge : ne fait rien.
/// (La v1 cible Windows ; cela permet juste au projet de compiler ailleurs.)
#[cfg(not(windows))]
pub fn start(_shortcuts: Vec<Shortcut>, _on_command: impl Fn(Command) + Send + 'static) {
    eprintln!("Raccourcis globaux non pris en charge sur ce système.");
}
