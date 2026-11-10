//! Le style d'affichage d'une fenêtre compteur : format, police, couleurs, ancrage.
//! (Réglages codés en dur pour l'instant ; ils seront configurables à l'étape 8.)

use eframe::egui::Color32;

/// De quel côté la fenêtre reste fixe quand le texte change de largeur.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anchor {
    /// Le bord gauche reste fixe : la fenêtre s'élargit vers la droite.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "choix de l'ancrage à l'étape 8")
    )]
    Left,
    /// Le centre reste fixe : la fenêtre s'élargit des deux côtés.
    Center,
    /// Le bord droit reste fixe : la fenêtre s'élargit vers la gauche.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "choix de l'ancrage à l'étape 8")
    )]
    Right,
}

#[derive(Debug, Clone)]
pub struct DisplayStyle {
    /// Modèle de format, par ex. `{value}[ / {goal}]`.
    pub template: String,
    pub font_size: f32,
    /// Couleur de fond. `Color32::TRANSPARENT` = fond transparent (seul le texte est visible).
    /// La fenêtre est TOUJOURS transparente : c'est nous qui dessinons ce fond (aux coins arrondis).
    pub background: Color32,
    /// Rayon des coins arrondis du fond, en points (0 = coins carrés).
    pub corner_radius: u8,
    /// Couleur du texte quand il n'y a pas d'objectif.
    pub text: Color32,
    /// Couleur du texte quand l'objectif n'est pas encore atteint.
    pub goal_pending: Color32,
    /// Couleur du texte quand l'objectif est atteint ou dépassé.
    pub goal_reached: Color32,
    /// Couleur des messages d'alerte (par ex. « Reset ? »).
    pub alert: Color32,
    pub anchor: Anchor,
    /// Marge autour du texte, en points.
    pub margin: f32,
}

// `impl Default for DisplayStyle` : on écrit nous-mêmes la valeur par défaut
// (le `#[derive(Default)]` automatique mettrait tout à zéro, ce qui ne convient pas ici).
impl Default for DisplayStyle {
    fn default() -> Self {
        Self {
            template: "{value}[ / {goal}]".into(),
            font_size: 48.0,
            background: Color32::from_rgb(30, 30, 30),
            corner_radius: 8,
            text: Color32::WHITE,
            goal_pending: Color32::from_rgb(255, 165, 0), // orange
            goal_reached: Color32::from_rgb(80, 200, 120), // vert
            alert: Color32::from_rgb(255, 90, 90),        // rouge
            anchor: Anchor::Center,
            margin: 12.0,
        }
    }
}

impl DisplayStyle {
    /// La couleur du texte selon l'état de l'objectif (voir `Counter::goal_reached`).
    pub fn text_color(&self, goal_reached: Option<bool>) -> Color32 {
        match goal_reached {
            None => self.text,
            Some(false) => self.goal_pending,
            Some(true) => self.goal_reached,
        }
    }
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
