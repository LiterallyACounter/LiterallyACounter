// Attribut appliqué à tout le programme (le `!` après `#` = « s'applique au fichier entier »).
// En mode release (`not(debug_assertions)`), Windows lance l'exe comme une appli graphique,
// donc SANS console noire derrière. En debug, on garde la console pour voir les messages.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// `mod xxx;` : déclare un module. Son code est dans `src/xxx.rs` ou `src/xxx/mod.rs`.
mod audio;
mod command;
mod counter;
mod display;
mod format;
mod hotkeys;
mod i18n;
mod native_window;
mod reset;
mod storage;

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use audio::{AudioEngine, SoundSlot};
use command::Command;
use counter::{Counter, CounterAction};
use display::DisplayStyle;
use eframe::egui;
use hotkeys::{Key, Shortcut};
use i18n::{Language, Text};
use native_window::NativeWindow;
use reset::ResetConfirmation;
use storage::SavedState;

/// Langue de l'interface (détectée et réglable à l'étape 10).
const LANGUAGE: Language = Language::French;

/// L'état de l'application : pour l'instant, un seul compteur, le compteur GLOBAL.
struct CounterWindow {
    counter: Counter,
    /// Bout « réception » du channel : les commandes envoyées par le thread des raccourcis arrivent ici.
    commands: Receiver<Command>,
    /// Le moteur audio. `None` s'il n'y a aucune sortie audio : l'appli marche alors sans son.
    audio: Option<AudioEngine>,
    /// Format, police, couleurs, ancrage.
    style: DisplayStyle,
    /// Accès direct à la fenêtre du système (`None` hors Windows pour l'instant).
    native: Option<NativeWindow>,
    /// Demande de reset en attente de validation.
    reset_confirmation: ResetConfirmation,
    /// Nom de la touche de validation, tiré des raccourcis configurés (ex. « Num Entrée »).
    confirm_key_name: String,
    /// Fichier de sauvegarde (`None` si le système n'a pas de dossier de configuration).
    state_path: Option<PathBuf>,
}

impl CounterWindow {
    /// Exécute une commande. Toutes les commandes (clavier comme souris) passent par ici.
    /// `ctx` : le contexte egui, pour pouvoir réveiller l'affichage plus tard.
    fn handle(&mut self, command: Command, now: Instant, ctx: &egui::Context) {
        match command {
            Command::Count(action) => {
                self.counter.apply(action);
                self.save();
                self.play(SoundSlot::for_action(action));
            }
            // Compteur global : un reset doit être VALIDÉ. On affiche la demande et on attend.
            Command::Reset => {
                self.reset_confirmation.request(now);
                self.play(SoundSlot::ResetPrompt);
                // À l'expiration du délai, il faut redessiner pour effacer « Reset ? », même si
                // rien d'autre ne se passe. `request_repaint_after` d'egui ne réveille pas une
                // fenêtre sans le focus : on utilise donc la même méthode que le hook clavier,
                // un petit thread qui attend puis réveille l'affichage.
                let ctx = ctx.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(reset::CONFIRM_DELAY + Duration::from_millis(50));
                    ctx.request_repaint();
                });
            }
            Command::ConfirmReset => {
                // Validation sans demande en cours (ou trop tard) : ignorée, sans bruit.
                if self.reset_confirmation.confirm(now) {
                    self.counter.reset();
                    self.save();
                    self.play(SoundSlot::ResetDone);
                }
            }
            Command::UndoReset => {
                if self.counter.undo_reset() {
                    self.save();
                    self.play(SoundSlot::ResetUndone);
                }
            }
        }
    }

    fn play(&mut self, slot: SoundSlot) {
        // `if let Some(audio) = &mut self.audio` : seulement si le moteur audio existe.
        if let Some(audio) = &mut self.audio {
            audio.play(slot);
        }
    }

    /// Sauvegarde la valeur du compteur global. En cas d'échec, on le signale sans planter :
    /// l'appli continue de compter, et réessaiera au prochain changement.
    fn save(&self) {
        let Some(path) = &self.state_path else {
            return;
        };
        let state = SavedState {
            global_value: self.counter.value(),
        };
        if let Err(error) = storage::save(path, &state) {
            eprintln!("Sauvegarde impossible : {error}");
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

    /// Met en page un texte DANS la fenêtre actuelle, sans la redimensionner : lignes centrées,
    /// retour à la ligne, et la plus grande police (au plus celle du compteur) qui tient.
    fn layout_in_current_window(
        &self,
        ui: &egui::Ui,
        text: String,
        color: egui::Color32,
    ) -> std::sync::Arc<egui::Galley> {
        // La place disponible : la fenêtre actuelle, moins les marges.
        let window = ui
            .ctx()
            .input(|i| i.viewport().inner_rect)
            .map_or(egui::vec2(200.0, 100.0), |r| r.size());
        let available = (window - egui::Vec2::splat(2.0 * self.style.margin)).max(egui::Vec2::ZERO);

        // Une mise en page à une taille donnée : texte coupé à la largeur disponible, centré.
        let layout = |size: f32| {
            let font = egui::FontId::proportional(size);
            let mut job = egui::text::LayoutJob::simple(text.clone(), font, color, available.x);
            job.halign = egui::Align::Center;
            ui.painter().layout_job(job)
        };
        let fits = |size: f32| {
            let s = layout(size).size();
            s.x <= available.x + 0.5 && s.y <= available.y + 0.5
        };
        let size = display::largest_fitting_font_size(self.style.font_size, 8.0, fits);
        layout(size)
    }

    /// Le texte à afficher et sa couleur : la demande de reset si elle est en attente,
    /// sinon le compteur formaté (ex. « 3 / 10 »).
    fn text_and_color(&self, now: Instant) -> (String, egui::Color32) {
        if self.reset_confirmation.is_pending(now) {
            let prompt = i18n::message(
                Text::ResetPrompt,
                LANGUAGE,
                &[("key", &self.confirm_key_name)],
            );
            return (prompt, self.style.alert);
        }
        let text = format::render(
            &self.style.template,
            format::Values {
                value: self.counter.value(),
                goal: self.counter.goal(),
            },
            LANGUAGE,
        );
        (text, self.style.text_color(self.counter.goal_reached()))
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
        let now = Instant::now();

        // On exécute toutes les commandes reçues du clavier depuis la dernière image.
        // `try_recv` ne bloque pas : il renvoie `Ok(commande)` s'il y en a une, une erreur sinon.
        // `while let` : boucle tant que le motif correspond (ici, tant qu'il y a des commandes).
        while let Ok(command) = self.commands.try_recv() {
            self.handle(command, now, ui.ctx());
        }

        // Échap : ferme la fenêtre (il n'y a plus de bouton ✕ puisqu'il n'y a plus de bordure).
        // `ui.input(|i| ...)` lit l'état du clavier et de la souris pour cette image.
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }

        // 1) Le texte à afficher et sa couleur.
        let (text, color) = self.text_and_color(now);

        // 2) On MESURE et met en page le texte (un `Galley` : le texte prêt à dessiner).
        let galley = if self.reset_confirmation.is_pending(now) {
            // Message de reset : la fenêtre GARDE sa taille (sinon elle peut sortir de l'écran).
            // C'est le texte qui s'adapte : retour à la ligne, puis police réduite si besoin.
            self.layout_in_current_window(ui, text, color)
        } else {
            // Compteur : texte sur une ligne, et c'est la fenêtre qui s'ajuste à lui,
            // en respectant l'ancrage.
            let font = egui::FontId::proportional(self.style.font_size);
            let galley = ui.painter().layout_no_wrap(text, font, color);
            let margins = egui::Vec2::splat(2.0 * self.style.margin);
            self.fit_window(ui.ctx(), (galley.size() + margins).ceil());
            galley
        };

        // `Frame::NONE.fill(...)` : un panneau sans marge, rempli avec la couleur de fond,
        // avec des coins arrondis (`CornerRadius::same` = le même rayon pour les 4 coins).
        let panel_frame = egui::Frame::NONE
            .fill(self.style.background)
            .corner_radius(egui::CornerRadius::same(self.style.corner_radius));
        egui::CentralPanel::default()
            .frame(panel_frame)
            .show(ui, |ui| {
                // 4) On dessine le texte, centré dans la fenêtre.
                //    `galley.rect` : la zone occupée par le texte, relative au point de dessin.
                //    On décale ce point pour que le centre du texte tombe au centre de la fenêtre.
                let rect = ui.max_rect();
                let position = rect.center() - galley.rect.center().to_vec2();
                ui.painter().galley(position, galley, color);

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
                // (Provisoire : à l'étape 8, le clic droit ouvrira un menu.)
                if response.clicked() {
                    self.handle(Command::Count(CounterAction::Add(1)), now, ui.ctx());
                }
                if response.secondary_clicked() {
                    self.handle(Command::Count(CounterAction::Subtract(1)), now, ui.ctx());
                }
            });
    }
}

/// Raccourcis codés en dur pour l'instant (ils deviendront configurables à l'étape 8).
fn default_shortcuts() -> Vec<Shortcut> {
    // Petite fonction locale pour alléger l'écriture de la liste.
    fn shortcut(held: &[Key], trigger: Key, command: Command) -> Shortcut {
        Shortcut {
            // `to_vec()` : copie la tranche dans un nouveau `Vec`.
            held: held.to_vec(),
            trigger,
            command,
        }
    }
    // `vec![...]` : macro qui crée un `Vec` avec les éléments donnés.
    vec![
        shortcut(&[], Key::NUMPAD_ADD, Command::Count(CounterAction::Add(1))),
        shortcut(
            &[],
            Key::NUMPAD_SUBTRACT,
            Command::Count(CounterAction::Subtract(1)),
        ),
        shortcut(
            &[Key::CONTROL],
            Key::NUMPAD_ADD,
            Command::Count(CounterAction::Add(5)),
        ),
        shortcut(&[Key::CONTROL], Key::NUMPAD_0, Command::Reset),
        shortcut(&[], Key::NUMPAD_ENTER, Command::ConfirmReset),
        shortcut(&[Key::CONTROL], Key::NUMPAD_DECIMAL, Command::UndoReset),
    ]
}

fn main() -> eframe::Result {
    // Style codé en dur pour l'instant (configurable à l'étape 8).
    // POUR TESTER : décommente une ligne pour changer un réglage, par exemple :
    let style = DisplayStyle {
        // background: egui::Color32::TRANSPARENT,    // fond transparent
        // corner_radius: 0,                          // coins carrés
        // anchor: display::Anchor::Right,            // ancrage à droite
        // template: "{value}[ ({ratio:p})]".into(),  // autre format
        ..DisplayStyle::default()
    };

    let shortcuts = default_shortcuts();
    // Le nom de la touche de validation est TIRÉ DES RACCOURCIS : si on la change, le message suit.
    // `.find(...)` : le premier raccourci qui valide un reset (`None` s'il n'y en a pas).
    let confirm_key_name = shortcuts
        .iter()
        .find(|s| s.command == Command::ConfirmReset)
        .map(|s| i18n::shortcut_name(s, LANGUAGE))
        .unwrap_or_else(|| "?".into());

    // On relit la valeur sauvegardée du compteur global.
    let state_path = storage::state_path();
    let saved = state_path.as_deref().map(storage::load).unwrap_or_default();

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
        Box::new(move |cc| {
            // Accès direct à la fenêtre du système, et retrait des décorations de Windows 11
            // (liseré, coins, ombre, ligne du haut), visibles avec un fond transparent.
            // `.inspect(|w| ...)` : exécute une action sur la valeur si elle existe, puis la rend.
            let native = NativeWindow::from_window(cc).inspect(|w| w.remove_decorations());

            // Un CHANNEL : un tuyau entre threads. `sender` envoie, `receiver` reçoit.
            let (sender, receiver) = mpsc::channel();
            // Le contexte egui, pour pouvoir réveiller l'interface depuis le thread des raccourcis.
            // `clone()` : on en fait une copie (c'est en fait un pointeur partagé, la copie est légère).
            let ctx = cc.egui_ctx.clone();

            hotkeys::start(shortcuts, move |command| {
                // `let _ =` : on ignore volontairement le résultat (échec seulement si l'appli est fermée).
                let _ = sender.send(command);
                // Sans le focus, egui ne redessine pas tout seul : on le réveille.
                ctx.request_repaint();
            });

            Ok(Box::new(CounterWindow {
                // Valeur reprise de la sauvegarde ; objectif codé en dur à 10 pour tester
                // (configurable à l'étape 8).
                counter: Counter::new(saved.global_value, 10),
                commands: receiver,
                audio: AudioEngine::new(),
                style,
                native,
                reset_confirmation: ResetConfirmation::default(),
                confirm_key_name,
                state_path,
            }))
        }),
    )
}
