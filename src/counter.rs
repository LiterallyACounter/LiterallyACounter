//! Logique pure d'un compteur : la valeur et les opérations dessus.
//!
//! Ce module ne connaît rien de l'affichage ni du clavier. C'est volontaire :
//! la logique est ainsi facile à tester automatiquement (voir les tests en bas du fichier).

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

    /// Ajoute 1.
    /// `&mut self` : on emprunte le compteur en MODIFICATION.
    pub fn increment(&mut self) {
        // `saturating_add` : si on dépasse le maximum possible, on reste bloqué au maximum
        // au lieu de faire planter le programme. Impossible en pratique, mais gratuit.
        self.value = self.value.saturating_add(1);
    }

    /// Retire 1, sans jamais descendre sous 0.
    pub fn decrement(&mut self) {
        // `saturating_sub` : 0 - 1 donne 0 au lieu d'une erreur.
        self.value = self.value.saturating_sub(1);
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
}
