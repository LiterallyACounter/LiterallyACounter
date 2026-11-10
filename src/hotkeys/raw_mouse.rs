//! Logique pure de l'écoute de la souris par Raw Input (testable sans souris).
//!
//! Raw Input : Windows envoie à l'appli une COPIE de chaque événement souris (mouvement, bouton,
//! molette). L'appli n'est pas sur le trajet des événements : aucun délai pour le jeu.
//! Chaque événement porte des « drapeaux » (un bit par changement de bouton, un bit pour la
//! molette) et, pour la molette, une quantité de rotation.

use std::time::{Duration, Instant};

use super::Key;
use super::mouse::ButtonChange;

/// Les drapeaux de Raw Input pour les boutons : (bit « enfoncé », bit « relâché », bouton).
const BUTTON_FLAGS: [(u16, u16, Key); 5] = [
    (0x0001, 0x0002, Key::MOUSE_LEFT),
    (0x0004, 0x0008, Key::MOUSE_RIGHT),
    (0x0010, 0x0020, Key::MOUSE_MIDDLE),
    (0x0040, 0x0080, Key::MOUSE_X1),
    (0x0100, 0x0200, Key::MOUSE_X2),
];

/// Bit « la molette verticale a tourné ».
pub const WHEEL_FLAG: u16 = 0x0400;
/// Bit « la molette horizontale a tourné » (inclinaison gauche / droite).
pub const HORIZONTAL_WHEEL_FLAG: u16 = 0x0800;

/// Une rotation d'UN cran, en unités Windows (`WHEEL_DELTA`).
const NOTCH: i32 = 120;

/// Les boutons enfoncés ou relâchés dans un événement.
/// `&` : « et » bit à bit, pour tester si un bit est à 1.
pub fn button_changes(flags: u16) -> Vec<ButtonChange> {
    let mut changes = Vec::new();
    for (down, up, key) in BUTTON_FLAGS {
        if flags & down != 0 {
            changes.push(ButtonChange::Pressed(key));
        }
        if flags & up != 0 {
            changes.push(ButtonChange::Released(key));
        }
    }
    changes
}

/// Cumule les rotations de la molette et en tire des crans entiers.
///
/// Une molette classique envoie 120 par cran. Une molette haute résolution ou « libre » envoie
/// des fractions (par ex. 30) : on les additionne jusqu'à un cran complet.
#[derive(Debug, Default)]
pub struct WheelAccumulator {
    vertical: i32,
    horizontal: i32,
}

impl WheelAccumulator {
    /// Molette verticale : positif = vers le haut (on pousse la molette loin de soi).
    pub fn add_vertical(&mut self, delta: i16) -> Vec<Key> {
        notches(&mut self.vertical, delta, Key::WHEEL_UP, Key::WHEEL_DOWN)
    }

    /// Molette horizontale : positif = vers la droite.
    pub fn add_horizontal(&mut self, delta: i16) -> Vec<Key> {
        notches(
            &mut self.horizontal,
            delta,
            Key::WHEEL_RIGHT,
            Key::WHEEL_LEFT,
        )
    }
}

/// Ajoute `delta` au cumul `total` et renvoie un déclencheur par cran complet.
fn notches(total: &mut i32, delta: i16, positive: Key, negative: Key) -> Vec<Key> {
    let delta = i32::from(delta);
    // Changement de sens : on oublie le reste accumulé dans l'autre sens.
    // `signum()` : -1, 0 ou 1 selon le signe.
    if delta.signum() != total.signum() {
        *total = 0;
    }
    *total += delta;
    let mut keys = Vec::new();
    while *total >= NOTCH {
        keys.push(positive);
        *total -= NOTCH;
    }
    while *total <= -NOTCH {
        keys.push(negative);
        *total += NOTCH;
    }
    keys
}

/// Durée d'une mesure de fréquence.
const RATE_WINDOW: Duration = Duration::from_secs(2);
/// Au-delà, la fréquence est jugée élevée (spec F5 : ~500 événements par seconde).
const HIGH_RATE: u32 = 500;
/// Nombre de mesures élevées consécutives avant d'avertir (2 × 2 s = 4 s).
const HIGH_WINDOWS_BEFORE_WARNING: u32 = 2;

/// Ce que la mesure de fréquence signale à l'appli.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateReport {
    /// Fréquence élevée, en événements par seconde.
    High(u32),
    /// Retour à la normale après un avertissement.
    Normal,
}

/// Mesure le nombre d'événements souris par seconde, par tranches de 2 secondes.
#[derive(Debug, Default)]
pub struct RateMonitor {
    /// Début de la tranche en cours.
    window_start: Option<Instant>,
    /// Événements reçus dans la tranche en cours.
    count: u32,
    /// Tranches élevées consécutives.
    high_windows: u32,
    /// Un avertissement est-il affiché ?
    warned: bool,
}

impl RateMonitor {
    /// À appeler à chaque événement. Renvoie un signalement quand il y a lieu.
    pub fn event(&mut self, now: Instant) -> Option<RateReport> {
        self.count += 1;
        // `get_or_insert(now)` : la valeur si elle existe, sinon on y met `now`.
        let start = *self.window_start.get_or_insert(now);
        let elapsed = now.duration_since(start);
        if elapsed < RATE_WINDOW {
            return None;
        }
        // Fin de la tranche : fréquence moyenne, puis nouvelle tranche.
        let rate = (f64::from(self.count) / elapsed.as_secs_f64()).round() as u32;
        self.count = 0;
        self.window_start = Some(now);
        if rate > HIGH_RATE {
            self.high_windows += 1;
            if self.high_windows >= HIGH_WINDOWS_BEFORE_WARNING {
                self.warned = true;
                return Some(RateReport::High(rate));
            }
        } else {
            self.high_windows = 0;
            if self.warned {
                self.warned = false;
                return Some(RateReport::Normal);
            }
        }
        None
    }

    /// Remet la mesure à zéro (abonnement arrêté). Renvoie `true` si un avertissement
    /// était affiché (il faut alors l'effacer).
    pub fn reset(&mut self) -> bool {
        let was_warned = self.warned;
        *self = Self::default();
        was_warned
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn button_flags_become_presses_and_releases() {
        assert_eq!(
            button_changes(0x0001),
            vec![ButtonChange::Pressed(Key::MOUSE_LEFT)]
        );
        assert_eq!(
            button_changes(0x0200),
            vec![ButtonChange::Released(Key::MOUSE_X2)]
        );
        // Plusieurs changements dans un même événement.
        assert_eq!(
            button_changes(0x0010 | 0x0080),
            vec![
                ButtonChange::Pressed(Key::MOUSE_MIDDLE),
                ButtonChange::Released(Key::MOUSE_X1)
            ]
        );
        // Un simple mouvement : aucun bouton.
        assert!(button_changes(0).is_empty());
    }

    #[test]
    fn one_notch_per_120() {
        let mut wheel = WheelAccumulator::default();
        assert_eq!(wheel.add_vertical(120), vec![Key::WHEEL_UP]);
        assert_eq!(
            wheel.add_vertical(-240),
            vec![Key::WHEEL_DOWN, Key::WHEEL_DOWN]
        );
        assert_eq!(wheel.add_horizontal(120), vec![Key::WHEEL_RIGHT]);
        assert_eq!(wheel.add_horizontal(-120), vec![Key::WHEEL_LEFT]);
    }

    #[test]
    fn fractions_add_up_to_a_notch() {
        let mut wheel = WheelAccumulator::default();
        assert!(wheel.add_vertical(40).is_empty());
        assert!(wheel.add_vertical(40).is_empty());
        assert_eq!(wheel.add_vertical(40), vec![Key::WHEEL_UP]);
    }

    #[test]
    fn changing_direction_forgets_the_rest() {
        let mut wheel = WheelAccumulator::default();
        wheel.add_vertical(100); // presque un cran vers le haut…
        // … puis 30 vers le bas : pas de cran (le reste vers le haut est oublié).
        assert!(wheel.add_vertical(-30).is_empty());
        assert_eq!(wheel.add_vertical(-90), vec![Key::WHEEL_DOWN]);
    }

    /// Envoie `per_second` événements par seconde pendant 2 s, à partir de `start`.
    fn feed(monitor: &mut RateMonitor, start: Instant, per_second: u32) -> Option<RateReport> {
        let total = per_second * 2;
        let mut report = None;
        for i in 0..=total {
            let at = start + Duration::from_secs(2) * i / total;
            if let Some(r) = monitor.event(at) {
                report = Some(r);
            }
        }
        report
    }

    #[test]
    fn normal_rate_reports_nothing() {
        let mut monitor = RateMonitor::default();
        let t0 = Instant::now();
        assert_eq!(feed(&mut monitor, t0, 130), None);
        assert_eq!(feed(&mut monitor, t0 + Duration::from_secs(2), 130), None);
    }

    #[test]
    fn high_rate_warns_after_two_windows_then_clears() {
        let mut monitor = RateMonitor::default();
        let t0 = Instant::now();
        let s = Duration::from_secs(2);
        // Première tranche élevée : pas encore d'avertissement (un pic bref ne compte pas).
        assert_eq!(feed(&mut monitor, t0, 1000), None);
        // Deuxième : avertissement, avec la fréquence mesurée.
        assert!(matches!(
            feed(&mut monitor, t0 + s, 1000),
            Some(RateReport::High(rate)) if (950..=1050).contains(&rate)
        ));
        // Retour à la normale : on efface l'avertissement.
        assert_eq!(
            feed(&mut monitor, t0 + s * 2, 100),
            Some(RateReport::Normal)
        );
    }

    #[test]
    fn reset_says_if_a_warning_was_shown() {
        let mut monitor = RateMonitor::default();
        assert!(!monitor.reset());
        let t0 = Instant::now();
        feed(&mut monitor, t0, 1000);
        feed(&mut monitor, t0 + Duration::from_secs(2), 1000);
        assert!(monitor.reset());
    }
}
