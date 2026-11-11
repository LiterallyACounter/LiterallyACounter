//! Sauvegarde de l'état dans un fichier JSON : la valeur du compteur global, les styles
//! d'affichage, l'option de copie et les fenêtres ouvertes.
//!
//! - Emplacement : le dossier de configuration standard du système, fourni par la crate `dirs`
//!   (Windows : `%APPDATA%\LiterallyACounter\state.json`).
//! - Écriture « atomique » : on écrit d'abord un fichier temporaire, puis on le RENOMME à la place
//!   de l'ancien. Le renommage est instantané : en cas de coupure pendant l'écriture, l'ancien
//!   fichier reste intact.
//! - Fichier illisible (abîmé, modifié à la main de travers) : on le met de côté
//!   (`state.json.corrupt`) au lieu de l'écraser, pour ne pas perdre la valeur.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

// `Serialize` / `Deserialize` : traits de `serde`. Avec `#[derive(...)]`, serde génère tout seul
// le code qui convertit la struct en JSON et inversement.
use serde::{Deserialize, Serialize};

use crate::display::DisplayStyle;
use crate::hotkeys::{self, MouseMethod, Shortcut};

const APP_FOLDER: &str = "LiterallyACounter";
const STATE_FILE: &str = "state.json";

/// Ce qui est sauvegardé.
///
/// `#[serde(default)]` : si un champ manque dans le fichier (par ex. un fichier écrit par une
/// version plus ancienne de l'appli, comme `{"global_value": 14}` de l'étape 5), on prend
/// sa valeur par défaut au lieu d'échouer.
///
/// La valeur de la PARTIE n'est jamais sauvegardée (elle repart de 0 au lancement).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SavedState {
    /// Valeur du compteur global.
    pub global_value: u64,
    /// Objectifs (0 = pas d'objectif). Celui de la partie est sauvegardé, pas sa valeur.
    pub global_goal: u64,
    pub round_goal: u64,
    /// Affichage du global.
    pub global_style: DisplayStyle,
    /// Affichage propre de la partie : utilisé seulement quand la copie est désactivée,
    /// mais conservé dans tous les cas.
    pub round_style: DisplayStyle,
    /// « Copier l'affichage du global » pour la partie.
    pub round_copies_global: bool,
    /// Fenêtres ouvertes (leur position n'est pas sauvegardée).
    pub show_global: bool,
    pub show_round: bool,
    /// Les raccourcis. Les touches sont écrites avec leur code Windows (par ex. 107 = Num +).
    pub shortcuts: Vec<Shortcut>,
    /// Méthode d'écoute de la souris (Raw Input par défaut).
    pub mouse_method: MouseMethod,
    /// Sortie audio choisie, par son identifiant (`None` : celle par défaut de Windows).
    pub audio_output: Option<String>,
    /// Volume général, de 0.0 à 1.0.
    pub volume: f32,
    /// Langue de l'interface, par son code (`None` : pas encore choisie, celle de Windows).
    pub language: Option<String>,
    /// « La partie utilise les sons du global » (`None` : réglage d'avant l'étape 11, où la
    /// copie de l'affichage copiait aussi les sons : on reprend alors `round_copies_global`).
    pub round_copies_global_sounds: Option<bool>,
    /// Sections repliées / dépliées de la configuration (seules celles modifiées y figurent).
    pub open_sections: BTreeMap<String, bool>,
}

// Valeurs par défaut écrites à la main : les booléens doivent valoir `true` (le `derive`
// automatique mettrait `false`).
impl Default for SavedState {
    fn default() -> Self {
        Self {
            global_value: 0,
            global_goal: 0,
            round_goal: 0,
            global_style: DisplayStyle::default(),
            round_style: DisplayStyle::round_default(),
            round_copies_global: true,
            show_global: true,
            show_round: true,
            shortcuts: hotkeys::default_shortcuts(),
            mouse_method: MouseMethod::RawInput,
            audio_output: None,
            volume: 1.0,
            language: None,
            round_copies_global_sounds: None,
            open_sections: BTreeMap::new(),
        }
    }
}

/// Le chemin du fichier de sauvegarde, ou `None` si le système ne fournit pas de dossier
/// de configuration (très rare).
pub fn state_path() -> Option<PathBuf> {
    Some(dirs::config_dir()?.join(APP_FOLDER).join(STATE_FILE))
}

/// Lit l'état sauvegardé. Fichier absent → état par défaut (premier lancement).
/// Fichier illisible → mis de côté, et état par défaut.
pub fn load(path: &Path) -> SavedState {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        // Premier lancement : pas encore de fichier, ce n'est pas une erreur.
        Err(error) if error.kind() == io::ErrorKind::NotFound => return SavedState::default(),
        Err(error) => {
            eprintln!("Lecture impossible de {} : {error}", path.display());
            return SavedState::default();
        }
    };

    // `serde_json::from_str` : du texte JSON vers notre struct. Le type voulu est indiqué
    // par l'annotation `Result<SavedState, _>` (`_` : « devine le type de l'erreur »).
    // Certains éditeurs (Bloc-notes, PowerShell…) ajoutent au début du fichier une marque
    // invisible, le « BOM » UTF-8 (caractère U+FEFF), que le lecteur JSON refuse : on l'ignore.
    // `strip_prefix` : le texte sans ce préfixe, ou `None` s'il n'y est pas.
    let content = content.strip_prefix('\u{feff}').unwrap_or(&content);
    let parsed: Result<SavedState, _> = serde_json::from_str(content);
    match parsed {
        Ok(state) => state,
        Err(error) => {
            eprintln!("Sauvegarde illisible ({error}) : mise de côté.");
            let _ = fs::rename(path, path.with_extension("json.corrupt"));
            SavedState::default()
        }
    }
}

/// Écrit l'état, de façon atomique.
/// `io::Result<()>` : rien en cas de succès (`()`), une erreur d'entrée/sortie sinon.
pub fn save(path: &Path, state: &SavedState) -> io::Result<()> {
    // Crée le dossier s'il n'existe pas encore (premier lancement).
    if let Some(folder) = path.parent() {
        fs::create_dir_all(folder)?;
    }
    // `to_string_pretty` : JSON indenté, lisible à la main.
    // `map_err` : convertit l'erreur de serde en erreur d'entrée/sortie, pour que `?` fonctionne.
    let json = serde_json::to_string_pretty(state).map_err(io::Error::other)?;
    let temporary = path.with_extension("json.tmp");
    fs::write(&temporary, json)?;
    // Remplace l'ancien fichier par le nouveau, en une seule opération.
    fs::rename(&temporary, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Un dossier de test unique et vide, dans le dossier temporaire du système.
    fn test_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("lac-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn missing_file_gives_default_state() {
        let path = test_dir("missing").join("state.json");
        assert_eq!(load(&path), SavedState::default());
    }

    #[test]
    fn save_then_load_gives_the_same_state() {
        let dir = test_dir("roundtrip");
        let path = dir.join("state.json");
        let state = SavedState {
            global_value: 152,
            round_copies_global: false,
            show_round: false,
            ..SavedState::default()
        };
        save(&path, &state).expect("la sauvegarde doit réussir");
        assert_eq!(load(&path), state);
        // Le fichier temporaire ne doit pas rester.
        assert!(!dir.join("state.json.tmp").exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_overwrites_previous_state() {
        let dir = test_dir("overwrite");
        let path = dir.join("state.json");
        let state = |global_value| SavedState {
            global_value,
            ..SavedState::default()
        };
        save(&path, &state(1)).unwrap();
        save(&path, &state(2)).unwrap();
        assert_eq!(load(&path).global_value, 2);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn corrupt_file_is_set_aside_not_lost() {
        let dir = test_dir("corrupt");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("state.json");
        fs::write(&path, "{ ceci n'est pas du JSON").unwrap();
        assert_eq!(load(&path), SavedState::default());
        assert!(dir.join("state.json.corrupt").exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_field_takes_its_default() {
        let dir = test_dir("oldformat");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("state.json");
        fs::write(&path, "{}").unwrap();
        assert_eq!(load(&path).global_value, 0);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn step_5_save_is_read_without_loss() {
        // Le fichier tel qu'écrit par l'étape 5 : seule la valeur du global.
        let dir = test_dir("step5");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("state.json");
        fs::write(&path, "{\"global_value\": 14}").unwrap();
        let state = load(&path);
        assert_eq!(state.global_value, 14);
        // Le reste prend les valeurs par défaut : copie activée, deux fenêtres ouvertes.
        assert!(state.round_copies_global);
        assert!(state.show_global && state.show_round);
        // Pas de raccourcis dans l'ancien fichier : ceux par défaut.
        assert_eq!(state.shortcuts, hotkeys::default_shortcuts());
        assert_eq!(state.mouse_method, MouseMethod::RawInput);
        assert_eq!(state.audio_output, None);
        assert_eq!(state.volume, 1.0);
        assert_eq!(state.language, None);
        assert_eq!(state.round_copies_global_sounds, None);
        assert!(state.open_sections.is_empty());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn file_with_bom_is_read() {
        // Fichier enregistré « UTF-8 avec BOM » par un éditeur.
        let dir = test_dir("bom");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("state.json");
        fs::write(&path, "\u{feff}{\"global_value\": 14}").unwrap();
        assert_eq!(load(&path).global_value, 14);
        assert!(!dir.join("state.json.corrupt").exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn json_is_human_readable() {
        let json = serde_json::to_string_pretty(&SavedState::default()).unwrap();
        assert!(json.starts_with(
            "{
  \"global_value\": 0,
  \"global_goal\": 0,
  \"round_goal\": 0,
  \"global_style\": {"
        ));
        assert!(json.contains("\"background\": \"#1E1E1EFF\""));
    }
}
