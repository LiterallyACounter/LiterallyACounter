//! Textes affichés à l'utilisateur, et noms des touches, dans chaque langue.
//!
//! Première brique de la traduction (l'étape 10 branchera des fichiers de traduction à la place
//! des textes écrits ici). Deux principes :
//! - un texte traduisible peut contenir des EMPLACEMENTS comme `{key}`, remplis au moment de
//!   l'affichage. La traduction ne « connaît » donc jamais la touche elle-même ;
//! - le nom d'une touche est calculé à partir de la touche RÉELLEMENT configurée, et traduit lui
//!   aussi. Si on change la touche, tous les messages qui la citent suivent automatiquement.

use crate::hotkeys::{Key, Shortcut};

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
    /// Fenêtre principale : case « afficher le global ».
    ShowGlobal,
    /// Fenêtre principale : case « afficher la partie ».
    ShowRound,
    /// Fenêtre principale : option de copie de l'affichage.
    RoundCopiesGlobal,
    /// Fenêtre principale : bouton pour quitter l'appli.
    Quit,
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
        (Text::ShowGlobal, Language::French) => "Afficher le compteur global",
        (Text::ShowGlobal, Language::English) => "Show the total counter",
        (Text::ShowRound, Language::French) => "Afficher le compteur de partie",
        (Text::ShowRound, Language::English) => "Show the round counter",
        (Text::RoundCopiesGlobal, Language::French) => "La partie copie l'affichage du global",
        (Text::RoundCopiesGlobal, Language::English) => "Round uses the total's display",
        (Text::Quit, Language::French) => "Quitter",
        (Text::Quit, Language::English) => "Quit",
    }
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
