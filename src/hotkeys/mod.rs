//! Raccourcis clavier globaux : détecter des touches ou combinaisons de touches
//! même quand une autre application (le jeu) a le focus, SANS bloquer ces touches.
//!
//! Organisation :
//! - ce fichier (`mod.rs`) : les types communs (`Key`, `Shortcut`) et la fonction `start` ;
//! - `matcher.rs` : la logique PURE « quelles touches sont enfoncées → quelle action »,
//!   indépendante du système, donc testable automatiquement ;
//! - `windows.rs` : le branchement sur Windows (le hook clavier), compilé uniquement sous Windows.

mod matcher;

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

/// Version « vide » pour les systèmes pas encore pris en charge : ne fait rien.
/// (La v1 cible Windows ; cela permet juste au projet de compiler ailleurs.)
#[cfg(not(windows))]
pub fn start(_shortcuts: Vec<Shortcut>, _on_command: impl Fn(Command) + Send + 'static) {
    eprintln!("Raccourcis globaux non pris en charge sur ce système.");
}
