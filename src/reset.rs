//! Confirmation du reset du compteur global : « Reset ? », puis une touche de validation
//! dans un délai donné. Sans validation, la demande expire toute seule.
//!
//! L'heure (`now`) est passée en paramètre plutôt que lue à l'intérieur : les tests peuvent
//! ainsi simuler le passage du temps sans attendre réellement.

use std::time::{Duration, Instant};

/// Délai pour valider un reset.
pub const CONFIRM_DELAY: Duration = Duration::from_secs(3);

#[derive(Debug, Default)]
pub struct ResetConfirmation {
    /// Moment de la demande en attente. `None` : aucune demande.
    requested_at: Option<Instant>,
}

impl ResetConfirmation {
    /// Demande un reset (ou relance le délai si une demande est déjà en cours).
    pub fn request(&mut self, now: Instant) {
        self.requested_at = Some(now);
    }

    /// Une demande est-elle en attente (et pas encore expirée) ?
    pub fn is_pending(&self, now: Instant) -> bool {
        self.remaining(now).is_some()
    }

    /// Temps restant pour valider, ou `None` s'il n'y a pas de demande en cours.
    pub fn remaining(&self, now: Instant) -> Option<Duration> {
        let requested_at = self.requested_at?;
        // `checked_sub` : `None` si le délai est dépassé (au lieu d'une durée négative, impossible).
        CONFIRM_DELAY
            .checked_sub(now.duration_since(requested_at))
            .filter(|remaining| !remaining.is_zero())
    }

    /// Valide la demande. Renvoie `true` si une demande était en attente (le reset doit
    /// alors être fait), `false` sinon (validation ignorée). Dans tous les cas, plus rien
    /// n'est en attente ensuite.
    pub fn confirm(&mut self, now: Instant) -> bool {
        let pending = self.is_pending(now);
        self.requested_at = None;
        pending
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_pending_at_start() {
        let confirmation = ResetConfirmation::default();
        assert!(!confirmation.is_pending(Instant::now()));
    }

    #[test]
    fn confirm_within_delay_resets() {
        let t0 = Instant::now();
        let mut confirmation = ResetConfirmation::default();
        confirmation.request(t0);
        assert!(confirmation.is_pending(t0 + Duration::from_secs(1)));
        assert!(confirmation.confirm(t0 + Duration::from_secs(2)));
    }

    #[test]
    fn confirm_after_delay_is_ignored() {
        let t0 = Instant::now();
        let mut confirmation = ResetConfirmation::default();
        confirmation.request(t0);
        assert!(!confirmation.is_pending(t0 + Duration::from_secs(3)));
        assert!(!confirmation.confirm(t0 + Duration::from_secs(4)));
    }

    #[test]
    fn confirm_without_request_is_ignored() {
        let mut confirmation = ResetConfirmation::default();
        assert!(!confirmation.confirm(Instant::now()));
    }

    #[test]
    fn confirm_only_once() {
        let t0 = Instant::now();
        let mut confirmation = ResetConfirmation::default();
        confirmation.request(t0);
        assert!(confirmation.confirm(t0));
        assert!(!confirmation.confirm(t0)); // la demande est consommée
    }

    #[test]
    fn new_request_restarts_the_delay() {
        let t0 = Instant::now();
        let mut confirmation = ResetConfirmation::default();
        confirmation.request(t0);
        confirmation.request(t0 + Duration::from_secs(2));
        // 4 s après la 1re demande, mais 2 s après la 2e : toujours en attente.
        assert!(confirmation.is_pending(t0 + Duration::from_secs(4)));
    }

    #[test]
    fn remaining_time_decreases() {
        let t0 = Instant::now();
        let mut confirmation = ResetConfirmation::default();
        confirmation.request(t0);
        assert_eq!(
            confirmation.remaining(t0 + Duration::from_secs(1)),
            Some(Duration::from_secs(2))
        );
    }
}
