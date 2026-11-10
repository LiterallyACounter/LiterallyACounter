//! Les éléments de la fenêtre de configuration : l'éditeur de style d'un compteur et la vue
//! d'ensemble des raccourcis.
//!
//! Chaque fonction reçoit les réglages en `&mut` (emprunt modifiable) et les modifie directement
//! quand l'utilisateur touche à un contrôle. Elle renvoie `true` si quelque chose a changé, pour
//! que `main.rs` sache qu'il faut sauvegarder.

use eframe::egui;
use egui::color_picker::{Alpha, color_edit_button_srgba};

use crate::LANGUAGE;
use crate::display::{Anchor, DisplayStyle};
use crate::hotkeys::Shortcut;
use crate::i18n::{self, Text};

/// Un texte traduit dans la langue de l'interface (pour alléger l'écriture).
fn t(text: Text) -> String {
    i18n::text(text, LANGUAGE)
}

/// Les réglages d'affichage d'un compteur, présentés en grille (libellé | contrôle).
/// `id_salt` : un identifiant propre à chaque compteur, car egui doit distinguer
/// les deux grilles affichées dans la même fenêtre.
pub fn style_editor(ui: &mut egui::Ui, id_salt: &str, style: &mut DisplayStyle) -> bool {
    let mut changed = false;

    // L'aide du format, AU-DESSUS de la grille : dans une cellule de grille, le texte ne peut
    // pas passer à la ligne (la cellule s'élargit sans limite) ; ici, il suit la largeur
    // de la fenêtre. `Label::new(...).wrap()` : retour à la ligne automatique.
    ui.add(egui::Label::new(egui::RichText::new(t(Text::TemplateHelp)).small()).wrap());

    egui::Grid::new(id_salt)
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            ui.label(t(Text::Template));
            changed |= ui.text_edit_singleline(&mut style.template).changed();
            ui.end_row();

            // `DragValue` : un nombre qu'on modifie en glissant la souris, ou en cliquant pour
            // le saisir. `range` borne la valeur, `speed` règle la vitesse du glissement.
            ui.label(t(Text::FontSize));
            changed |= ui
                .add(
                    egui::DragValue::new(&mut style.font_size)
                        .range(8.0..=300.0)
                        .speed(0.5),
                )
                .changed();
            ui.end_row();

            ui.label(t(Text::Margin));
            changed |= ui
                .add(egui::DragValue::new(&mut style.margin).range(0.0..=100.0))
                .changed();
            ui.end_row();

            ui.label(t(Text::CornerRadius));
            changed |= ui
                .add(egui::DragValue::new(&mut style.corner_radius).range(0..=100))
                .changed();
            ui.end_row();

            ui.label(t(Text::Anchor));
            ui.horizontal(|ui| {
                // `radio_value` : un bouton radio qui met `style.anchor` à la valeur donnée.
                for (anchor, label) in [
                    (Anchor::Left, Text::AnchorLeft),
                    (Anchor::Center, Text::AnchorCenter),
                    (Anchor::Right, Text::AnchorRight),
                ] {
                    changed |= ui
                        .radio_value(&mut style.anchor, anchor, t(label))
                        .changed();
                }
            });
            ui.end_row();

            ui.label("");
            changed |= ui
                .checkbox(&mut style.always_on_top, t(Text::AlwaysOnTop))
                .changed();
            ui.end_row();

            ui.label(t(Text::Background));
            ui.horizontal(|ui| {
                // Fond transparent : opacité à 0. Le décocher remet le fond par défaut.
                let mut transparent = style.background.a() == 0;
                if ui
                    .checkbox(&mut transparent, t(Text::TransparentBackground))
                    .changed()
                {
                    style.background = if transparent {
                        egui::Color32::TRANSPARENT
                    } else {
                        DisplayStyle::default().background
                    };
                    changed = true;
                }
                if !transparent {
                    // `Alpha::OnlyBlend` : le sélecteur propose aussi l'opacité (fond semi-transparent).
                    changed |= color_edit_button_srgba(ui, &mut style.background, Alpha::OnlyBlend)
                        .changed();
                }
            });
            ui.end_row();

            // Les couleurs du texte, toujours opaques.
            for (label, color) in [
                (Text::PlainTextColor, &mut style.text),
                (Text::GoalPendingColor, &mut style.goal_pending),
                (Text::GoalReachedColor, &mut style.goal_reached),
                (Text::AlertColor, &mut style.alert),
            ] {
                ui.label(t(label));
                changed |= color_edit_button_srgba(ui, color, Alpha::Opaque).changed();
                ui.end_row();
            }
        });
    changed
}

/// La liste des raccourcis actifs (lecture seule pour l'instant ; modifiables à l'étape 8b).
pub fn shortcuts_overview(ui: &mut egui::Ui, shortcuts: &[Shortcut]) {
    ui.label(t(Text::ShortcutsInfo));
    ui.add_space(8.0);
    egui::Grid::new("shortcuts")
        .num_columns(2)
        .striped(true) // une ligne sur deux légèrement colorée, pour la lisibilité
        .spacing([24.0, 6.0])
        .show(ui, |ui| {
            ui.strong(t(Text::ColumnAction));
            ui.strong(t(Text::ColumnShortcut));
            ui.end_row();
            for shortcut in shortcuts {
                ui.label(i18n::command_name(shortcut.command, LANGUAGE));
                ui.label(i18n::shortcut_name(shortcut, LANGUAGE));
                ui.end_row();
            }
        });
}
