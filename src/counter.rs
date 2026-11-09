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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
/// - `Default` : un compteur par défaut, avec `value` à 0.
#[derive(Debug, Default)]
pub struct Counter {
    // Pas de `pub` : le champ est PRIVÉ, seul ce module peut le modifier.
    // Le reste du programme doit passer par les méthodes ci-dessous, ce qui garantit
    // que les règles (comme « jamais sous 0 ») sont toujours respectées.
    value: u64,
}

// `impl Counter` : les MÉTHODES du compteur (des fonctions attachées à la struct).
impl Counter {
    /// Renvoie la valeur actuelle.
    /// `&self` : on emprunte le compteur en LECTURE SEULE.
    pub fn value(&self) -> u64 {
        // En Rust, la dernière expression d'une fonction (sans `;`) est la valeur renvoyée.
        self.value
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
    }

    /// Raccourci pour `apply(Add(1))`.
    pub fn increment(&mut self) {
        self.apply(CounterAction::Add(1));
    }

    /// Raccourci pour `apply(Subtract(1))`.
    pub fn decrement(&mut self) {
        self.apply(CounterAction::Subtract(1));
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
        counter.increment();
        counter.increment();
        assert_eq!(counter.value(), 2);
    }

    #[test]
    fn decrement_removes_one() {
        let mut counter = Counter::default();
        counter.increment();
        counter.increment();
        counter.decrement();
        assert_eq!(counter.value(), 1);
    }

    #[test]
    fn decrement_never_goes_below_zero() {
        let mut counter = Counter::default();
        counter.decrement();
        assert_eq!(counter.value(), 0);
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
}
