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

### Hello World egui : première fenêtre

Projet d'essai : `RustSandbox/hello_egui/` (hors dépôt).

- **Ajouter une dépendance** : `cargo add eframe`. Cargo ajoute la ligne `eframe = "0.37.0"` dans `Cargo.toml`, télécharge la crate et toutes ses sous-dépendances (plus de 400 ici !), puis fige les versions exactes dans `Cargo.lock`.
- **`use eframe::egui;`** : importe un élément pour l'utiliser sans écrire son chemin complet.
- **`struct`** : regroupe des données sous un nom. `struct CounterApp { value: i64 }` est l'état de l'appli.
- **`#[derive(Default)]`** : un *attribut* qui fait générer du code par le compilateur, ici une valeur par défaut (`value` = 0).
- **Trait** : un contrat. `impl eframe::App for CounterApp` veut dire « CounterApp respecte le contrat d'une appli eframe », ce qui oblige à fournir la méthode `ui`, appelée à chaque image.
- **Emprunt (`&` / `&mut`)** : on *prête* une valeur au lieu de la donner. `&mut self` = la méthode peut modifier l'appli. C'est le cœur du système de propriété de Rust (*ownership*), qu'on approfondira.
- **Closure (`|ui| { ... }`)** : une fonction anonyme passée en paramètre. egui l'appelle pour remplir un panneau.
- **Interface « immédiate »** : egui redessine toute l'interface à chaque image. `if ui.button("+1").clicked() { ... }` dessine le bouton ET teste le clic dans la même ligne. Pas de callbacks à brancher.
- **`..Default::default()`** : « pour tous les autres champs de la struct, prends les valeurs par défaut ».
- **`Result`** : un type qui vaut soit `Ok(valeur)`, soit `Err(erreur)`. Rust n'a pas d'exceptions : les erreurs sont des valeurs qu'on doit traiter.
- **`Box::new(...)`** : place une valeur dans le tas (mémoire dynamique).

**Taille** : l'exe release fait environ 14 Mo sans aucune optimisation de taille. On la réduira à l'étape 10 (`strip`, LTO, `opt-level = "z"`, désactivation des features inutiles).

**Leçon** : les bibliothèques Rust évoluent vite. En 0.37, egui a remplacé `fn update(&mut self, ctx, frame)` par `fn ui(&mut self, ui, frame)`. Le compilateur l'a signalé tout de suite, et la vraie référence, c'est le code source ou la doc de la version utilisée (`cargo doc --open`).
