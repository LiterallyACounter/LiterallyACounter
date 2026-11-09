// Attribut appliqué à tout le programme (le `!` après `#` = « s'applique au fichier entier »).
// En mode release (`not(debug_assertions)`), Windows lance l'exe comme une appli graphique,
// donc SANS console noire derrière. En debug, on garde la console pour voir les messages.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// `mod xxx;` : déclare un module. Son code est dans `src/xxx.rs` ou `src/xxx/mod.rs`.
mod counter;
mod hotkeys;

use std::sync::mpsc::{self, Receiver};

use counter::{Counter, CounterAction};
use eframe::egui;
use hotkeys::{Key, Shortcut};

/// L'état de l'application : pour l'instant, un seul compteur.
struct CounterWindow {
    counter: Counter,
    /// Bout « réception » du channel : les actions envoyées par le thread des raccourcis arrivent ici.
    actions: Receiver<CounterAction>,
}

impl eframe::App for CounterWindow {
    // Appelée à chaque image affichée.
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // On applique toutes les actions reçues du clavier depuis la dernière image.
        // `try_recv` ne bloque pas : il renvoie `Ok(action)` s'il y en a une, une erreur sinon.
        // `while let` : boucle tant que le motif correspond (ici, tant qu'il y a des actions).
        while let Ok(action) = self.actions.try_recv() {
            self.counter.apply(action);
        }

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

/// Raccourcis codés en dur pour l'étape 2 (ils deviendront configurables à l'étape 7).
fn default_shortcuts() -> Vec<Shortcut> {
    // `vec![...]` : macro qui crée un `Vec` avec les éléments donnés.
    vec![
        Shortcut {
            held: vec![],
            trigger: Key::NUMPAD_ADD,
            action: CounterAction::Add(1),
        },
        Shortcut {
            held: vec![],
            trigger: Key::NUMPAD_SUBTRACT,
            action: CounterAction::Subtract(1),
        },
        Shortcut {
            held: vec![Key::CONTROL],
            trigger: Key::NUMPAD_ADD,
            action: CounterAction::Add(5),
        },
    ]
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
        // `cc` (« creation context ») donne accès au contexte egui au démarrage.
        Box::new(|cc| {
            // Un CHANNEL : un tuyau entre threads. `sender` envoie, `receiver` reçoit.
            let (sender, receiver) = mpsc::channel();
            // Le contexte egui, pour pouvoir réveiller l'interface depuis le thread des raccourcis.
            // `clone()` : on en fait une copie (c'est en fait un pointeur partagé, la copie est légère).
            let ctx = cc.egui_ctx.clone();

            hotkeys::start(default_shortcuts(), move |action| {
                // `let _ =` : on ignore volontairement le résultat (échec seulement si l'appli est fermée).
                let _ = sender.send(action);
                // Sans le focus, egui ne redessine pas tout seul : on le réveille.
                ctx.request_repaint();
            });

            Ok(Box::new(CounterWindow {
                counter: Counter::default(),
                actions: receiver,
            }))
        }),
    )
}
