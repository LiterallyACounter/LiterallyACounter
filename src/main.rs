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
mod placement;
mod reset;
mod storage;
mod tracker;

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use audio::AudioEngine;
use command::Command;
use counter::{Counter, CounterAction};
use display::DisplayStyle;
use eframe::egui;
use hotkeys::{Key, Shortcut};
use i18n::{Language, Text};
use native_window::NativeWindow;
use storage::SavedState;
use tracker::{CounterId, Targets, Tracker};
use winit::event_loop::{DeviceEvents, EventLoop};

/// Langue de l'interface (détectée et réglable à l'étape 10).
const LANGUAGE: Language = Language::French;

/// Taille des fenêtres compteurs à leur création ; elle s'ajuste au texte dès la première image.
const INITIAL_COUNTER_SIZE: egui::Vec2 = egui::vec2(200.0, 100.0);

/// Objectifs codés en dur pour tester (configurables à l'étape 8). 0 = pas d'objectif.
const GLOBAL_GOAL: u64 = 10;
const ROUND_GOAL: u64 = 3;

/// Identifiant egui de la fenêtre principale.
/// (Il est calculé par une fonction : `from_hash_of` ne peut pas servir dans une `const`.)
fn main_viewport() -> egui::ViewportId {
    egui::ViewportId::from_hash_of("main")
}

/// Une fenêtre de compteur.
struct CounterWindow {
    id: CounterId,
    /// Identifiant egui de la fenêtre (un « viewport » = une fenêtre du système).
    viewport: egui::ViewportId,
    /// Titre de la fenêtre, UNIQUE dans l'appli : il sert aussi à retrouver la fenêtre système.
    title: String,
    /// La fenêtre doit-elle être affichée ? (case de la fenêtre principale, sauvegardée)
    visible: bool,
    /// Position choisie à l'ouverture. `None` : pas encore ouverte, ou cachée.
    opened_at: Option<egui::Pos2>,
    /// Accès direct à la fenêtre du système, une fois retrouvée (`None` avant, ou hors Windows).
    native: Option<NativeWindow>,
}

impl CounterWindow {
    fn new(id: CounterId, visible: bool) -> Self {
        let (key, title) = match id {
            CounterId::Global => ("global", Text::GlobalWindowTitle),
            CounterId::Round => ("round", Text::RoundWindowTitle),
        };
        Self {
            id,
            viewport: egui::ViewportId::from_hash_of(key),
            title: i18n::message(title, LANGUAGE, &[]),
            visible,
            opened_at: None,
            native: None,
        }
    }

    /// Cache la fenêtre. Elle sera replacée (sans chevauchement) à sa réouverture.
    fn hide(&mut self) {
        self.visible = false;
        self.opened_at = None;
        self.native = None;
    }
}

/// L'état de l'application.
///
/// Organisation des fenêtres : la fenêtre « racine » d'eframe est une fenêtre HÔTE invisible
/// (1 pixel transparent, hors de la barre des tâches et d'Alt + Tab). Toutes les fenêtres
/// visibles (principale, global, partie) sont des « viewports immediate » : egui les dessine
/// pendant le passage de la fenêtre hôte. Pourquoi cet hôte ? eframe ralentit une fenêtre
/// racine réduite (au plus une image tous les 100 ms) : si la fenêtre principale était la racine,
/// la réduire retarderait l'affichage ET le son de chaque appui sur une touche.
struct App {
    tracker: Tracker,
    /// Bout « réception » du channel : les commandes envoyées par le thread des raccourcis arrivent ici.
    commands: Receiver<Command>,
    /// Le moteur audio. `None` s'il n'y a aucune sortie audio : l'appli marche alors sans son.
    audio: Option<AudioEngine>,
    /// Affichage du global.
    global_style: DisplayStyle,
    /// Affichage propre de la partie (utilisé quand la copie est désactivée).
    round_style: DisplayStyle,
    /// « La partie copie l'affichage du global ».
    round_copies_global: bool,
    /// Les deux fenêtres de compteur, dans l'ordre de `CounterId::ALL`.
    /// `[T; 2]` : un tableau de taille FIXE (2), connue à la compilation.
    windows: [CounterWindow; 2],
    /// Nom de la touche de validation, tiré des raccourcis configurés (ex. « Num Entrée »).
    confirm_key_name: String,
    /// Fichier de sauvegarde (`None` si le système n'a pas de dossier de configuration).
    state_path: Option<PathBuf>,
}

impl App {
    /// Exécute une commande. Toutes les commandes (clavier, souris, clics) passent par ici.
    /// `ctx` : le contexte egui, pour pouvoir réveiller l'affichage plus tard.
    fn handle(&mut self, command: Command, now: Instant, ctx: &egui::Context) {
        // La logique est dans `Tracker` (testée automatiquement) ; ici, on exécute ce qu'elle
        // demande : jouer un son, sauvegarder, prévoir un redessin.
        let outcome = self.tracker.handle(command, now);
        // `if let A && let B` : les deux motifs doivent correspondre (un son ET un moteur audio).
        if let Some(slot) = outcome.sound
            && let Some(audio) = &mut self.audio
        {
            audio.play(slot);
        }
        if outcome.save {
            self.save();
        }
        if outcome.reset_prompt_started {
            // À l'expiration du délai, il faut redessiner pour effacer « Reset ? », même si
            // rien d'autre ne se passe : un petit thread attend, puis réveille l'affichage.
            let ctx = ctx.clone();
            std::thread::spawn(move || {
                std::thread::sleep(reset::CONFIRM_DELAY + Duration::from_millis(50));
                ctx.request_repaint();
            });
        }
    }

    /// Sauvegarde l'état. En cas d'échec, on le signale sans planter : l'appli continue
    /// de compter, et réessaiera au prochain changement.
    fn save(&self) {
        let Some(path) = &self.state_path else {
            return;
        };
        // Déstructuration d'un tableau : on nomme ses deux éléments d'un coup.
        let [global, round] = &self.windows;
        let state = SavedState {
            global_value: self.tracker.counter(CounterId::Global).value(),
            global_style: self.global_style.clone(),
            round_style: self.round_style.clone(),
            round_copies_global: self.round_copies_global,
            show_global: global.visible,
            show_round: round.visible,
        };
        if let Err(error) = storage::save(path, &state) {
            eprintln!("Sauvegarde impossible : {error}");
        }
    }

    /// Le style effectivement utilisé par un compteur : la partie prend celui du global quand
    /// la copie est activée. L'objectif, lui, n'est pas dans le style : il n'est donc jamais copié.
    fn style(&self, id: CounterId) -> &DisplayStyle {
        effective_style(
            id,
            &self.global_style,
            &self.round_style,
            self.round_copies_global,
        )
    }

    /// Le texte à afficher et sa couleur : la demande de reset si elle est en attente
    /// (global seulement), sinon le compteur formaté (ex. « 3 / 10 »).
    fn text_and_color(&self, id: CounterId, now: Instant) -> (String, egui::Color32) {
        let style = self.style(id);
        if id == CounterId::Global && self.tracker.reset_pending(now) {
            let prompt = i18n::message(
                Text::ResetPrompt,
                LANGUAGE,
                &[("key", &self.confirm_key_name)],
            );
            return (prompt, style.alert);
        }
        let counter = self.tracker.counter(id);
        let text = format::render(
            &style.template,
            format::Values {
                value: counter.value(),
                goal: counter.goal(),
            },
            LANGUAGE,
        );
        (text, style.text_color(counter.goal_reached()))
    }

    /// La fenêtre principale (provisoire : la vraie configuration arrive à l'étape 8).
    fn main_window(&mut self, ctx: &egui::Context) {
        let builder = egui::ViewportBuilder::default()
            .with_title("Literally A Counter")
            .with_inner_size([340.0, 150.0]);
        // `show_viewport_immediate` : affiche une fenêtre supplémentaire, dessinée TOUT DE SUITE
        // par la closure. La closure peut donc emprunter `self` (pas besoin de partager les
        // données entre threads, contrairement aux viewports « deferred »).
        ctx.show_viewport_immediate(main_viewport(), builder, |ui, _class| {
            // Fermer la fenêtre principale (✕) quitte l'appli.
            if ui.input(|i| i.viewport().close_requested()) {
                quit(ui.ctx());
            }
            egui::CentralPanel::default().show(ui, |ui| {
                let mut changed = false;
                // `&mut self.windows` : on parcourt les fenêtres en pouvant les modifier.
                for window in &mut self.windows {
                    let label = match window.id {
                        CounterId::Global => Text::ShowGlobal,
                        CounterId::Round => Text::ShowRound,
                    };
                    let mut visible = window.visible;
                    // `checkbox` modifie `visible` quand on clique ; `.changed()` le signale.
                    if ui
                        .checkbox(&mut visible, i18n::message(label, LANGUAGE, &[]))
                        .changed()
                    {
                        if visible {
                            window.visible = true;
                        } else {
                            window.hide();
                        }
                        changed = true;
                    }
                }
                let label = i18n::message(Text::RoundCopiesGlobal, LANGUAGE, &[]);
                // `|=` : « ou » cumulatif (`changed` reste vrai s'il l'était déjà).
                changed |= ui.checkbox(&mut self.round_copies_global, label).changed();
                ui.separator();
                if ui
                    .button(i18n::message(Text::Quit, LANGUAGE, &[]))
                    .clicked()
                {
                    quit(ui.ctx());
                }
                if changed {
                    self.save();
                }
            });
        });
    }

    /// Où ouvrir la fenêtre n° `index` : à côté des fenêtres déjà ouvertes, sans les chevaucher.
    /// `None` : une fenêtre ouverte n'a pas encore de position connue (elle vient d'être créée) ;
    /// on réessaiera à l'image suivante.
    fn opening_position(&self, ctx: &egui::Context, index: usize) -> Option<egui::Pos2> {
        // Les autres compteurs déjà ouverts.
        // `enumerate` : donne (numéro, élément) ; `filter` garde les autres fenêtres ouvertes.
        let others = self
            .windows
            .iter()
            .enumerate()
            .filter(|(i, w)| *i != index && w.opened_at.is_some())
            .map(|(_, w)| w.viewport);
        let mut open = Vec::new();
        // La fenêtre principale d'abord (`once` : une suite d'un seul élément), puis les autres.
        for viewport in std::iter::once(main_viewport()).chain(others) {
            // `input_for` : lit l'état d'une AUTRE fenêtre que celle en cours de dessin.
            // `?` : position inconnue → on renvoie `None` tout de suite.
            open.push(ctx.input_for(viewport, |i| i.viewport().outer_rect)?);
        }
        let screen = ctx.input_for(main_viewport(), |i| i.viewport().monitor_size);
        Some(placement::opening_position(
            &open,
            screen,
            INITIAL_COUNTER_SIZE,
        ))
    }

    /// Affiche (ou non) la fenêtre de compteur n° `index`.
    fn counter_window(&mut self, ctx: &egui::Context, index: usize, now: Instant) {
        if !self.windows[index].visible {
            // Une fenêtre qu'on ne montre plus est fermée par egui.
            return;
        }
        // Position d'ouverture, choisie une seule fois.
        let position = match self.windows[index].opened_at {
            Some(position) => position,
            None => match self.opening_position(ctx, index) {
                Some(position) => {
                    self.windows[index].opened_at = Some(position);
                    position
                }
                None => {
                    ctx.request_repaint(); // pour réessayer tout de suite
                    return;
                }
            },
        };
        let window = &self.windows[index];
        let builder = egui::ViewportBuilder::default()
            .with_title(window.title.clone())
            .with_position(position)
            .with_inner_size(INITIAL_COUNTER_SIZE)
            .with_decorations(false) // pas de bordure ni de barre de titre
            .with_always_on_top() // toujours au premier plan
            .with_resizable(false)
            // Fenêtre TOUJOURS transparente (demandé à la création) : on dessine nous-mêmes
            // le fond, ce qui permet les coins arrondis et le fond transparent.
            .with_transparent(true);
        let viewport = window.viewport;
        ctx.show_viewport_immediate(viewport, builder, |ui, _class| {
            self.counter_ui(ui, index, now);
        });
    }

    /// Le contenu d'une fenêtre de compteur, appelé à chaque image de cette fenêtre.
    fn counter_ui(&mut self, ui: &mut egui::Ui, index: usize, now: Instant) {
        let window = &mut self.windows[index];
        // Retrait des décorations de Windows 11 (liseré, coins, ombre, ligne du haut), une fois
        // la fenêtre système retrouvée par son titre (elle existe dès la première image).
        if window.native.is_none() {
            window.native =
                NativeWindow::find_by_title(&window.title).inspect(|w| w.remove_decorations());
        }

        // Échap ou fermeture par le système (Alt + F4) : on CACHE la fenêtre (la case de la
        // fenêtre principale se décoche), on ne quitte pas l'appli.
        if ui.input(|i| i.key_pressed(egui::Key::Escape) || i.viewport().close_requested()) {
            window.hide();
            self.save();
            return;
        }

        let id = window.id;
        // 1) Le texte à afficher et sa couleur.
        let (text, color) = self.text_and_color(id, now);
        // `clone()` : une copie du style, pour ne plus emprunter `self` pendant le dessin
        // (on aura besoin de `&mut self` pour exécuter les clics, plus bas).
        let style = self.style(id).clone();
        let native = self.windows[index].native.as_ref();

        // 2) On MESURE et met en page le texte (un `Galley` : le texte prêt à dessiner).
        let galley = if id == CounterId::Global && self.tracker.reset_pending(now) {
            // Message de reset : la fenêtre GARDE sa taille (sinon elle peut sortir de l'écran).
            // C'est le texte qui s'adapte : retour à la ligne, puis police réduite si besoin.
            layout_in_current_window(ui, &style, text, color)
        } else {
            // Compteur : texte sur une ligne, et c'est la fenêtre qui s'ajuste à lui,
            // en respectant l'ancrage.
            let font = egui::FontId::proportional(style.font_size);
            let galley = ui.painter().layout_no_wrap(text, font, color);
            let margins = egui::Vec2::splat(2.0 * style.margin);
            fit_window(
                ui.ctx(),
                native,
                style.anchor,
                (galley.size() + margins).ceil(),
            );
            galley
        };

        // 3) On dessine. Les clics sont notés dans `clicked`, et exécutés APRÈS le dessin.
        let mut clicked = None;
        // `Frame::NONE.fill(...)` : un panneau sans marge, rempli avec la couleur de fond,
        // avec des coins arrondis (`CornerRadius::same` = le même rayon pour les 4 coins).
        let panel_frame = egui::Frame::NONE
            .fill(style.background)
            .corner_radius(egui::CornerRadius::same(style.corner_radius));
        egui::CentralPanel::default()
            .frame(panel_frame)
            .show(ui, |ui| {
                // Le texte, centré dans la fenêtre.
                // `galley.rect` : la zone occupée par le texte, relative au point de dessin.
                // On décale ce point pour que le centre du texte tombe au centre de la fenêtre.
                let rect = ui.max_rect();
                let position = rect.center() - galley.rect.center().to_vec2();
                ui.painter().galley(position, galley, color);

                // PUIS on rend toute la surface cliquable et déplaçable.
                // Pour egui, ce qui est ajouté en dernier est « au-dessus » et reçoit les clics en priorité.
                let response = ui.interact(
                    ui.max_rect(),
                    egui::Id::unique("counter_area"), // identifiant unique de cette zone pour egui
                    egui::Sense::click_and_drag(),
                );
                // Glisser : on demande au système de déplacer la fenêtre (comme avec une barre de titre).
                if response.drag_started() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }
                // Clic gauche : +1 sur CE compteur. Clic droit (« secondary ») : -1.
                // (Provisoire : à l'étape 8, le clic droit ouvrira un menu.)
                if response.clicked() {
                    clicked = Some(CounterAction::Add(1));
                }
                if response.secondary_clicked() {
                    clicked = Some(CounterAction::Subtract(1));
                }
            });
        if let Some(action) = clicked {
            let command = Command::Count(Targets::only(id), action);
            self.handle(command, now, ui.ctx());
        }
    }
}

/// Le style à utiliser pour un compteur (voir `App::style`). Fonction libre, donc testable
/// sans créer de fenêtre.
/// `'a` : une « durée de vie ». Elle dit au compilateur que le style renvoyé est emprunté
/// à l'un des deux styles reçus (il vit donc aussi longtemps qu'eux).
fn effective_style<'a>(
    id: CounterId,
    global: &'a DisplayStyle,
    round: &'a DisplayStyle,
    round_copies_global: bool,
) -> &'a DisplayStyle {
    match id {
        CounterId::Global => global,
        CounterId::Round if round_copies_global => global,
        CounterId::Round => round,
    }
}

/// Quitte l'appli : on ferme la fenêtre racine (l'hôte), ce qui ferme tout.
fn quit(ctx: &egui::Context) {
    ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Close);
}

/// Redimensionne la fenêtre en cours de dessin à `desired_size` si besoin, en gardant fixe
/// le côté d'ancrage.
fn fit_window(
    ctx: &egui::Context,
    native: Option<&NativeWindow>,
    anchor: display::Anchor,
    desired_size: egui::Vec2,
) {
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

    let new_left = display::anchored_left(outer.left(), inner.width(), desired_size.x, anchor);
    match native {
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
    ui: &egui::Ui,
    style: &DisplayStyle,
    text: String,
    color: egui::Color32,
) -> std::sync::Arc<egui::Galley> {
    // La place disponible : la fenêtre actuelle, moins les marges.
    let window = ui
        .ctx()
        .input(|i| i.viewport().inner_rect)
        .map_or(egui::vec2(200.0, 100.0), |r| r.size());
    let available = (window - egui::Vec2::splat(2.0 * style.margin)).max(egui::Vec2::ZERO);

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
    let size = display::largest_fitting_font_size(style.font_size, 8.0, fits);
    layout(size)
}

impl eframe::App for App {
    /// La couleur avec laquelle les fenêtres sont effacées avant chaque image : TRANSPARENTE.
    /// Le fond visible (avec ses coins arrondis) est dessiné ensuite, par nous.
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Color32::TRANSPARENT.to_normalized_gamma_f32()
    }

    // Appelée à chaque image de la fenêtre hôte. Elle ne dessine rien elle-même : elle exécute
    // les commandes reçues, puis affiche les autres fenêtres.
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let now = Instant::now();
        // Une copie du contexte (un pointeur partagé, la copie est légère), pour ne pas
        // emprunter `ui` pendant qu'on appelle des méthodes qui modifient `self`.
        let ctx = ui.ctx().clone();

        // On exécute toutes les commandes reçues depuis la dernière image.
        // `try_recv` ne bloque pas : il renvoie `Ok(commande)` s'il y en a une, une erreur sinon.
        // `while let` : boucle tant que le motif correspond (ici, tant qu'il y a des commandes).
        while let Ok(command) = self.commands.try_recv() {
            self.handle(command, now, &ctx);
        }

        self.main_window(&ctx);
        for index in 0..self.windows.len() {
            self.counter_window(&ctx, index, now);
        }
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
    // Les commandes « compter » visent les DEUX compteurs (partie + global), spec M2 / M3.
    let both = |action| Command::Count(Targets::BOTH, action);
    // `vec![...]` : macro qui crée un `Vec` avec les éléments donnés.
    vec![
        shortcut(&[], Key::NUMPAD_ADD, both(CounterAction::Add(1))),
        shortcut(&[], Key::NUMPAD_SUBTRACT, both(CounterAction::Subtract(1))),
        shortcut(
            &[Key::CONTROL],
            Key::NUMPAD_ADD,
            both(CounterAction::Add(5)),
        ),
        shortcut(&[], Key::NUMPAD_0, Command::Reset(CounterId::Round)),
        shortcut(
            &[Key::CONTROL],
            Key::NUMPAD_0,
            Command::Reset(CounterId::Global),
        ),
        shortcut(&[], Key::NUMPAD_ENTER, Command::ConfirmReset),
        shortcut(&[Key::CONTROL], Key::NUMPAD_DECIMAL, Command::UndoReset),
        // Souris (étape 6).
        shortcut(
            &[Key::CONTROL],
            Key::MOUSE_MIDDLE,
            both(CounterAction::Add(1)),
        ),
        shortcut(&[], Key::MOUSE_X2, both(CounterAction::Add(1))),
        shortcut(&[], Key::MOUSE_X1, both(CounterAction::Subtract(1))),
    ]
}

fn main() -> eframe::Result {
    // On écarte les raccourcis non autorisés (clic gauche / droit sans touche de modification).
    // `partition` : sépare une liste en deux selon une condition (autorisés / refusés).
    let (shortcuts, refused): (Vec<_>, Vec<_>) = default_shortcuts()
        .into_iter()
        .partition(Shortcut::is_allowed);
    for shortcut in &refused {
        eprintln!(
            "Raccourci ignoré (clic gauche/droit sans Ctrl, Maj ou Alt) : {}",
            i18n::shortcut_name(shortcut, LANGUAGE)
        );
    }
    // Le nom de la touche de validation est TIRÉ DES RACCOURCIS : si on la change, le message suit.
    // `.find(...)` : le premier raccourci qui valide un reset (`None` s'il n'y en a pas).
    let confirm_key_name = shortcuts
        .iter()
        .find(|s| s.command == Command::ConfirmReset)
        .map(|s| i18n::shortcut_name(s, LANGUAGE))
        .unwrap_or_else(|| "?".into());

    // On relit la sauvegarde (valeur du global, styles, fenêtres ouvertes).
    // POUR TESTER un autre style : modifie `global_style` / `round_style` dans le fichier
    // %APPDATA%\LiterallyACounter\state.json, appli fermée.
    let state_path = storage::state_path();
    let saved = state_path.as_deref().map(storage::load).unwrap_or_default();

    // La fenêtre hôte (racine) : 1 pixel transparent, qui laisse passer les clics,
    // hors de la barre des tâches, et qui ne prend pas le focus.
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Literally A Counter (host)")
            .with_inner_size([1.0, 1.0])
            .with_position([0.0, 0.0])
            .with_decorations(false)
            .with_resizable(false)
            .with_transparent(true)
            .with_mouse_passthrough(true)
            .with_taskbar(false)
            .with_active(false),
        ..Default::default()
    };

    // On crée nous-mêmes la « boucle d'événements » (la boucle qui reçoit les messages de Windows)
    // au lieu de laisser `eframe::run_native` le faire, pour pouvoir la régler AVANT de démarrer :
    // par défaut, winit s'abonne à TOUS les mouvements bruts de la souris quand une fenêtre a le
    // focus, et eframe redessine à chaque mouvement (jusqu'à 20 % d'un cœur mesuré). On n'en a pas
    // besoin : `DeviceEvents::Never` coupe cet abonnement.
    // `?` : en cas d'erreur, on quitte `main` tout de suite en renvoyant cette erreur.
    let event_loop = EventLoop::<eframe::UserEvent>::with_user_event().build()?;
    event_loop.listen_device_events(DeviceEvents::Never);

    let mut app = eframe::create_native(
        "Literally A Counter",
        options,
        // `cc` (« creation context ») donne accès au contexte egui au démarrage.
        Box::new(move |cc| {
            // L'hôte : sans décorations Windows 11, et absent d'Alt + Tab.
            if let Some(host) = NativeWindow::from_window(cc) {
                host.remove_decorations();
                host.hide_from_task_switcher();
            }

            // Un CHANNEL : un tuyau entre threads. `sender` envoie, `receiver` reçoit.
            let (sender, receiver) = mpsc::channel();
            // Le contexte egui, pour pouvoir réveiller l'interface depuis le thread des raccourcis.
            let ctx = cc.egui_ctx.clone();

            hotkeys::start(shortcuts, move |command| {
                // `let _ =` : on ignore volontairement le résultat (échec seulement si l'appli est fermée).
                let _ = sender.send(command);
                // Sans le focus, egui ne redessine pas tout seul : on le réveille.
                ctx.request_repaint();
            });

            Ok(Box::new(App {
                tracker: Tracker::new(
                    Counter::new(saved.global_value, GLOBAL_GOAL),
                    // La partie repart TOUJOURS de 0 (sa valeur n'est jamais sauvegardée).
                    Counter::new(0, ROUND_GOAL),
                ),
                commands: receiver,
                audio: AudioEngine::new(),
                windows: [
                    CounterWindow::new(CounterId::Global, saved.show_global),
                    CounterWindow::new(CounterId::Round, saved.show_round),
                ],
                global_style: saved.global_style,
                round_style: saved.round_style,
                round_copies_global: saved.round_copies_global,
                confirm_key_name,
                state_path,
            }))
        }),
        &event_loop,
    );
    event_loop.run_app(&mut app)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_copies_the_global_style_when_enabled() {
        let global = DisplayStyle::default();
        let round = DisplayStyle::round_default();
        let style = effective_style(CounterId::Round, &global, &round, true);
        assert_eq!(style, &global);
    }

    #[test]
    fn round_uses_its_own_style_when_copy_is_disabled() {
        let global = DisplayStyle::default();
        let round = DisplayStyle::round_default();
        assert_eq!(
            effective_style(CounterId::Round, &global, &round, false),
            &round
        );
        // Le global garde toujours son propre style.
        assert_eq!(
            effective_style(CounterId::Global, &global, &round, false),
            &global
        );
    }
}
