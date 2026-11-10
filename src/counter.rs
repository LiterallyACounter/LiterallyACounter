//! Logique pure d'un compteur : la valeur et les opérations dessus.
//!
//! Ce module ne connaît rien de l'affichage ni du clavier. C'est volontaire :
//! la logique est ainsi facile à tester automatiquement (voir les tests en bas du fichier).

/// Une action applicable à un compteur.
///
/// Un `enum` (énumération) est un type qui vaut UNE des variantes listées.
/// Ici, chaque variante transporte une donnée : le nombre à ajouter ou à retirer.
/// Exemple : `CounterAction::Add(5)` = « ajouter 5 ».
///
/// `derive` génère automatiquement :
/// - `Clone, Copy` : l'action peut être copiée librement (c'est juste un petit nombre) ;
/// - `PartialEq, Eq` : on peut comparer deux actions avec `==` (utile dans les tests).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CounterAction {
    Add(u64),
    Subtract(u64),
}

/// Un compteur.
///
/// La valeur est un `u64` : un entier NON SIGNÉ sur 64 bits (de 0 à environ 18 milliards de milliards).
/// Comme il ne peut pas être négatif, c'est le TYPE lui-même qui garantit la règle « jamais sous 0 ».
/// (Si un jour on veut autoriser le négatif, on passera à `i64`, l'entier signé.)
///
/// `#[derive(...)]` fait générer automatiquement par le compilateur :
/// - `Debug` : la possibilité d'afficher le compteur pour déboguer (`println!("{:?}", c)`) ;
/// - `Default` : un compteur par défaut, avec `value` et `goal` à 0.
#[derive(Debug, Default)]
pub struct Counter {
    // Pas de `pub` : le champ est PRIVÉ, seul ce module peut le modifier.
    // Le reste du programme doit passer par les méthodes ci-dessous, ce qui garantit
    // que les règles (comme « jamais sous 0 ») sont toujours respectées.
    value: u64,
    /// L'objectif. 0 = pas d'objectif.
    goal: u64,
    /// La valeur d'avant le dernier reset, tant qu'on peut encore l'annuler.
    /// `None` : rien à annuler (pas de reset, ou on a recompté depuis).
    value_before_reset: Option<u64>,
}

// `impl Counter` : les MÉTHODES du compteur (des fonctions attachées à la struct).
impl Counter {
    /// Crée un compteur avec une valeur de départ (par ex. la valeur sauvegardée)
    /// et un objectif (0 = pas d'objectif).
    /// `..Self::default()` : les autres champs prennent leur valeur par défaut.
    pub fn new(value: u64, goal: u64) -> Self {
        Self {
            value,
            goal,
            ..Self::default()
        }
    }

    /// Renvoie la valeur actuelle.
    /// `&self` : on emprunte le compteur en LECTURE SEULE.
    pub fn value(&self) -> u64 {
        // En Rust, la dernière expression d'une fonction (sans `;`) est la valeur renvoyée.
        self.value
    }

    /// Renvoie l'objectif (0 = pas d'objectif).
    pub fn goal(&self) -> u64 {
        self.goal
    }

    /// Change l'objectif (0 = pas d'objectif). La valeur, elle, ne bouge pas.
    pub fn set_goal(&mut self, goal: u64) {
        self.goal = goal;
    }

    /// L'objectif est-il atteint ?
    /// `None` s'il n'y a pas d'objectif, `Some(true)` si atteint ou dépassé, `Some(false)` sinon.
    pub fn goal_reached(&self) -> Option<bool> {
        if self.goal == 0 {
            None
        } else {
            Some(self.value >= self.goal)
        }
    }

    /// Applique une action au compteur.
    /// `&mut self` : on emprunte le compteur en MODIFICATION.
    pub fn apply(&mut self, action: CounterAction) {
        // `match` examine la valeur et exécute la branche correspondant à sa variante.
        // Le compilateur VÉRIFIE que toutes les variantes sont traitées : si on ajoute
        // une variante à l'enum plus tard, ce code ne compilera plus tant qu'on ne l'aura pas gérée.
        // `Add(n)` « déballe » la variante et met le nombre transporté dans la variable `n`.
        match action {
            // `saturating_add` : si on dépasse le maximum possible, on reste bloqué au maximum
            // au lieu de faire planter le programme. Impossible en pratique, mais gratuit.
            CounterAction::Add(n) => self.value = self.value.saturating_add(n),
            // `saturating_sub` : 0 - 1 donne 0 au lieu d'une erreur → jamais sous 0.
            CounterAction::Subtract(n) => self.value = self.value.saturating_sub(n),
        }
        // On a recompté : le reset précédent était voulu, on ne peut plus l'annuler.
        self.value_before_reset = None;
    }

    /// Fixe directement la valeur (saisie dans le menu). Ce n'est pas un reset : il n'y a rien
    /// à annuler ensuite, comme quand on recompte après un reset.
    pub fn set_value(&mut self, value: u64) {
        self.value = value;
        self.value_before_reset = None;
    }

    /// Remet le compteur à 0, en retenant l'ancienne valeur pour pouvoir annuler.
    pub fn reset(&mut self) {
        self.value_before_reset = Some(self.value);
        self.value = 0;
    }

    /// Annule le dernier reset. Renvoie `true` si c'était possible, `false` sinon.
    pub fn undo_reset(&mut self) -> bool {
        // `take()` : récupère le contenu de l'`Option` ET la remet à `None` (une seule annulation).
        match self.value_before_reset.take() {
            Some(previous) => {
                self.value = previous;
                true
            }
            None => false,
        }
    }
}

// ---------------------------------------------------------------------------
// TESTS AUTOMATIQUES
//
// `#[cfg(test)]` : ce module n'est compilé QUE pour `cargo test`, jamais dans l'exe final.
// Chaque fonction marquée `#[test]` est un test. `assert_eq!(a, b)` fait échouer le test si a ≠ b.
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    // `super::*` : importe tout ce qui est défini dans le module parent (donc `Counter`).
    use super::*;

    #[test]
    fn starts_at_zero() {
        let counter = Counter::default();
        assert_eq!(counter.value(), 0);
    }

    #[test]
    fn increment_adds_one() {
        let mut counter = Counter::default();
        counter.apply(CounterAction::Add(1));
        counter.apply(CounterAction::Add(1));
        assert_eq!(counter.value(), 2);
    }

    #[test]
    fn decrement_removes_one() {
        let mut counter = Counter::default();
        counter.apply(CounterAction::Add(1));
        counter.apply(CounterAction::Add(1));
        counter.apply(CounterAction::Subtract(1));
        assert_eq!(counter.value(), 1);
    }

    #[test]
    fn decrement_never_goes_below_zero() {
        let mut counter = Counter::default();
        counter.apply(CounterAction::Subtract(1));
        assert_eq!(counter.value(), 0);
    }

    #[test]
    fn set_value_forbids_undo() {
        let mut counter = Counter::new(7, 0);
        counter.reset();
        counter.set_value(42);
        assert_eq!(counter.value(), 42);
        assert!(!counter.undo_reset());
        assert_eq!(counter.value(), 42);
    }

    #[test]
    fn set_goal_keeps_the_value() {
        let mut counter = Counter::new(7, 10);
        counter.set_goal(5);
        assert_eq!(counter.goal(), 5);
        assert_eq!(counter.value(), 7);
        assert_eq!(counter.goal_reached(), Some(true));
    }

    #[test]
    fn no_goal_by_default() {
        let counter = Counter::default();
        assert_eq!(counter.goal(), 0);
        assert_eq!(counter.goal_reached(), None);
    }

    #[test]
    fn goal_reached_when_equal_or_above() {
        let mut counter = Counter::new(0, 2);
        assert_eq!(counter.goal_reached(), Some(false));
        counter.apply(CounterAction::Add(2));
        assert_eq!(counter.goal_reached(), Some(true)); // égal = atteint
        counter.apply(CounterAction::Add(1));
        assert_eq!(counter.goal_reached(), Some(true)); // dépassé = atteint
    }

    #[test]
    fn add_n_and_subtract_n() {
        let mut counter = Counter::default();
        counter.apply(CounterAction::Add(5));
        assert_eq!(counter.value(), 5);
        counter.apply(CounterAction::Subtract(2));
        assert_eq!(counter.value(), 3);
    }

    #[test]
    fn subtract_n_never_goes_below_zero() {
        let mut counter = Counter::default();
        counter.apply(CounterAction::Add(3));
        counter.apply(CounterAction::Subtract(10));
        assert_eq!(counter.value(), 0);
    }

    #[test]
    fn new_starts_at_given_value() {
        let counter = Counter::new(152, 10);
        assert_eq!(counter.value(), 152);
        assert_eq!(counter.goal(), 10);
    }

    #[test]
    fn reset_sets_zero_and_can_be_undone() {
        let mut counter = Counter::new(152, 0);
        counter.reset();
        assert_eq!(counter.value(), 0);
        assert!(counter.undo_reset());
        assert_eq!(counter.value(), 152);
    }

    #[test]
    fn undo_works_only_once() {
        let mut counter = Counter::new(152, 0);
        counter.reset();
        assert!(counter.undo_reset());
        assert!(!counter.undo_reset()); // plus rien à annuler
        assert_eq!(counter.value(), 152);
    }

    #[test]
    fn nothing_to_undo_without_reset() {
        let mut counter = Counter::new(152, 0);
        assert!(!counter.undo_reset());
        assert_eq!(counter.value(), 152);
    }

    #[test]
    fn counting_after_reset_forbids_undo() {
        // Après un reset, on recompte : le reset était voulu, on ne peut plus l'annuler.
        let mut counter = Counter::new(152, 0);
        counter.reset();
        counter.apply(CounterAction::Add(1));
        assert!(!counter.undo_reset());
        assert_eq!(counter.value(), 1);
    }

    #[test]
    fn undo_restores_the_last_reset_only() {
        // 152 → reset → (annulation impossible après recompte) … on vérifie qu'un 2e reset
        // retient bien la valeur de juste avant LUI.
        let mut counter = Counter::new(152, 0);
        counter.reset();
        counter.apply(CounterAction::Add(7));
        counter.reset();
        assert!(counter.undo_reset());
        assert_eq!(counter.value(), 7);
    }
}
