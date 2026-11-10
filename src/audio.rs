//! Moteur audio : joue un son à chaque action, sans bloquer l'interface ni le hook clavier.
//!
//! - Les sons sont décodés UNE SEULE FOIS au chargement et gardés en mémoire :
//!   au moment de l'appui, aucun accès disque (latence minimale, pas de saccade en jeu).
//! - Chaque « emplacement » (`SoundSlot` : +, −…) a son propre lecteur. Un nouveau son
//!   COUPE le précédent du même emplacement, pour éviter la cacophonie en cas d'appuis rapides.
//! - Si un son Windows est introuvable, on le remplace par un bip généré (son de secours).
//! - S'il n'y a aucun périphérique audio, l'appli fonctionne quand même, sans son.

use std::collections::HashMap;
use std::fs::File;
use std::num::NonZero;
use std::path::PathBuf;

use rodio::buffer::SamplesBuffer;
use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player, Source};

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

pub struct AudioEngine {
    /// La connexion au périphérique audio. Si on la détruit, plus aucun son ne sort :
    /// elle doit donc vivre aussi longtemps que le moteur (d'où ce champ, même s'il semble inutilisé).
    device: MixerDeviceSink,
    /// Volume général, de 0.0 (silence) à 1.0 (volume d'origine du son).
    volume: f32,
    sounds: HashMap<SoundSlot, SlotSound>,
    /// Le lecteur en cours pour chaque emplacement.
    players: HashMap<SoundSlot, Player>,
}

impl AudioEngine {
    /// Ouvre la sortie audio par défaut et charge les sons par défaut.
    /// Renvoie `None` s'il n'y a pas de sortie audio utilisable : l'appli tournera sans son.
    pub fn new() -> Option<Self> {
        // `.ok()?` : convertit le `Result` en `Option`, puis `?` sort de la fonction
        // en renvoyant `None` si l'ouverture a échoué.
        let mut device = DeviceSinkBuilder::open_default_sink().ok()?;
        // Évite un message dans la console quand l'appli se ferme.
        device.log_on_drop(false);

        let mut engine = Self {
            device,
            volume: 1.0,
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
    #[expect(dead_code, reason = "curseur de volume branché à l'étape 8")]
    pub fn set_volume(&mut self, volume: f32) {
        self.volume = clamp_volume(volume);
    }

    /// Joue un bip de test au volume actuel (au relâchement du curseur de volume).
    #[expect(dead_code, reason = "curseur de volume branché à l'étape 8")]
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
        let player = Player::connect_new(self.device.mixer());
        player.set_volume(self.volume);
        player.append(samples);
        // On range le nouveau lecteur à la place de l'ancien. L'ancien est alors DÉTRUIT
        // (« drop »), ce qui ARRÊTE automatiquement son son : c'est comme ça que le
        // nouveau son coupe le précédent, sans code supplémentaire.
        self.players.insert(slot, player);
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
