//! Les commandes que l'utilisateur peut déclencher (par raccourci clavier ou à la souris).

use crate::counter::CounterAction;
use crate::tracker::{CounterId, Targets};

/// Une commande.
///
/// `Count(Targets, CounterAction)` : une variante qui transporte DEUX données, les compteurs
/// visés et l'action. On réutilise ainsi `CounterAction` (Add / Subtract) sans le dupliquer.
///
/// `Serialize` / `Deserialize` : les raccourcis (et donc leur commande) sont sauvegardés.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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

/// Le « genre » d'une commande, sans les compteurs visés : c'est une ligne du tableau des
/// raccourcis (« +1 », « Reset du global »…). Les raccourcis +1 « global et partie » et +1
/// « partie » sont du même genre, et apparaissent donc sur la même ligne.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CommandKind {
    Add1,
    Subtract1,
    Add5,
    Subtract5,
    ResetRound,
    ResetGlobal,
    ConfirmReset,
    UndoReset,
}

impl CommandKind {
    /// Toutes les lignes du tableau, dans l'ordre d'affichage.
    pub const ALL: [CommandKind; 8] = [
        CommandKind::Add1,
        CommandKind::Subtract1,
        CommandKind::Add5,
        CommandKind::Subtract5,
        CommandKind::ResetRound,
        CommandKind::ResetGlobal,
        CommandKind::ConfirmReset,
        CommandKind::UndoReset,
    ];

    /// Le genre d'une commande. `None` pour une quantité sans ligne dédiée (par ex. +3,
    /// possible seulement en modifiant la sauvegarde à la main).
    pub fn of(command: Command) -> Option<Self> {
        Some(match command {
            Command::Count(_, CounterAction::Add(1)) => CommandKind::Add1,
            Command::Count(_, CounterAction::Subtract(1)) => CommandKind::Subtract1,
            Command::Count(_, CounterAction::Add(5)) => CommandKind::Add5,
            Command::Count(_, CounterAction::Subtract(5)) => CommandKind::Subtract5,
            Command::Count(..) => return None,
            Command::Reset(CounterId::Round) => CommandKind::ResetRound,
            Command::Reset(CounterId::Global) => CommandKind::ResetGlobal,
            Command::ConfirmReset => CommandKind::ConfirmReset,
            Command::UndoReset => CommandKind::UndoReset,
        })
    }

    /// La commande de ce genre. `targets` ne sert qu'aux genres qui comptent.
    pub fn command(self, targets: Targets) -> Command {
        match self {
            CommandKind::Add1 => Command::Count(targets, CounterAction::Add(1)),
            CommandKind::Subtract1 => Command::Count(targets, CounterAction::Subtract(1)),
            CommandKind::Add5 => Command::Count(targets, CounterAction::Add(5)),
            CommandKind::Subtract5 => Command::Count(targets, CounterAction::Subtract(5)),
            CommandKind::ResetRound => Command::Reset(CounterId::Round),
            CommandKind::ResetGlobal => Command::Reset(CounterId::Global),
            CommandKind::ConfirmReset => Command::ConfirmReset,
            CommandKind::UndoReset => Command::UndoReset,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_ignores_the_targets() {
        let both = Command::Count(Targets::BOTH, CounterAction::Add(1));
        let round = Command::Count(Targets::only(CounterId::Round), CounterAction::Add(1));
        assert_eq!(CommandKind::of(both), Some(CommandKind::Add1));
        assert_eq!(CommandKind::of(round), Some(CommandKind::Add1));
    }

    #[test]
    fn kind_and_command_go_both_ways() {
        for kind in CommandKind::ALL {
            let command = kind.command(Targets::BOTH);
            assert_eq!(CommandKind::of(command), Some(kind));
        }
    }

    #[test]
    fn other_amounts_have_no_row() {
        let plus_three = Command::Count(Targets::BOTH, CounterAction::Add(3));
        assert_eq!(CommandKind::of(plus_three), None);
    }
}
