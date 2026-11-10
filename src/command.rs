//! Les commandes que l'utilisateur peut déclencher (par raccourci clavier ou à la souris).

use crate::counter::CounterAction;
use crate::tracker::{CounterId, Targets};

/// Une commande.
///
/// `Count(Targets, CounterAction)` : une variante qui transporte DEUX données, les compteurs
/// visés et l'action. On réutilise ainsi `CounterAction` (Add / Subtract) sans le dupliquer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    /// Ajouter ou retirer une quantité, sur un ou deux compteurs.
    Count(Targets, CounterAction),
    /// Demander un reset : immédiat pour la partie, à confirmer pour le global.
    Reset(CounterId),
    /// Confirmer le reset du global demandé.
    ConfirmReset,
    /// Annuler le dernier reset (celui de la partie ou du global, le plus récent).
    UndoReset,
}
