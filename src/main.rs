// Attribut appliqué à tout le programme (le `!` après `#` = « s'applique au fichier entier »).
// En mode release (`not(debug_assertions)`), Windows lance l'exe comme une appli graphique,
// donc SANS console noire derrière. En debug, on garde la console pour voir les messages.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// `mod xxx;` : déclare un module. Son code est dans `src/xxx.rs` ou `src/xxx/mod.rs`.
mod audio;
mod command;
mod config_window;
mod counter;
mod display;
mod file_dialog;
mod fonts;
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
use config_window::{CaptureRefused, CaptureTarget, ShortcutRequest, SoundCatalog, SoundRequest};
use counter::{Counter, CounterAction};
use display::DisplayStyle;
use eframe::egui;
use hotkeys::{Control, HotkeyEvent, HotkeyHandle, Key, MouseMethod, Shortcut};
use i18n::Text;
use native_window::NativeWindow;
use storage::SavedState;
use tracker::{CounterId, Targets, Tracker};
use winit::event_loop::{DeviceEvents, EventLoop};

/// Taille des fenêtres compteurs à leur création ; elle s'ajuste au texte dès la première image.
const INITIAL_COUNTER_SIZE: egui::Vec2 = egui::vec2(200.0, 100.0);

/// Les onglets de la fenêtre de configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConfigTab {
    Counters,
    Shortcuts,
    Sound,
    Info,
}

/// Taille de la fenêtre du menu d'un compteur, sans la liste des raccourcis.
const MENU_BASE_SIZE: egui::Vec2 = egui::vec2(330.0, 190.0);
/// Hauteur d'une ligne de la liste des raccourcis dans le menu (mesurée à l'écran).
const MENU_ROW_HEIGHT: f32 = 24.0;
/// Hauteur ajoutée par la section des raccourcis (titre, bouton « Modifier… »).
const MENU_SECTION_HEIGHT: f32 = 22.0;

/// Taille du menu quand il liste `rows` lignes de raccourcis (un raccourci par ligne).
fn menu_size(rows: usize) -> egui::Vec2 {
    MENU_BASE_SIZE + egui::vec2(0.0, MENU_SECTION_HEIGHT + rows as f32 * MENU_ROW_HEIGHT)
}

/// Le menu ouvert d'un compteur.
struct CounterMenu {
    /// Le compteur concerné (numéro dans `App::windows`).
    index: usize,
    /// La valeur en cours de saisie.
    value: u64,
    /// Position du menu à l'écran, choisie à l'ouverture.
    position: egui::Pos2,
    /// Taille du menu, calculée à l'ouverture selon le nombre de raccourcis listés.
    size: egui::Vec2,
    /// Le menu a-t-il déjà eu le focus ? (Il se ferme quand il le PERD, pas avant de l'avoir eu.)
    was_focused: bool,
    native: Option<NativeWindow>,
}

/// Ce que l'utilisateur a choisi dans le menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MenuAction {
    SetValue(u64),
    Reset,
    SetAlwaysOnTop(bool),
    Hide,
    OpenSettings,
    EditShortcuts,
    Close,
}

/// Taille du bouton « ••• ».
const MENU_BUTTON_SIZE: egui::Vec2 = egui::vec2(22.0, 14.0);

/// Le bouton « ••• » d'un compteur. Il vit dans SA PROPRE mini-fenêtre, posée sur le coin du
/// compteur et exclue des captures d'écran : OBS (et toute capture) ne le voit jamais.
/// Il n'existe que pendant le survol : le reste du temps, aucune fenêtre, aucun coût.
struct MenuButton {
    /// Le compteur concerné.
    index: usize,
    /// La souris était-elle sur le bouton à l'image précédente ? (Elle quitte alors la fenêtre
    /// du compteur : sans cela, le bouton disparaîtrait au moment où on veut cliquer dessus.)
    hovered: bool,
    native: Option<NativeWindow>,
}

/// Identifiant egui de la mini-fenêtre du bouton « ••• ».
fn menu_button_viewport() -> egui::ViewportId {
    egui::ViewportId::from_hash_of("menu_button")
}

/// Identifiant egui de la fenêtre du menu (un seul menu ouvert à la fois).
fn menu_viewport() -> egui::ViewportId {
    egui::ViewportId::from_hash_of("counter_menu")
}

/// Identifiant egui de la fenêtre principale (la configuration).
/// (Il est calculé par une fonction : `from_hash_of` ne peut pas servir dans une `const`.)
fn main_viewport() -> egui::ViewportId {
    egui::ViewportId::from_hash_of("main")
}

/// Une fenêtre de compteur.
struct CounterWindow {
    id: CounterId,
    /// Identifiant egui de la fenêtre (un « viewport » = une fenêtre du système).
    viewport: egui::ViewportId,
    /// La fenêtre doit-elle être affichée ? (case de la fenêtre principale, sauvegardée)
    visible: bool,
    /// Position choisie à l'ouverture. `None` : pas encore ouverte, ou cachée.
    opened_at: Option<egui::Pos2>,
    /// Accès direct à la fenêtre du système, une fois retrouvée (`None` avant, ou hors Windows).
    native: Option<NativeWindow>,
}

impl CounterWindow {
    fn new(id: CounterId, visible: bool) -> Self {
        let key = match id {
            CounterId::Global => "global",
            CounterId::Round => "round",
        };
        Self {
            id,
            viewport: egui::ViewportId::from_hash_of(key),
            visible,
            opened_at: None,
            native: None,
        }
    }

    /// Titre de la fenêtre, UNIQUE dans l'appli : il sert aussi à retrouver la fenêtre système.
    /// Calculé à chaque fois : il suit la langue choisie, même changée en cours de route.
    fn title(&self) -> String {
        i18n::text(match self.id {
            CounterId::Global => Text::GlobalWindowTitle,
            CounterId::Round => Text::RoundWindowTitle,
        })
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
    /// Bout « réception » du channel : les événements du thread des raccourcis arrivent ici
    /// (raccourci déclenché, capture terminée…).
    events: Receiver<HotkeyEvent>,
    /// Le moyen de piloter le thread des raccourcis (`None` s'il n'a pas pu démarrer).
    hotkeys: Option<HotkeyHandle>,
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
    /// Les raccourcis (modifiables dans la configuration).
    shortcuts: Vec<Shortcut>,
    /// Capture d'un raccourci en cours.
    capture: Option<CaptureTarget>,
    /// Message affiché après une capture refusée.
    capture_error: Option<String>,
    /// L'onglet affiché dans la configuration.
    tab: ConfigTab,
    /// Un réglage a changé et n'est pas encore sauvegardé. On attend que le bouton de la souris
    /// soit relâché : glisser un curseur change la valeur à chaque image, et on ne veut pas
    /// réécrire le fichier 60 fois par seconde.
    unsaved_changes: bool,
    /// Les polices installées, chargées à la demande.
    fonts: fonts::FontLibrary,
    /// Le menu d'un compteur, s'il est ouvert.
    menu: Option<CounterMenu>,
    /// Le bouton « ••• », s'il est affiché.
    menu_button: Option<MenuButton>,
    /// Le compteur survolé par la souris à cette image (`None` : aucun).
    hovered_counter: Option<usize>,
    /// Méthode d'écoute de la souris.
    mouse_method: MouseMethod,
    /// Fréquence des événements souris, si elle est élevée (protection automatique).
    mouse_rate: Option<u32>,
    /// « La partie utilise les sons du global ».
    round_copies_global_sounds: bool,
    /// Sections repliées / dépliées de la configuration.
    sections: config_window::Sections,
    /// Sortie audio choisie (`None` : celle par défaut de Windows).
    audio_output: Option<String>,
    /// Volume général, de 0.0 à 1.0.
    volume: f32,
    /// Muet général : aucun son (comptage, « Écouter », bip de test).
    all_muted: bool,
    /// Les sorties audio disponibles (relues quand on ouvre la liste).
    audio_outputs: Vec<audio::OutputDevice>,
    /// La sortie par défaut de Windows, relue au plus une fois par seconde dans l'onglet Son
    /// (la demander coûte ~4 ms).
    default_output: Option<(Instant, Option<String>)>,
    /// Message après le choix d'une sortie qui a refusé de s'ouvrir.
    output_error: Option<String>,
    /// Les sons proposés dans les listes.
    sound_catalog: SoundCatalog,
    /// La langue choisie (`None` : pas encore choisie, celle du système est utilisée).
    language: Option<String>,
    /// Les langues disponibles (intégrées et packs).
    languages: Vec<i18n::LanguageInfo>,
    /// Import en cours : le compteur et l'emplacement visés, et le résultat attendu de la
    /// fenêtre « Ouvrir ».
    pending_import: Option<(CounterId, audio::SoundSlot, Receiver<Option<PathBuf>>)>,
    /// Message après un import raté.
    import_error: Option<String>,
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
        if let Some((counter, slot)) = outcome.sound
            && !self.all_muted
            && let Some(audio) = &mut self.audio
        {
            audio.play(counter, slot);
        }
        if outcome.save {
            self.save_state();
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
    fn save_state(&self) {
        let Some(path) = &self.state_path else {
            return;
        };
        // Déstructuration d'un tableau : on nomme ses deux éléments d'un coup.
        let [global, round] = &self.windows;
        let state = SavedState {
            global_value: self.tracker.counter(CounterId::Global).value(),
            global_goal: self.tracker.counter(CounterId::Global).goal(),
            round_goal: self.tracker.counter(CounterId::Round).goal(),
            global_style: self.global_style.clone(),
            round_style: self.round_style.clone(),
            round_copies_global: self.round_copies_global,
            show_global: global.visible,
            show_round: round.visible,
            shortcuts: self.shortcuts.clone(),
            mouse_method: self.mouse_method,
            audio_output: self.audio_output.clone(),
            volume: self.volume,
            language: self.language.clone(),
            round_copies_global_sounds: Some(self.round_copies_global_sounds),
            open_sections: self.sections.clone(),
            all_muted: self.all_muted,
            sound_priority: self.tracker.sound_priority,
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
            let prompt = i18n::message(Text::ResetPrompt, &[("key", &self.confirm_key_name)]);
            return (prompt, style.alert);
        }
        let counter = self.tracker.counter(id);
        let text = format::render(
            &style.template,
            format::Values {
                value: counter.value(),
                goal: counter.goal(),
            },
            &i18n::number_format(),
        );
        (text, style.text_color(counter.goal_reached()))
    }

    /// La fenêtre principale : la configuration, en quatre onglets.
    fn main_window(&mut self, ctx: &egui::Context) {
        // Assez large pour les raccourcis en deux colonnes (touche, compteurs visés, ×).
        let builder = egui::ViewportBuilder::default()
            .with_title(i18n::text(Text::ConfigWindowTitle))
            .with_inner_size([720.0, 640.0])
            .with_min_inner_size([620.0, 240.0]);
        // `show_viewport_immediate` : affiche une fenêtre supplémentaire, dessinée TOUT DE SUITE
        // par la closure. La closure peut donc emprunter `self` (pas besoin de partager les
        // données entre threads, contrairement aux viewports « deferred »).
        ctx.show_viewport_immediate(main_viewport(), builder, |ui, _class| {
            // Fermer la fenêtre principale (✕) quitte l'appli.
            if ui.input(|i| i.viewport().close_requested()) {
                quit(ui.ctx());
            }
            egui::CentralPanel::default().show(ui, |ui| {
                // La ligne des onglets, avec « Quitter » tout à droite.
                ui.horizontal(|ui| {
                    // `selectable_value` : un bouton « sélectionné » quand `self.tab` vaut cette
                    // valeur ; un clic met `self.tab` à cette valeur.
                    for (tab, label) in [
                        (ConfigTab::Counters, Text::TabCounters),
                        (ConfigTab::Shortcuts, Text::TabShortcuts),
                        (ConfigTab::Sound, Text::TabSound),
                        (ConfigTab::Info, Text::TabInfo),
                    ] {
                        ui.selectable_value(&mut self.tab, tab, i18n::text(label));
                    }
                    // `right_to_left` : la suite est placée en partant de la droite.
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(i18n::text(Text::Quit)).clicked() {
                            quit(ui.ctx());
                        }
                    });
                });
                ui.separator();
                // Barre de défilement si la fenêtre est trop petite pour tout afficher.
                egui::ScrollArea::vertical().show(ui, |ui| match self.tab {
                    ConfigTab::Counters => {
                        for index in 0..self.windows.len() {
                            let key = match self.windows[index].id {
                                CounterId::Global => "counter_global",
                                CounterId::Round => "counter_round",
                            };
                            let title =
                                egui::RichText::new(i18n::text(match self.windows[index].id {
                                    CounterId::Global => Text::GlobalCounter,
                                    CounterId::Round => Text::RoundCounter,
                                }))
                                .heading();
                            // `std::mem::take` : on sort la carte des sections de `self` le temps
                            // de l'appel (et on la remet après), pour pouvoir prêter `self` en
                            // entier à la closure. Sinon, deux emprunts de `self` se chevauchent.
                            let mut sections = std::mem::take(&mut self.sections);
                            let open = config_window::is_open(&sections, key, true);
                            let mut changed = false;
                            let clicked = config_window::section(ui, key, title, open, |ui| {
                                changed |= self.counter_settings(ui, index, &mut sections);
                            });
                            if clicked {
                                sections.insert(key.to_string(), !open);
                                changed = true;
                            }
                            self.sections = sections;
                            self.unsaved_changes |= changed;
                            ui.add_space(8.0);
                        }
                    }
                    ConfigTab::Info => self.info_tab(ui),
                    ConfigTab::Sound => self.sound_settings(ui),
                    ConfigTab::Shortcuts => {
                        let mut sections = std::mem::take(&mut self.sections);
                        let request = config_window::shortcuts_editor(
                            ui,
                            &self.shortcuts,
                            self.capture,
                            self.capture_error.as_deref(),
                            self.mouse_method,
                            self.mouse_rate,
                            &mut sections,
                        );
                        if sections != self.sections {
                            self.unsaved_changes = true;
                        }
                        self.sections = sections;
                        if let Some(request) = request {
                            self.on_shortcut_request(request);
                        }
                    }
                });
            });
            // Sauvegarde dès que la souris est relâchée (voir `unsaved_changes`).
            if self.unsaved_changes && !ui.input(|i| i.pointer.any_down()) {
                self.save_state();
                self.unsaved_changes = false;
            }
        });
    }

    /// L'onglet Infos : nom et version, langue, mode d'emploi, crédits, dossier de l'appli.
    fn info_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Literally A Counter");
        // `env!` : une valeur connue à la COMPILATION (ici, la version écrite dans Cargo.toml).
        ui.label(i18n::message(
            Text::AppVersion,
            &[("version", env!("CARGO_PKG_VERSION"))],
        ));
        ui.add_space(8.0);
        egui::Grid::new("general_settings")
            .num_columns(2)
            .spacing([12.0, 10.0])
            .show(ui, |ui| {
                ui.label(i18n::text(Text::LanguageLabel));
                let current = i18n::current_code();
                let current_name = self
                    .languages
                    .iter()
                    .find(|l| l.code == current)
                    .map_or(current.clone(), |l| l.name.clone());
                let mut chosen = None;
                egui::ComboBox::from_id_salt("language")
                    .selected_text(current_name)
                    .show_ui(ui, |ui| {
                        for language in &self.languages {
                            if ui
                                .selectable_label(language.code == current, &language.name)
                                .clicked()
                            {
                                chosen = Some(language.code.clone());
                            }
                        }
                    });
                // Changement de langue : immédiat, et sauvegardé.
                if let Some(code) = chosen {
                    i18n::set_language(&code);
                    self.language = Some(code);
                    // La touche de validation a un nom traduit (« Num Entrée » / « Num Enter »).
                    self.confirm_key_name = confirm_key_name(&self.shortcuts);
                    self.save_state();
                }
                ui.end_row();
            });

        ui.add_space(12.0);
        ui.strong(i18n::text(Text::GuideTitle));
        for text in [
            Text::GuideCount,
            Text::GuideReset,
            Text::MenuHint,
            Text::GuideMove,
            Text::GuideStream,
        ] {
            // « • » en tête de ligne, et retour à la ligne automatique.
            ui.add(egui::Label::new(format!("•  {}", i18n::text(text))).wrap());
        }

        ui.add_space(12.0);
        ui.add(egui::Label::new(egui::RichText::new(i18n::text(Text::Credits)).small()).wrap());
        ui.add_space(8.0);
        let folder = storage::state_path().and_then(|path| path.parent().map(|p| p.to_path_buf()));
        if let Some(folder) = folder
            && ui
                .button(i18n::text(Text::OpenAppFolder))
                .on_hover_text(i18n::text(Text::OpenAppFolderTooltip))
                .clicked()
        {
            // Le dossier peut ne pas exister encore (rien sauvegardé) : on le crée.
            let _ = std::fs::create_dir_all(&folder);
            // L'explorateur de fichiers de Windows, ouvert sur ce dossier.
            let _ = std::process::Command::new("explorer").arg(&folder).spawn();
        }
    }

    /// L'onglet Son et tonalités : sortie audio, volume, et les sons de chaque compteur.
    fn sound_settings(&mut self, ui: &mut egui::Ui) {
        if self.audio.is_none() {
            ui.colored_label(ui.visuals().warn_fg_color, i18n::text(Text::NoAudio));
            ui.add_space(8.0);
        }
        egui::Grid::new("sound_settings")
            .num_columns(2)
            .spacing([12.0, 10.0])
            .show(ui, |ui| {
                ui.label(i18n::text(Text::AudioOutput));
                let default_label = i18n::text(Text::DefaultOutput);
                // Le nom de la sortie choisie ; « introuvable » si elle n'est plus branchée.
                let current = match &self.audio_output {
                    None => default_label.clone(),
                    Some(id) => match self.audio_outputs.iter().find(|o| &o.id == id) {
                        Some(output) => output.name.clone(),
                        None => i18n::text(Text::OutputMissing),
                    },
                };
                let mut chosen = None;
                egui::ComboBox::from_id_salt("audio_output")
                    .selected_text(current)
                    .width(300.0)
                    .show_ui(ui, |ui| {
                        // On relit la liste pendant qu'elle est ouverte : un casque branché
                        // entre-temps apparaît.
                        self.audio_outputs = audio::list_outputs();
                        if ui
                            .selectable_label(self.audio_output.is_none(), &default_label)
                            .clicked()
                        {
                            chosen = Some(None);
                        }
                        for output in &self.audio_outputs {
                            let selected = self.audio_output.as_ref() == Some(&output.id);
                            if ui.selectable_label(selected, &output.name).clicked() {
                                chosen = Some(Some(output.id.clone()));
                            }
                        }
                    });
                // `Some(choix)` : l'utilisateur a choisi (le choix lui-même peut être `None`).
                if let Some(output) = chosen {
                    self.choose_output(output);
                }
                ui.end_row();

                // Une sortie choisie a refusé de s'ouvrir : on le dit (la sélection est revenue
                // sur la précédente).
                if let Some(error) = &self.output_error {
                    ui.label("");
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(error).color(ui.visuals().warn_fg_color),
                        )
                        .wrap(),
                    );
                    ui.end_row();
                }

                // La sortie réellement utilisée diffère de celle voulue : on le dit.
                if let Some(used) = self.used_output_warning() {
                    ui.label("");
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(used).color(ui.visuals().warn_fg_color),
                        )
                        .wrap(),
                    );
                    ui.end_row();
                }

                ui.label(i18n::text(Text::Volume));
                // Le curseur travaille en pourcents (0 à 100), le moteur en 0.0 à 1.0.
                let mut percent = (self.volume * 100.0).round();
                // `ui.horizontal(...)` renvoie le résultat de la closure dans `.inner` :
                // ici, la réponse du curseur (pour savoir s'il a bougé ou été relâché).
                let response = ui
                    .horizontal(|ui| {
                        let response = ui.add(
                            egui::Slider::new(&mut percent, 0.0..=100.0)
                                .suffix(" %")
                                .step_by(1.0)
                                .fixed_decimals(0), // « 100 % » et non « 100.0 % »
                        );
                        // Le muet général, juste à droite du volume.
                        if config_window::mute_toggle(ui, &mut self.all_muted, Text::MuteAllTooltip)
                        {
                            self.save_state();
                        }
                        response
                    })
                    .inner;
                if response.changed() {
                    self.volume = percent / 100.0;
                    // Régler un nouveau volume réactive le son.
                    self.all_muted = false;
                    if let Some(audio) = &mut self.audio {
                        audio.set_volume(self.volume);
                    }
                }
                // Au RELÂCHEMENT du curseur (ou après une saisie au clavier) : bip de test,
                // puis sauvegarde.
                let released =
                    response.drag_stopped() || (response.changed() && !response.dragged());
                if released {
                    if let Some(audio) = &mut self.audio {
                        audio.play_test_beep();
                    }
                    self.save_state();
                }
                ui.end_row();

                // Le son joué quand un raccourci fait compter les deux compteurs.
                ui.label(i18n::text(Text::SoundPriority))
                    .on_hover_text(i18n::text(Text::SoundPriorityTooltip));
                ui.horizontal(|ui| {
                    for (id, label) in [
                        (CounterId::Global, Text::PriorityGlobal),
                        (CounterId::Round, Text::PriorityRound),
                    ] {
                        // `selectable_value` : un bouton « enfoncé » quand la valeur est la
                        // sienne ; un clic y met la valeur. Deux boutons = un interrupteur.
                        if ui
                            .selectable_value(
                                &mut self.tracker.sound_priority,
                                id,
                                i18n::text(label),
                            )
                            .on_hover_text(i18n::text(Text::SoundPriorityTooltip))
                            .changed()
                        {
                            self.save_state();
                        }
                    }
                });
                ui.end_row();
            });

        // Les sons de chaque compteur, dans des sections repliées par défaut.
        ui.add_space(8.0);
        if let Some(error) = &self.import_error {
            ui.add(
                egui::Label::new(egui::RichText::new(error).color(ui.visuals().warn_fg_color))
                    .wrap(),
            );
        }
        let mut changed = false;
        let open = config_window::is_open(&self.sections, "sounds_global", false);
        let clicked = config_window::section(
            ui,
            "sounds_global",
            i18n::text(Text::GlobalCounter),
            open,
            |ui| {
                let (sounds_changed, request) = config_window::sound_editor(
                    ui,
                    "global_sounds",
                    &mut self.global_style.sounds,
                    &self.sound_catalog,
                );
                changed |= sounds_changed;
                if let Some(request) = request {
                    self.on_sound_request(ui.ctx(), CounterId::Global, request);
                }
            },
        );
        if clicked {
            self.sections.insert("sounds_global".to_string(), !open);
            changed = true;
        }
        let open = config_window::is_open(&self.sections, "sounds_round", false);
        let clicked = config_window::section(
            ui,
            "sounds_round",
            i18n::text(Text::RoundCounter),
            open,
            |ui| {
                changed |= ui
                    .checkbox(
                        &mut self.round_copies_global_sounds,
                        i18n::text(Text::RoundUsesGlobalSounds),
                    )
                    .changed();
                // Sons du global utilisés : ceux de la partie restent visibles, mais grisés.
                let enabled = !self.round_copies_global_sounds;
                ui.add_enabled_ui(enabled, |ui| {
                    let (sounds_changed, request) = config_window::sound_editor(
                        ui,
                        "round_sounds",
                        &mut self.round_style.sounds,
                        &self.sound_catalog,
                    );
                    changed |= sounds_changed;
                    if let Some(request) = request {
                        self.on_sound_request(ui.ctx(), CounterId::Round, request);
                    }
                });
            },
        );
        if clicked {
            self.sections.insert("sounds_round".to_string(), !open);
            changed = true;
        }
        self.unsaved_changes |= changed;
    }

    /// Les sons effectivement utilisés par un compteur : la partie prend ceux du global quand
    /// « Utiliser les sons du global » est cochée.
    fn sounds(&self, id: CounterId) -> &audio::SoundSet {
        match id {
            CounterId::Round if !self.round_copies_global_sounds => &self.round_style.sounds,
            _ => &self.global_style.sounds,
        }
    }

    /// Exécute une demande de la section Sons d'un compteur.
    fn on_sound_request(&mut self, ctx: &egui::Context, id: CounterId, request: SoundRequest) {
        match request {
            SoundRequest::Preview(choice) => {
                if !self.all_muted
                    && let Some(audio) = &mut self.audio
                {
                    audio.preview(&choice);
                }
            }
            SoundRequest::Import(slot) => {
                // Une seule fenêtre « Ouvrir » à la fois.
                if self.pending_import.is_none() {
                    self.import_error = None;
                    let receiver = file_dialog::pick_sound_file(
                        ctx,
                        i18n::text(Text::ImportTitle),
                        i18n::text(Text::SoundFiles),
                    );
                    self.pending_import = Some((id, slot, receiver));
                }
            }
        }
    }

    /// La fenêtre « Ouvrir » a-t-elle répondu ? Si un fichier a été choisi, on l'importe et on
    /// le met sur l'emplacement visé.
    fn poll_import(&mut self) {
        let Some((id, slot, receiver)) = &self.pending_import else {
            return;
        };
        // `try_recv` : pas encore de réponse → on réessaiera à l'image suivante.
        let Ok(picked) = receiver.try_recv() else {
            return;
        };
        let (id, slot) = (*id, *slot);
        self.pending_import = None;
        let Some(path) = picked else {
            return; // annulé
        };
        let file_name = path
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        match audio::import_sound(&path) {
            Ok(name) => {
                let style = match id {
                    CounterId::Global => &mut self.global_style,
                    CounterId::Round => &mut self.round_style,
                };
                style.sounds.get_mut(slot).sound = audio::SoundChoice::Imported(name);
                self.sound_catalog = SoundCatalog::load();
                self.save_state();
            }
            Err(error) => {
                let text = match error {
                    audio::ImportError::Unreadable => Text::ImportUnreadable,
                    audio::ImportError::CopyFailed => Text::ImportCopyFailed,
                };
                self.import_error = Some(i18n::message(text, &[("name", &file_name)]));
            }
        }
    }

    /// Choisit une sortie audio (`None` : celle par défaut de Windows). Si une sortie précise
    /// refuse de s'ouvrir (réservée par une autre appli…), on revient à la sortie précédente et
    /// on le signale en nommant la sortie refusée.
    fn choose_output(&mut self, output: Option<String>) {
        let previous = self.audio_output.clone();
        self.output_error = None;
        match &mut self.audio {
            Some(audio) => audio.set_output(output.clone()),
            // Pas de son jusqu'ici : on retente d'ouvrir une sortie.
            None => self.audio = AudioEngine::new(output.clone(), self.volume),
        }
        let refused = match (&output, &self.audio) {
            (Some(id), Some(audio)) => audio.opened_output() != id,
            _ => false,
        };
        if refused {
            let name = self
                .audio_outputs
                .iter()
                .find(|o| Some(&o.id) == output.as_ref())
                .map_or_else(String::new, |o| o.name.clone());
            self.output_error = Some(i18n::message(Text::OutputRefused, &[("name", &name)]));
            // Retour à la sortie précédente.
            if let Some(audio) = &mut self.audio {
                audio.set_output(previous.clone());
            }
            self.audio_output = previous;
        } else {
            self.audio_output = output;
        }
        if !self.all_muted
            && let Some(audio) = &mut self.audio
        {
            audio.play_test_beep();
        }
        self.save_state();
    }

    /// Message si la sortie voulue (choisie, ou celle par défaut de Windows) n'a pas pu être
    /// ouverte : le son passe alors par une autre, qu'on nomme. `None` si tout va bien.
    fn used_output_warning(&mut self) -> Option<String> {
        let opened = self.audio.as_ref()?.opened_output().to_string();
        let wanted = match &self.audio_output {
            Some(id) => {
                // Choisie mais débranchée : déjà signalé par « Sortie introuvable ».
                if !self.audio_outputs.iter().any(|o| &o.id == id) {
                    return None;
                }
                id.clone()
            }
            None => {
                let stale = self
                    .default_output
                    .as_ref()
                    .is_none_or(|(at, _)| at.elapsed() > Duration::from_secs(1));
                if stale {
                    self.default_output = Some((Instant::now(), audio::default_output_id()));
                }
                self.default_output.as_ref()?.1.clone()?
            }
        };
        if wanted == opened {
            return None;
        }
        let name = self
            .audio_outputs
            .iter()
            .find(|o| o.id == opened)
            .map_or(opened.clone(), |o| o.name.clone());
        Some(i18n::message(Text::OutputUnavailable, &[("name", &name)]))
    }

    /// La section de configuration d'un compteur. Renvoie `true` si un réglage a changé.
    fn counter_settings(
        &mut self,
        ui: &mut egui::Ui,
        index: usize,
        sections: &mut config_window::Sections,
    ) -> bool {
        let mut changed = false;
        let window = &mut self.windows[index];
        let id = window.id;

        let mut visible = window.visible;
        // `checkbox` modifie `visible` quand on clique ; `.changed()` le signale.
        if ui
            .checkbox(&mut visible, i18n::text(Text::ShowWindow))
            .changed()
        {
            if visible {
                window.visible = true;
            } else {
                window.hide();
            }
            changed = true;
        }

        // L'objectif est dans le compteur : on en modifie une copie, puis on la renvoie.
        let mut goal = self.tracker.counter(id).goal();
        ui.horizontal(|ui| {
            ui.label(i18n::text(Text::Goal));
            if ui.add(egui::DragValue::new(&mut goal)).changed() {
                self.tracker.set_goal(id, goal);
                changed = true;
            }
        });

        // Les polices utilisées par les compteurs : à garder en mémoire.
        let in_use: Vec<String> = [&self.global_style.font, &self.round_style.font]
            .into_iter()
            .flatten() // `flatten` sur des `Option` : garde les `Some`, écarte les `None`
            .cloned()
            .collect();
        let in_use: Vec<&str> = in_use.iter().map(String::as_str).collect();
        if id == CounterId::Round {
            // `|=` : « ou » cumulatif (`changed` reste vrai s'il l'était déjà).
            changed |= ui
                .checkbox(
                    &mut self.round_copies_global,
                    i18n::text(Text::RoundCopiesGlobal),
                )
                .on_hover_text(i18n::text(Text::TooltipCopyDisplay))
                .changed();
        }
        // L'affichage, dans une section repliée par défaut.
        let (key, style, defaults) = match id {
            CounterId::Global => (
                "global_style",
                &mut self.global_style,
                DisplayStyle::default(),
            ),
            CounterId::Round => (
                "round_style",
                &mut self.round_style,
                DisplayStyle::round_default(),
            ),
        };
        // Copie activée : les réglages propres de la partie restent visibles, mais grisés
        // (`add_enabled_ui(false, ...)` désactive tout ce qui est dessiné dedans).
        let enabled = !(id == CounterId::Round && self.round_copies_global);
        let fonts = &mut self.fonts;
        let display_key = format!("{key}_display");
        let open = config_window::is_open(sections, &display_key, false);
        let clicked = config_window::section(
            ui,
            &display_key,
            i18n::text(Text::DisplaySection),
            open,
            |ui| {
                ui.add_enabled_ui(enabled, |ui| {
                    changed |= config_window::style_editor(
                        ui, key, style, fonts, &in_use, sections, &defaults,
                    );
                });
            },
        );
        if clicked {
            sections.insert(display_key, !open);
            changed = true;
        }
        changed
    }

    /// Exécute une demande de l'onglet Raccourcis.
    fn on_shortcut_request(&mut self, request: ShortcutRequest) {
        // Toute nouvelle action efface le message d'un refus précédent.
        self.capture_error = None;
        match request {
            ShortcutRequest::StartCapture(target) => {
                self.capture = Some(target);
                self.send_to_hotkeys(Control::StartCapture);
            }
            ShortcutRequest::CancelCapture => {
                self.capture = None;
                self.send_to_hotkeys(Control::CancelCapture);
            }
            ShortcutRequest::Remove(index) => {
                self.shortcuts.remove(index);
                self.shortcuts_changed();
            }
            ShortcutRequest::SetTargets(index, targets) => {
                if let Command::Count(_, action) = self.shortcuts[index].command {
                    self.shortcuts[index].command = Command::Count(targets, action);
                    self.shortcuts_changed();
                }
            }
            ShortcutRequest::RestoreDefaults => {
                self.shortcuts = hotkeys::default_shortcuts();
                self.shortcuts_changed();
            }
            ShortcutRequest::SetMouseMethod(method) => {
                self.mouse_method = method;
                // En passant à l'interrogation, l'abonnement s'arrête : plus d'avertissement.
                if method == MouseMethod::Polling {
                    self.mouse_rate = None;
                }
                self.send_to_hotkeys(Control::SetMouseMethod(method));
                self.save_state();
            }
        }
    }

    /// Une capture est terminée : on enregistre la combinaison (ou on explique le refus).
    fn on_captured(&mut self, held: Vec<Key>, trigger: Key) {
        // `take()` : la capture est finie, on récupère sa cible et on laisse `None`.
        let Some(target) = self.capture.take() else {
            return;
        };
        match config_window::apply_capture(&mut self.shortcuts, target, held, trigger) {
            Ok(()) => self.shortcuts_changed(),
            Err(CaptureRefused::MainClickNeedsModifier) => {
                self.capture_error = Some(i18n::text(Text::MainClickNeedsModifier));
            }
        }
    }

    /// Après toute modification des raccourcis : le thread du hook reçoit la nouvelle liste,
    /// le message « Reset ? » suit la touche de validation, et on sauvegarde.
    fn shortcuts_changed(&mut self) {
        self.send_to_hotkeys(Control::SetShortcuts(self.shortcuts.clone()));
        self.confirm_key_name = confirm_key_name(&self.shortcuts);
        self.save_state();
    }

    fn send_to_hotkeys(&self, control: Control) {
        if let Some(hotkeys) = &self.hotkeys {
            hotkeys.send(control);
        }
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
        // Premier plan ou non : réglé dans la configuration. egui compare ce réglage avec
        // celui de l'image précédente, et ne prévient Windows qu'en cas de changement.
        let level = if self.style(window.id).always_on_top {
            egui::WindowLevel::AlwaysOnTop
        } else {
            egui::WindowLevel::Normal
        };
        let builder = egui::ViewportBuilder::default()
            .with_title(window.title())
            .with_position(position)
            .with_inner_size(INITIAL_COUNTER_SIZE)
            .with_decorations(false) // pas de bordure ni de barre de titre
            .with_window_level(level)
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
                NativeWindow::find_by_title(&window.title()).inspect(|w| w.remove_decorations());
        }

        // Échap ou fermeture par le système (Alt + F4) : on CACHE la fenêtre (la case de la
        // fenêtre principale se décoche), on ne quitte pas l'appli.
        if ui.input(|i| i.key_pressed(egui::Key::Escape) || i.viewport().close_requested()) {
            window.hide();
            self.save_state();
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
            let family = self
                .fonts
                .font_id(ui.ctx(), style.font.as_deref(), style.font_size)
                .family;
            layout_in_current_window(ui, &style, family, text, color)
        } else {
            // Compteur : texte sur une ligne, et c'est la fenêtre qui s'ajuste à lui,
            // en respectant l'ancrage.
            let font = self
                .fonts
                .font_id(ui.ctx(), style.font.as_deref(), style.font_size);
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
        let mut hovered = false;
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
                if response.clicked() {
                    clicked = Some(CounterAction::Add(1));
                }
                if response.secondary_clicked() {
                    clicked = Some(CounterAction::Subtract(1));
                }

                // La souris survole-t-elle le compteur ? (Pour afficher le bouton « ••• ».)
                hovered = ui.rect_contains_pointer(rect);
            });
        if hovered {
            self.hovered_counter = Some(index);
        }
        if let Some(action) = clicked {
            let command = Command::Count(Targets::only(id), action);
            self.handle(command, now, ui.ctx());
        }
    }

    /// Le bouton « ••• » : affiché tant que la souris est sur un compteur ou sur le bouton.
    fn menu_button_window(&mut self, ctx: &egui::Context) {
        // Le compteur survolé, ou celui dont le bouton est survolé.
        let still_on_button = self
            .menu_button
            .as_ref()
            .filter(|button| button.hovered)
            .map(|button| button.index);
        let Some(index) = self.hovered_counter.or(still_on_button) else {
            // Plus de survol : on ne montre plus la mini-fenêtre, egui la ferme.
            self.menu_button = None;
            return;
        };
        // Un autre compteur qu'avant : on repart d'un bouton neuf.
        if self.menu_button.as_ref().is_none_or(|b| b.index != index) {
            self.menu_button = Some(MenuButton {
                index,
                hovered: false,
                native: None,
            });
        }
        // Place du compteur à l'écran : le bouton se pose dans son coin en haut à droite.
        let Some(counter) =
            ctx.input_for(self.windows[index].viewport, |i| i.viewport().outer_rect)
        else {
            return;
        };
        let position = egui::pos2(
            counter.right() - MENU_BUTTON_SIZE.x - 4.0,
            counter.top() + 4.0,
        );
        let title = i18n::text(Text::MenuButtonWindowTitle);
        let builder = egui::ViewportBuilder::default()
            .with_title(title.clone())
            .with_position(position)
            .with_inner_size(MENU_BUTTON_SIZE)
            .with_decorations(false)
            .with_transparent(true)
            .with_always_on_top()
            .with_resizable(false)
            .with_taskbar(false)
            .with_active(false); // ne prend pas le focus en apparaissant
        let mut clicked = false;
        ctx.show_viewport_immediate(menu_button_viewport(), builder, |ui, _class| {
            let Some(button) = &mut self.menu_button else {
                return;
            };
            if button.native.is_none() {
                button.native = NativeWindow::find_by_title(&title).inspect(|w| {
                    w.remove_decorations();
                    w.exclude_from_capture();
                });
            }
            // La souris est-elle dans cette mini-fenêtre ?
            button.hovered = ui.input(|i| i.pointer.hover_pos().is_some());
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE)
                .show(ui, |ui| {
                    clicked = draw_menu_button(ui, ui.max_rect()).clicked();
                });
        });
        if clicked {
            self.open_menu(ctx, index);
        }
    }

    /// Ouvre le menu du compteur n° `index`, sous le compteur.
    fn open_menu(&mut self, ctx: &egui::Context, index: usize) {
        let window = &self.windows[index];
        let id = window.id;
        // `outer_rect` : la place de la fenêtre du compteur à l'écran.
        let counter_rect = ctx.input_for(window.viewport, |i| i.viewport().outer_rect);
        let screen = ctx.input_for(window.viewport, |i| i.viewport().monitor_size);
        // Une ligne par raccourci listé.
        let rows = hotkeys::shortcuts_for(&self.shortcuts, id)
            .iter()
            .map(|(_, shortcuts)| shortcuts.len())
            .sum();
        let size = menu_size(rows);
        self.menu = counter_rect.map(|rect| CounterMenu {
            index,
            value: self.tracker.counter(id).value(),
            position: menu_position(rect, screen, size),
            size,
            was_focused: false,
            native: None,
        });
    }

    /// Le menu d'un compteur, dans sa propre petite fenêtre (un compteur est trop petit pour
    /// qu'on y dessine un menu : egui ne peut pas dessiner hors de la fenêtre).
    fn menu_window(&mut self, ctx: &egui::Context, now: Instant) {
        let Some(menu) = &self.menu else {
            return;
        };
        let index = menu.index;
        let id = self.windows[index].id;
        let title = i18n::text(Text::MenuWindowTitle);
        let builder = egui::ViewportBuilder::default()
            .with_title(title.clone())
            .with_position(menu.position)
            .with_inner_size(menu.size)
            .with_decorations(false)
            .with_always_on_top()
            .with_resizable(false)
            .with_taskbar(false);
        // Ce que l'utilisateur a choisi dans le menu, exécuté après le dessin.
        let mut action = None;
        ctx.show_viewport_immediate(menu_viewport(), builder, |ui, _class| {
            let Some(menu) = &mut self.menu else {
                return;
            };
            if menu.native.is_none() {
                menu.native = NativeWindow::find_by_title(&title).inspect(|w| {
                    w.remove_decorations();
                    // Le menu n'apparaît dans aucune capture (même une capture d'écran complète).
                    w.exclude_from_capture();
                });
            }
            // Fermeture : Échap, ou dès que le menu perd le focus (clic ailleurs).
            let focused = ui.input(|i| i.viewport().focused);
            if focused == Some(true) {
                menu.was_focused = true;
            }
            if ui.input(|i| i.key_pressed(egui::Key::Escape))
                || (menu.was_focused && focused == Some(false))
            {
                action = Some(MenuAction::Close);
                return;
            }

            let frame = egui::Frame::window(ui.style()).corner_radius(0);
            egui::CentralPanel::default().frame(frame).show(ui, |ui| {
                ui.strong(i18n::text(match id {
                    CounterId::Global => Text::GlobalCounter,
                    CounterId::Round => Text::RoundCounter,
                }));
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label(i18n::text(Text::Value));
                    let field = ui.add(egui::DragValue::new(&mut menu.value));
                    // Entrée dans le champ, ou le bouton « Valider ».
                    let entered =
                        field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    if ui.button(i18n::text(Text::Apply)).clicked() || entered {
                        action = Some(MenuAction::SetValue(menu.value));
                    }
                });
                if ui.button(i18n::text(Text::ResetAction)).clicked() {
                    action = Some(MenuAction::Reset);
                }
                // Les réglages d'affichage : grisés pour la partie quand elle copie le global.
                let display_editable = !(id == CounterId::Round && self.round_copies_global);
                let mut on_top = effective_style(
                    id,
                    &self.global_style,
                    &self.round_style,
                    self.round_copies_global,
                )
                .always_on_top;
                ui.add_enabled_ui(display_editable, |ui| {
                    if ui
                        .checkbox(&mut on_top, i18n::text(Text::AlwaysOnTop))
                        .changed()
                    {
                        action = Some(MenuAction::SetAlwaysOnTop(on_top));
                    }
                });
                ui.separator();
                // Les raccourcis de ce compteur (lecture seule), pour s'en souvenir en stream.
                ui.strong(i18n::text(Text::TabShortcuts));
                egui::Grid::new("menu_shortcuts")
                    .num_columns(2)
                    .spacing([16.0, 4.0])
                    .show(ui, |ui| {
                        // Un raccourci par ligne (une cellule de grille ne passe pas à la ligne) ;
                        // le nom de l'action seulement sur la première.
                        for (kind, shortcuts) in hotkeys::shortcuts_for(&self.shortcuts, id) {
                            for (n, shortcut) in shortcuts.iter().enumerate() {
                                if n == 0 {
                                    ui.label(i18n::command_kind_name(kind));
                                } else {
                                    ui.label("");
                                }
                                ui.label(i18n::shortcut_name(shortcut));
                                ui.end_row();
                            }
                        }
                    });
                if ui.button(i18n::text(Text::EditShortcuts)).clicked() {
                    action = Some(MenuAction::EditShortcuts);
                }
                ui.separator();
                if ui.button(i18n::text(Text::HideWindow)).clicked() {
                    action = Some(MenuAction::Hide);
                }
                if ui.button(i18n::text(Text::OpenSettings)).clicked() {
                    action = Some(MenuAction::OpenSettings);
                }
            });
        });

        let Some(action) = action else {
            return;
        };
        match action {
            MenuAction::SetValue(value) => {
                if self.tracker.set_value(id, value) {
                    self.save_state();
                }
                self.menu = None;
            }
            MenuAction::Reset => {
                self.handle(Command::Reset(id), now, ctx);
                self.menu = None;
            }
            // Le menu reste ouvert : on voit la case changer.
            MenuAction::SetAlwaysOnTop(on_top) => {
                match id {
                    CounterId::Global => self.global_style.always_on_top = on_top,
                    CounterId::Round => self.round_style.always_on_top = on_top,
                }
                self.save_state();
            }
            MenuAction::Hide => {
                self.windows[index].hide();
                self.save_state();
                self.menu = None;
            }
            MenuAction::OpenSettings | MenuAction::EditShortcuts => {
                // Ramène la configuration : sortie de la barre des tâches si elle était réduite,
                // puis au premier plan, sur l'onglet voulu.
                ctx.send_viewport_cmd_to(main_viewport(), egui::ViewportCommand::Minimized(false));
                ctx.send_viewport_cmd_to(main_viewport(), egui::ViewportCommand::Focus);
                self.tab = if action == MenuAction::EditShortcuts {
                    ConfigTab::Shortcuts
                } else {
                    ConfigTab::Counters
                };
                self.menu = None;
            }
            MenuAction::Close => self.menu = None,
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

/// Le bouton du menu, dessiné dans `button` : trois points sur un fond discret.
/// (On dessine les points nous-mêmes : la police intégrée n'a pas de symbole « engrenage ».)
fn draw_menu_button(ui: &mut egui::Ui, button: egui::Rect) -> egui::Response {
    let response = ui.interact(
        button,
        egui::Id::unique("menu_button"),
        egui::Sense::click(),
    );
    // Plus visible quand la souris est dessus.
    let alpha = if response.hovered() { 200 } else { 120 };
    let painter = ui.painter();
    painter.rect_filled(button, 4.0, egui::Color32::from_black_alpha(alpha));
    for dx in [-6.0, 0.0, 6.0] {
        painter.circle_filled(
            button.center() + egui::vec2(dx, 0.0),
            1.8,
            egui::Color32::WHITE,
        );
    }
    response
}

/// Où ouvrir le menu d'un compteur : sous le compteur, aligné à gauche ; au-dessus s'il
/// sortirait de l'écran par le bas.
fn menu_position(counter: egui::Rect, screen: Option<egui::Vec2>, size: egui::Vec2) -> egui::Pos2 {
    let below = egui::pos2(counter.left(), counter.bottom() + 4.0);
    let fits = screen.is_none_or(|screen| below.y + size.y <= screen.y);
    if fits {
        below
    } else {
        egui::pos2(counter.left(), counter.top() - size.y - 4.0)
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
    family: egui::FontFamily,
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
        let font = egui::FontId::new(size, family.clone());
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
    /// Appelée une fois à la fermeture de l'appli : on sauvegarde un dernier réglage éventuel.
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.save_state();
    }

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

        // On traite tous les événements reçus depuis la dernière image.
        // `try_recv` ne bloque pas : il renvoie `Ok(événement)` s'il y en a un, une erreur sinon.
        // `while let` : boucle tant que le motif correspond (ici, tant qu'il y a des événements).
        // Les sons des compteurs : rechargés seulement s'ils ont changé (presque gratuit sinon).
        for id in CounterId::ALL {
            // Une copie, pour ne pas emprunter `self` en lecture et `self.audio` en écriture.
            let sounds = self.sounds(id).clone();
            if let Some(audio) = &mut self.audio {
                audio.sync(id, &sounds);
            }
        }
        self.poll_import();

        // Les polices choisies pour les compteurs : chargées si besoin (sans effet si déjà fait).
        for font in [&self.global_style.font, &self.round_style.font]
            .into_iter()
            .flatten()
        {
            self.fonts.require(&ctx, font);
        }

        while let Ok(event) = self.events.try_recv() {
            match event {
                HotkeyEvent::Command(command) => self.handle(command, now, &ctx),
                HotkeyEvent::Captured { held, trigger } => self.on_captured(held, trigger),
                HotkeyEvent::CaptureCancelled => self.capture = None,
                HotkeyEvent::MouseRate(rate) => self.mouse_rate = rate,
            }
        }

        self.main_window(&ctx);
        self.hovered_counter = None;
        for index in 0..self.windows.len() {
            self.counter_window(&ctx, index, now);
        }
        self.menu_button_window(&ctx);
        self.menu_window(&ctx, now);
    }
}

/// Le nom de la touche de validation du reset, TIRÉ DES RACCOURCIS : si on la change,
/// le message « Reset ? » suit. `.find(...)` : le premier raccourci qui valide un reset.
fn confirm_key_name(shortcuts: &[Shortcut]) -> String {
    shortcuts
        .iter()
        .find(|s| s.command == Command::ConfirmReset)
        .map(i18n::shortcut_name)
        .unwrap_or_else(|| "?".into())
}

fn main() -> eframe::Result {
    // On relit la sauvegarde (valeur du global, styles, fenêtres ouvertes, raccourcis).
    let state_path = storage::state_path();
    let saved = state_path.as_deref().map(storage::load).unwrap_or_default();

    // La langue : celle choisie, sinon celle de Windows si elle est disponible, sinon l'anglais.
    let languages = i18n::available_languages();
    let language = saved.language.clone().unwrap_or_else(|| {
        let codes: Vec<String> = languages.iter().map(|l| l.code.clone()).collect();
        i18n::pick_language(&i18n::system_language(), &codes)
    });
    i18n::set_language(&language);

    // On écarte les raccourcis non autorisés (clic gauche / droit sans touche de modification),
    // au cas où la sauvegarde aurait été modifiée à la main.
    // `partition` : sépare une liste en deux selon une condition (autorisés / refusés).
    let (shortcuts, refused): (Vec<_>, Vec<_>) =
        saved.shortcuts.into_iter().partition(Shortcut::is_allowed);
    for shortcut in &refused {
        eprintln!(
            "Raccourci ignoré (clic gauche/droit sans Ctrl, Maj ou Alt) : {}",
            i18n::shortcut_name(shortcut)
        );
    }
    let confirm_key_name = confirm_key_name(&shortcuts);

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
            // egui zoome l'interface avec Ctrl + « - » / « + », y compris ceux du pavé numérique :
            // nos raccourcis Ctrl + Num ± rétréciraient la configuration quand elle a le focus.
            cc.egui_ctx
                .options_mut(|options| options.zoom_with_keyboard = false);

            // L'hôte : sans décorations Windows 11, et absent d'Alt + Tab.
            if let Some(host) = NativeWindow::from_window(cc) {
                host.remove_decorations();
                host.hide_from_task_switcher();
            }

            // Un CHANNEL : un tuyau entre threads. `sender` envoie, `receiver` reçoit.
            let (sender, receiver) = mpsc::channel();
            // Le contexte egui, pour pouvoir réveiller l'interface depuis le thread des raccourcis.
            let ctx = cc.egui_ctx.clone();

            // Une copie des raccourcis part dans le thread ; l'appli garde l'original.
            let hotkeys = hotkeys::start(shortcuts.clone(), saved.mouse_method, move |event| {
                // `let _ =` : on ignore volontairement le résultat (échec seulement si l'appli est fermée).
                let _ = sender.send(event);
                // Sans le focus, egui ne redessine pas tout seul : on le réveille.
                ctx.request_repaint();
            });

            Ok(Box::new(App {
                tracker: {
                    let mut tracker = Tracker::new(
                        Counter::new(saved.global_value, saved.global_goal),
                        // La session repart TOUJOURS de 0 (sa valeur n'est jamais sauvegardée).
                        Counter::new(0, saved.round_goal),
                    );
                    tracker.sound_priority = saved.sound_priority;
                    tracker
                },
                events: receiver,
                hotkeys,
                audio: AudioEngine::new(saved.audio_output.clone(), saved.volume),
                windows: [
                    CounterWindow::new(CounterId::Global, saved.show_global),
                    CounterWindow::new(CounterId::Round, saved.show_round),
                ],
                global_style: saved.global_style,
                round_style: saved.round_style,
                round_copies_global: saved.round_copies_global,
                confirm_key_name,
                shortcuts,
                capture: None,
                capture_error: None,
                fonts: fonts::FontLibrary::default(),
                menu: None,
                menu_button: None,
                hovered_counter: None,
                mouse_method: saved.mouse_method,
                mouse_rate: None,
                round_copies_global_sounds: saved
                    .round_copies_global_sounds
                    .unwrap_or(saved.round_copies_global),
                sections: saved.open_sections.clone(),
                audio_output: saved.audio_output.clone(),
                volume: saved.volume,
                all_muted: saved.all_muted,
                audio_outputs: audio::list_outputs(),
                default_output: None,
                output_error: None,
                sound_catalog: SoundCatalog::load(),
                language: saved.language.clone(),
                languages,
                pending_import: None,
                import_error: None,
                tab: ConfigTab::Counters,
                unsaved_changes: false,
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
    fn menu_grows_with_the_shortcuts() {
        assert!(menu_size(7).y > menu_size(2).y);
        assert_eq!(menu_size(2).x, MENU_BASE_SIZE.x);
    }

    #[test]
    fn menu_opens_below_the_counter() {
        let counter = egui::Rect::from_min_size(egui::pos2(100.0, 100.0), egui::vec2(80.0, 80.0));
        let screen = Some(egui::vec2(1920.0, 1080.0));
        assert_eq!(
            menu_position(counter, screen, MENU_BASE_SIZE),
            egui::pos2(100.0, 184.0)
        );
    }

    #[test]
    fn menu_opens_above_when_too_low() {
        let counter = egui::Rect::from_min_size(egui::pos2(100.0, 950.0), egui::vec2(80.0, 80.0));
        let screen = Some(egui::vec2(1920.0, 1080.0));
        let size = menu_size(7);
        let position = menu_position(counter, screen, size);
        assert_eq!(position.y, 950.0 - size.y - 4.0);
    }

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
