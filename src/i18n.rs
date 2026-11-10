//! Textes affichés à l'utilisateur, et noms des touches, dans chaque langue.
//!
//! Première brique de la traduction (l'étape 10 branchera des fichiers de traduction à la place
//! des textes écrits ici). Deux principes :
//! - un texte traduisible peut contenir des EMPLACEMENTS comme `{key}`, remplis au moment de
//!   l'affichage. La traduction ne « connaît » donc jamais la touche elle-même ;
//! - le nom d'une touche est calculé à partir de la touche RÉELLEMENT configurée, et traduit lui
//!   aussi. Si on change la touche, tous les messages qui la citent suivent automatiquement.

use crate::command::CommandKind;
use crate::hotkeys::{Key, Shortcut};
use crate::tracker::Targets;

/// La langue de l'interface. Elle sera détectée et réglable à l'étape 10.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    French,
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "choix de la langue à l'étape 10")
    )]
    English,
}

/// Les textes traduisibles. Une variante par message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Text {
    /// Demande de confirmation du reset. Emplacement : `{key}` (touche de validation).
    ResetPrompt,
    /// Titre de la fenêtre du compteur global (visible dans la barre des tâches et OBS).
    GlobalWindowTitle,
    /// Titre de la fenêtre du compteur de partie.
    RoundWindowTitle,
    /// Titre de la fenêtre de configuration.
    ConfigWindowTitle,
    /// Onglet des compteurs.
    TabCounters,
    /// Onglet des raccourcis.
    TabShortcuts,
    /// Bouton pour quitter l'appli.
    Quit,
    /// Titre de la section du global.
    GlobalCounter,
    /// Titre de la section de la partie.
    RoundCounter,
    /// Case « afficher la fenêtre ».
    ShowWindow,
    /// Objectif d'un compteur.
    Goal,
    /// Option de copie de l'affichage.
    RoundCopiesGlobal,
    /// Champ du format.
    Template,
    /// Aide du format.
    TemplateHelp,
    /// Taille de police.
    FontSize,
    /// Marge autour du texte.
    Margin,
    /// Rayon des coins.
    CornerRadius,
    /// Ancrage.
    Anchor,
    /// Ancrage à gauche.
    AnchorLeft,
    /// Ancrage au centre.
    AnchorCenter,
    /// Ancrage à droite.
    AnchorRight,
    /// Premier plan.
    AlwaysOnTop,
    /// Couleur de fond.
    Background,
    /// Fond transparent.
    TransparentBackground,
    /// Couleur du texte sans objectif.
    PlainTextColor,
    /// Couleur objectif pas atteint.
    GoalPendingColor,
    /// Couleur objectif atteint.
    GoalReachedColor,
    /// Couleur des alertes.
    AlertColor,
    /// Compteurs visés : les deux.
    TargetBoth,
    /// Compteurs visés : le global.
    TargetGlobal,
    /// Compteurs visés : la partie.
    TargetRound,
    /// Action reset du global.
    ActionResetGlobal,
    /// Action reset de la partie.
    ActionResetRound,
    /// Action valider le reset.
    ActionConfirmReset,
    /// Action annuler le reset.
    ActionUndoReset,
    /// Aide de l'onglet raccourcis.
    ShortcutsHelp,
    /// Capture en cours.
    PressShortcut,
    /// Refus d'un clic gauche/droit seul.
    MainClickNeedsModifier,
    /// Avertissement de conflit.
    ShortcutConflict,
    /// Bouton de remise à zéro des raccourcis.
    RestoreDefaultShortcuts,
    /// Infobulle du bouton ＋.
    AddShortcut,
    /// Infobulle du bouton ✕.
    RemoveShortcut,
    /// Ligne +1.
    ActionAdd1,
    /// Ligne −1.
    ActionSubtract1,
    /// Ligne +5.
    ActionAdd5,
    /// Ligne −5.
    ActionSubtract5,
    /// Titre de la fenêtre du menu d'un compteur.
    MenuWindowTitle,
    /// Champ de saisie de la valeur.
    Value,
    /// Bouton de validation.
    Apply,
    /// Bouton de reset dans le menu.
    ResetAction,
    /// Bouton pour cacher la fenêtre.
    HideWindow,
    /// Bouton pour ouvrir la configuration.
    OpenSettings,
    /// Rappel du menu dans la configuration.
    MenuHint,
}

/// Le texte d'un message dans une langue, AVANT remplissage des emplacements.
/// `&'static str` : un texte écrit dans le programme, qui existe pendant toute son exécution.
pub fn template(text: Text, language: Language) -> &'static str {
    match (text, language) {
        (Text::ResetPrompt, Language::French) => "Reset ? ({key} pour valider)",
        (Text::ResetPrompt, Language::English) => "Reset? ({key} to confirm)",
        (Text::GlobalWindowTitle, Language::French) => "Literally A Counter : global",
        (Text::GlobalWindowTitle, Language::English) => "Literally A Counter: total",
        (Text::RoundWindowTitle, Language::French) => "Literally A Counter : partie",
        (Text::RoundWindowTitle, Language::English) => "Literally A Counter: round",
        (Text::ConfigWindowTitle, Language::French) => "Literally A Counter : configuration",
        (Text::ConfigWindowTitle, Language::English) => "Literally A Counter: settings",
        (Text::TabCounters, Language::French) => "Compteurs",
        (Text::TabCounters, Language::English) => "Counters",
        (Text::TabShortcuts, Language::French) => "Raccourcis",
        (Text::TabShortcuts, Language::English) => "Shortcuts",
        (Text::Quit, Language::French) => "Quitter",
        (Text::Quit, Language::English) => "Quit",
        (Text::GlobalCounter, Language::French) => "Compteur global",
        (Text::GlobalCounter, Language::English) => "Total counter",
        (Text::RoundCounter, Language::French) => "Compteur de partie",
        (Text::RoundCounter, Language::English) => "Round counter",
        (Text::ShowWindow, Language::French) => "Afficher la fenêtre",
        (Text::ShowWindow, Language::English) => "Show the window",
        (Text::Goal, Language::French) => "Objectif (0 = aucun)",
        (Text::Goal, Language::English) => "Goal (0 = none)",
        (Text::RoundCopiesGlobal, Language::French) => "Copier l'affichage du global",
        (Text::RoundCopiesGlobal, Language::English) => "Use the total's display",
        (Text::Template, Language::French) => "Format",
        (Text::Template, Language::English) => "Format",
        (Text::TemplateHelp, Language::French) => {
            "Variables : {value}, {goal}, {remaining}, {ratio}, {ratio:p}. Entre crochets : affiché seulement s'il y a un objectif."
        }
        (Text::TemplateHelp, Language::English) => {
            "Variables: {value}, {goal}, {remaining}, {ratio}, {ratio:p}. Inside brackets: shown only when there is a goal."
        }
        (Text::FontSize, Language::French) => "Taille du texte",
        (Text::FontSize, Language::English) => "Text size",
        (Text::Margin, Language::French) => "Marge",
        (Text::Margin, Language::English) => "Margin",
        (Text::CornerRadius, Language::French) => "Arrondi des coins",
        (Text::CornerRadius, Language::English) => "Corner radius",
        (Text::Anchor, Language::French) => "Quand la largeur change",
        (Text::Anchor, Language::English) => "When the width changes",
        (Text::AnchorLeft, Language::French) => "Bord gauche fixe",
        (Text::AnchorLeft, Language::English) => "Left edge fixed",
        (Text::AnchorCenter, Language::French) => "Centre fixe",
        (Text::AnchorCenter, Language::English) => "Center fixed",
        (Text::AnchorRight, Language::French) => "Bord droit fixe",
        (Text::AnchorRight, Language::English) => "Right edge fixed",
        (Text::AlwaysOnTop, Language::French) => "Toujours au premier plan",
        (Text::AlwaysOnTop, Language::English) => "Always on top",
        (Text::Background, Language::French) => "Fond",
        (Text::Background, Language::English) => "Background",
        (Text::TransparentBackground, Language::French) => "Transparent",
        (Text::TransparentBackground, Language::English) => "Transparent",
        (Text::PlainTextColor, Language::French) => "Texte (sans objectif)",
        (Text::PlainTextColor, Language::English) => "Text (no goal)",
        (Text::GoalPendingColor, Language::French) => "Objectif pas atteint",
        (Text::GoalPendingColor, Language::English) => "Goal not reached",
        (Text::GoalReachedColor, Language::French) => "Objectif atteint",
        (Text::GoalReachedColor, Language::English) => "Goal reached",
        (Text::AlertColor, Language::French) => "Alertes (« Reset ? »)",
        (Text::AlertColor, Language::English) => "Alerts (\"Reset?\")",
        (Text::TargetBoth, Language::French) => "global et partie",
        (Text::TargetBoth, Language::English) => "total and round",
        (Text::TargetGlobal, Language::French) => "global",
        (Text::TargetGlobal, Language::English) => "total",
        (Text::TargetRound, Language::French) => "partie",
        (Text::TargetRound, Language::English) => "round",
        (Text::ActionResetGlobal, Language::French) => "Reset du global",
        (Text::ActionResetGlobal, Language::English) => "Reset the total",
        (Text::ActionResetRound, Language::French) => "Reset de la partie",
        (Text::ActionResetRound, Language::English) => "Reset the round",
        (Text::ActionConfirmReset, Language::French) => "Valider le reset",
        (Text::ActionConfirmReset, Language::English) => "Confirm the reset",
        (Text::ActionUndoReset, Language::French) => "Annuler le dernier reset",
        (Text::ActionUndoReset, Language::English) => "Undo the last reset",
        (Text::ShortcutsHelp, Language::French) => {
            "Clique sur un raccourci pour le changer, ou sur + pour en ajouter. Appuie sur les touches voulues : la combinaison est enregistrée au premier relâchement."
        }
        (Text::ShortcutsHelp, Language::English) => {
            "Click a shortcut to change it, or + to add one. Press the keys you want: the combination is saved when you release the first key."
        }
        (Text::PressShortcut, Language::French) => {
            "Appuie sur la combinaison… (Échap pour annuler)"
        }
        (Text::PressShortcut, Language::English) => "Press the combination… (Esc to cancel)",
        (Text::MainClickNeedsModifier, Language::French) => {
            "Clic gauche ou droit : ajoute Ctrl, Maj ou Alt, sinon il se déclencherait à chaque tir ou visée en jeu."
        }
        (Text::MainClickNeedsModifier, Language::English) => {
            "Left or right click: add Ctrl, Shift or Alt, otherwise it would trigger on every shot or aim in game."
        }
        (Text::ShortcutConflict, Language::French) => {
            "Cette combinaison sert aussi à une autre action : les deux se déclencheront ensemble."
        }
        (Text::ShortcutConflict, Language::English) => {
            "This combination is also used by another action: both will trigger together."
        }
        (Text::RestoreDefaultShortcuts, Language::French) => "Rétablir les raccourcis par défaut",
        (Text::RestoreDefaultShortcuts, Language::English) => "Restore default shortcuts",
        (Text::AddShortcut, Language::French) => "Ajouter un raccourci",
        (Text::AddShortcut, Language::English) => "Add a shortcut",
        (Text::RemoveShortcut, Language::French) => "Supprimer ce raccourci",
        (Text::RemoveShortcut, Language::English) => "Remove this shortcut",
        (Text::ActionAdd1, Language::French) => "+1",
        (Text::ActionAdd1, Language::English) => "+1",
        (Text::ActionSubtract1, Language::French) => "−1",
        (Text::ActionSubtract1, Language::English) => "−1",
        (Text::ActionAdd5, Language::French) => "+5",
        (Text::ActionAdd5, Language::English) => "+5",
        (Text::ActionSubtract5, Language::French) => "−5",
        (Text::ActionSubtract5, Language::English) => "−5",
        (Text::MenuWindowTitle, Language::French) => "Literally A Counter : menu",
        (Text::MenuWindowTitle, Language::English) => "Literally A Counter: menu",
        (Text::Value, Language::French) => "Valeur",
        (Text::Value, Language::English) => "Value",
        (Text::Apply, Language::French) => "Valider",
        (Text::Apply, Language::English) => "Apply",
        (Text::ResetAction, Language::French) => "Reset",
        (Text::ResetAction, Language::English) => "Reset",
        (Text::HideWindow, Language::French) => "Cacher la fenêtre",
        (Text::HideWindow, Language::English) => "Hide the window",
        (Text::OpenSettings, Language::French) => "Configuration…",
        (Text::OpenSettings, Language::English) => "Settings…",
        (Text::MenuHint, Language::French) => {
            "Le menu d'un compteur (saisie de la valeur, reset…) s'ouvre avec le bouton ••• qui apparaît quand la souris le survole."
        }
        (Text::MenuHint, Language::English) => {
            "A counter's menu (set value, reset…) opens with the ••• button shown when the mouse hovers it."
        }
    }
}

/// Un message sans emplacement (raccourci pour `message(text, language, &[])`).
pub fn text(text: Text, language: Language) -> String {
    template(text, language).to_string()
}

/// Le message, avec ses emplacements remplis.
/// `values` : des paires (nom de l'emplacement, valeur), par ex. `[("key", "Num Entrée")]`.
/// `&[(&str, &str)]` : une TRANCHE (une vue sur un tableau) de tuples de textes empruntés.
pub fn message(text: Text, language: Language, values: &[(&str, &str)]) -> String {
    let mut result = template(text, language).to_string();
    for (name, value) in values {
        result = result.replace(&format!("{{{name}}}"), value);
    }
    result
}

/// Le nom d'une touche, dans la langue demandée.
pub fn key_name(key: Key, language: Language) -> String {
    let french = language == Language::French;
    let name = match key {
        Key::CONTROL => "Ctrl",
        Key::SHIFT if french => "Maj",
        Key::SHIFT => "Shift",
        Key::ALT => "Alt",
        Key::ENTER if french => "Entrée",
        Key::ENTER => "Enter",
        Key::NUMPAD_ENTER if french => "Num Entrée",
        Key::NUMPAD_ENTER => "Num Enter",
        Key::NUMPAD_ADD => "Num +",
        Key::NUMPAD_SUBTRACT => "Num -",
        Key::NUMPAD_DECIMAL => "Num .",
        Key::MOUSE_LEFT if french => "Clic gauche",
        Key::MOUSE_LEFT => "Left click",
        Key::MOUSE_RIGHT if french => "Clic droit",
        Key::MOUSE_RIGHT => "Right click",
        Key::MOUSE_MIDDLE if french => "Clic molette",
        Key::MOUSE_MIDDLE => "Middle click",
        Key::MOUSE_X1 if french => "Souris X1 (arrière)",
        Key::MOUSE_X1 => "Mouse X1 (back)",
        Key::MOUSE_X2 if french => "Souris X2 (avant)",
        Key::MOUSE_X2 => "Mouse X2 (forward)",
        // Les autres touches sont calculées plus bas.
        _ => "",
    };
    if !name.is_empty() {
        return name.to_string();
    }
    match key.0 {
        // Num 0 à Num 9 : codes 0x60 à 0x69.
        code @ 0x60..=0x69 => format!("Num {}", code - 0x60),
        // Chiffres et lettres : leur code est celui du caractère ('0' = 0x30, 'A' = 0x41).
        // `code @ motif` : donne le nom `code` à la valeur qui correspond au motif.
        code @ (0x30..=0x39 | 0x41..=0x5A) => char::from(code as u8).to_string(),
        // F1 à F24 : codes 0x70 à 0x87.
        code @ 0x70..=0x87 => format!("F{}", code - 0x70 + 1),
        // Inconnue : on affiche son code, en hexadécimal (`{:02X}`).
        code if french => format!("Touche 0x{code:02X}"),
        code => format!("Key 0x{code:02X}"),
    }
}

/// Le nom d'un raccourci complet, par ex. « Ctrl + Num 0 ».
pub fn shortcut_name(shortcut: &Shortcut, language: Language) -> String {
    // On met bout à bout les touches à tenir puis la touche déclencheuse, séparées par « + ».
    shortcut
        .held
        .iter()
        .chain(std::iter::once(&shortcut.trigger))
        .map(|&key| key_name(key, language))
        .collect::<Vec<_>>()
        .join(" + ")
}

/// Le nom d'une ligne du tableau des raccourcis (« +1 », « Reset du global »…).
pub fn command_kind_name(kind: CommandKind, language: Language) -> String {
    let text_id = match kind {
        CommandKind::Add1 => Text::ActionAdd1,
        CommandKind::Subtract1 => Text::ActionSubtract1,
        CommandKind::Add5 => Text::ActionAdd5,
        CommandKind::Subtract5 => Text::ActionSubtract5,
        CommandKind::ResetRound => Text::ActionResetRound,
        CommandKind::ResetGlobal => Text::ActionResetGlobal,
        CommandKind::ConfirmReset => Text::ActionConfirmReset,
        CommandKind::UndoReset => Text::ActionUndoReset,
    };
    text(text_id, language)
}

/// Le nom des compteurs visés (« global et partie », « global », « partie »).
pub fn targets_name(targets: Targets, language: Language) -> String {
    let text_id = match (targets.global, targets.round) {
        (true, true) => Text::TargetBoth,
        (true, false) => Text::TargetGlobal,
        // `_` : les autres cas (partie seule ; « aucun » n'existe pas en pratique).
        _ => Text::TargetRound,
    };
    text(text_id, language)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::Command;
    use crate::tracker::CounterId;

    #[test]
    fn reset_prompt_uses_the_configured_key() {
        assert_eq!(
            message(
                Text::ResetPrompt,
                Language::French,
                &[("key", "Num Entrée")]
            ),
            "Reset ? (Num Entrée pour valider)"
        );
        // Une autre touche configurée : le message suit, sans toucher à la traduction.
        assert_eq!(
            message(Text::ResetPrompt, Language::English, &[("key", "F5")]),
            "Reset? (F5 to confirm)"
        );
    }

    #[test]
    fn key_names_are_translated() {
        assert_eq!(key_name(Key::NUMPAD_ENTER, Language::French), "Num Entrée");
        assert_eq!(key_name(Key::NUMPAD_ENTER, Language::English), "Num Enter");
        assert_eq!(key_name(Key::SHIFT, Language::French), "Maj");
        assert_eq!(key_name(Key::SHIFT, Language::English), "Shift");
    }

    #[test]
    fn mouse_button_names() {
        assert_eq!(
            key_name(Key::MOUSE_MIDDLE, Language::French),
            "Clic molette"
        );
        assert_eq!(
            key_name(Key::MOUSE_MIDDLE, Language::English),
            "Middle click"
        );
        let shortcut = Shortcut {
            held: vec![Key::CONTROL],
            trigger: Key::MOUSE_MIDDLE,
            command: Command::Reset(CounterId::Global),
        };
        assert_eq!(
            shortcut_name(&shortcut, Language::French),
            "Ctrl + Clic molette"
        );
    }

    #[test]
    fn computed_key_names() {
        assert_eq!(key_name(Key::NUMPAD_0, Language::French), "Num 0");
        assert_eq!(key_name(Key(0x69), Language::French), "Num 9");
        assert_eq!(key_name(Key(0x41), Language::French), "A");
        assert_eq!(key_name(Key(0x35), Language::French), "5");
        assert_eq!(key_name(Key(0x74), Language::French), "F5");
        assert_eq!(key_name(Key(0xE9), Language::French), "Touche 0xE9");
        assert_eq!(key_name(Key(0xE9), Language::English), "Key 0xE9");
    }

    #[test]
    fn shortcut_names() {
        let reset = Shortcut {
            held: vec![Key::CONTROL],
            trigger: Key::NUMPAD_0,
            command: Command::Reset(CounterId::Global),
        };
        assert_eq!(shortcut_name(&reset, Language::French), "Ctrl + Num 0");

        let confirm = Shortcut {
            held: vec![],
            trigger: Key::NUMPAD_ENTER,
            command: Command::ConfirmReset,
        };
        assert_eq!(shortcut_name(&confirm, Language::French), "Num Entrée");
    }
}
