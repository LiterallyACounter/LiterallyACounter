//! Logique pure des deux compteurs de l'appli : le GLOBAL (persistant) et la PARTIE
//! (temporaire). On y décide quel compteur est touché par chaque commande, quel son jouer
//! et s'il faut sauvegarder.
//!
//! Comme `counter.rs`, ce module ne connaît ni l'affichage, ni le clavier, ni le son réel :
//! il RENVOIE ce qu'il faut faire (`Outcome`), et c'est `main.rs` qui l'exécute.
//! On peut donc tout tester automatiquement.

use std::time::Instant;

use crate::audio::SoundSlot;
use crate::command::Command;
use crate::counter::{Counter, CounterAction};
use crate::reset::ResetConfirmation;

/// Désigne un des deux compteurs.
///
/// Un enum SANS donnée sert ici d'identifiant : impossible de désigner un compteur
/// qui n'existe pas (contrairement à un numéro, qui pourrait valoir 7).
///
/// `Default` + `#[default]` : la valeur par défaut est la session ; elle sert pour le son
/// prioritaire (`Tracker::sound_priority`), dont la session est le réglage d'origine.
#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize,
)]
pub enum CounterId {
    /// Compteur persistant (sauvegardé, reset avec confirmation).
    Global,
    /// Compteur de session (repart de 0 au lancement, reset immédiat).
    #[default]
    Round,
}

impl CounterId {
    /// Les deux compteurs, pour les parcourir avec `for id in CounterId::ALL`.
    pub const ALL: [CounterId; 2] = [CounterId::Global, CounterId::Round];
}

/// Les compteurs visés par une commande « compter » : l'un, l'autre, ou les deux.
///
/// Deux booléens plutôt qu'une liste : la struct reste `Copy` (copiable librement,
/// comme un nombre), et on ne peut pas citer deux fois le même compteur.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Targets {
    pub global: bool,
    pub round: bool,
}

impl Targets {
    /// Les deux compteurs (le cas par défaut pour +1 / −1).
    pub const BOTH: Targets = Targets {
        global: true,
        round: true,
    };

    /// Un seul compteur.
    pub fn only(id: CounterId) -> Self {
        Targets {
            global: id == CounterId::Global,
            round: id == CounterId::Round,
        }
    }

    /// Ce compteur est-il visé ?
    pub fn contains(self, id: CounterId) -> bool {
        match id {
            CounterId::Global => self.global,
            CounterId::Round => self.round,
        }
    }
}

/// Ce que `main.rs` doit faire après une commande.
///
/// `#[derive(Default)]` : par défaut, rien à faire (`None`, `false`, `false`).
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Outcome {
    /// Son à jouer, et le compteur dont on prend le son (un seul son, même si la commande a
    /// touché les deux compteurs : c'est alors celui de la partie).
    pub sound: Option<(CounterId, SoundSlot)>,
    /// La valeur du global a changé : il faut sauvegarder.
    pub save: bool,
    /// Une demande « Reset ? » vient de commencer : il faudra redessiner à son expiration.
    pub reset_prompt_started: bool,
}

/// L'état des deux compteurs.
#[derive(Debug, Default)]
pub struct Tracker {
    global: Counter,
    round: Counter,
    /// Demande de reset du global en attente de validation.
    reset_confirmation: ResetConfirmation,
    /// Le compteur resetté en dernier : c'est lui que « Annuler le reset » restaure.
    last_reset: Option<CounterId>,
    /// Le compteur dont le son est joué quand une commande fait compter les deux
    /// (réglage « Son prioritaire » ; la session par défaut).
    pub sound_priority: CounterId,
}

impl Tracker {
    /// `global` : le compteur global, repris de la sauvegarde. La partie, elle, repart de 0.
    pub fn new(global: Counter, round: Counter) -> Self {
        Self {
            global,
            round,
            ..Self::default()
        }
    }

    /// Lecture d'un compteur.
    pub fn counter(&self, id: CounterId) -> &Counter {
        match id {
            CounterId::Global => &self.global,
            CounterId::Round => &self.round,
        }
    }

    /// Écriture d'un compteur (privée : on passe par `handle` pour respecter les règles).
    /// `&mut Counter` : un emprunt MODIFIABLE, prélevé dans `self` emprunté en modification.
    fn counter_mut(&mut self, id: CounterId) -> &mut Counter {
        match id {
            CounterId::Global => &mut self.global,
            CounterId::Round => &mut self.round,
        }
    }

    /// Fixe la valeur d'un compteur (saisie dans le menu). Renvoie `true` s'il faut
    /// sauvegarder (le global a changé).
    pub fn set_value(&mut self, id: CounterId, value: u64) -> bool {
        self.counter_mut(id).set_value(value);
        if self.last_reset == Some(id) {
            self.last_reset = None;
        }
        id == CounterId::Global
    }

    /// Change l'objectif d'un compteur (depuis la configuration).
    pub fn set_goal(&mut self, id: CounterId, goal: u64) {
        self.counter_mut(id).set_goal(goal);
    }

    /// Une demande de reset du global est-elle affichée ?
    pub fn reset_pending(&self, now: Instant) -> bool {
        self.reset_confirmation.is_pending(now)
    }

    /// Exécute une commande et dit ce qu'il reste à faire (son, sauvegarde…).
    pub fn handle(&mut self, command: Command, now: Instant) -> Outcome {
        match command {
            Command::Count(targets, action) => self.count(targets, action),
            // Partie : reset immédiat.
            Command::Reset(CounterId::Round) => {
                self.reset(CounterId::Round);
                Outcome {
                    sound: Some((CounterId::Round, SoundSlot::ResetDone)),
                    ..Outcome::default()
                }
            }
            // Global : on affiche « Reset ? » et on attend la validation.
            Command::Reset(CounterId::Global) => {
                self.reset_confirmation.request(now);
                Outcome {
                    sound: Some((CounterId::Global, SoundSlot::ResetPrompt)),
                    reset_prompt_started: true,
                    ..Outcome::default()
                }
            }
            // Validation sans demande en cours (ou trop tard) : ignorée, sans bruit.
            Command::ConfirmReset => {
                if !self.reset_confirmation.confirm(now) {
                    return Outcome::default();
                }
                self.reset(CounterId::Global);
                Outcome {
                    sound: Some((CounterId::Global, SoundSlot::ResetDone)),
                    save: true,
                    ..Outcome::default()
                }
            }
            Command::UndoReset => self.undo_reset(),
        }
    }

    fn count(&mut self, targets: Targets, action: CounterAction) -> Outcome {
        for id in CounterId::ALL {
            if targets.contains(id) {
                self.counter_mut(id).apply(action);
            }
        }
        // Recompter rend l'annulation impossible (règle de `Counter`) : on oublie alors
        // le dernier reset, s'il concernait un compteur qu'on vient de toucher.
        // `is_some_and` : vrai si c'est `Some(x)` ET que la condition sur `x` est vraie.
        if self.last_reset.is_some_and(|id| targets.contains(id)) {
            self.last_reset = None;
        }
        // Un seul son : si les deux comptent, celui du compteur prioritaire ; sinon, celui
        // du seul compteur touché.
        let sound_counter = match (targets.global, targets.round) {
            (true, true) => self.sound_priority,
            (false, true) => CounterId::Round,
            _ => CounterId::Global,
        };
        Outcome {
            sound: Some((sound_counter, SoundSlot::for_action(action))),
            save: targets.global,
            ..Outcome::default()
        }
    }

    fn reset(&mut self, id: CounterId) {
        self.counter_mut(id).reset();
        self.last_reset = Some(id);
    }

    fn undo_reset(&mut self) -> Outcome {
        // `let Some(id) = ... else { return ...; }` : on sort tout de suite s'il n'y a rien.
        // `take()` : récupère la valeur et laisse `None` à sa place (on n'annule qu'une fois).
        let Some(id) = self.last_reset.take() else {
            return Outcome::default();
        };
        if !self.counter_mut(id).undo_reset() {
            return Outcome::default();
        }
        Outcome {
            sound: Some((id, SoundSlot::ResetUndone)),
            save: id == CounterId::Global,
            ..Outcome::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    const PLUS_BOTH: Command = Command::Count(Targets::BOTH, CounterAction::Add(1));

    /// Un tracker avec le global à `global` et la partie à `round`.
    fn tracker(global: u64, round: u64) -> Tracker {
        Tracker::new(Counter::new(global, 0), Counter::new(round, 0))
    }

    fn values(t: &Tracker) -> (u64, u64) {
        (
            t.counter(CounterId::Global).value(),
            t.counter(CounterId::Round).value(),
        )
    }

    #[test]
    fn plus_counts_in_both_and_plays_one_sound() {
        let mut t = tracker(14, 0);
        let outcome = t.handle(PLUS_BOTH, Instant::now());
        assert_eq!(values(&t), (15, 1));
        // Les deux compteurs comptent : c'est le son de la partie.
        assert_eq!(
            outcome.sound,
            Some((CounterId::Round, SoundSlot::Increment))
        );
        assert!(outcome.save);
    }

    #[test]
    fn sound_priority_picks_the_sound_when_both_count() {
        let mut t = tracker(14, 0);
        t.sound_priority = CounterId::Global;
        let outcome = t.handle(PLUS_BOTH, Instant::now());
        assert_eq!(
            outcome.sound,
            Some((CounterId::Global, SoundSlot::Increment))
        );
        // La priorité ne joue que si les deux comptent : la session seule garde son son.
        let only_round = Targets::only(CounterId::Round);
        let outcome = t.handle(
            Command::Count(only_round, CounterAction::Add(1)),
            Instant::now(),
        );
        assert_eq!(
            outcome.sound,
            Some((CounterId::Round, SoundSlot::Increment))
        );
    }

    #[test]
    fn counting_only_the_global_plays_the_global_sound() {
        let mut t = tracker(14, 0);
        let only_global = Targets::only(CounterId::Global);
        let outcome = t.handle(
            Command::Count(only_global, CounterAction::Subtract(1)),
            Instant::now(),
        );
        assert_eq!(
            outcome.sound,
            Some((CounterId::Global, SoundSlot::Decrement))
        );
    }

    #[test]
    fn counting_only_the_round_does_not_save() {
        let mut t = tracker(14, 0);
        let only_round = Targets::only(CounterId::Round);
        let outcome = t.handle(
            Command::Count(only_round, CounterAction::Add(1)),
            Instant::now(),
        );
        assert_eq!(values(&t), (14, 1));
        assert!(!outcome.save);
    }

    #[test]
    fn round_reset_is_immediate_and_keeps_global() {
        let mut t = tracker(14, 3);
        let outcome = t.handle(Command::Reset(CounterId::Round), Instant::now());
        assert_eq!(values(&t), (14, 0));
        assert_eq!(
            outcome.sound,
            Some((CounterId::Round, SoundSlot::ResetDone))
        );
        assert!(!outcome.save);
    }

    #[test]
    fn global_reset_needs_confirmation_and_keeps_round() {
        let t0 = Instant::now();
        let mut t = tracker(14, 3);
        let outcome = t.handle(Command::Reset(CounterId::Global), t0);
        assert!(outcome.reset_prompt_started);
        assert_eq!(values(&t), (14, 3)); // rien n'a encore changé
        assert!(t.reset_pending(t0));

        let outcome = t.handle(Command::ConfirmReset, t0 + Duration::from_secs(1));
        assert_eq!(values(&t), (0, 3));
        assert!(outcome.save);
    }

    #[test]
    fn late_confirmation_does_nothing() {
        let t0 = Instant::now();
        let mut t = tracker(14, 3);
        t.handle(Command::Reset(CounterId::Global), t0);
        let outcome = t.handle(Command::ConfirmReset, t0 + Duration::from_secs(5));
        assert_eq!(values(&t), (14, 3));
        assert_eq!(outcome, Outcome::default());
    }

    #[test]
    fn undo_restores_the_most_recent_reset() {
        let t0 = Instant::now();
        let mut t = tracker(14, 3);
        t.handle(Command::Reset(CounterId::Round), t0);
        t.handle(Command::Reset(CounterId::Global), t0);
        t.handle(Command::ConfirmReset, t0);
        assert_eq!(values(&t), (0, 0));

        // Le dernier reset est celui du global : c'est lui qui revient.
        let outcome = t.handle(Command::UndoReset, t0);
        assert_eq!(values(&t), (14, 0));
        assert!(outcome.save);
        // Une seule annulation : la partie ne revient pas.
        assert_eq!(t.handle(Command::UndoReset, t0), Outcome::default());
        assert_eq!(values(&t), (14, 0));
    }

    #[test]
    fn undo_round_reset() {
        let mut t = tracker(14, 3);
        t.handle(Command::Reset(CounterId::Round), Instant::now());
        let outcome = t.handle(Command::UndoReset, Instant::now());
        assert_eq!(values(&t), (14, 3));
        assert_eq!(
            outcome.sound,
            Some((CounterId::Round, SoundSlot::ResetUndone))
        );
        assert!(!outcome.save);
    }

    #[test]
    fn counting_after_reset_forbids_undo() {
        let mut t = tracker(14, 3);
        t.handle(Command::Reset(CounterId::Round), Instant::now());
        t.handle(PLUS_BOTH, Instant::now());
        assert_eq!(
            t.handle(Command::UndoReset, Instant::now()),
            Outcome::default()
        );
        assert_eq!(values(&t), (15, 1));
    }

    #[test]
    fn set_value_saves_only_the_global() {
        let mut t = tracker(14, 3);
        assert!(t.set_value(CounterId::Global, 100));
        assert!(!t.set_value(CounterId::Round, 7));
        assert_eq!(values(&t), (100, 7));
    }

    #[test]
    fn set_value_forgets_the_last_reset() {
        let mut t = tracker(14, 3);
        t.handle(Command::Reset(CounterId::Round), Instant::now());
        t.set_value(CounterId::Round, 9);
        assert_eq!(
            t.handle(Command::UndoReset, Instant::now()),
            Outcome::default()
        );
        assert_eq!(values(&t), (14, 9));
    }

    #[test]
    fn goals_are_independent() {
        let mut t = tracker(14, 3);
        t.set_goal(CounterId::Round, 5);
        assert_eq!(t.counter(CounterId::Round).goal(), 5);
        assert_eq!(t.counter(CounterId::Global).goal(), 0);
    }

    #[test]
    fn targets_only_contains_one_counter() {
        let global = Targets::only(CounterId::Global);
        assert!(global.contains(CounterId::Global));
        assert!(!global.contains(CounterId::Round));
        assert!(Targets::BOTH.contains(CounterId::Round));
    }
}
