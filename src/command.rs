//! Les commandes que l'utilisateur peut déclencher (par raccourci clavier ou à la souris).

use crate::counter::CounterAction;

/// Une commande.
///
/// `Count(CounterAction)` : une variante qui ENVELOPPE un autre enum. On réutilise ainsi
/// `CounterAction` (Add / Subtract) sans le dupliquer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    /// Ajouter ou retirer une quantité.
    Count(CounterAction),
    /// Demander un reset (pour le compteur global, il faudra ensuite le confirmer).
    Reset,
    /// Confirmer le reset demandé.
    ConfirmReset,
    /// Annuler le dernier reset.
    UndoReset,
}
