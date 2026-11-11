//! Les éléments de la fenêtre de configuration : l'éditeur de style d'un compteur et
//! l'éditeur des raccourcis.
//!
//! Chaque fonction reçoit les réglages en `&mut` (emprunt modifiable) et les modifie directement
//! quand l'utilisateur touche à un contrôle. Elle renvoie `true` si quelque chose a changé, pour
//! que `main.rs` sache qu'il faut sauvegarder.

use std::collections::BTreeMap;

use eframe::egui;
use egui::color_picker::{Alpha, color_edit_button_srgba};

use crate::audio::{self, SoundChoice, SoundSet, SoundSlot};
use crate::command::{Command, CommandKind};
use crate::display::{Anchor, DisplayStyle};
use crate::fonts::FontLibrary;
use crate::hotkeys::{self, Key, MouseMethod, Shortcut};
use crate::i18n::{self, Text};
use crate::tracker::{CounterId, Targets};

/// Un texte traduit dans la langue de l'interface (pour alléger l'écriture).
fn t(text: Text) -> String {
    i18n::text(text)
}

/// L'état replié / déplié des sections de la configuration, par nom (sauvegardé).
/// `BTreeMap` : un dictionnaire (clé → valeur) trié par clé ; trié, il s'écrit toujours dans
/// le même ordre dans la sauvegarde.
pub type Sections = BTreeMap<String, bool>;

/// La section `key` est-elle dépliée ? (`default` tant que l'utilisateur n'y a pas touché.)
pub fn is_open(sections: &Sections, key: &str, default: bool) -> bool {
    sections.get(key).copied().unwrap_or(default)
}

/// Une section repliable : un titre cliquable (▸ / ▾) et son contenu, affiché seulement si
/// `open`. Renvoie `true` si l'utilisateur vient de cliquer sur le titre : c'est à l'appelant
/// d'inverser l'état (en deux temps, pour que le contenu puisse lui aussi modifier les sections).
pub fn section(
    ui: &mut egui::Ui,
    key: &str,
    title: impl Into<egui::WidgetText>,
    open: bool,
    body: impl FnOnce(&mut egui::Ui),
) -> bool {
    // `.open(Some(open))` : c'est NOUS qui décidons de l'état (celui de la sauvegarde).
    egui::CollapsingHeader::new(title)
        .id_salt(key)
        .open(Some(open))
        .show(ui, body)
        .header_response
        .clicked()
}

/// Les réglages d'affichage d'un compteur : format, police, taille, fond, couleurs du texte,
/// puis une section « Avancé » repliée (marge, arrondi, ancrage, premier plan).
/// `id_salt` : un identifiant propre à chaque compteur, car egui doit distinguer
/// les deux grilles affichées dans la même fenêtre.
/// `fonts` : les polices (liste et chargement) ; `in_use` : les polices des compteurs,
/// à garder en mémoire. `defaults` : l'affichage rétabli par le bouton « Rétablir ».
pub fn style_editor(
    ui: &mut egui::Ui,
    id_salt: &str,
    style: &mut DisplayStyle,
    fonts: &mut FontLibrary,
    in_use: &[&str],
    sections: &mut Sections,
    defaults: &DisplayStyle,
) -> bool {
    let mut changed = false;

    // L'aide du format, AU-DESSUS de la grille : dans une cellule de grille, le texte ne peut
    // pas passer à la ligne (la cellule s'élargit sans limite) ; ici, il suit la largeur
    // de la fenêtre. `Label::new(...).wrap()` : retour à la ligne automatique.
    ui.add(egui::Label::new(egui::RichText::new(t(Text::TemplateHelp)).small()).wrap());

    egui::Grid::new(id_salt)
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            ui.label(t(Text::Template))
                .on_hover_text(t(Text::TooltipFormat));
            changed |= ui
                .text_edit_singleline(&mut style.template)
                .on_hover_text(t(Text::TooltipFormat))
                .changed();
            ui.end_row();

            ui.label(t(Text::Font));
            ui.horizontal(|ui| {
                changed |= font_picker(ui, id_salt, style, fonts, in_use);
            });
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

            ui.label(t(Text::Background));
            ui.horizontal(|ui| {
                // Fond transparent : opacité à 0. Le décocher remet le fond par défaut.
                let mut transparent = style.background.a() == 0;
                if ui
                    .checkbox(&mut transparent, t(Text::TransparentBackground))
                    .on_hover_text(t(Text::TooltipTransparent))
                    .changed()
                {
                    style.background = if transparent {
                        egui::Color32::TRANSPARENT
                    } else {
                        defaults.background
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
        });

    // Les couleurs du texte, toujours opaques, en grille 2 × 2 : libellé puis pastille.
    ui.label(t(Text::ColorsSection));
    egui::Grid::new((id_salt, "colors"))
        .num_columns(4)
        .spacing([10.0, 6.0])
        .show(ui, |ui| {
            let pairs = [
                [
                    (Text::PlainTextColor, &mut style.text),
                    (Text::AlertColor, &mut style.alert),
                ],
                [
                    (Text::GoalPendingColor, &mut style.goal_pending),
                    (Text::GoalReachedColor, &mut style.goal_reached),
                ],
            ];
            for row in pairs {
                for (label, color) in row {
                    ui.label(t(label));
                    changed |= color_edit_button_srgba(ui, color, Alpha::Opaque).changed();
                }
                ui.end_row();
            }
        });

    // Les réglages rarement modifiés, dans une section repliée par défaut.
    let advanced_key = format!("{id_salt}_advanced");
    let advanced_open = is_open(sections, &advanced_key, false);
    let clicked = section(
        ui,
        &advanced_key,
        t(Text::AdvancedSection),
        advanced_open,
        |ui| {
            egui::Grid::new((id_salt, "advanced"))
                .num_columns(2)
                .spacing([12.0, 6.0])
                .show(ui, |ui| {
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

                    ui.label(t(Text::Anchor))
                        .on_hover_text(t(Text::TooltipAnchor));
                    ui.horizontal_wrapped(|ui| {
                        // `radio_value` : un bouton radio qui met `style.anchor` à la valeur donnée.
                        for (anchor, label) in [
                            (Anchor::Left, Text::AnchorLeft),
                            (Anchor::Center, Text::AnchorCenter),
                            (Anchor::Right, Text::AnchorRight),
                        ] {
                            changed |= ui
                                .radio_value(&mut style.anchor, anchor, t(label))
                                .on_hover_text(t(Text::TooltipAnchor))
                                .changed();
                        }
                    });
                    ui.end_row();

                    ui.label("");
                    changed |= ui
                        .checkbox(&mut style.always_on_top, t(Text::AlwaysOnTop))
                        .on_hover_text(t(Text::TooltipAlwaysOnTop))
                        .changed();
                    ui.end_row();
                });
        },
    );
    if clicked {
        sections.insert(advanced_key, !advanced_open);
        changed = true;
    }

    if ui.button(t(Text::ResetDisplay)).clicked() {
        // On rétablit l'affichage, mais on GARDE les sons (réglés dans un autre onglet).
        // `..defaults.clone()` : tous les autres champs viennent de l'affichage par défaut.
        *style = reset_display(style, defaults);
        changed = true;
    }
    changed
}

/// L'affichage par défaut, en gardant les sons du compteur (ils se règlent dans un autre onglet).
fn reset_display(style: &DisplayStyle, defaults: &DisplayStyle) -> DisplayStyle {
    // `..defaults.clone()` : tous les autres champs viennent de l'affichage par défaut.
    DisplayStyle {
        sounds: style.sounds.clone(),
        ..defaults.clone()
    }
}

/// Le bouton « haut-parleur » : coupe ou réactive un son. Dessiné à la main (la police
/// intégrée n'a pas toutes les icônes) : un haut-parleur, avec des ondes s'il est actif,
/// une croix s'il est coupé. Renvoie `true` si l'état a changé.
fn mute_toggle(ui: &mut egui::Ui, muted: &mut bool) -> bool {
    let size = egui::vec2(26.0, 20.0);
    // `allocate_exact_size` : réserve la place et donne la réponse (clic, survol).
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    let response = response.on_hover_text(t(Text::MuteTooltip));
    let visuals = ui.style().interact(&response);
    let painter = ui.painter();
    painter.rect_filled(rect, 4.0, visuals.weak_bg_fill);
    let color = if *muted {
        ui.visuals().warn_fg_color
    } else {
        visuals.fg_stroke.color
    };
    let c = rect.center() + egui::vec2(-5.0, 0.0);
    // Le corps du haut-parleur : un petit rectangle et un trapèze.
    let body = egui::Rect::from_center_size(c + egui::vec2(-3.0, 0.0), egui::vec2(3.0, 6.0));
    painter.rect_filled(body, 0.0, color);
    painter.add(egui::Shape::convex_polygon(
        vec![
            c + egui::vec2(-1.5, -3.0),
            c + egui::vec2(3.5, -7.0),
            c + egui::vec2(3.5, 7.0),
            c + egui::vec2(-1.5, 3.0),
        ],
        color,
        egui::Stroke::NONE,
    ));
    let stroke = egui::Stroke::new(1.6, color);
    if *muted {
        // Une croix à droite.
        let x = c + egui::vec2(9.5, 0.0);
        painter.line_segment(
            [x + egui::vec2(-3.0, -3.0), x + egui::vec2(3.0, 3.0)],
            stroke,
        );
        painter.line_segment(
            [x + egui::vec2(-3.0, 3.0), x + egui::vec2(3.0, -3.0)],
            stroke,
        );
    } else {
        // Deux « ondes » : des petits arcs approchés par des segments.
        for radius in [5.5_f32, 9.0] {
            let points: Vec<egui::Pos2> = (-3..=3)
                .map(|i| {
                    let angle = i as f32 * 0.22;
                    c + egui::vec2(2.0 + radius * angle.cos(), radius * angle.sin())
                })
                .collect();
            painter.add(egui::Shape::line(points, stroke));
        }
    }
    if response.clicked() {
        *muted = !*muted;
        return true;
    }
    false
}

/// Les sons proposés dans les listes (relus au démarrage et après un import).
pub struct SoundCatalog {
    pub windows: Vec<String>,
    pub imported: Vec<String>,
}

impl SoundCatalog {
    pub fn load() -> Self {
        Self {
            windows: audio::windows_sounds(),
            imported: audio::imported_sounds(),
        }
    }
}

/// Ce que l'utilisateur a demandé dans la section Sons.
#[derive(Debug, Clone, PartialEq)]
pub enum SoundRequest {
    /// Écouter un son (bouton « Écouter »).
    Preview(SoundChoice),
    /// Importer un fichier pour cet emplacement.
    Import(SoundSlot),
}

/// Le nom d'un fichier sans son extension (« Speech On.wav » → « Speech On »).
fn stem(name: &str) -> String {
    std::path::Path::new(name)
        .file_stem()
        .map_or_else(|| name.to_string(), |s| s.to_string_lossy().into_owned())
}

/// Le nom affiché d'un choix de son.
pub fn sound_choice_name(choice: &SoundChoice) -> String {
    match choice {
        SoundChoice::Windows(name) => stem(name),
        SoundChoice::Beep(frequency) => {
            // `position` : le rang de cette fréquence parmi les bips proposés.
            let index = audio::BEEPS.iter().position(|f| f == frequency);
            match index {
                Some(0) => t(Text::BeepLow),
                Some(1) => t(Text::BeepMid),
                Some(2) => t(Text::BeepHigh),
                Some(_) => t(Text::BeepVeryHigh),
                None => format!("{frequency} Hz"),
            }
        }
        SoundChoice::Imported(name) => i18n::message(Text::ImportedSound, &[("name", &stem(name))]),
        // « pluck_002 » → « pluck 2 » : lisible, sans traduction à maintenir.
        SoundChoice::Embedded(name) => {
            let mut words = name.split('_');
            let base = words.next().unwrap_or_default();
            match words.next().and_then(|n| n.parse::<u32>().ok()) {
                Some(number) => format!("{base} {number}"),
                None => base.to_string(),
            }
        }
    }
}

/// Le nom d'un emplacement de son.
fn slot_name(slot: SoundSlot) -> String {
    t(match slot {
        SoundSlot::Increment => Text::SlotIncrement,
        SoundSlot::Decrement => Text::SlotDecrement,
        SoundSlot::ResetPrompt => Text::SlotResetPrompt,
        SoundSlot::ResetDone => Text::SlotResetDone,
        SoundSlot::ResetUndone => Text::SlotResetUndone,
    })
}

/// La section Sons d'un compteur : pour chaque emplacement, le son, « Écouter » et « Muet ».
/// Renvoie `true` si un réglage a changé, et l'éventuelle demande (écouter, importer).
pub fn sound_editor(
    ui: &mut egui::Ui,
    id_salt: &str,
    sounds: &mut SoundSet,
    catalog: &SoundCatalog,
) -> (bool, Option<SoundRequest>) {
    let mut changed = false;
    let mut request = None;
    ui.strong(t(Text::Sounds));
    egui::Grid::new((id_salt, "sounds"))
        .num_columns(4)
        .spacing([10.0, 6.0])
        .show(ui, |ui| {
            for slot in SoundSlot::ALL {
                ui.label(slot_name(slot));
                let settings = sounds.get_mut(slot);
                egui::ComboBox::from_id_salt((id_salt, slot))
                    .selected_text(sound_choice_name(&settings.sound))
                    .width(170.0)
                    .height(320.0)
                    .show_ui(ui, |ui| {
                        // Un petit utilitaire local : une ligne de la liste, qui choisit ce son.
                        let mut option = |ui: &mut egui::Ui, choice: SoundChoice| {
                            let selected = settings.sound == choice;
                            if ui
                                .selectable_label(selected, sound_choice_name(&choice))
                                .clicked()
                            {
                                settings.sound = choice;
                                changed = true;
                            }
                        };
                        if !catalog.imported.is_empty() {
                            ui.label(egui::RichText::new(t(Text::ImportedSounds)).weak());
                            for name in &catalog.imported {
                                option(ui, SoundChoice::Imported(name.clone()));
                            }
                            ui.separator();
                        }
                        ui.label(egui::RichText::new(t(Text::EmbeddedSounds)).weak());
                        for (name, _) in audio::EMBEDDED {
                            option(ui, SoundChoice::Embedded(name.to_string()));
                        }
                        ui.separator();
                        ui.label(egui::RichText::new(t(Text::Beeps)).weak());
                        for frequency in audio::BEEPS {
                            option(ui, SoundChoice::Beep(frequency));
                        }
                        ui.separator();
                        ui.label(egui::RichText::new(t(Text::WindowsSounds)).weak());
                        for name in &catalog.windows {
                            option(ui, SoundChoice::Windows(name.clone()));
                        }
                        ui.separator();
                        if ui.selectable_label(false, t(Text::ImportFile)).clicked() {
                            request = Some(SoundRequest::Import(slot));
                        }
                    });
                if ui.button(t(Text::Listen)).clicked() {
                    request = Some(SoundRequest::Preview(settings.sound.clone()));
                }
                changed |= mute_toggle(ui, &mut settings.muted);
                ui.end_row();
            }
        });
    (changed, request)
}

/// Le choix de la police : une liste déroulante avec recherche, puis un aperçu de la police
/// survolée (ou choisie). Renvoie `true` si la police a changé.
fn font_picker(
    ui: &mut egui::Ui,
    id_salt: &str,
    style: &mut DisplayStyle,
    fonts: &mut FontLibrary,
    in_use: &[&str],
) -> bool {
    let mut changed = false;
    let mut hovered: Option<String> = None;
    let current = style.font.clone().unwrap_or_else(|| t(Text::DefaultFont));
    egui::ComboBox::from_id_salt((id_salt, "font"))
        .selected_text(current)
        .width(200.0)
        .height(320.0)
        // Par défaut, egui ferme la liste à TOUT clic, même dedans : on ne pourrait pas
        // cliquer dans le champ de recherche. On la ferme nous-mêmes au choix d'une police.
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show_ui(ui, |ui| {
            // Le texte recherché est rangé dans la mémoire d'egui (`data_mut`), propre à ce
            // champ : il survit d'une image à l'autre sans champ dédié dans notre appli.
            let search_id = ui.scope_id().with("search");
            let mut search: String = ui.data_mut(|d| d.get_temp(search_id).unwrap_or_default());
            ui.add(egui::TextEdit::singleline(&mut search).hint_text(t(Text::SearchFont)));
            ui.data_mut(|d| d.insert_temp(search_id, search.clone()));

            if ui
                .selectable_label(style.font.is_none(), t(Text::DefaultFont))
                .clicked()
            {
                style.font = None;
                changed = true;
                ui.close(); // referme la liste
            }
            let needle = search.to_lowercase();
            // Une copie des noms filtrés : `fonts` resservira juste après pour l'aperçu.
            let names: Vec<String> = fonts
                .system_fonts()
                .iter()
                .filter(|font| font.name.to_lowercase().contains(&needle))
                .map(|font| font.name.clone())
                .collect();
            for name in names {
                let selected = style.font.as_deref() == Some(name.as_str());
                let response = ui.selectable_label(selected, &name);
                if response.hovered() {
                    hovered = Some(name.clone());
                }
                if response.clicked() {
                    style.font = Some(name);
                    changed = true;
                    ui.close();
                }
            }
        });

    // Aperçu : la police survolée dans la liste, sinon la police choisie. À DROITE de la
    // liste (en dessous, la liste ouverte le cacherait), avec un texte court pour tenir.
    if let Some(name) = hovered.or_else(|| style.font.clone()) {
        fonts.preview(ui.ctx(), &name, in_use);
        let font = fonts.font_id(ui.ctx(), Some(&name), 22.0);
        ui.label(egui::RichText::new("0123").font(font));
    }
    changed
}

/// Ce qu'on capture : un nouveau raccourci pour une ligne, ou le remplacement d'un raccourci.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaptureTarget {
    pub kind: CommandKind,
    /// `Some(n)` : remplacer le raccourci n° `n` ; `None` : en ajouter un.
    pub replace: Option<usize>,
}

/// Ce que l'utilisateur a demandé dans l'onglet Raccourcis (au plus une chose par image).
/// L'éditeur ne modifie rien lui-même : il RENVOIE la demande, et `main.rs` l'exécute
/// (car il faut aussi prévenir le thread du hook et sauvegarder).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShortcutRequest {
    StartCapture(CaptureTarget),
    CancelCapture,
    Remove(usize),
    SetTargets(usize, Targets),
    RestoreDefaults,
    SetMouseMethod(MouseMethod),
}

/// Pourquoi une combinaison capturée est refusée.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureRefused {
    /// Clic gauche ou droit sans Ctrl, Maj ou Alt.
    MainClickNeedsModifier,
}

/// Enregistre une combinaison capturée dans la liste des raccourcis.
/// - Remplacement : le raccourci garde sa commande (et ses compteurs visés), seules les touches changent.
/// - Ajout : nouveau raccourci, sur les deux compteurs pour les actions qui comptent.
/// - Un doublon exact (mêmes touches, même commande) n'est pas ajouté.
///
/// `Result<(), CaptureRefused>` : `Ok(())` si c'est fait, `Err(raison)` sinon.
pub fn apply_capture(
    shortcuts: &mut Vec<Shortcut>,
    target: CaptureTarget,
    held: Vec<Key>,
    trigger: Key,
) -> Result<(), CaptureRefused> {
    // Un remplacement dont le numéro n'existe plus (liste changée entre-temps) devient un ajout.
    // `filter` sur une `Option` : garde la valeur seulement si la condition est vraie.
    let replace = target.replace.filter(|&i| i < shortcuts.len());
    let command = match replace {
        Some(i) => shortcuts[i].command,
        None => target.kind.command(Targets::BOTH),
    };
    let new = Shortcut {
        held,
        trigger,
        command,
    };
    if !new.is_allowed() {
        return Err(CaptureRefused::MainClickNeedsModifier);
    }
    let duplicate = shortcuts
        .iter()
        .enumerate()
        .any(|(i, s)| Some(i) != replace && s.command == command && s.same_keys(&new));
    match (replace, duplicate) {
        // Le raccourci remplacé devient le doublon d'un autre : on le retire simplement.
        (Some(i), true) => {
            shortcuts.remove(i);
        }
        (Some(i), false) => shortcuts[i] = new,
        (None, true) => {}
        (None, false) => shortcuts.push(new),
    }
    Ok(())
}

/// L'onglet Raccourcis : une section par action, avec ses raccourcis.
/// `capturing` : la capture en cours ; `error` : le message d'un refus récent.
/// `mouse_rate` : fréquence des événements souris si elle est élevée (protection automatique).
pub fn shortcuts_editor(
    ui: &mut egui::Ui,
    shortcuts: &[Shortcut],
    capturing: Option<CaptureTarget>,
    error: Option<&str>,
    mouse_method: MouseMethod,
    mouse_rate: Option<u32>,
    sections: &mut Sections,
) -> Option<ShortcutRequest> {
    let mut request = None;
    // Pendant une capture, tous les autres boutons sont désactivés.
    let idle = capturing.is_none();
    let warning = ui.visuals().warn_fg_color;

    // Protection automatique : bandeau si la souris envoie trop d'événements (Raw Input).
    if let Some(rate) = mouse_rate {
        egui::Frame::group(ui.style()).show(ui, |ui| {
            let text = i18n::message(Text::MouseRateWarning, &[("rate", &rate.to_string())]);
            ui.add(egui::Label::new(egui::RichText::new(text).color(warning)).wrap());
            if ui.button(t(Text::SwitchToPolling)).clicked() {
                request = Some(ShortcutRequest::SetMouseMethod(MouseMethod::Polling));
            }
        });
        ui.add_space(8.0);
    }

    ui.add(egui::Label::new(t(Text::ShortcutsHelp)).wrap());
    if let Some(error) = error {
        ui.colored_label(warning, error);
    }
    ui.add_space(8.0);

    let conflicts = hotkeys::conflicts(shortcuts);
    // Une section par action : son nom en gras, puis un raccourci par ligne, puis « + ».
    for kind in CommandKind::ALL {
        ui.strong(i18n::command_kind_name(kind));
        // `indent` : décale le contenu vers la droite, sous le nom de l'action.
        // `kind` sert d'identifiant à egui pour distinguer les sections.
        ui.indent(kind, |ui| {
            for (index, shortcut) in shortcuts.iter().enumerate() {
                if CommandKind::of(shortcut.command) != Some(kind) {
                    continue; // pas dans cette section
                }
                let target = CaptureTarget {
                    kind,
                    replace: Some(index),
                };
                // Une ligne : la combinaison, ⚠ éventuel, les compteurs visés, ×.
                ui.horizontal(|ui| {
                    if capturing == Some(target) {
                        if ui.button(t(Text::PressShortcut)).clicked() {
                            request = Some(ShortcutRequest::CancelCapture);
                        }
                    } else if ui
                        .add_enabled(idle, egui::Button::new(i18n::shortcut_name(shortcut)))
                        .clicked()
                    {
                        request = Some(ShortcutRequest::StartCapture(target));
                    }
                    // Molette en mode interrogation : le raccourci ne peut pas fonctionner.
                    if mouse_method == MouseMethod::Polling && shortcut.uses_wheel() {
                        ui.label(egui::RichText::new(" ! ").strong().color(warning))
                            .on_hover_text(t(Text::WheelNeedsRawInput));
                    }
                    if conflicts[index] {
                        // « ! » plutôt que « ⚠ », absent de la police intégrée d'egui.
                        ui.label(egui::RichText::new(" ! ").strong().color(warning))
                            .on_hover_text(t(Text::ShortcutConflict));
                    }
                    // Les actions qui comptent : choix des compteurs visés.
                    if let Command::Count(targets, _) = shortcut.command {
                        ui.add_enabled_ui(idle, |ui| {
                            egui::ComboBox::from_id_salt(("targets", index))
                                .selected_text(i18n::targets_name(targets))
                                .show_ui(ui, |ui| {
                                    for choice in [
                                        Targets::BOTH,
                                        Targets::only(CounterId::Global),
                                        Targets::only(CounterId::Round),
                                    ] {
                                        let label = i18n::targets_name(choice);
                                        if ui.selectable_label(choice == targets, label).clicked() {
                                            request =
                                                Some(ShortcutRequest::SetTargets(index, choice));
                                        }
                                    }
                                });
                        });
                    }
                    if ui
                        .add_enabled(idle, egui::Button::new("×"))
                        .on_hover_text(t(Text::RemoveShortcut))
                        .clicked()
                    {
                        request = Some(ShortcutRequest::Remove(index));
                    }
                });
            }
            // Le bouton d'ajout, ou l'invite si on est en train d'ajouter dans cette section.
            let target = CaptureTarget {
                kind,
                replace: None,
            };
            if capturing == Some(target) {
                if ui.button(t(Text::PressShortcut)).clicked() {
                    request = Some(ShortcutRequest::CancelCapture);
                }
            } else if ui
                .add_enabled(idle, egui::Button::new(" + "))
                .on_hover_text(t(Text::AddShortcut))
                .clicked()
            {
                request = Some(ShortcutRequest::StartCapture(target));
            }
        });
        ui.add_space(6.0);
    }

    ui.add_space(12.0);
    if ui
        .add_enabled(idle, egui::Button::new(t(Text::RestoreDefaultShortcuts)))
        .clicked()
    {
        request = Some(ShortcutRequest::RestoreDefaults);
    }

    // Méthode d'écoute de la souris : rarement modifiée, dans une section repliée.
    ui.add_space(8.0);
    let open = is_open(sections, "shortcuts_advanced", false);
    let clicked = section(
        ui,
        "shortcuts_advanced",
        t(Text::AdvancedSection),
        open,
        |ui| {
            ui.label(t(Text::MouseMethodLabel));
            for (method, label, tooltip) in [
                (
                    MouseMethod::RawInput,
                    Text::MouseRawInput,
                    Text::TooltipRawInput,
                ),
                (
                    MouseMethod::Polling,
                    Text::MousePolling,
                    Text::TooltipPolling,
                ),
            ] {
                if ui
                    .add_enabled(
                        idle,
                        egui::RadioButton::new(mouse_method == method, t(label)),
                    )
                    .on_hover_text(t(tooltip))
                    .clicked()
                {
                    request = Some(ShortcutRequest::SetMouseMethod(method));
                }
            }
        },
    );
    if clicked {
        sections.insert("shortcuts_advanced".to_string(), !open);
    }
    request
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::counter::CounterAction;

    #[test]
    fn reset_display_keeps_the_sounds() {
        let mut style = DisplayStyle {
            font_size: 99.0,
            template: "Coffres : {value}".into(),
            ..DisplayStyle::default()
        };
        style.sounds.increment.muted = true;
        let reset = reset_display(&style, &DisplayStyle::default());
        assert_eq!(reset.font_size, DisplayStyle::default().font_size);
        assert_eq!(reset.template, DisplayStyle::default().template);
        assert!(reset.sounds.increment.muted); // les sons ne bougent pas
    }

    #[test]
    fn section_state_uses_default_until_changed() {
        let mut sections = Sections::new();
        assert!(is_open(&sections, "counter_global", true));
        assert!(!is_open(&sections, "global_style_display", false));
        sections.insert("counter_global".into(), false);
        assert!(!is_open(&sections, "counter_global", true));
    }

    #[test]
    fn embedded_sound_names_are_readable() {
        let name = |n: &str| sound_choice_name(&SoundChoice::Embedded(n.into()));
        assert_eq!(name("pluck_002"), "pluck 2");
        assert_eq!(name("confirmation_001"), "confirmation 1");
        assert_eq!(name("sans_numero"), "sans");
    }

    fn add(kind: CommandKind) -> CaptureTarget {
        CaptureTarget {
            kind,
            replace: None,
        }
    }

    #[test]
    fn adding_a_shortcut_targets_both_counters() {
        let mut shortcuts = Vec::new();
        apply_capture(
            &mut shortcuts,
            add(CommandKind::Add1),
            vec![],
            Key::NUMPAD_ADD,
        )
        .unwrap();
        assert_eq!(
            shortcuts[0].command,
            Command::Count(Targets::BOTH, CounterAction::Add(1))
        );
    }

    #[test]
    fn replacing_keeps_the_command_and_its_targets() {
        let round_only = Command::Count(Targets::only(CounterId::Round), CounterAction::Add(1));
        let mut shortcuts = vec![Shortcut {
            held: vec![],
            trigger: Key::NUMPAD_ADD,
            command: round_only,
        }];
        let target = CaptureTarget {
            kind: CommandKind::Add1,
            replace: Some(0),
        };
        apply_capture(
            &mut shortcuts,
            target,
            vec![Key::CONTROL],
            Key::MOUSE_MIDDLE,
        )
        .unwrap();
        assert_eq!(shortcuts.len(), 1);
        assert_eq!(shortcuts[0].trigger, Key::MOUSE_MIDDLE);
        assert_eq!(shortcuts[0].held, vec![Key::CONTROL]);
        assert_eq!(shortcuts[0].command, round_only);
    }

    #[test]
    fn main_click_without_modifier_is_refused() {
        let mut shortcuts = Vec::new();
        let result = apply_capture(
            &mut shortcuts,
            add(CommandKind::Add1),
            vec![],
            Key::MOUSE_LEFT,
        );
        assert_eq!(result, Err(CaptureRefused::MainClickNeedsModifier));
        assert!(shortcuts.is_empty());
        // Avec Ctrl, c'est accepté.
        apply_capture(
            &mut shortcuts,
            add(CommandKind::Add1),
            vec![Key::CONTROL],
            Key::MOUSE_LEFT,
        )
        .unwrap();
        assert_eq!(shortcuts.len(), 1);
    }

    #[test]
    fn exact_duplicate_is_not_added() {
        let mut shortcuts = hotkeys::default_shortcuts();
        let count = shortcuts.len();
        apply_capture(
            &mut shortcuts,
            add(CommandKind::Add1),
            vec![],
            Key::NUMPAD_ADD,
        )
        .unwrap();
        assert_eq!(shortcuts.len(), count);
    }

    #[test]
    fn same_keys_for_another_action_is_added_with_a_conflict() {
        let mut shortcuts = hotkeys::default_shortcuts();
        apply_capture(
            &mut shortcuts,
            add(CommandKind::ResetRound),
            vec![],
            Key::NUMPAD_ADD,
        )
        .unwrap();
        // Ajouté (simple avertissement) : Num + est en conflit, des deux côtés.
        let conflicts = hotkeys::conflicts(&shortcuts);
        assert!(conflicts[0]); // le +1 sur Num +
        assert!(*conflicts.last().unwrap()); // le nouveau reset sur Num +
    }

    #[test]
    fn stale_replace_index_becomes_an_add() {
        let mut shortcuts = Vec::new();
        let target = CaptureTarget {
            kind: CommandKind::UndoReset,
            replace: Some(7),
        };
        apply_capture(&mut shortcuts, target, vec![], Key::NUMPAD_DECIMAL).unwrap();
        assert_eq!(shortcuts[0].command, Command::UndoReset);
    }
}
