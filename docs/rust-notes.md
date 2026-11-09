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

---

## Étape 1 : la fenêtre compteur

### Modules

- `mod counter;` dans `main.rs` déclare un module dont le code est dans `src/counter.rs`. Un module = un fichier = un sujet.
- **Visibilité** : tout est privé par défaut. `pub` rend une struct, un champ ou une méthode accessible depuis les autres modules. Le champ `value` de `Counter` est privé : on ne peut le modifier que via `increment()` / `decrement()`, qui font respecter la règle « jamais sous 0 ».
- `use counter::Counter;` importe la struct pour l'utiliser dans `main.rs`.

### Types et méthodes

- **`u64` ou `i64`** : `u` = *unsigned* (non signé, ≥ 0), `i` = *integer* signé. Choisir `u64` fait garantir « jamais négatif » par le type lui-même.
- **`saturating_sub` / `saturating_add`** : arithmétique qui « bute » sur les limites au lieu de planter (0 - 1 = 0). En Rust, un dépassement d'entier fait planter le programme en debug : il faut choisir explicitement le comportement voulu.
- **`impl Counter { ... }`** : on y définit les méthodes de la struct.
- **`&self` / `&mut self`** : la méthode emprunte l'objet en lecture seule ou en modification.
- **Valeur de retour implicite** : la dernière expression d'une fonction, *sans* `;`, est renvoyée (`self.value`).
- **Commentaires de documentation** : `///` documente l'élément qui suit, `//!` documente le module entier. `cargo doc --open` génère une doc HTML à partir de ces commentaires.

### Tests

- Les tests vivent dans le même fichier, dans un module `#[cfg(test)] mod tests { ... }`, compilé uniquement par `cargo test`.
- Chaque fonction `#[test]` est un test. `assert_eq!(obtenu, attendu)` le fait échouer si les deux diffèrent.
- `use super::*;` importe tout le contenu du module parent dans le module de tests.

### Attributs de compilation

- `#![...]` (avec `!`) s'applique au fichier entier ; `#[...]` s'applique à l'élément qui suit.
- `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]` : en release seulement, l'exe est une appli graphique Windows, sans console.

### egui

- `ui.interact(rect, id, Sense::click_and_drag())` rend une zone sensible aux clics et au glisser. On récupère une `Response` : `.clicked()`, `.secondary_clicked()` (clic droit), `.drag_started()`.
- `ViewportCommand::StartDrag` demande à Windows de déplacer la fenêtre, comme si on tenait la barre de titre.
- `ViewportBuilder` : `.with_decorations(false)` (sans bordure), `.with_always_on_top()`, `.with_resizable(false)`.
- **Les avertissements de dépréciation** (*deprecated*) signalent une API renommée : `Ui::id` → `Ui::scope_id`, `Id::new` → `Id::unique` / `IdSalt::new`. Le compilateur indique quoi utiliser à la place.
- **Piège rencontré : ordre et superposition.** Pour egui, un élément ajouté *après* un autre est *au-dessus* et reçoit les clics en priorité. Or les labels sont sélectionnables par défaut (`selectable_labels`), donc ils captent les clics. Notre label, qui occupait toute la fenêtre, interceptait tout. Correction : `Label::new(...).selectable(false)`, et on ajoute la zone cliquable *après* le label.

---

## Étape 2 : le hook clavier global

### Enums et `match`

- **`enum CounterAction { Add(u64), Subtract(u64) }`** : une valeur qui est *une* des variantes, chaque variante pouvant transporter des données.
- **`match`** traite chaque variante. Le compilateur vérifie qu'**aucun cas n'est oublié** : si on ajoute une variante, tous les `match` concernés refusent de compiler jusqu'à ce qu'on la gère.
- `_ => {}` : le cas « tout le reste ».

### `Option` : pas de `null` en Rust

- `Option<T>` vaut `Some(valeur)` ou `None`. L'absence de valeur est explicite, et on est obligé de traiter le cas `None`.
- `let Some(x) = ... else { return ...; };` : extrait la valeur, ou sort de la fonction si c'est `None`.
- `if let Some(x) = ... { ... }` : exécute le bloc seulement s'il y a une valeur.
- `while let Ok(x) = ... { ... }` : boucle tant que le motif correspond.

### Collections et itérateurs

- `Vec<T>` : tableau de taille variable (`vec![a, b, c]` pour en créer un).
- `HashSet<T>` : ensemble sans doublons, recherche rapide. `insert` renvoie `false` si l'élément y était déjà (on s'en sert pour détecter la répétition d'une touche).
- **Chaînes d'itérateurs** : `.iter().filter(...).map(...).collect()` parcourt, filtre, transforme et rassemble, sans boucle écrite à la main. `.all(...)`, `.max()`, `.retain(...)` sont du même genre.

### Newtype et constantes associées

- `struct Key(pub u16);` enveloppe un nombre dans un type dédié : impossible de confondre une touche avec un autre nombre.
- `impl Key { pub const NUMPAD_ADD: Key = Key(0x6B); }` : constantes rangées « dans » le type → `Key::NUMPAD_ADD`.

### Fonctions en paramètre et closures

- `impl Fn(Key) -> bool` : « n'importe quelle fonction qui prend une `Key` et renvoie un `bool` ». Pratique pour les tests : on passe `|_| false` pour simuler le système.
- `Box<dyn Fn(CounterAction)>` : une fonction quelconque de cette forme, rangée dans le tas (quand on doit la *stocker*).
- `move |...| { ... }` : la closure **prend possession** des variables qu'elle utilise (nécessaire pour les emporter dans un autre thread).

### Threads et channels

- `std::thread::spawn` (ou `Builder::new().spawn`) lance un fil d'exécution parallèle.
- `mpsc::channel()` crée un tuyau : `sender.send(x)` d'un côté, `receiver.try_recv()` de l'autre (`try_` = sans attendre).
- **`Send` et `'static`** : le compilateur vérifie qu'une donnée envoyée à un autre thread peut l'être sans risque. C'est ce qui rend les bugs de concurrence (*data races*) impossibles en Rust « sûr ».
- `thread_local!` : une variable propre à chaque thread. On s'en sert parce que Windows appelle notre `hook_proc` sans nous laisser lui passer de données.
- `RefCell` : permet de modifier une valeur partagée, avec une vérification *à l'exécution* au lieu de la compilation.

### `unsafe` et FFI (appel de code C)

- L'API Windows est en C. Rust ne peut pas vérifier ce code, donc chaque appel est dans un bloc `unsafe { ... }`, où le programmeur garantit lui-même la correction.
- Bonne pratique : des blocs `unsafe` **minuscules**, chacun avec un commentaire `SÉCURITÉ :` qui explique pourquoi c'est correct, et tout le reste du code en Rust sûr.
- `unsafe extern "system" fn` : une fonction que Windows peut appeler (convention d'appel « system »).

### Cargo : dépendances par plateforme

- `[target.'cfg(windows)'.dependencies]` : dépendances téléchargées et compilées seulement pour Windows.
- `#[cfg(windows)]` / `#[cfg(not(windows))]` dans le code : n'inclut un élément que sur certaines plateformes.
- Les **features** d'une crate (`features = [...]`) n'activent que les parties utiles : moins de code compilé, exe plus léger.

---

## Étape 3 : le moteur audio

### Drop et RAII : le nettoyage automatique

- Quand une valeur n'est plus utilisée (fin de bloc, remplacement…), Rust la **détruit** automatiquement en appelant sa méthode `drop`. C'est le principe RAII : une ressource est libérée au moment exact où son propriétaire disparaît.
- On s'en sert pour couper un son : `self.players.insert(slot, nouveau_player)` remplace l'ancien `Player`, qui est détruit, et son `drop` arrête le son. Aucun code de nettoyage à écrire.
- À l'inverse, `MixerDeviceSink` doit rester vivant tant qu'on veut du son : on le garde donc dans un champ de la struct, même s'il semble peu utilisé.

### `Option`, `Result` et l'opérateur `?`

- `Result<T, E>` = succès `Ok(T)` ou erreur `Err(E)`.
- `.ok()` convertit un `Result` en `Option` (on oublie le détail de l'erreur).
- `?` : « si c'est une erreur / `None`, sors tout de suite de la fonction en la renvoyant ». Ça évite des cascades de `match`.
- `.unwrap_or_else(|| ...)` : prend la valeur, ou calcule une valeur de remplacement si elle est absente (notre bip de secours).
- `.is_some_and(|x| ...)` : « il y a une valeur ET elle vérifie la condition ».

### Partage sans copie : `Arc`

- `SamplesBuffer` garde ses échantillons dans un `Arc` (*Atomically Reference Counted*) : plusieurs propriétaires partagent les mêmes données, libérées quand le dernier disparaît. `clone()` ne copie donc pas les données, il incrémente juste un compteur. Jouer un son ne coûte presque rien.

### Divers

- `NonZero<u32>` : un entier dont le type garantit qu'il n'est pas nul.
- `.expect("raison")` : récupère la valeur, ou fait planter avec ce message si c'est une erreur. À réserver aux cas *impossibles*, avec une raison claire.
- `#[expect(dead_code, reason = "...")]` : on déclare qu'un élément n'est pas encore utilisé. Le compilateur se tait, puis **avertit dès qu'il devient utilisé**, pour qu'on retire l'annotation. Mieux que `#[allow]`, qui reste silencieux pour toujours.
- Itérateur → `Vec` : un décodeur audio est un itérateur d'échantillons. `decoder.collect()` les décode tous d'un coup.
- `(0..n).map(|i| ...).collect()` : génère une liste calculée (ici, les échantillons d'une onde sinusoïdale).
- `#[cfg(windows)]` sur un test : ce test ne tourne que sous Windows.
- Les noms d'import d'une crate peuvent bouger d'une version à l'autre : `rodio::SamplesBuffer` n'existe plus en 0.23, c'est `rodio::buffer::SamplesBuffer`. Le compilateur l'a signalé tout de suite (`unresolved import`).
