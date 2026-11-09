# Journal d'apprentissage Rust

Aide-mémoire des concepts Rust vus au fil du projet, étape par étape.

---

## Étape 0 : installation et premiers pas

### Les outils

| Outil | Rôle |
|---|---|
| `rustup` | Installe et met à jour Rust (`rustup update`) |
| `rustc` | Le compilateur. On l'appelle rarement directement : `cargo` s'en charge |
| `cargo` | L'outil central : crée les projets, télécharge les dépendances, compile, teste, lance |
| `cargo fmt` | Formate le code automatiquement |
| `cargo clippy` | Linter : signale les erreurs probables et le code non idiomatique |

### Commandes cargo essentielles

| Commande | Effet |
|---|---|
| `cargo new mon_projet` | Crée un projet (`Cargo.toml` + `src/main.rs`) |
| `cargo run` | Compile en mode debug et lance le programme |
| `cargo build --release` | Compile en mode optimisé → `target/release/mon_projet.exe` |
| `cargo test` | Lance les tests |
| `cargo check` | Vérifie que le code compile, sans produire d'exe (très rapide) |

**Debug ou release ?** Le mode debug compile vite mais produit un programme lent, avec des infos pour déboguer. Le mode release compile plus lentement mais produit un programme optimisé : c'est lui qu'on distribue.

### Structure d'un projet

```
mon_projet/
├── Cargo.toml   ← nom, version, édition de Rust, dépendances (« crates »)
├── src/
│   └── main.rs  ← code ; `fn main()` est le point d'entrée
└── target/      ← résultat de la compilation (jamais versionné)
```

### Premiers concepts

- **`fn main()`** : la fonction exécutée au lancement.
- **Macros (`println!`)** : le `!` signale une macro, du code généré à la compilation. `println!` vérifie dès la compilation que le texte et les valeurs correspondent.
- **Variables immuables par défaut** : `let x = 5;` ne peut plus changer. Il faut écrire `let mut x = 5;` pour pouvoir la modifier. Rust force à dire explicitement ce qui change, ce qui évite beaucoup de bugs.
- **Inférence de type** : Rust devine le type (`let goal = 10;` → entier). On peut le préciser : `let counter: i64 = 0;` (`i64` = entier signé 64 bits, peut être négatif).
- **Boucle `for`** : `for i in 1..=5` parcourt 1 à 5 inclus (`1..5` exclurait le 5).
- **Formatage** : `println!("{} / {}", a, b)` ou directement `println!("{a} / {b}")`.
