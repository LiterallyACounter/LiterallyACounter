//! Où ouvrir une fenêtre pour qu'elle ne chevauche aucune fenêtre déjà ouverte (spec M5).
//!
//! Règle : à droite de la fenêtre ouverte la plus à droite. Si elle sortirait de l'écran,
//! on passe EN DESSOUS de toutes les fenêtres, alignée à gauche. Dans les deux cas,
//! aucune fenêtre déjà ouverte ne peut être recouverte.
//!
//! Fonction pure (des rectangles en entrée, une position en sortie) : facile à tester.

use eframe::egui::{Pos2, Rect, Vec2, pos2};

/// Espace entre deux fenêtres, en points.
pub const GAP: f32 = 16.0;

/// Position utilisée quand aucune autre fenêtre n'est ouverte.
pub const DEFAULT_POSITION: Pos2 = pos2(100.0, 100.0);

/// Position du coin haut-gauche de la nouvelle fenêtre.
/// - `open` : les rectangles des fenêtres déjà ouvertes ;
/// - `screen` : la taille de l'écran, si on la connaît ;
/// - `size` : la taille estimée de la nouvelle fenêtre.
pub fn opening_position(open: &[Rect], screen: Option<Vec2>, size: Vec2) -> Pos2 {
    // `max_by` : l'élément le plus grand selon une comparaison. `total_cmp` compare deux `f32`
    // (les nombres à virgule n'ont pas d'ordre « simple » à cause de la valeur spéciale NaN).
    let Some(rightmost) = open.iter().max_by(|a, b| a.right().total_cmp(&b.right())) else {
        return DEFAULT_POSITION;
    };
    let beside = pos2(rightmost.right() + GAP, rightmost.top());
    // `is_none_or(...)` : écran inconnu → on accepte ; sinon, on vérifie que ça tient.
    let fits = screen.is_none_or(|screen| beside.x + size.x <= screen.x);
    if fits {
        return beside;
    }
    // En dessous de toutes les fenêtres, au niveau de la plus à gauche.
    // `fold` : parcourt la liste en gardant un résultat qui évolue (ici, le maximum / minimum).
    let lowest = open.iter().fold(f32::MIN, |acc, r| acc.max(r.bottom()));
    let leftmost = open.iter().fold(f32::MAX, |acc, r| acc.min(r.left()));
    pos2(leftmost, lowest + GAP)
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::vec2;

    const SIZE: Vec2 = vec2(200.0, 100.0);
    const SCREEN: Option<Vec2> = Some(vec2(1920.0, 1080.0));

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect::from_min_size(pos2(x, y), vec2(w, h))
    }

    #[test]
    fn first_window_goes_to_the_default_position() {
        assert_eq!(opening_position(&[], SCREEN, SIZE), DEFAULT_POSITION);
    }

    #[test]
    fn next_window_goes_right_of_the_rightmost() {
        let open = [
            rect(100.0, 100.0, 300.0, 200.0),
            rect(500.0, 50.0, 120.0, 80.0),
        ];
        assert_eq!(
            opening_position(&open, SCREEN, SIZE),
            pos2(500.0 + 120.0 + GAP, 50.0)
        );
    }

    #[test]
    fn goes_below_when_the_screen_is_too_narrow() {
        let open = [
            rect(100.0, 100.0, 300.0, 200.0),
            rect(1700.0, 50.0, 120.0, 80.0),
        ];
        // À droite : 1836 + 200 > 1920. On passe sous la plus basse (bas à 300).
        assert_eq!(
            opening_position(&open, SCREEN, SIZE),
            pos2(100.0, 300.0 + GAP)
        );
    }

    #[test]
    fn never_overlaps_an_open_window() {
        let open = [
            rect(100.0, 100.0, 300.0, 200.0),
            rect(450.0, 400.0, 200.0, 100.0),
            rect(1700.0, 50.0, 120.0, 80.0),
        ];
        for screen in [SCREEN, None] {
            let new = Rect::from_min_size(opening_position(&open, screen, SIZE), SIZE);
            // `all` : vrai si TOUS les éléments vérifient la condition.
            assert!(open.iter().all(|r| !r.intersects(new)));
        }
    }
}
