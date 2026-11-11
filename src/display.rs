//! Le style d'affichage d'une fenêtre compteur : format, police, couleurs, ancrage.
//! (Réglages modifiables à la main dans la sauvegarde JSON ; configurables à l'étape 8.)

use eframe::egui::Color32;
use serde::{Deserialize, Serialize};

/// De quel côté la fenêtre reste fixe quand le texte change de largeur.
///
/// `#[serde(rename_all = "lowercase")]` : dans le JSON, les variantes s'écrivent
/// `"left"`, `"center"`, `"right"` (plus lisible à la main que `"Left"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Anchor {
    /// Le bord gauche reste fixe : la fenêtre s'élargit vers la droite.
    Left,
    /// Le centre reste fixe : la fenêtre s'élargit des deux côtés.
    Center,
    /// Le bord droit reste fixe : la fenêtre s'élargit vers la gauche.
    Right,
}

/// Le style d'affichage, sauvegardé dans le JSON.
///
/// `#[serde(default)]` : un réglage absent du fichier prend sa valeur par défaut
/// (on peut donc n'écrire à la main que ce qu'on veut changer).
/// `#[serde(with = "color_hex")]` : ce champ est converti par NOTRE module `color_hex`
/// (texte `"#RRGGBBAA"`), car `Color32` d'egui ne sait pas se sauvegarder tout seul.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DisplayStyle {
    /// Modèle de format, par ex. `{value}[ / {goal}]`.
    pub template: String,
    /// Police installée sur l'ordinateur, par son nom (`None` : police par défaut d'egui).
    pub font: Option<String>,
    pub font_size: f32,
    /// Couleur de fond. Transparent (`#00000000`) = seul le texte est visible.
    /// La fenêtre est TOUJOURS transparente : c'est nous qui dessinons ce fond (aux coins arrondis).
    #[serde(with = "color_hex")]
    pub background: Color32,
    /// Rayon des coins arrondis du fond, en points (0 = coins carrés).
    pub corner_radius: u8,
    /// Couleur du texte quand il n'y a pas d'objectif.
    #[serde(with = "color_hex")]
    pub text: Color32,
    /// Couleur du texte quand l'objectif n'est pas encore atteint.
    #[serde(with = "color_hex")]
    pub goal_pending: Color32,
    /// Couleur du texte quand l'objectif est atteint ou dépassé.
    #[serde(with = "color_hex")]
    pub goal_reached: Color32,
    /// Couleur des messages d'alerte (par ex. « Reset ? »).
    #[serde(with = "color_hex")]
    pub alert: Color32,
    pub anchor: Anchor,
    /// Fenêtre toujours au premier plan (au-dessus du jeu).
    pub always_on_top: bool,
    /// Marge autour du texte, en points.
    pub margin: f32,
}

// `impl Default for DisplayStyle` : on écrit nous-mêmes la valeur par défaut
// (le `#[derive(Default)]` automatique mettrait tout à zéro, ce qui ne convient pas ici).
impl Default for DisplayStyle {
    fn default() -> Self {
        Self {
            template: "{value}[ / {goal}]".into(),
            font: None,
            font_size: 48.0,
            background: Color32::from_rgb(30, 30, 30),
            corner_radius: 8,
            text: Color32::WHITE,
            goal_pending: Color32::from_rgb(255, 165, 0), // orange
            goal_reached: Color32::from_rgb(80, 200, 120), // vert
            alert: Color32::from_rgb(255, 90, 90),        // rouge
            anchor: Anchor::Center,
            always_on_top: true,
            margin: 12.0,
        }
    }
}

impl DisplayStyle {
    /// Le style propre de la partie par défaut : fond bleu nuit, pour la distinguer du global
    /// quand la copie est désactivée.
    pub fn round_default() -> Self {
        Self {
            background: Color32::from_rgb(20, 40, 70),
            ..Self::default()
        }
    }

    /// La couleur du texte selon l'état de l'objectif (voir `Counter::goal_reached`).
    pub fn text_color(&self, goal_reached: Option<bool>) -> Color32 {
        match goal_reached {
            None => self.text,
            Some(false) => self.goal_pending,
            Some(true) => self.goal_reached,
        }
    }
}

/// Conversion d'une couleur en texte `"#RRGGBBAA"` (et retour), pour la sauvegarde.
///
/// serde attend, pour `#[serde(with = "...")]`, un module contenant deux fonctions
/// nommées exactement `serialize` et `deserialize`.
mod color_hex {
    use eframe::egui::Color32;
    use serde::{Deserialize, Deserializer, Serializer, de::Error};

    pub fn serialize<S: Serializer>(color: &Color32, serializer: S) -> Result<S::Ok, S::Error> {
        // `to_srgba_unmultiplied` : rouge, vert, bleu, opacité, chacun de 0 à 255.
        let [r, g, b, a] = color.to_srgba_unmultiplied();
        // `{r:02X}` : en hexadécimal majuscule, sur 2 chiffres (ex. 30 → « 1E »).
        serializer.serialize_str(&format!("#{r:02X}{g:02X}{b:02X}{a:02X}"))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Color32, D::Error> {
        let text = String::deserialize(deserializer)?;
        super::parse_hex_color(&text)
            .ok_or_else(|| D::Error::custom(format!("couleur invalide : {text}")))
    }
}

/// Lit une couleur `#RRGGBB` (opaque) ou `#RRGGBBAA`. `None` si le texte est invalide.
pub fn parse_hex_color(text: &str) -> Option<Color32> {
    // `strip_prefix` : retire « # » au début, ou `None` s'il n'y est pas.
    let hex = text.strip_prefix('#')?;
    // Lit l'octet n° `i` (2 chiffres hexadécimaux). `get` renvoie `None` hors du texte.
    let byte = |i: usize| u8::from_str_radix(hex.get(2 * i..2 * i + 2)?, 16).ok();
    let alpha = match hex.len() {
        6 => 255,
        8 => byte(3)?,
        _ => return None,
    };
    Some(Color32::from_rgba_unmultiplied(
        byte(0)?,
        byte(1)?,
        byte(2)?,
        alpha,
    ))
}

/// La plus grande taille de police, de `max` jusqu'à `min` par pas de 1, pour laquelle le texte
/// tient dans la place disponible. `fits(taille)` répond « est-ce que ça tient à cette taille ? ».
///
/// Passer la mesure en paramètre (`impl Fn`) rend cette fonction indépendante d'egui :
/// les tests peuvent simuler un texte sans rien dessiner.
pub fn largest_fitting_font_size(max: f32, min: f32, fits: impl Fn(f32) -> bool) -> f32 {
    let mut size = max;
    while size > min && !fits(size) {
        size -= 1.0;
    }
    size.max(min)
}

/// Nouvelle position du bord gauche quand la largeur passe de `old_width` à `new_width`,
/// pour que le côté choisi (`anchor`) reste fixe.
pub fn anchored_left(old_left: f32, old_width: f32, new_width: f32, anchor: Anchor) -> f32 {
    match anchor {
        Anchor::Left => old_left,
        Anchor::Center => old_left + (old_width - new_width) / 2.0,
        Anchor::Right => old_left + old_width - new_width,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_color_follows_goal_state() {
        let style = DisplayStyle::default();
        assert_eq!(style.text_color(None), Color32::WHITE);
        assert_eq!(
            style.text_color(Some(false)),
            Color32::from_rgb(255, 165, 0)
        );
        assert_eq!(
            style.text_color(Some(true)),
            Color32::from_rgb(80, 200, 120)
        );
    }

    #[test]
    fn hex_colors_are_parsed() {
        assert_eq!(
            parse_hex_color("#1E1E1E"),
            Some(Color32::from_rgb(30, 30, 30))
        );
        assert_eq!(parse_hex_color("#00000000"), Some(Color32::TRANSPARENT));
        assert_eq!(parse_hex_color("1E1E1E"), None); // il manque « # »
        assert_eq!(parse_hex_color("#1E1E"), None); // trop court
        assert_eq!(parse_hex_color("#GG0000"), None); // pas de l'hexadécimal
    }

    #[test]
    fn style_survives_a_save_and_reload() {
        let style = DisplayStyle {
            background: Color32::from_rgba_unmultiplied(10, 20, 30, 128),
            anchor: Anchor::Right,
            ..DisplayStyle::default()
        };
        let json = serde_json::to_string(&style).unwrap();
        assert!(json.contains("\"background\":\"#0A141E80\""));
        assert!(json.contains("\"anchor\":\"right\""));
        let back: DisplayStyle = serde_json::from_str(&json).unwrap();
        assert_eq!(back, style);
    }

    #[test]
    fn missing_style_settings_take_their_default() {
        let style: DisplayStyle = serde_json::from_str("{\"font_size\": 20.0}").unwrap();
        assert_eq!(style.font_size, 20.0);
        assert_eq!(style.template, DisplayStyle::default().template);
    }

    #[test]
    fn font_size_stays_at_max_when_text_fits() {
        assert_eq!(largest_fitting_font_size(48.0, 8.0, |_| true), 48.0);
    }

    #[test]
    fn font_size_shrinks_until_text_fits() {
        // Simule un texte qui ne tient qu'à partir de la taille 20.
        assert_eq!(
            largest_fitting_font_size(48.0, 8.0, |size| size <= 20.0),
            20.0
        );
    }

    #[test]
    fn font_size_never_goes_below_min() {
        // Même si rien ne tient, on ne descend pas sous le minimum (texte lisible malgré tout).
        assert_eq!(largest_fitting_font_size(48.0, 8.0, |_| false), 8.0);
    }

    // Fenêtre de 100 de large à x = 500 ; le texte grandit et la fenêtre passe à 140.
    #[test]
    fn left_anchor_keeps_left_edge() {
        assert_eq!(anchored_left(500.0, 100.0, 140.0, Anchor::Left), 500.0);
    }

    #[test]
    fn center_anchor_grows_on_both_sides() {
        // 20 de chaque côté : le bord gauche recule de 20, le centre (550) ne bouge pas.
        assert_eq!(anchored_left(500.0, 100.0, 140.0, Anchor::Center), 480.0);
    }

    #[test]
    fn right_anchor_keeps_right_edge() {
        // Le bord droit reste à 600 : 600 - 140 = 460.
        assert_eq!(anchored_left(500.0, 100.0, 140.0, Anchor::Right), 460.0);
    }

    #[test]
    fn anchors_also_work_when_shrinking() {
        assert_eq!(anchored_left(500.0, 140.0, 100.0, Anchor::Center), 520.0);
        assert_eq!(anchored_left(500.0, 140.0, 100.0, Anchor::Right), 540.0);
    }
}
