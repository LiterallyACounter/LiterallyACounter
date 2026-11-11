//! Moteur audio : joue un son à chaque action, sans bloquer l'interface ni le hook clavier.
//!
//! - Les sons sont décodés UNE SEULE FOIS au chargement et gardés en mémoire :
//!   au moment de l'appui, aucun accès disque (latence minimale, pas de saccade en jeu).
//! - Chaque « emplacement » (`SoundSlot` : +, −…) a son propre lecteur. Un nouveau son
//!   COUPE le précédent du même emplacement, pour éviter la cacophonie en cas d'appuis rapides.
//! - Si un son Windows est introuvable, on le remplace par un bip généré (son de secours).
//! - S'il n'y a aucun périphérique audio, l'appli fonctionne quand même, sans son.
//! - Sortie audio : celle par défaut de Windows (on suit ses changements : casque branché…)
//!   ou une sortie précise. Si elle disparaît, on se rabat sur celle par défaut.

use std::collections::HashMap;
use std::fs::File;
use std::num::NonZero;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use rodio::buffer::SamplesBuffer;
use rodio::cpal::traits::HostTrait;
use rodio::{Decoder, DeviceSinkBuilder, DeviceTrait, MixerDeviceSink, Player, Source};

use crate::counter::CounterAction;

/// Les emplacements de son : à quel type d'action un son est associé.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SoundSlot {
    Increment,
    Decrement,
    /// Alerte « Reset ? » (en attente de validation).
    ResetPrompt,
    /// Reset effectué.
    ResetDone,
    /// Reset annulé.
    ResetUndone,
}

impl SoundSlot {
    /// Tous les emplacements (pour charger leurs sons au démarrage).
    pub const ALL: [SoundSlot; 5] = [
        SoundSlot::Increment,
        SoundSlot::Decrement,
        SoundSlot::ResetPrompt,
        SoundSlot::ResetDone,
        SoundSlot::ResetUndone,
    ];

    /// L'emplacement de son correspondant à une action sur le compteur.
    pub fn for_action(action: CounterAction) -> Self {
        match action {
            CounterAction::Add(_) => SoundSlot::Increment,
            CounterAction::Subtract(_) => SoundSlot::Decrement,
        }
    }

    /// Le son par défaut de chaque emplacement.
    pub fn default_sound(self) -> SoundChoice {
        match self {
            // « Windows - Synthèse vocale activée »
            SoundSlot::Increment => SoundChoice::Windows("Speech On.wav".into()),
            // « Windows - Synthèse vocale désactivée »
            SoundSlot::Decrement => SoundChoice::Windows("Speech Off.wav".into()),
            // Sons provisoires (seront revus avec les sons embarqués, étape 9).
            SoundSlot::ResetPrompt => SoundChoice::Windows("Windows Exclamation.wav".into()),
            // Le bruit de la corbeille qu'on vide : de circonstance pour un reset.
            SoundSlot::ResetDone => SoundChoice::Windows("Windows Recycle.wav".into()),
            SoundSlot::ResetUndone => SoundChoice::Windows("Windows Restore.wav".into()),
        }
    }

    /// Fréquence du bip de secours, si le son Windows est introuvable.
    fn fallback_frequency(self) -> f32 {
        match self {
            SoundSlot::Increment => 880.0,    // La 5 (aigu)
            SoundSlot::Decrement => 440.0,    // La 4 (grave)
            SoundSlot::ResetPrompt => 1200.0, // très aigu : alerte
            SoundSlot::ResetDone => 330.0,    // Mi 4 : très grave
            SoundSlot::ResetUndone => 660.0,  // Mi 5
        }
    }
}

/// D'où vient un son.
#[derive(Debug, Clone, PartialEq)]
pub enum SoundChoice {
    /// Un fichier du dossier des sons de Windows (`C:\Windows\Media`), par son nom.
    Windows(String),
    /// Un bip généré, à la fréquence donnée (en hertz).
    #[expect(
        dead_code,
        reason = "choix proposé dans la fenêtre de config (étape 8)"
    )]
    Beep(f32),
}

/// Le son d'un emplacement : les échantillons déjà décodés, et l'état « muet ».
struct SlotSound {
    samples: SamplesBuffer,
    muted: bool,
}

/// Une sortie audio de l'ordinateur.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputDevice {
    /// Identifiant stable (sauvegardé) : le nom affiché peut être le même pour deux sorties.
    pub id: String,
    /// Nom affiché, par ex. « Haut-parleurs (Realtek(R) Audio) ».
    pub name: String,
}

/// Les sorties audio disponibles.
pub fn list_outputs() -> Vec<OutputDevice> {
    let Ok(devices) = rodio::cpal::default_host().output_devices() else {
        return Vec::new();
    };
    devices
        .filter_map(|device| {
            // `.ok()?` dans une closure de `filter_map` : une sortie illisible est ignorée.
            let id = device.id().ok()?.to_string();
            let name = device.description().ok()?.name().to_string();
            Some(OutputDevice { id, name })
        })
        .collect()
}

/// Intervalle minimal entre deux vérifications de la sortie par défaut de Windows.
const DEFAULT_CHECK_EVERY: Duration = Duration::from_secs(2);

/// L'identifiant de la sortie par défaut de Windows, en ce moment (coûte ~4 ms).
pub fn default_output_id() -> Option<String> {
    let device = rodio::cpal::default_host().default_output_device()?;
    Some(device.id().ok()?.to_string())
}

/// La sortie à ouvrir : celle demandée si elle existe encore, sinon `None` (= par défaut).
fn choose_output(wanted: Option<&str>, available: &[String]) -> Option<String> {
    let wanted = wanted?;
    available
        .iter()
        .any(|id| id == wanted)
        .then(|| wanted.to_string())
}

/// Les sorties à essayer, dans l'ordre (`None` = celle par défaut de Windows) :
/// 1. la sortie choisie, si elle est branchée ;
/// 2. celle par défaut de Windows ;
/// 3. la dernière qui marchait ;
/// 4. toutes les autres.
///
/// Une sortie peut refuser de s'ouvrir, par exemple quand une autre appli se la réserve
/// (Voicemeeter avec ses sorties matérielles) : on passe alors à la suivante, pour que le son
/// sorte quelque part plutôt que pas du tout.
fn candidates(
    wanted: Option<&str>,
    available: &[String],
    last: Option<&str>,
) -> Vec<Option<String>> {
    let mut list = Vec::new();
    if let Some(wanted) = choose_output(wanted, available) {
        list.push(Some(wanted));
    }
    list.push(None);
    if let Some(last) = last
        && available.iter().any(|id| id == last)
    {
        list.push(Some(last.to_string()));
    }
    for id in available {
        let candidate = Some(id.clone());
        if !list.contains(&candidate) {
            list.push(candidate);
        }
    }
    list
}

/// Ouvre la première sortie de la liste qui accepte.
fn open_first(
    candidates: &[Option<String>],
    lost: &Arc<AtomicBool>,
) -> Option<(MixerDeviceSink, String)> {
    // `find_map` : le premier élément pour lequel la fonction renvoie `Some`.
    candidates
        .iter()
        .find_map(|candidate| open_output(candidate.as_deref(), lost))
}

/// Ouvre une sortie (par son identifiant) ou celle par défaut (`None`). `lost` passe à `true`
/// si la sortie rencontre une erreur (par ex. casque débranché). Renvoie la sortie ouverte
/// et son identifiant.
fn open_output(id: Option<&str>, lost: &Arc<AtomicBool>) -> Option<(MixerDeviceSink, String)> {
    let host = rodio::cpal::default_host();
    let device = match id {
        Some(id) => host
            .output_devices()
            .ok()?
            .find(|device| device.id().is_ok_and(|d| d.to_string() == id))?,
        None => host.default_output_device()?,
    };
    let opened_id = device.id().ok()?.to_string();
    let lost = lost.clone();
    let mut sink = DeviceSinkBuilder::from_device(device)
        .ok()?
        // Appelée par la bibliothèque audio, depuis SON thread, en cas d'erreur de la sortie.
        // `AtomicBool` : un booléen modifiable sans risque depuis plusieurs threads.
        .with_error_callback(move |_error| lost.store(true, Ordering::Relaxed))
        .open_sink_or_fallback()
        .ok()?;
    // Évite un message dans la console quand l'appli se ferme.
    sink.log_on_drop(false);
    Some((sink, opened_id))
}

pub struct AudioEngine {
    /// La connexion au périphérique audio. Si on la détruit, plus aucun son ne sort :
    /// elle doit donc vivre aussi longtemps que le moteur (d'où ce champ, même s'il semble inutilisé).
    device: MixerDeviceSink,
    /// La sortie choisie (`None` : celle par défaut de Windows).
    wanted_output: Option<String>,
    /// L'identifiant de la sortie réellement ouverte.
    opened_output: String,
    /// Mis à `true` par la bibliothèque audio si la sortie a une erreur (débranchée…).
    lost: Arc<AtomicBool>,
    /// Dernière vérification de la sortie par défaut de Windows.
    last_default_check: Instant,
    /// Windows a changé de sortie par défaut : à rouvrir avant le prochain son.
    default_changed: bool,
    /// Sortie par défaut de Windows qui a refusé de s'ouvrir : on ne réessaie pas à chaque
    /// vérification (chaque essai retarderait un son), seulement si Windows en change.
    unusable_default: Option<String>,
    /// Volume général, de 0.0 (silence) à 1.0 (volume d'origine du son).
    volume: f32,
    sounds: HashMap<SoundSlot, SlotSound>,
    /// Le lecteur en cours pour chaque emplacement.
    players: HashMap<SoundSlot, Player>,
}

impl AudioEngine {
    /// Ouvre la sortie audio choisie (`None` : celle par défaut ; introuvable : celle par
    /// défaut aussi), règle le volume et charge les sons par défaut.
    /// Renvoie `None` s'il n'y a pas de sortie audio utilisable : l'appli tournera sans son.
    pub fn new(output: Option<String>, volume: f32) -> Option<Self> {
        let lost = Arc::new(AtomicBool::new(false));
        let available: Vec<String> = list_outputs().into_iter().map(|o| o.id).collect();
        let (device, opened_output) =
            open_first(&candidates(output.as_deref(), &available, None), &lost)?;

        let mut engine = Self {
            device,
            wanted_output: output,
            opened_output,
            lost,
            last_default_check: Instant::now(),
            default_changed: false,
            unusable_default: None,
            volume: clamp_volume(volume),
            sounds: HashMap::new(),
            players: HashMap::new(),
        };
        for slot in SoundSlot::ALL {
            engine.set_sound(slot, &slot.default_sound());
        }
        Some(engine)
    }

    /// Choisit le son d'un emplacement (décodé immédiatement et gardé en mémoire).
    pub fn set_sound(&mut self, slot: SoundSlot, choice: &SoundChoice) {
        let samples = load(choice).unwrap_or_else(|| {
            eprintln!("Son introuvable ({choice:?}) : bip de secours utilisé.");
            beep(slot.fallback_frequency())
        });
        // On conserve l'état « muet » s'il existait déjà.
        let muted = self.sounds.get(&slot).is_some_and(|s| s.muted);
        self.sounds.insert(slot, SlotSound { samples, muted });
    }

    /// Change de sortie audio (`None` : celle par défaut de Windows).
    pub fn set_output(&mut self, output: Option<String>) {
        self.wanted_output = output;
        self.reopen();
    }

    /// L'identifiant de la sortie réellement utilisée (elle peut différer de celle choisie,
    /// si celle-ci a refusé de s'ouvrir).
    pub fn opened_output(&self) -> &str {
        &self.opened_output
    }

    /// Rouvre une sortie, en suivant l'ordre de `candidates`.
    fn reopen(&mut self) {
        self.lost.store(false, Ordering::Relaxed);
        self.default_changed = false;
        let available: Vec<String> = list_outputs().into_iter().map(|o| o.id).collect();
        let list = candidates(
            self.wanted_output.as_deref(),
            &available,
            Some(&self.opened_output),
        );
        if let Some((device, id)) = open_first(&list, &self.lost) {
            // Les lecteurs en cours étaient branchés sur l'ancienne sortie : on les oublie.
            self.players.clear();
            self.device = device;
            self.opened_output = id;
        }
        // En mode « par défaut » : si la sortie par défaut a refusé, on le retient.
        self.unusable_default = None;
        if self.wanted_output.is_none() {
            self.unusable_default =
                default_output_id().filter(|default| *default != self.opened_output);
        }
    }

    /// AVANT de jouer : la sortie a-t-elle eu une erreur (débranchée), ou Windows a-t-il
    /// changé de sortie par défaut ? Si oui, on la rouvre. (Lecture de deux booléens : gratuit
    /// tant que rien n'a changé.)
    fn reopen_if_needed(&mut self) {
        if self.lost.load(Ordering::Relaxed) || self.default_changed {
            self.reopen();
        }
    }

    /// APRÈS avoir lancé un son : en mode « par défaut », Windows a-t-il changé de sortie par
    /// défaut (casque branché…) ? Demander à Windows coûte ~4 ms : on le fait après le départ
    /// du son (il joue dans un autre thread, il n'est donc pas retardé), et au plus toutes
    /// les 2 secondes. On ne rouvre PAS tout de suite (fermer l'ancienne sortie couperait le
    /// son qui vient de partir) : c'est le son SUIVANT qui partira sur la nouvelle sortie.
    fn follow_default_output(&mut self) {
        if self.wanted_output.is_some() || self.last_default_check.elapsed() < DEFAULT_CHECK_EVERY {
            return;
        }
        self.last_default_check = Instant::now();
        // Changement : une sortie par défaut différente de celle ouverte, et qui n'a pas déjà
        // refusé de s'ouvrir.
        let default = default_output_id();
        if default.as_ref().is_some_and(|id| {
            *id != self.opened_output && Some(id) != self.unusable_default.as_ref()
        }) {
            self.default_changed = true;
        }
    }

    /// Coupe ou réactive le son d'un emplacement.
    // `#[expect(dead_code)]` : on SAIT que cette fonction n'est pas encore utilisée
    // (le réglage arrive à l'étape 8). Quand elle le sera, le compilateur nous dira
    // de retirer cette ligne.
    #[expect(dead_code, reason = "réglage branché à l'étape 8 (fenêtre de config)")]
    pub fn set_muted(&mut self, slot: SoundSlot, muted: bool) {
        if let Some(sound) = self.sounds.get_mut(&slot) {
            sound.muted = muted;
        }
    }

    /// Règle le volume général (ramené entre 0.0 et 1.0).
    pub fn set_volume(&mut self, volume: f32) {
        self.volume = clamp_volume(volume);
    }

    /// Joue un bip de test au volume actuel (au relâchement du curseur de volume).
    pub fn play_test_beep(&mut self) {
        self.play_samples(SoundSlot::Increment, beep(660.0));
    }

    /// Joue le son d'un emplacement (sauf s'il est muet).
    pub fn play(&mut self, slot: SoundSlot) {
        let Some(sound) = self.sounds.get(&slot) else {
            return;
        };
        if sound.muted {
            return;
        }
        // `clone()` d'un `SamplesBuffer` est quasi gratuit : les échantillons sont partagés
        // (compteur de références `Arc`), on ne copie pas les données.
        let samples = sound.samples.clone();
        self.play_samples(slot, samples);
    }

    fn play_samples(&mut self, slot: SoundSlot, samples: SamplesBuffer) {
        self.reopen_if_needed();
        let player = Player::connect_new(self.device.mixer());
        player.set_volume(self.volume);
        player.append(samples);
        // On range le nouveau lecteur à la place de l'ancien. L'ancien est alors DÉTRUIT
        // (« drop »), ce qui ARRÊTE automatiquement son son : c'est comme ça que le
        // nouveau son coupe le précédent, sans code supplémentaire.
        self.players.insert(slot, player);
        self.follow_default_output();
    }
}

/// Charge et décode entièrement un son. `None` si c'est impossible.
fn load(choice: &SoundChoice) -> Option<SamplesBuffer> {
    match choice {
        SoundChoice::Beep(frequency) => Some(beep(*frequency)),
        SoundChoice::Windows(name) => {
            let file = File::open(windows_media_dir().join(name)).ok()?;
            let decoder = Decoder::try_from(file).ok()?;
            let channels = decoder.channels();
            let sample_rate = decoder.sample_rate();
            // Un décodeur est un ITÉRATEUR d'échantillons : `collect()` les lit tous d'un coup.
            let samples: Vec<f32> = decoder.collect();
            Some(SamplesBuffer::new(channels, sample_rate, samples))
        }
    }
}

/// Le dossier des sons de Windows : `%SystemRoot%\Media` (en général `C:\Windows\Media`).
fn windows_media_dir() -> PathBuf {
    let system_root = std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into());
    PathBuf::from(system_root).join("Media")
}

/// Ramène un volume entre 0.0 et 1.0.
fn clamp_volume(volume: f32) -> f32 {
    volume.clamp(0.0, 1.0)
}

const BEEP_SAMPLE_RATE: u32 = 48_000; // échantillons par seconde
const BEEP_DURATION_SECS: f32 = 0.08; // 80 ms
const BEEP_FADE_SECS: f32 = 0.01; // fondu de 10 ms au début et à la fin
const BEEP_AMPLITUDE: f32 = 0.3; // pas trop fort

/// Génère un bip : une onde sinusoïdale courte, avec un fondu pour éviter les « clics ».
fn beep(frequency: f32) -> SamplesBuffer {
    let samples = beep_samples(frequency);
    // `NonZero` : un entier dont le TYPE garantit qu'il n'est pas nul (une fréquence
    // d'échantillonnage de 0 n'a pas de sens). `.expect(...)` : on est sûr que ce n'est pas 0.
    let channels = NonZero::new(1).expect("1 n'est pas nul");
    let rate = NonZero::new(BEEP_SAMPLE_RATE).expect("la fréquence n'est pas nulle");
    SamplesBuffer::new(channels, rate, samples)
}

/// Les échantillons du bip (séparés de `beep` pour pouvoir les tester).
fn beep_samples(frequency: f32) -> Vec<f32> {
    let rate = BEEP_SAMPLE_RATE as f32;
    let total = (rate * BEEP_DURATION_SECS) as usize;
    let fade = (rate * BEEP_FADE_SECS) as usize;

    (0..total)
        .map(|i| {
            let t = i as f32 / rate; // le temps, en secondes
            let wave = (2.0 * std::f32::consts::PI * frequency * t).sin();
            // Enveloppe : monte de 0 à 1 au début, redescend à 0 à la fin.
            let envelope = (i.min(total - 1 - i) as f32 / fade as f32).min(1.0);
            wave * envelope * BEEP_AMPLITUDE
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_action_has_its_slot() {
        assert_eq!(
            SoundSlot::for_action(CounterAction::Add(1)),
            SoundSlot::Increment
        );
        assert_eq!(
            SoundSlot::for_action(CounterAction::Add(5)),
            SoundSlot::Increment
        );
        assert_eq!(
            SoundSlot::for_action(CounterAction::Subtract(1)),
            SoundSlot::Decrement
        );
    }

    #[test]
    fn default_sounds_are_speech_on_and_off() {
        assert_eq!(
            SoundSlot::Increment.default_sound(),
            SoundChoice::Windows("Speech On.wav".into())
        );
        assert_eq!(
            SoundSlot::Decrement.default_sound(),
            SoundChoice::Windows("Speech Off.wav".into())
        );
    }

    #[test]
    fn volume_is_clamped_between_0_and_1() {
        assert_eq!(clamp_volume(-0.5), 0.0);
        assert_eq!(clamp_volume(0.4), 0.4);
        assert_eq!(clamp_volume(3.0), 1.0);
    }

    #[test]
    fn beep_lasts_80_ms() {
        assert_eq!(beep_samples(440.0).len(), 3_840); // 48 000 × 0,08
    }

    #[test]
    fn beep_starts_and_ends_silent_and_never_exceeds_amplitude() {
        let samples = beep_samples(440.0);
        assert_eq!(samples[0], 0.0);
        assert!(samples[samples.len() - 1].abs() < 1e-6);
        assert!(samples.iter().all(|s| s.abs() <= BEEP_AMPLITUDE));
    }

    #[test]
    fn chosen_output_is_kept_if_available() {
        let available = vec!["A".to_string(), "B".to_string()];
        assert_eq!(choose_output(Some("B"), &available), Some("B".to_string()));
    }

    #[test]
    fn missing_or_default_output_gives_default() {
        let available = vec!["A".to_string()];
        assert_eq!(choose_output(Some("casque débranché"), &available), None);
        assert_eq!(choose_output(None, &available), None);
    }

    #[test]
    fn candidates_order() {
        let available = vec!["A".to_string(), "B".to_string(), "C".to_string()];
        // Choisie (B), puis défaut, puis la dernière qui marchait (C), puis les autres (A).
        assert_eq!(
            candidates(Some("B"), &available, Some("C")),
            vec![
                Some("B".to_string()),
                None,
                Some("C".to_string()),
                Some("A".to_string())
            ]
        );
    }

    #[test]
    fn unplugged_choice_is_skipped_in_candidates() {
        let available = vec!["A".to_string()];
        assert_eq!(
            candidates(Some("casque débranché"), &available, None),
            vec![None, Some("A".to_string())]
        );
    }

    #[test]
    fn missing_windows_sound_cannot_be_loaded() {
        // Un fichier inexistant donne `None` : le moteur utilisera alors le bip de secours.
        assert!(load(&SoundChoice::Windows("n'existe pas.wav".into())).is_none());
    }

    #[test]
    #[cfg(windows)]
    fn default_windows_sounds_load() {
        // Vérifie sur CE PC que les deux sons par défaut existent et se décodent.
        for slot in SoundSlot::ALL {
            assert!(load(&slot.default_sound()).is_some(), "{slot:?}");
        }
    }
}
