// Attribut appliqué à tout le programme (le `!` après `#` = « s'applique au fichier entier »).
// En mode release (`not(debug_assertions)`), Windows lance l'exe comme une appli graphique,
// donc SANS console noire derrière. En debug, on garde la console pour voir les messages.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// `mod xxx;` : déclare un module. Son code est dans `src/xxx.rs` ou `src/xxx/mod.rs`.
mod audio;
mod counter;
mod display;
mod format;
mod hotkeys;
mod native_window;

use std::sync::mpsc::{self, Receiver};

use audio::{AudioEngine, SoundSlot};
use counter::{Counter, CounterAction};
use display::DisplayStyle;
use eframe::egui;
use hotkeys::{Key, Shortcut};
use native_window::NativeWindow;

/// L'état de l'application : pour l'instant, un seul compteur.
struct CounterWindow {
    counter: Counter,
    /// Bout « réception » du channel : les actions envoyées par le thread des raccourcis arrivent ici.
    actions: Receiver<CounterAction>,
    /// Le moteur audio. `None` s'il n'y a aucune sortie audio : l'appli marche alors sans son.
    audio: Option<AudioEngine>,
    /// Format, police, couleurs, ancrage.
    style: DisplayStyle,
    /// Accès direct à la fenêtre du système (`None` hors Windows pour l'instant).
    native: Option<NativeWindow>,
}

impl CounterWindow {
    /// Applique une action au compteur ET joue le son correspondant.
    /// Toutes les actions (clavier comme souris) passent par ici.
    fn handle(&mut self, action: CounterAction) {
        self.counter.apply(action);
        // `if let Some(audio) = &mut self.audio` : seulement si le moteur audio existe.
        if let Some(audio) = &mut self.audio {
            audio.play(SoundSlot::for_action(action));
        }
    }

    /// Redimensionne la fenêtre à `desired_size` si besoin, en gardant fixe le côté d'ancrage.
    fn fit_window(&self, ctx: &egui::Context, desired_size: egui::Vec2) {
        // Position et taille actuelles de la fenêtre, fournies par egui.
        // `let (Some(a), Some(b)) = ... else { return; }` : on sort si l'une des deux est inconnue.
        let (Some(outer), Some(inner)) =
            ctx.input(|i| (i.viewport().outer_rect, i.viewport().inner_rect))
        else {
            return;
        };

        // Déjà à la bonne taille (à un demi-point près) : rien à faire.
        if (inner.size() - desired_size).abs().max_elem() < 0.5 {
            return;
        }

        let new_left = display::anchored_left(
            outer.left(),
            inner.width(),
            desired_size.x,
            self.style.anchor,
        );
        match &self.native {
            // Sous Windows : taille + position en un seul appel, sans recopie de l'ancienne image.
            // egui compte en « points », Windows en pixels physiques : on multiplie par
            // `pixels_per_point` (1,25 si l'affichage Windows est réglé à 125 %, par exemple).
            Some(native) => {
                let ppp = ctx.pixels_per_point();
                let px = |points: f32| (points * ppp).round() as i32;
                native.set_rect(
                    px(new_left),
                    px(outer.top()),
                    px(desired_size.x),
                    px(desired_size.y),
                );
            }
            // Ailleurs : les commandes d'egui (taille, puis position).
            None => {
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(desired_size));
                ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(
                    new_left,
                    outer.top(),
                )));
            }
        }
    }
}

impl eframe::App for CounterWindow {
    /// La couleur avec laquelle la fenêtre est effacée avant chaque image : TRANSPARENTE.
    /// Le fond visible (avec ses coins arrondis) est dessiné ensuite, par nous.
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Color32::TRANSPARENT.to_normalized_gamma_f32()
    }

    // Appelée à chaque image affichée.
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // On applique toutes les actions reçues du clavier depuis la dernière image.
        // `try_recv` ne bloque pas : il renvoie `Ok(action)` s'il y en a une, une erreur sinon.
        // `while let` : boucle tant que le motif correspond (ici, tant qu'il y a des actions).
        while let Ok(action) = self.actions.try_recv() {
            self.handle(action);
        }

        // Échap : ferme la fenêtre (il n'y a plus de bouton ✕ puisqu'il n'y a plus de bordure).
        // `ui.input(|i| ...)` lit l'état du clavier et de la souris pour cette image.
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }

        // 1) Le texte à afficher, d'après le format (ex. « 3 / 10 »), et sa couleur.
        let text = format::render(
            &self.style.template,
            format::Values {
                value: self.counter.value(),
                goal: self.counter.goal(),
            },
            format::Language::French,
        );
        let color = self.style.text_color(self.counter.goal_reached());

        // 2) On MESURE le texte : egui le « met en page » (un `Galley`) et nous donne sa taille.
        let font = egui::FontId::proportional(self.style.font_size);
        let galley = ui.painter().layout_no_wrap(text, font, color);
        let text_size = galley.size();

        // 3) On ajuste la fenêtre à cette taille + la marge, en respectant l'ancrage.
        let desired_size = (text_size + egui::Vec2::splat(2.0 * self.style.margin)).ceil();
        self.fit_window(ui.ctx(), desired_size);

        // `Frame::NONE.fill(...)` : un panneau sans marge, rempli avec la couleur de fond,
        // avec des coins arrondis (`CornerRadius::same` = le même rayon pour les 4 coins).
        let panel_frame = egui::Frame::NONE
            .fill(self.style.background)
            .corner_radius(egui::CornerRadius::same(self.style.corner_radius));
        egui::CentralPanel::default()
            .frame(panel_frame)
            .show(ui, |ui| {
                // 4) On dessine le texte, centré dans la fenêtre.
                let rect = ui.max_rect();
                let top_left = rect.center() - text_size / 2.0;
                ui.painter().galley(top_left, galley, color);

                // 5) PUIS on rend toute la surface cliquable et déplaçable.
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
                    self.handle(CounterAction::Add(1));
                }
                if response.secondary_clicked() {
                    self.handle(CounterAction::Subtract(1));
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
    // Style codé en dur pour l'instant (configurable à l'étape 7).
    // POUR TESTER : décommente une ligne pour changer un réglage, par exemple :
    let style = DisplayStyle {
        // background: egui::Color32::TRANSPARENT,    // fond transparent
        // corner_radius: 0,                          // coins carrés
        // anchor: display::Anchor::Right,            // ancrage à droite
        // template: "{value}[ ({ratio:p})]".into(),  // autre format
        ..DisplayStyle::default()
    };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Literally A Counter")
            // Taille de départ ; elle s'ajuste au texte dès la première image.
            .with_inner_size([200.0, 100.0])
            .with_decorations(false) // pas de bordure ni de barre de titre
            .with_always_on_top() // toujours au premier plan
            .with_resizable(false)
            // Fenêtre TOUJOURS transparente (demandé à la création) : on dessine nous-mêmes
            // le fond, ce qui permet les coins arrondis et le fond transparent.
            .with_transparent(true),
        ..Default::default()
    };

    eframe::run_native(
        "Literally A Counter",
        options,
        // `cc` (« creation context ») donne accès au contexte egui au démarrage.
        Box::new(|cc| {
            // Accès direct à la fenêtre du système, et retrait des décorations de Windows 11
            // (liseré, coins, ombre, ligne du haut), visibles avec un fond transparent.
            // `.inspect(|w| ...)` : exécute une action sur la valeur si elle existe, puis la rend.
            let native = NativeWindow::from_window(cc).inspect(|w| w.remove_decorations());

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
                // Objectif codé en dur à 10 pour tester (configurable à l'étape 7).
                counter: Counter::with_goal(10),
                actions: receiver,
                audio: AudioEngine::new(),
                style,
                native,
            }))
        }),
    )
}
