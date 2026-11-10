//! Raccourcis clavier globaux : détecter des touches ou combinaisons de touches
//! même quand une autre application (le jeu) a le focus, SANS bloquer ces touches.
//!
//! Organisation :
//! - ce fichier (`mod.rs`) : les types communs (`Key`, `Shortcut`) et la fonction `start` ;
//! - `matcher.rs` : la logique PURE « quelles touches sont enfoncées → quelle action »,
//!   indépendante du système, donc testable automatiquement ;
//! - `mouse.rs` : la logique PURE de l'interrogation des boutons de souris ;
//! - `capture.rs` : la logique PURE de la capture d'un raccourci « à l'appui » ;
//! - `windows.rs` : le branchement sur Windows (hook clavier + interrogation de la souris),
//!   compilé uniquement sous Windows.

mod capture;
mod matcher;
mod mouse;

// `#[cfg(windows)]` : ce module n'est compilé que sous Windows.
// Pour Linux/Mac, on ajoutera plus tard d'autres modules derrière la même fonction `start`.
#[cfg(windows)]
mod windows;

// `pub use` : on RÉ-EXPORTE ces éléments pour que le reste du programme écrive
// `hotkeys::start` au lieu de `hotkeys::windows::start`.
#[cfg(windows)]
pub use windows::{HotkeyHandle, start};

use crate::command::Command;
use crate::counter::CounterAction;
use crate::tracker::{CounterId, Targets};

/// Ce que le thread des raccourcis envoie à l'appli.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HotkeyEvent {
    /// Un raccourci s'est déclenché.
    Command(Command),
    /// Une capture est terminée : la combinaison appuyée.
    Captured { held: Vec<Key>, trigger: Key },
    /// La capture a été annulée (Échap).
    CaptureCancelled,
}

/// Ce que l'appli demande au thread des raccourcis, pendant qu'il tourne.
#[derive(Debug, Clone)]
pub enum Control {
    /// Remplacer la liste des raccourcis (après une modification dans la configuration).
    SetShortcuts(Vec<Shortcut>),
    /// Capturer la prochaine combinaison. Les raccourcis sont suspendus pendant la capture.
    StartCapture,
    /// Abandonner la capture en cours.
    CancelCapture,
}

/// Une touche du clavier, identifiée par son code « virtual-key » Windows.
///
/// C'est une « newtype » : une struct qui enveloppe un seul nombre (`u16`).
/// Avantage : le compilateur empêche de confondre une touche avec un nombre quelconque.
/// `Hash` permet de ranger des `Key` dans un ensemble (`HashSet`).
///
/// `#[serde(transparent)]` : dans la sauvegarde, une touche s'écrit comme son simple nombre.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct Key(pub u16);

// Constantes associées au type `Key` : on écrit `Key::NUMPAD_ADD` plutôt qu'un nombre magique.
// Les valeurs sont les codes virtual-key de Windows.
impl Key {
    pub const SHIFT: Key = Key(0x10);
    pub const CONTROL: Key = Key(0x11);
    pub const ALT: Key = Key(0x12);
    pub const ENTER: Key = Key(0x0D);
    /// Échap : annule la capture d'un raccourci (elle ne peut donc pas devenir un raccourci).
    pub const ESCAPE: Key = Key(0x1B);
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
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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

/// Tous les boutons de souris (surveillés pendant une capture, pour pouvoir en choisir un).
pub const ALL_MOUSE_BUTTONS: [Key; 5] = [
    Key::MOUSE_LEFT,
    Key::MOUSE_RIGHT,
    Key::MOUSE_MIDDLE,
    Key::MOUSE_X1,
    Key::MOUSE_X2,
];

impl Shortcut {
    /// Les deux raccourcis utilisent-ils exactement les mêmes touches ?
    /// L'ordre des touches tenues ne compte pas (« Ctrl + Maj + A » = « Maj + Ctrl + A »).
    pub fn same_keys(&self, other: &Shortcut) -> bool {
        self.trigger == other.trigger
            && self.held.len() == other.held.len()
            && self.held.iter().all(|k| other.held.contains(k))
    }
}

/// Pour chaque raccourci : `true` si ses touches servent aussi à une AUTRE commande
/// (les deux se déclencheraient en même temps). Sert à afficher un avertissement.
pub fn conflicts(shortcuts: &[Shortcut]) -> Vec<bool> {
    shortcuts
        .iter()
        .map(|s| {
            shortcuts
                .iter()
                .any(|other| other.same_keys(s) && other.command != s.command)
        })
        .collect()
}

/// Les raccourcis par défaut (spec M2).
pub fn default_shortcuts() -> Vec<Shortcut> {
    // Petite fonction locale pour alléger l'écriture de la liste.
    fn shortcut(held: &[Key], trigger: Key, command: Command) -> Shortcut {
        Shortcut {
            // `to_vec()` : copie la tranche dans un nouveau `Vec`.
            held: held.to_vec(),
            trigger,
            command,
        }
    }
    // Les commandes « compter » visent les DEUX compteurs (partie + global).
    let both = |action| Command::Count(Targets::BOTH, action);
    // `vec![...]` : macro qui crée un `Vec` avec les éléments donnés.
    vec![
        shortcut(&[], Key::NUMPAD_ADD, both(CounterAction::Add(1))),
        shortcut(&[], Key::NUMPAD_SUBTRACT, both(CounterAction::Subtract(1))),
        shortcut(
            &[Key::CONTROL],
            Key::NUMPAD_ADD,
            both(CounterAction::Add(5)),
        ),
        shortcut(
            &[Key::CONTROL],
            Key::NUMPAD_SUBTRACT,
            both(CounterAction::Subtract(5)),
        ),
        shortcut(&[], Key::NUMPAD_0, Command::Reset(CounterId::Round)),
        shortcut(
            &[Key::CONTROL],
            Key::NUMPAD_0,
            Command::Reset(CounterId::Global),
        ),
        shortcut(&[], Key::NUMPAD_ENTER, Command::ConfirmReset),
        shortcut(&[Key::CONTROL], Key::NUMPAD_DECIMAL, Command::UndoReset),
        shortcut(
            &[Key::CONTROL],
            Key::MOUSE_MIDDLE,
            both(CounterAction::Add(1)),
        ),
        shortcut(&[], Key::MOUSE_X2, both(CounterAction::Add(1))),
        shortcut(&[], Key::MOUSE_X1, both(CounterAction::Subtract(1))),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn same_keys_ignores_the_order_of_held_keys() {
        let a = shortcut(&[Key::CONTROL, Key::SHIFT], Key::NUMPAD_ADD);
        let b = shortcut(&[Key::SHIFT, Key::CONTROL], Key::NUMPAD_ADD);
        assert!(a.same_keys(&b));
        assert!(!a.same_keys(&shortcut(&[Key::CONTROL], Key::NUMPAD_ADD)));
    }

    #[test]
    fn conflict_only_between_different_commands() {
        let mut reset = shortcut(&[], Key::NUMPAD_ADD);
        reset.command = Command::Reset(CounterId::Round);
        let shortcuts = vec![
            shortcut(&[], Key::NUMPAD_ADD),             // +1 sur Num +
            reset,                                      // reset partie sur Num + aussi : conflit
            shortcut(&[], Key::MOUSE_X2), // +1 sur X2 : même commande ailleurs, pas de conflit
            shortcut(&[Key::CONTROL], Key::NUMPAD_ADD), // plus précis : pas de conflit
        ];
        assert_eq!(conflicts(&shortcuts), vec![true, true, false, false]);
    }

    #[test]
    fn same_command_on_different_counters_is_a_conflict() {
        // +1 « global et partie » et +1 « global » sur la même touche : le global compterait 2.
        let mut global_only = shortcut(&[], Key::NUMPAD_ADD);
        global_only.command =
            Command::Count(Targets::only(CounterId::Global), CounterAction::Add(1));
        let shortcuts = vec![shortcut(&[], Key::NUMPAD_ADD), global_only];
        assert_eq!(conflicts(&shortcuts), vec![true, true]);
    }

    #[test]
    fn default_shortcuts_have_no_conflict_and_are_allowed() {
        let shortcuts = default_shortcuts();
        assert!(conflicts(&shortcuts).iter().all(|c| !c));
        assert!(shortcuts.iter().all(Shortcut::is_allowed));
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
pub fn start(
    _shortcuts: Vec<Shortcut>,
    _on_event: impl Fn(HotkeyEvent) + Send + 'static,
) -> Option<HotkeyHandle> {
    eprintln!("Raccourcis globaux non pris en charge sur ce système.");
    None
}

/// Version « vide » de la poignée de pilotage, hors Windows.
#[cfg(not(windows))]
pub struct HotkeyHandle;

#[cfg(not(windows))]
impl HotkeyHandle {
    pub fn send(&self, _control: Control) {}
}
