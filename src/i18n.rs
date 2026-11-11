//! Textes affichés à l'utilisateur, et noms des touches, dans chaque langue (spec F13).
//!
//! Les traductions sont des fichiers au format **Fluent** (`.ftl`, https://projectfluent.org) :
//! `identifiant = texte`, avec des emplacements `{ $nom }` remplis par l'appli.
//! - Le français et l'anglais sont INTÉGRÉS à l'exe (`lang/fr.ftl`, `lang/en.ftl`).
//! - Des PACKS supplémentaires, posés dans un dossier `lang/` à côté de l'exe, ajoutent une
//!   langue (`lang/es.ftl`) ou remplacent une langue intégrée.
//! - Un texte absent d'une traduction est pris dans la langue intégrée de même code, puis en
//!   anglais : une traduction incomplète ne casse jamais l'interface.
//!
//! Deux principes, déjà là avant Fluent :
//! - un texte traduisible peut contenir des EMPLACEMENTS comme `{ $key }`, remplis au moment de
//!   l'affichage. La traduction ne « connaît » donc jamais la touche elle-même ;
//! - le nom d'une touche est calculé à partir de la touche RÉELLEMENT configurée, et traduit lui
//!   aussi. Si on change la touche, tous les messages qui la citent suivent automatiquement.

use std::path::PathBuf;
use std::sync::{Arc, RwLock};

// `concurrent` : la version de Fluent utilisable depuis plusieurs threads (nécessaire pour la
// ranger dans une variable globale, voir `CURRENT`).
use fluent_bundle::concurrent::FluentBundle;
use fluent_bundle::{FluentArgs, FluentResource};

use crate::command::CommandKind;
use crate::hotkeys::{Key, Shortcut};
use crate::tracker::Targets;

/// Les traductions intégrées à l'exe : (code de langue, contenu du fichier).
/// `include_str!` : le fichier est copié dans le programme au moment de la compilation.
const BUILTIN: [(&str, &str); 2] = [
    ("fr", include_str!("../lang/fr.ftl")),
    ("en", include_str!("../lang/en.ftl")),
];

/// La langue de secours, pour tout texte manquant.
const FALLBACK: &str = "en";

/// Les textes traduisibles. Une variante par message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Text {
    /// Onglet des réglages généraux.
    TabGeneral,
    /// Choix de la langue de l'interface.
    LanguageLabel,
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
    /// Choix de la méthode d'écoute de la souris.
    MouseMethodLabel,
    /// Option Raw Input.
    MouseRawInput,
    /// Option interrogation.
    MousePolling,
    /// Raccourci de molette inactif en mode interrogation.
    WheelNeedsRawInput,
    /// Bandeau de protection. Emplacement : `{rate}` (événements par seconde).
    MouseRateWarning,
    /// Bouton du bandeau.
    SwitchToPolling,
    /// Bouton du menu vers l'onglet Raccourcis.
    EditShortcuts,
    /// Titre de la mini-fenêtre du bouton « ••• » (jamais visible).
    MenuButtonWindowTitle,
    /// Choix de la police.
    Font,
    /// Police par défaut.
    DefaultFont,
    /// Champ de recherche de police.
    SearchFont,
    /// Onglet du son.
    TabSound,
    /// Choix de la sortie audio.
    AudioOutput,
    /// Sortie par défaut de Windows.
    DefaultOutput,
    /// Sortie choisie mais débranchée.
    OutputMissing,
    /// Volume général.
    Volume,
    /// Aucune sortie audio.
    NoAudio,
    /// La sortie voulue refuse de s'ouvrir. Emplacement : `{name}` (sortie réellement utilisée).
    OutputUnavailable,
    /// Sortie choisie qui refuse de s'ouvrir. Emplacement : `{name}`.
    OutputRefused,
    /// Titre de la section des sons.
    Sounds,
    /// Emplacement de son : +.
    SlotIncrement,
    /// Emplacement de son : −.
    SlotDecrement,
    /// Emplacement de son : alerte.
    SlotResetPrompt,
    /// Emplacement de son : reset fait.
    SlotResetDone,
    /// Emplacement de son : reset annulé.
    SlotResetUndone,
    /// Bouton pour écouter un son.
    Listen,
    /// Case pour couper un son.
    Mute,
    /// Entrée de liste pour importer.
    ImportFile,
    /// Titre de groupe.
    WindowsSounds,
    /// Titre de groupe.
    Beeps,
    /// Titre de groupe.
    ImportedSounds,
    /// Bip grave.
    BeepLow,
    /// Bip moyen.
    BeepMid,
    /// Bip aigu.
    BeepHigh,
    /// Bip très aigu.
    BeepVeryHigh,
    /// Nom d'un son importé. Emplacement : `{name}`.
    ImportedSound,
    /// Titre de la fenêtre Ouvrir.
    ImportTitle,
    /// Nom du filtre de fichiers.
    SoundFiles,
    /// Import raté : fichier illisible. Emplacement : `{name}`.
    ImportUnreadable,
    /// Import raté : copie impossible. Emplacement : `{name}`.
    ImportCopyFailed,
    /// Titre de groupe : sons embarqués.
    EmbeddedSounds,
}

impl Text {
    /// L'identifiant du texte dans les fichiers `.ftl` (par ex. `reset-prompt`).
    pub fn id(self) -> &'static str {
        match self {
            Text::TabGeneral => "tab-general",
            Text::LanguageLabel => "language-label",
            Text::ResetPrompt => "reset-prompt",
            Text::GlobalWindowTitle => "global-window-title",
            Text::RoundWindowTitle => "round-window-title",
            Text::ConfigWindowTitle => "config-window-title",
            Text::TabCounters => "tab-counters",
            Text::TabShortcuts => "tab-shortcuts",
            Text::Quit => "quit",
            Text::GlobalCounter => "global-counter",
            Text::RoundCounter => "round-counter",
            Text::ShowWindow => "show-window",
            Text::Goal => "goal",
            Text::RoundCopiesGlobal => "round-copies-global",
            Text::Template => "template",
            Text::TemplateHelp => "template-help",
            Text::FontSize => "font-size",
            Text::Margin => "margin",
            Text::CornerRadius => "corner-radius",
            Text::Anchor => "anchor",
            Text::AnchorLeft => "anchor-left",
            Text::AnchorCenter => "anchor-center",
            Text::AnchorRight => "anchor-right",
            Text::AlwaysOnTop => "always-on-top",
            Text::Background => "background",
            Text::TransparentBackground => "transparent-background",
            Text::PlainTextColor => "plain-text-color",
            Text::GoalPendingColor => "goal-pending-color",
            Text::GoalReachedColor => "goal-reached-color",
            Text::AlertColor => "alert-color",
            Text::TargetBoth => "target-both",
            Text::TargetGlobal => "target-global",
            Text::TargetRound => "target-round",
            Text::ActionResetGlobal => "action-reset-global",
            Text::ActionResetRound => "action-reset-round",
            Text::ActionConfirmReset => "action-confirm-reset",
            Text::ActionUndoReset => "action-undo-reset",
            Text::ShortcutsHelp => "shortcuts-help",
            Text::PressShortcut => "press-shortcut",
            Text::MainClickNeedsModifier => "main-click-needs-modifier",
            Text::ShortcutConflict => "shortcut-conflict",
            Text::RestoreDefaultShortcuts => "restore-default-shortcuts",
            Text::AddShortcut => "add-shortcut",
            Text::RemoveShortcut => "remove-shortcut",
            Text::ActionAdd1 => "action-add1",
            Text::ActionSubtract1 => "action-subtract1",
            Text::ActionAdd5 => "action-add5",
            Text::ActionSubtract5 => "action-subtract5",
            Text::MenuWindowTitle => "menu-window-title",
            Text::Value => "value",
            Text::Apply => "apply",
            Text::ResetAction => "reset-action",
            Text::HideWindow => "hide-window",
            Text::OpenSettings => "open-settings",
            Text::MenuHint => "menu-hint",
            Text::MouseMethodLabel => "mouse-method-label",
            Text::MouseRawInput => "mouse-raw-input",
            Text::MousePolling => "mouse-polling",
            Text::WheelNeedsRawInput => "wheel-needs-raw-input",
            Text::MouseRateWarning => "mouse-rate-warning",
            Text::SwitchToPolling => "switch-to-polling",
            Text::EditShortcuts => "edit-shortcuts",
            Text::MenuButtonWindowTitle => "menu-button-window-title",
            Text::Font => "font",
            Text::DefaultFont => "default-font",
            Text::SearchFont => "search-font",
            Text::TabSound => "tab-sound",
            Text::AudioOutput => "audio-output",
            Text::DefaultOutput => "default-output",
            Text::OutputMissing => "output-missing",
            Text::Volume => "volume",
            Text::NoAudio => "no-audio",
            Text::OutputUnavailable => "output-unavailable",
            Text::OutputRefused => "output-refused",
            Text::Sounds => "sounds",
            Text::SlotIncrement => "slot-increment",
            Text::SlotDecrement => "slot-decrement",
            Text::SlotResetPrompt => "slot-reset-prompt",
            Text::SlotResetDone => "slot-reset-done",
            Text::SlotResetUndone => "slot-reset-undone",
            Text::Listen => "listen",
            Text::Mute => "mute",
            Text::ImportFile => "import-file",
            Text::WindowsSounds => "windows-sounds",
            Text::Beeps => "beeps",
            Text::ImportedSounds => "imported-sounds",
            Text::BeepLow => "beep-low",
            Text::BeepMid => "beep-mid",
            Text::BeepHigh => "beep-high",
            Text::BeepVeryHigh => "beep-very-high",
            Text::ImportedSound => "imported-sound",
            Text::ImportTitle => "import-title",
            Text::SoundFiles => "sound-files",
            Text::ImportUnreadable => "import-unreadable",
            Text::ImportCopyFailed => "import-copy-failed",
            Text::EmbeddedSounds => "embedded-sounds",
        }
    }

    /// Tous les textes (pour vérifier qu'aucune traduction intégrée n'en oublie).
    #[cfg(test)]
    pub const ALL: [Text; 96] = [
        Text::TabGeneral,
        Text::LanguageLabel,
        Text::ResetPrompt,
        Text::GlobalWindowTitle,
        Text::RoundWindowTitle,
        Text::ConfigWindowTitle,
        Text::TabCounters,
        Text::TabShortcuts,
        Text::Quit,
        Text::GlobalCounter,
        Text::RoundCounter,
        Text::ShowWindow,
        Text::Goal,
        Text::RoundCopiesGlobal,
        Text::Template,
        Text::TemplateHelp,
        Text::FontSize,
        Text::Margin,
        Text::CornerRadius,
        Text::Anchor,
        Text::AnchorLeft,
        Text::AnchorCenter,
        Text::AnchorRight,
        Text::AlwaysOnTop,
        Text::Background,
        Text::TransparentBackground,
        Text::PlainTextColor,
        Text::GoalPendingColor,
        Text::GoalReachedColor,
        Text::AlertColor,
        Text::TargetBoth,
        Text::TargetGlobal,
        Text::TargetRound,
        Text::ActionResetGlobal,
        Text::ActionResetRound,
        Text::ActionConfirmReset,
        Text::ActionUndoReset,
        Text::ShortcutsHelp,
        Text::PressShortcut,
        Text::MainClickNeedsModifier,
        Text::ShortcutConflict,
        Text::RestoreDefaultShortcuts,
        Text::AddShortcut,
        Text::RemoveShortcut,
        Text::ActionAdd1,
        Text::ActionSubtract1,
        Text::ActionAdd5,
        Text::ActionSubtract5,
        Text::MenuWindowTitle,
        Text::Value,
        Text::Apply,
        Text::ResetAction,
        Text::HideWindow,
        Text::OpenSettings,
        Text::MenuHint,
        Text::MouseMethodLabel,
        Text::MouseRawInput,
        Text::MousePolling,
        Text::WheelNeedsRawInput,
        Text::MouseRateWarning,
        Text::SwitchToPolling,
        Text::EditShortcuts,
        Text::MenuButtonWindowTitle,
        Text::Font,
        Text::DefaultFont,
        Text::SearchFont,
        Text::TabSound,
        Text::AudioOutput,
        Text::DefaultOutput,
        Text::OutputMissing,
        Text::Volume,
        Text::NoAudio,
        Text::OutputUnavailable,
        Text::OutputRefused,
        Text::Sounds,
        Text::SlotIncrement,
        Text::SlotDecrement,
        Text::SlotResetPrompt,
        Text::SlotResetDone,
        Text::SlotResetUndone,
        Text::Listen,
        Text::Mute,
        Text::ImportFile,
        Text::WindowsSounds,
        Text::Beeps,
        Text::ImportedSounds,
        Text::BeepLow,
        Text::BeepMid,
        Text::BeepHigh,
        Text::BeepVeryHigh,
        Text::ImportedSound,
        Text::ImportTitle,
        Text::SoundFiles,
        Text::ImportUnreadable,
        Text::ImportCopyFailed,
        Text::EmbeddedSounds,
    ];
}

/// Une langue disponible : son code (nom du fichier, par ex. `fr`) et son nom affiché.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageInfo {
    pub code: String,
    pub name: String,
}

/// La façon d'écrire les nombres dans une langue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NumberFormat {
    /// Séparateur décimal (« , » en français, « . » en anglais).
    pub decimal_separator: String,
    /// Modèle du pourcentage, avec `{}` à la place du nombre (« {} % » en français).
    pub percent: String,
}

/// Un traducteur : une chaîne de traductions, essayées dans l'ordre (la langue voulue, puis
/// les secours).
pub struct Translator {
    code: String,
    chain: Vec<FluentBundle<FluentResource>>,
}

impl Translator {
    /// Le traducteur d'une langue : le pack (s'il existe), puis la langue intégrée de même
    /// code, puis l'anglais.
    pub fn load(code: &str) -> Self {
        let mut sources = Vec::new();
        if let Some(pack) = read_pack(code) {
            sources.push(pack);
        }
        for (builtin_code, content) in BUILTIN {
            if builtin_code == code {
                sources.push(content.to_string());
            }
        }
        if code != FALLBACK {
            sources.push(builtin(FALLBACK).to_string());
        }
        Self::from_sources(code, sources)
    }

    /// Un traducteur à partir de textes `.ftl`, dans l'ordre où les essayer.
    fn from_sources(code: &str, sources: Vec<String>) -> Self {
        // `parse` : « fr » → identifiant de langue. Code invalide : on prend l'anglais (il ne
        // sert qu'aux règles propres à la langue, comme les pluriels).
        let language: unic_langid::LanguageIdentifier = code
            .parse()
            .unwrap_or_else(|_| FALLBACK.parse().expect("code valide"));
        let chain = sources
            .into_iter()
            .map(|source| {
                // Un fichier avec des erreurs est quand même utilisé : les lignes valides comptent.
                let resource =
                    FluentResource::try_new(source).unwrap_or_else(|(partial, errors)| {
                        eprintln!(
                            "Traduction « {code} » : {} ligne(s) illisible(s).",
                            errors.len()
                        );
                        partial
                    });
                let mut bundle = FluentBundle::new_concurrent(vec![language.clone()]);
                // Sans ça, Fluent entoure chaque emplacement de caractères invisibles (pour
                // l'écriture de droite à gauche), que la police affiche comme des carrés.
                bundle.set_use_isolating(false);
                // `let _ =` : un identifiant en double dans un pack n'empêche pas le reste.
                let _ = bundle.add_resource(resource);
                bundle
            })
            .collect();
        Self {
            code: code.to_string(),
            chain,
        }
    }

    /// Le code de cette langue (par ex. `fr`).
    pub fn code(&self) -> &str {
        &self.code
    }

    /// Le texte `id`, avec ses emplacements remplis : le premier de la chaîne qui le connaît.
    fn get(&self, id: &str, args: Option<&FluentArgs>) -> Option<String> {
        self.chain.iter().find_map(|bundle| {
            let pattern = bundle.get_message(id)?.value()?;
            let mut errors = Vec::new();
            Some(
                bundle
                    .format_pattern(pattern, args, &mut errors)
                    .into_owned(),
            )
        })
    }

    /// Un message sans emplacement.
    pub fn text(&self, text: Text) -> String {
        self.message(text, &[])
    }

    /// Le message, avec ses emplacements remplis.
    /// `values` : des paires (nom de l'emplacement, valeur), par ex. `[("key", "Num Entrée")]`.
    pub fn message(&self, text: Text, values: &[(&str, &str)]) -> String {
        let mut args = FluentArgs::new();
        for (name, value) in values {
            args.set(*name, value.to_string());
        }
        // Texte introuvable partout (impossible avec l'anglais intégré) : son identifiant.
        self.get(text.id(), Some(&args))
            .unwrap_or_else(|| text.id().to_string())
    }

    /// Le nom d'une touche.
    pub fn key_name(&self, key: Key) -> String {
        // Noms identiques dans toutes les langues : restent dans le code.
        let fixed = match key {
            Key::CONTROL => Some("Ctrl"),
            Key::ALT => Some("Alt"),
            Key::NUMPAD_ADD => Some("Num +"),
            Key::NUMPAD_SUBTRACT => Some("Num -"),
            Key::NUMPAD_DECIMAL => Some("Num ."),
            _ => None,
        };
        if let Some(name) = fixed {
            return name.to_string();
        }
        let translated = match key {
            Key::SHIFT => Some("key-shift"),
            Key::ENTER => Some("key-enter"),
            Key::NUMPAD_ENTER => Some("key-numpad-enter"),
            Key::MOUSE_LEFT => Some("key-mouse-left"),
            Key::MOUSE_RIGHT => Some("key-mouse-right"),
            Key::MOUSE_MIDDLE => Some("key-mouse-middle"),
            Key::MOUSE_X1 => Some("key-mouse-x1"),
            Key::MOUSE_X2 => Some("key-mouse-x2"),
            Key::WHEEL_UP => Some("key-wheel-up"),
            Key::WHEEL_DOWN => Some("key-wheel-down"),
            Key::WHEEL_LEFT => Some("key-wheel-left"),
            Key::WHEEL_RIGHT => Some("key-wheel-right"),
            _ => None,
        };
        if let Some(name) = translated.and_then(|id| self.get(id, None)) {
            return name;
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
            code => {
                let mut args = FluentArgs::new();
                args.set("code", format!("0x{code:02X}"));
                self.get("key-unknown", Some(&args))
                    .unwrap_or_else(|| format!("0x{code:02X}"))
            }
        }
    }

    /// Le nom d'un raccourci complet, par ex. « Ctrl + Num 0 ».
    pub fn shortcut_name(&self, shortcut: &Shortcut) -> String {
        // On met bout à bout les touches à tenir puis la touche déclencheuse, séparées par « + ».
        shortcut
            .held
            .iter()
            .chain(std::iter::once(&shortcut.trigger))
            .map(|&key| self.key_name(key))
            .collect::<Vec<_>>()
            .join(" + ")
    }

    /// Le nom d'une ligne du tableau des raccourcis (« +1 », « Reset du global »…).
    pub fn command_kind_name(&self, kind: CommandKind) -> String {
        self.text(match kind {
            CommandKind::Add1 => Text::ActionAdd1,
            CommandKind::Subtract1 => Text::ActionSubtract1,
            CommandKind::Add5 => Text::ActionAdd5,
            CommandKind::Subtract5 => Text::ActionSubtract5,
            CommandKind::ResetRound => Text::ActionResetRound,
            CommandKind::ResetGlobal => Text::ActionResetGlobal,
            CommandKind::ConfirmReset => Text::ActionConfirmReset,
            CommandKind::UndoReset => Text::ActionUndoReset,
        })
    }

    /// Le nom des compteurs visés (« global et partie », « global », « partie »).
    pub fn targets_name(&self, targets: Targets) -> String {
        self.text(match (targets.global, targets.round) {
            (true, true) => Text::TargetBoth,
            (true, false) => Text::TargetGlobal,
            // `_` : les autres cas (partie seule ; « aucun » n'existe pas en pratique).
            _ => Text::TargetRound,
        })
    }

    /// La façon d'écrire les nombres dans cette langue.
    pub fn number_format(&self) -> NumberFormat {
        let mut args = FluentArgs::new();
        args.set("percent", "{}");
        NumberFormat {
            decimal_separator: self
                .get("number-decimal-separator", None)
                .unwrap_or_else(|| ".".into()),
            percent: self
                .get("number-percent", Some(&args))
                .unwrap_or_else(|| "{}%".into()),
        }
    }

    /// Le nom de cette langue, tel qu'écrit dans sa traduction (« Français »).
    fn language_name(&self) -> String {
        self.get("language-name", None)
            .unwrap_or_else(|| self.code.clone())
    }
}

/// Une traduction intégrée, par son code.
fn builtin(code: &str) -> &'static str {
    BUILTIN
        .iter()
        .find(|(c, _)| *c == code)
        .map_or("", |(_, content)| content)
}

/// Le dossier des packs : `lang/` à côté de l'exe.
fn packs_dir() -> Option<PathBuf> {
    Some(std::env::current_exe().ok()?.parent()?.join("lang"))
}

/// Le contenu du pack d'une langue, s'il existe.
fn read_pack(code: &str) -> Option<String> {
    std::fs::read_to_string(packs_dir()?.join(format!("{code}.ftl"))).ok()
}

/// Les langues disponibles : intégrées, puis packs du dossier `lang/` (un pack du même code
/// qu'une langue intégrée la remplace). Triées par nom.
pub fn available_languages() -> Vec<LanguageInfo> {
    let mut codes: Vec<String> = BUILTIN.iter().map(|(code, _)| code.to_string()).collect();
    if let Some(entries) = packs_dir().and_then(|dir| std::fs::read_dir(dir).ok()) {
        for entry in entries.flatten() {
            let path = entry.path();
            let is_pack = path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("ftl"));
            if let (true, Some(stem)) = (is_pack, path.file_stem()) {
                let code = stem.to_string_lossy().to_lowercase();
                if !codes.contains(&code) {
                    codes.push(code);
                }
            }
        }
    }
    let mut languages: Vec<LanguageInfo> = codes
        .into_iter()
        .map(|code| LanguageInfo {
            name: Translator::load(&code).language_name(),
            code,
        })
        .collect();
    languages.sort_by_key(|language| language.name.to_lowercase());
    languages
}

/// La langue à utiliser au premier lancement : celle du système (« fr-FR » → « fr ») si elle
/// est disponible, sinon l'anglais.
pub fn pick_language(system: &str, available: &[String]) -> String {
    // `split(['-', '_'])` : coupe sur « - » ou « _ » ; on garde la première partie.
    let code = system
        .split(['-', '_'])
        .next()
        .unwrap_or_default()
        .to_lowercase();
    if available.contains(&code) {
        code
    } else {
        FALLBACK.to_string()
    }
}

/// La langue de l'interface de Windows, par ex. « fr-FR » (vide si inconnue).
pub fn system_language() -> String {
    #[cfg(windows)]
    {
        use windows_sys::Win32::Globalization::{GetUserDefaultUILanguage, LCIDToLocaleName};
        let mut buffer = [0u16; 85];
        // SÉCURITÉ : simple lecture ; `buffer` peut recevoir 85 caractères, la taille indiquée.
        let length = unsafe {
            LCIDToLocaleName(
                u32::from(GetUserDefaultUILanguage()),
                buffer.as_mut_ptr(),
                buffer.len() as i32,
                0,
            )
        };
        // `length` compte le 0 final.
        let length = (length.max(1) - 1) as usize;
        String::from_utf16_lossy(&buffer[..length])
    }
    #[cfg(not(windows))]
    String::new()
}

// La langue courante, partagée par toute l'appli : une variable GLOBALE (`static`).
// - `RwLock` : un verrou « plusieurs lecteurs OU un seul écrivain » ; on lit à chaque texte
//   affiché, on écrit seulement quand l'utilisateur change de langue.
// - `Arc` : la traduction est partagée ; un lecteur garde la sienne même si on en change.
// - `Option` : `None` tant qu'aucune langue n'est choisie (on prend alors l'anglais).
static CURRENT: RwLock<Option<Arc<Translator>>> = RwLock::new(None);

/// Change la langue de l'interface (effet immédiat).
pub fn set_language(code: &str) {
    let translator = Arc::new(Translator::load(code));
    // `write()` : prend le verrou en écriture ; `unwrap_or_else(|e| e.into_inner())` : même si
    // un autre thread a planté en le tenant, on continue (la donnée reste valide).
    *CURRENT.write().unwrap_or_else(|e| e.into_inner()) = Some(translator);
}

/// La traduction courante (l'anglais si aucune langue n'a été choisie).
fn current() -> Arc<Translator> {
    let existing = CURRENT.read().unwrap_or_else(|e| e.into_inner()).clone();
    existing.unwrap_or_else(|| {
        set_language(FALLBACK);
        current()
    })
}

/// Le code de la langue courante.
pub fn current_code() -> String {
    current().code().to_string()
}

/// Un message sans emplacement, dans la langue courante.
pub fn text(text: Text) -> String {
    current().text(text)
}

/// Un message avec ses emplacements remplis, dans la langue courante.
pub fn message(text: Text, values: &[(&str, &str)]) -> String {
    current().message(text, values)
}

/// Le nom d'un raccourci, dans la langue courante.
pub fn shortcut_name(shortcut: &Shortcut) -> String {
    current().shortcut_name(shortcut)
}

/// Le nom d'une ligne du tableau des raccourcis, dans la langue courante.
pub fn command_kind_name(kind: CommandKind) -> String {
    current().command_kind_name(kind)
}

/// Le nom des compteurs visés, dans la langue courante.
pub fn targets_name(targets: Targets) -> String {
    current().targets_name(targets)
}

/// La façon d'écrire les nombres, dans la langue courante.
pub fn number_format() -> NumberFormat {
    current().number_format()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fr() -> Translator {
        Translator::load("fr")
    }

    fn en() -> Translator {
        Translator::load("en")
    }

    #[test]
    fn every_text_is_translated_in_builtin_languages() {
        // Chaque texte doit exister dans CHAQUE fichier intégré (pas seulement via l'anglais).
        for (code, content) in BUILTIN {
            let alone = Translator::from_sources(code, vec![content.to_string()]);
            for text in Text::ALL {
                assert!(
                    alone.get(text.id(), None).is_some(),
                    "« {} » manque dans {code}.ftl",
                    text.id()
                );
            }
        }
    }

    #[test]
    fn builtin_files_have_no_syntax_error() {
        for (code, content) in BUILTIN {
            assert!(
                FluentResource::try_new(content.to_string()).is_ok(),
                "{code}.ftl"
            );
        }
    }

    #[test]
    fn missing_text_in_a_pack_falls_back_to_english() {
        let pack = "language-name = Español\ntab-counters = Contadores\n".to_string();
        let spanish = Translator::from_sources("es", vec![pack, builtin("en").to_string()]);
        assert_eq!(spanish.text(Text::TabCounters), "Contadores");
        assert_eq!(spanish.text(Text::Quit), "Quit"); // absent du pack : anglais
        assert_eq!(spanish.language_name(), "Español");
    }

    #[test]
    fn literal_braces_stay_in_the_format_help() {
        // L'aide du format contient de vraies accolades, qui ne sont pas des emplacements.
        assert!(
            fr().text(Text::TemplateHelp)
                .starts_with("Variables : {value}, {goal}")
        );
    }

    #[test]
    fn no_invisible_characters_around_placeholders() {
        let text = fr().message(Text::ResetPrompt, &[("key", "F5")]);
        assert!(!text.contains('\u{2068}') && !text.contains('\u{2069}'));
    }

    #[test]
    fn system_language_picks_an_available_one() {
        let available = vec!["en".to_string(), "fr".to_string()];
        assert_eq!(pick_language("fr-FR", &available), "fr");
        assert_eq!(pick_language("en_US", &available), "en");
        assert_eq!(pick_language("de-DE", &available), "en"); // indisponible : anglais
        assert_eq!(pick_language("", &available), "en");
    }

    #[test]
    fn number_formats() {
        assert_eq!(fr().number_format().decimal_separator, ",");
        assert_eq!(fr().number_format().percent, "{} %");
        assert_eq!(en().number_format().percent, "{}%");
    }
    use crate::command::Command;
    use crate::tracker::CounterId;

    #[test]
    fn reset_prompt_uses_the_configured_key() {
        assert_eq!(
            fr().message(Text::ResetPrompt, &[("key", "Num Entrée")]),
            "Reset ? (Num Entrée pour valider)"
        );
        // Une autre touche configurée : le message suit, sans toucher à la traduction.
        assert_eq!(
            en().message(Text::ResetPrompt, &[("key", "F5")]),
            "Reset? (F5 to confirm)"
        );
    }

    #[test]
    fn key_names_are_translated() {
        assert_eq!(fr().key_name(Key::NUMPAD_ENTER), "Num Entrée");
        assert_eq!(en().key_name(Key::NUMPAD_ENTER), "Num Enter");
        assert_eq!(fr().key_name(Key::SHIFT), "Maj");
        assert_eq!(en().key_name(Key::SHIFT), "Shift");
    }

    #[test]
    fn mouse_button_names() {
        assert_eq!(fr().key_name(Key::MOUSE_MIDDLE), "Clic molette");
        assert_eq!(en().key_name(Key::MOUSE_MIDDLE), "Middle click");
        let shortcut = Shortcut {
            held: vec![Key::CONTROL],
            trigger: Key::MOUSE_MIDDLE,
            command: Command::Reset(CounterId::Global),
        };
        assert_eq!(fr().shortcut_name(&shortcut), "Ctrl + Clic molette");
    }

    #[test]
    fn wheel_names() {
        assert_eq!(fr().key_name(Key::WHEEL_UP), "Molette haut");
        assert_eq!(en().key_name(Key::WHEEL_DOWN), "Wheel down");
        let shortcut = Shortcut {
            held: vec![Key::CONTROL],
            trigger: Key::WHEEL_UP,
            command: Command::UndoReset,
        };
        assert_eq!(fr().shortcut_name(&shortcut), "Ctrl + Molette haut");
    }

    #[test]
    fn computed_key_names() {
        assert_eq!(fr().key_name(Key::NUMPAD_0), "Num 0");
        assert_eq!(fr().key_name(Key(0x69)), "Num 9");
        assert_eq!(fr().key_name(Key(0x41)), "A");
        assert_eq!(fr().key_name(Key(0x35)), "5");
        assert_eq!(fr().key_name(Key(0x74)), "F5");
        assert_eq!(fr().key_name(Key(0xE9)), "Touche 0xE9");
        assert_eq!(en().key_name(Key(0xE9)), "Key 0xE9");
    }

    #[test]
    fn shortcut_names() {
        let reset = Shortcut {
            held: vec![Key::CONTROL],
            trigger: Key::NUMPAD_0,
            command: Command::Reset(CounterId::Global),
        };
        assert_eq!(fr().shortcut_name(&reset), "Ctrl + Num 0");

        let confirm = Shortcut {
            held: vec![],
            trigger: Key::NUMPAD_ENTER,
            command: Command::ConfirmReset,
        };
        assert_eq!(fr().shortcut_name(&confirm), "Num Entrée");
    }
}
