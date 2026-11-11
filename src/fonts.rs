//! Les polices installées sur l'ordinateur (spec F3).
//!
//! - La LISTE vient du registre de Windows, qui associe chaque nom de police à son fichier
//!   (clé `...\Windows NT\CurrentVersion\Fonts`, pour tout l'ordinateur et pour l'utilisateur).
//! - Une police n'est CHARGÉE (fichier lu, donnée à egui) qu'au moment où on en a besoin :
//!   celles des compteurs, plus quelques-unes pour l'aperçu au survol. Charger les centaines
//!   de polices installées coûterait des centaines de Mo.
//! - Une police introuvable (désinstallée) : on revient à la police par défaut, sans planter.

use std::collections::{BTreeMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use eframe::egui::{self, FontData, FontDefinitions, FontFamily, FontId};

/// Nombre maximal de polices gardées en mémoire pour l'aperçu au survol.
const PREVIEW_CACHE: usize = 8;

/// Une police installée.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemFont {
    /// Nom affiché, par ex. « Arial Black ».
    pub name: String,
    pub path: PathBuf,
    /// Numéro de la police dans le fichier : un fichier `.ttc` (« collection ») en contient
    /// plusieurs, par ex. « Cambria & Cambria Math ».
    pub index: u32,
}

/// Les polices décrites par UNE entrée du registre.
/// - `value_name` : par ex. « Cambria & Cambria Math (TrueType) » ;
/// - `file` : par ex. « cambria.ttc » (relatif au dossier des polices) ou un chemin complet
///   (polices installées pour l'utilisateur seul).
///
/// Seuls les formats lisibles par egui sont gardés (`.ttf`, `.otf`, `.ttc`) : les vieilles
/// polices « bitmap » (`.fon`) sont ignorées.
pub fn parse_entry(value_name: &str, file: &str, fonts_dir: &Path) -> Vec<SystemFont> {
    let path = Path::new(file);
    // `and_then` : enchaîne une opération qui peut échouer (pas d'extension → `None`).
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase());
    let collection = match extension.as_deref() {
        Some("ttc") => true,
        Some("ttf" | "otf") => false,
        _ => return Vec::new(),
    };
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        fonts_dir.join(path)
    };
    // On retire le suffixe « (TrueType) » / « (OpenType) ».
    let names = value_name
        .trim_end_matches(" (TrueType)")
        .trim_end_matches(" (OpenType)");
    names
        .split(" & ")
        .enumerate()
        .map(|(i, name)| SystemFont {
            name: name.trim().to_string(),
            path: path.clone(),
            index: if collection { i as u32 } else { 0 },
        })
        .filter(|font| !font.name.is_empty())
        .collect()
}

/// Trie par nom (sans tenir compte des majuscules) et retire les doublons de nom.
pub fn sort_and_dedup(fonts: &mut Vec<SystemFont>) {
    fonts.sort_by_key(|font| font.name.to_lowercase());
    // `dedup_by` : retire un élément s'il « égale » le précédent selon notre règle.
    fonts.dedup_by(|a, b| a.name.eq_ignore_ascii_case(&b.name));
}

/// La liste des polices installées, triée par nom.
pub fn list_system_fonts() -> Vec<SystemFont> {
    #[cfg(windows)]
    let mut fonts = windows::registry_fonts();
    #[cfg(not(windows))]
    let mut fonts = Vec::new();
    sort_and_dedup(&mut fonts);
    fonts
}

/// La famille egui d'une police chargée (un nom à nous, propre à chaque police).
fn family(name: &str) -> FontFamily {
    FontFamily::Name(format!("system:{name}").into())
}

/// Les polices chargées dans egui, à la demande.
#[derive(Default)]
pub struct FontLibrary {
    /// La liste des polices installées (lue au premier besoin, puis gardée).
    system: Option<Vec<SystemFont>>,
    /// Les polices chargées (nom → données du fichier).
    loaded: BTreeMap<String, Arc<FontData>>,
    /// Les polices introuvables ou illisibles : on ne réessaie pas à chaque image.
    failed: HashSet<String>,
    /// Les polices chargées pour l'aperçu, de la plus ancienne à la plus récente.
    previews: VecDeque<String>,
}

impl FontLibrary {
    /// La liste des polices installées.
    pub fn system_fonts(&mut self) -> &[SystemFont] {
        // `get_or_insert_with` : la valeur si elle existe, sinon on la calcule (une fois).
        self.system.get_or_insert_with(list_system_fonts)
    }

    /// La police (taille + famille) à utiliser pour dessiner. Police inconnue, pas encore
    /// chargée, ou pas encore active dans egui : police par défaut.
    pub fn font_id(&self, ctx: &egui::Context, name: Option<&str>, size: f32) -> FontId {
        if let Some(name) = name
            && self.loaded.contains_key(name)
        {
            let family = family(name);
            // Une police ajoutée ne devient active qu'à l'image suivante : on vérifie qu'egui
            // la connaît déjà (sinon egui s'arrêterait sur une famille inconnue).
            if ctx.fonts(|fonts| fonts.definitions().families.contains_key(&family)) {
                return FontId::new(size, family);
            }
        }
        FontId::proportional(size)
    }

    /// Demande le chargement d'une police utilisée par un compteur (gardée en mémoire).
    pub fn require(&mut self, ctx: &egui::Context, name: &str) {
        if self.load(name) {
            self.apply(ctx);
        }
    }

    /// Demande le chargement d'une police pour l'aperçu. Les plus anciennes sont oubliées
    /// au-delà de `PREVIEW_CACHE`, sauf celles utilisées par un compteur (`in_use`).
    pub fn preview(&mut self, ctx: &egui::Context, name: &str, in_use: &[&str]) {
        if self.loaded.contains_key(name) || self.failed.contains(name) {
            return;
        }
        if !self.load(name) {
            return;
        }
        self.previews.push_back(name.to_string());
        while self.previews.len() > PREVIEW_CACHE {
            if let Some(old) = self.previews.pop_front()
                && !in_use.contains(&old.as_str())
            {
                self.loaded.remove(&old);
            }
        }
        self.apply(ctx);
    }

    /// Lit le fichier d'une police. Renvoie `true` si une police a été ajoutée.
    fn load(&mut self, name: &str) -> bool {
        if self.loaded.contains_key(name) || self.failed.contains(name) {
            return false;
        }
        let font = self
            .system_fonts()
            .iter()
            .find(|font| font.name == name)
            .cloned();
        let Some(font) = font else {
            self.failed.insert(name.to_string());
            return false;
        };
        match std::fs::read(&font.path) {
            Ok(bytes) => {
                let mut data = FontData::from_owned(bytes);
                data.index = font.index;
                self.loaded.insert(name.to_string(), Arc::new(data));
                true
            }
            Err(error) => {
                eprintln!("Police illisible ({}) : {error}", font.path.display());
                self.failed.insert(name.to_string());
                false
            }
        }
    }

    /// Donne à egui l'ensemble des polices chargées. Chaque police a sa propre famille,
    /// complétée par les polices par défaut : un caractère absent de la police choisie
    /// (symbole, emoji…) s'affiche quand même.
    fn apply(&self, ctx: &egui::Context) {
        let mut definitions = FontDefinitions::default();
        let defaults = definitions
            .families
            .get(&FontFamily::Proportional)
            .cloned()
            .unwrap_or_default();
        for (name, data) in &self.loaded {
            let key = format!("system:{name}");
            definitions.font_data.insert(key.clone(), data.clone());
            let mut chain = vec![key];
            chain.extend(defaults.iter().cloned());
            definitions.families.insert(family(name), chain);
        }
        ctx.set_fonts(definitions);
    }
}

#[cfg(windows)]
mod windows {
    use std::path::PathBuf;
    use std::ptr::null_mut;

    use windows_sys::Win32::System::Registry::{
        HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, REG_SZ, RegCloseKey, RegEnumValueW,
        RegOpenKeyExW,
    };

    use super::{SystemFont, parse_entry};

    const FONTS_KEY: &str = "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Fonts";

    /// Les polices déclarées dans le registre : pour tout l'ordinateur, puis pour l'utilisateur.
    pub fn registry_fonts() -> Vec<SystemFont> {
        // Le dossier des polices de Windows (en général `C:\Windows\Fonts`).
        let windows_dir = std::env::var_os("WINDIR").unwrap_or_else(|| "C:\\Windows".into());
        let fonts_dir = PathBuf::from(windows_dir).join("Fonts");
        let mut fonts = Vec::new();
        for root in [HKEY_LOCAL_MACHINE, HKEY_CURRENT_USER] {
            for (name, file) in read_values(root) {
                fonts.extend(parse_entry(&name, &file, &fonts_dir));
            }
        }
        fonts
    }

    /// Les paires (nom, texte) d'une clé du registre.
    fn read_values(root: HKEY) -> Vec<(String, String)> {
        let key_name: Vec<u16> = FONTS_KEY.encode_utf16().chain(std::iter::once(0)).collect();
        let mut key: HKEY = null_mut();
        // SÉCURITÉ : `key_name` est un texte UTF-16 terminé par 0 ; `key` reçoit la clé ouverte.
        if unsafe { RegOpenKeyExW(root, key_name.as_ptr(), 0, KEY_READ, &mut key) } != 0 {
            return Vec::new();
        }
        let mut values = Vec::new();
        let mut name = [0u16; 512];
        let mut data = [0u16; 1024];
        // `0..` : 0, 1, 2… sans fin ; on sort avec `break` quand il n'y a plus de valeur.
        for index in 0.. {
            let mut name_len = name.len() as u32;
            let mut data_len = (data.len() * 2) as u32; // en octets
            let mut kind = 0;
            // SÉCURITÉ : les tampons et leurs tailles sont valides pendant l'appel.
            let status = unsafe {
                RegEnumValueW(
                    key,
                    index,
                    name.as_mut_ptr(),
                    &mut name_len,
                    null_mut(),
                    &mut kind,
                    data.as_mut_ptr().cast(),
                    &mut data_len,
                )
            };
            if status != 0 {
                break; // plus de valeur (ou erreur) : fin de la liste
            }
            if kind != REG_SZ {
                continue;
            }
            let name = String::from_utf16_lossy(&name[..name_len as usize]);
            // `data_len` est en octets, et le texte se termine souvent par un 0 à retirer.
            let chars = (data_len as usize / 2).min(data.len());
            let file = String::from_utf16_lossy(&data[..chars]);
            values.push((name, file.trim_end_matches('\0').to_string()));
        }
        // SÉCURITÉ : on ferme la clé ouverte plus haut.
        unsafe { RegCloseKey(key) };
        values
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir() -> PathBuf {
        PathBuf::from("C:\\Windows\\Fonts")
    }

    #[test]
    fn simple_font() {
        let fonts = parse_entry("Arial Black (TrueType)", "ariblk.ttf", &dir());
        assert_eq!(
            fonts,
            vec![SystemFont {
                name: "Arial Black".into(),
                path: dir().join("ariblk.ttf"),
                index: 0,
            }]
        );
    }

    #[test]
    fn collection_gives_one_font_per_name() {
        let fonts = parse_entry("Cambria & Cambria Math (TrueType)", "cambria.ttc", &dir());
        assert_eq!(fonts.len(), 2);
        assert_eq!(fonts[1].name, "Cambria Math");
        assert_eq!(fonts[1].index, 1);
    }

    #[test]
    fn user_font_with_full_path() {
        let path = "C:\\Users\\X\\AppData\\Local\\Microsoft\\Windows\\Fonts\\Mine.otf";
        let fonts = parse_entry("Mine (OpenType)", path, &dir());
        assert_eq!(fonts[0].path, PathBuf::from(path));
    }

    #[test]
    fn bitmap_fonts_are_ignored() {
        assert!(parse_entry("Courier 10,12,15", "COURE.FON", &dir()).is_empty());
    }

    #[test]
    fn extension_case_does_not_matter() {
        assert_eq!(parse_entry("Old (TrueType)", "OLD.TTF", &dir()).len(), 1);
    }

    /// Lit le VRAI registre de l'ordinateur : Arial est installée sur tout Windows.
    #[cfg(windows)]
    #[test]
    fn system_fonts_are_listed() {
        let fonts = list_system_fonts();
        assert!(
            fonts.len() > 20,
            "seulement {} polices trouvées",
            fonts.len()
        );
        let arial = fonts
            .iter()
            .find(|f| f.name == "Arial")
            .expect("Arial introuvable");
        assert!(arial.path.exists());
    }

    #[test]
    fn sorted_without_duplicates() {
        let font = |name: &str| SystemFont {
            name: name.into(),
            path: dir(),
            index: 0,
        };
        let mut fonts = vec![font("Verdana"), font("arial"), font("Arial")];
        sort_and_dedup(&mut fonts);
        let names: Vec<&str> = fonts.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, vec!["arial", "Verdana"]);
    }
}
