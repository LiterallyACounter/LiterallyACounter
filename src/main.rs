// Attribut appliqué à tout le programme (le `!` après `#` = « s'applique au fichier entier »).
// En mode release (`not(debug_assertions)`), Windows lance l'exe comme une appli graphique,
// donc SANS console noire derrière. En debug, on garde la console pour voir les messages.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// `mod counter;` : déclare le module `counter`, dont le code est dans `src/counter.rs`.
mod counter;

use counter::Counter;
use eframe::egui;

/// L'état de l'application : pour l'instant, un seul compteur.
#[derive(Default)]
struct CounterWindow {
    counter: Counter,
}

impl eframe::App for CounterWindow {
    // Appelée à chaque image affichée.
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Échap : ferme la fenêtre (il n'y a plus de bouton ✕ puisqu'il n'y a plus de bordure).
        // `ui.input(|i| ...)` lit l'état du clavier et de la souris pour cette image.
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }

        egui::CentralPanel::default().show(ui, |ui| {
            // 1) On dessine d'abord la valeur, en grand, centrée dans la fenêtre.
            //    `selectable(false)` : par défaut, egui rend le texte sélectionnable (comme sur une
            //    page web), et le label capterait alors lui-même les clics.
            ui.centered_and_justified(|ui| {
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(self.counter.value().to_string()).size(48.0),
                    )
                    .selectable(false),
                );
            });

            // 2) PUIS on rend toute la surface cliquable et déplaçable.
            //    Pour egui, ce qui est ajouté en dernier est « au-dessus » et reçoit les clics en priorité.
            // `max_rect()` = le rectangle complet disponible ;
            // `Sense::click_and_drag()` = on veut détecter les clics ET les glisser.
            let response = ui.interact(
                ui.max_rect(),
                egui::Id::unique("counter_area"), // identifiant unique de cette zone pour egui
                egui::Sense::click_and_drag(),
            );

            // Glisser : on demande au système de déplacer la fenêtre (comme avec une barre de titre).
            if response.drag_started() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }
            // Clic gauche : +1. Clic droit (« secondary ») : -1.
            // (Provisoire : à l'étape 7, le clic droit ouvrira un menu.)
            if response.clicked() {
                self.counter.increment();
            }
            if response.secondary_clicked() {
                self.counter.decrement();
            }
        });
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Literally A Counter")
            .with_inner_size([200.0, 100.0]) // taille fixe pour l'instant (dynamique à l'étape 4)
            .with_decorations(false) // pas de bordure ni de barre de titre
            .with_always_on_top() // toujours au premier plan
            .with_resizable(false),
        ..Default::default()
    };

    eframe::run_native(
        "Literally A Counter",
        options,
        Box::new(|_cc| Ok(Box::new(CounterWindow::default()))),
    )
}
