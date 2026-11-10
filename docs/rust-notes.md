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

---

## Étape 4 : format, objectif, couleurs, taille dynamique

### Chaînes de caractères

- **`String`** (texte possédé, modifiable) ou **`&str`** (texte emprunté, en lecture). Une fonction qui *lit* un texte prend un `&str` ; une fonction qui en *fabrique* un renvoie un `String`.
- `.into()` convertit `"texte"` (`&str`) en `String` quand le type attendu est connu.
- Les chaînes Rust sont en **UTF-8** : un caractère peut occuper plusieurs octets, donc on ne peut pas écrire `texte[3]`. On passe par `.chars()` (ici collecté dans un `Vec<char>` pour pouvoir regarder « le suivant »).
- Méthodes utiles : `split_once(':')`, `trim()`, `to_lowercase()`, `trim_end_matches('0')`, `replace('.', ",")`, `push(c)`, `push_str(s)`.
- `format!("{ratio:.2}")` : 2 décimales. `format!("{{{content}}}")` : `{{` et `}}` donnent une accolade littérale, comme dans notre format.

### `match` avancé

- **Plusieurs motifs** : `"value" | "valeur" => ...`.
- **Gardes** : `']' if in_conditional => ...` (la branche ne s'applique que si la condition est vraie).
- **Tuples** : `match (name.as_str(), spec) { ("ratio", Some("p")) => ... }` pour tester deux valeurs à la fois.
- **`matches!(valeur, Motif)`** : un `bool` qui vaut `true` si la valeur correspond au motif.
- **Variantes « struct »** : `Ratio { percent: bool }`, une variante avec un champ nommé.

### Récursivité et `&mut` partagé

- `parse_sequence` s'appelle **elle-même** pour analyser l'intérieur des crochets.
- `position: &mut usize` : l'appelant et la fonction appelée partagent la même position. Quand la fonction intérieure avance, l'appelant le voit. `*position += 1` : l'étoile *déréférence*, c'est-à-dire qu'on modifie la valeur pointée et non la référence.

### Implémenter un trait soi-même

- `impl Default for DisplayStyle { fn default() -> Self { ... } }` : on écrit la valeur par défaut à la main, quand `#[derive(Default)]` (tout à zéro) ne convient pas.
- `DisplayStyle { background: ..., ..DisplayStyle::default() }` : on change un champ et on garde les autres par défaut.
- `fn clear_color(&self, ...)` dans `impl eframe::App` : un trait peut avoir des méthodes **facultatives** (avec une version par défaut), qu'on remplace si besoin.

### Annotations conditionnelles

- `#[cfg_attr(not(test), expect(dead_code, reason = "..."))]` : applique `expect(dead_code)` **seulement hors tests**. Les tests utilisent déjà ces variantes, donc en mode test elles ne sont pas « inutilisées ».

### egui : dessiner soi-même

- `painter.layout_no_wrap(texte, police, couleur)` met en page un texte (un `Galley`) et donne sa taille **avant** de le dessiner : c'est ce qui permet d'ajuster la fenêtre au texte.
- `painter.galley(position, galley, couleur)` dessine ce texte.
- `ViewportCommand::InnerSize(...)` / `OuterPosition(...)` redimensionnent et déplacent la fenêtre. `ctx.input(|i| i.viewport().outer_rect)` donne sa position actuelle.
- La transparence se demande **à la création** de la fenêtre (`with_transparent(true)`), et la couleur d'effacement (`clear_color`) doit aussi être transparente.

### Transparence sous Windows : ce qu'on a appris

- **Moteur de rendu** : eframe peut dessiner avec `wgpu` (DirectX 12, par défaut) ou `glow` (OpenGL). La transparence `wgpu` est cassée par la capture OBS ; `glow` fonctionne et pèse beaucoup moins lourd (exe de 14,85 → 6,25 Mo). On le choisit via les features : `eframe = { default-features = false, features = ["glow", "default_fonts"] }`.
- **Lire le code des dépendances** : le code source de toutes les crates est dans `~/.cargo/registry/src/`. C'est en lisant winit qu'on a trouvé l'origine de la ligne noire en haut de la fenêtre.
- **`raw-window-handle`** : la crate standard pour obtenir l'identifiant système d'une fenêtre (`HWND` sous Windows). `RawWindowHandle` est un enum avec une variante par système.
- **Sous-classe de fenêtre** (`SetWindowSubclass`) : insère notre fonction devant celle de winit pour intercepter un message Windows (`WM_NCCALCSIZE`) et y répondre autrement. Tous les autres messages sont retransmis (`DefSubclassProc`).
- `&value as *const u32 as *const c_void` : on transforme une référence Rust en **pointeur brut** pour la passer à une fonction C. Un pointeur brut n'a aucune garantie : c'est pour ça que son utilisation demande `unsafe`.
- **Coins arrondis maison** : la fenêtre est *toujours* transparente (`with_transparent(true)`, `clear_color` transparent), et on dessine nous-mêmes le fond avec `Frame::NONE.fill(couleur).corner_radius(CornerRadius::same(8))`. Les 4 coins sont identiques sur toutes les machines, et le même code sert au fond opaque comme au fond transparent.
- **Redimensionner en une fois** : `SetWindowPos` change taille *et* position en un seul appel, avec `SWP_NOCOPYBITS` (ne pas recopier l'ancienne image). C'est plus net que les deux commandes successives d'egui. Windows compte en **pixels physiques** et egui en **points** : on convertit avec `ctx.pixels_per_point()` (1,25 si l'affichage est à 125 %).
- **`Option::inspect(|x| ...)`** : exécute une action sur la valeur si elle existe, puis rend l'`Option` inchangée.
- **`#[cfg(...)]` sur un champ de struct** ou sur une branche de `match` : le champ `hwnd` de `NativeWindow` n'existe que sous Windows.
- **Barre de titre fantôme** : winit garde le style `WS_CAPTION` (barre de titre) pour obtenir l'ombre. Comme on a désactivé le dessin des bordures par Windows 11, l'ancien moteur de Windows redessinait une barre de titre classique (✕, □, _) à chaque redimensionnement. La sous-classe bloque donc aussi `WM_NCPAINT`, `WM_NCACTIVATE` et deux messages internes (`0xAE`, `0xAF`). Le `match` avec garde (`WM_NCCALCSIZE if wparam != 0 => ...`) rend ce tri lisible.
- **Piège `WM_NCACTIVATE`** : l'ignorer complètement (`return 1`) empêchait le glisser, car Windows y met à jour son état interne d'activation. La bonne façon : le transmettre avec `lparam = -1`, la valeur prévue par Windows pour « traite-le sans redessiner la bordure ».
- **Tester un glisser** : simuler la souris avec `SetCursorPos` ne génère pas de vrais mouvements. Il faut `mouse_event(MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE, ...)`. Toujours valider un outil de test sur une version qui marche avant de conclure.

---

## Étape 5 : compteur global persistant, reset et annulation

### serde : sauvegarder une struct

- `#[derive(Serialize, Deserialize)]` : serde génère tout seul la conversion struct ↔ données. `serde_json::to_string_pretty(&state)` donne le JSON, `serde_json::from_str::<SavedState>(&texte)` fait l'inverse.
- `#[serde(default)]` : un champ absent du fichier prend sa valeur par défaut. Les anciennes sauvegardes restent lisibles quand on ajoute des champs.
- **Écriture atomique** : `fs::write` dans un fichier `.tmp`, puis `fs::rename` à la place de l'ancien. Le renommage est instantané, donc une coupure pendant l'écriture ne peut pas abîmer la sauvegarde.

### Fichiers et chemins

- `PathBuf` (chemin possédé) et `&Path` (emprunté) : l'équivalent de `String` / `&str` pour les chemins. `.join("x")` ajoute un élément, `.parent()` remonte d'un cran, `.with_extension("json.tmp")` change l'extension.
- `dirs::config_dir()` : le dossier de configuration standard de chaque OS.
- `io::Result<()>` : « rien en cas de succès, une erreur d'entrée/sortie sinon ». Avec `?`, chaque étape qui échoue fait sortir de la fonction avec son erreur.
- `Err(e) if e.kind() == io::ErrorKind::NotFound => ...` : un `match` avec garde pour traiter « fichier absent » différemment des autres erreurs.
- `.as_deref()` : transforme un `Option<PathBuf>` en `Option<&Path>` (on emprunte le contenu sans le prendre).

### Options et temps

- `Option::take()` : récupère le contenu **et** remet `None` à la place. Parfait pour une action utilisable une seule fois (l'annulation du reset).
- `Instant` (un moment) et `Duration` (une durée). `now.duration_since(t0)`, `checked_sub` (renvoie `None` au lieu d'une durée négative), `.filter(...)` sur une `Option`.
- **Passer l'heure en paramètre** (`now: Instant`) plutôt que la lire à l'intérieur : les tests simulent le temps (`t0 + Duration::from_secs(4)`) sans attendre.
- `ctx.request_repaint_after(durée)` : demande à egui de redessiner plus tard, ici pour effacer « Reset ? » à l'expiration.

### Enums imbriqués et textes

- `Command::Count(CounterAction::Add(1))` : une variante peut envelopper un autre enum, ce qui évite de dupliquer `Add` / `Subtract`.
- `&'static str` : un texte écrit dans le programme, valable pendant toute l'exécution (nos textes de traduction).
- `&[(&str, &str)]` : une **tranche** de tuples, par ex. `&[("key", "Num Entrée")]`, pour remplir les emplacements d'un message.
- `code @ 0x60..=0x69 => ...` : donne un nom à la valeur qui correspond à un motif d'intervalle.
- `iter().chain(std::iter::once(x))` : ajoute un élément au bout d'un itérateur ; `.collect::<Vec<_>>().join(" + ")` assemble les noms.
- **Fins de ligne** : Git sous Windows convertit les fichiers en CRLF. Un remplacement de texte qui cherche des `\n` seuls échoue alors en silence. À garder en tête pour les scripts.
- **Retirer un style de fenêtre** : `GetWindowLongW(hwnd, GWL_STYLE)` lit le style (un ensemble de bits), `SetWindowLongW` l'écrit. `(style & !(WS_CAPTION | WS_THICKFRAME)) | WS_POPUP` : `& !X` éteint les bits de X, `| Y` allume ceux de Y. Une fenêtre sans `WS_CAPTION` ne peut plus jamais afficher de barre de titre (fini le flash au démarrage).
- **`WM_STYLECHANGING`** : Windows prévient *avant* tout changement de style, avec un pointeur vers le futur style (`STYLESTRUCT`), qu'on peut modifier. On s'en sert pour empêcher winit de remettre la barre de titre. `&mut *(lparam as *mut STYLESTRUCT)` : on transforme le pointeur brut en référence modifiable (dans un bloc `unsafe`).

---

## Étape 6 : raccourcis souris

### Itérateurs : quelques outils de plus

- `flat_map(|s| ...)` : pour chaque élément, produit **plusieurs** éléments, mis bout à bout en une seule suite. Ici, chaque raccourci donne toutes ses touches (`held` + `trigger`).
- `.filter(|key| key.is_mouse())` puis `.collect()` : ne garde que les boutons de souris.
- `sort_by_key(...)` puis `dedup()` : `dedup` ne retire que les doublons **consécutifs**, d'où le tri juste avant.
- `partition(Shortcut::is_allowed)` : sépare une liste en deux (`(autorisés, refusés)`) selon une condition. On peut passer directement une méthode (`Shortcut::is_allowed`) au lieu d'écrire une closure.
- `.any(|key| key.is_modifier())` : vrai si au moins un élément vérifie la condition.
- `matches!(valeur, A | B | C)` : vrai si la valeur correspond à l'un des motifs. Plus court qu'un `match` qui renvoie `true` / `false`.

### `match` sur un tuple

```rust
match (was_down, now_down) {
    (false, true) => { /* vient d'être enfoncé */ }
    (true, false) => { /* vient d'être relâché */ }
    _ => {} // pas de changement
}
```

On regroupe deux valeurs dans un tuple pour traiter les 4 combinaisons d'un coup.

### Interroger plutôt qu'écouter

- `SetTimer(null, 0, 15, None)` : un minuteur Windows rattaché au thread. Il envoie un message `WM_TIMER` toutes les ~15 ms, reçu dans la même boucle `GetMessageW` que le hook clavier. Un seul thread gère donc clavier et souris, sans verrou.
- À chaque tic, `GetAsyncKeyState` donne l'état de chaque bouton surveillé. `MousePoller` compare avec le tic précédent et produit des `Pressed` / `Released`, traités comme des touches du clavier.
- La fonction de lecture d'état est **passée en paramètre** (`is_down: impl Fn(Key) -> bool`) : les tests simulent ainsi la souris, comme on simulait l'heure à l'étape 5.

### Mesurer avant de choisir

- Banc de mesure hors dépôt (`RustSandbox/mouse_bench`) : interrogation contre Raw Input, avec une souris simulée de 125 à 6 400 Hz (`SendInput`) et la vraie souris. Résultats dans la spec (F5).
- `GetProcessTimes` : le temps CPU consommé par le programme (noyau + utilisateur), en tranches de 100 ns. Windows ne le met à jour que toutes les ~15,6 ms : les petites mesures sont imprécises, il faut mesurer sur plusieurs secondes.
- **Surprise de la mesure** : le coût venait de winit, pas de notre code. On crée donc nous-mêmes la boucle d'événements pour la régler :

```rust
let event_loop = EventLoop::<eframe::UserEvent>::with_user_event().build()?;
event_loop.listen_device_events(DeviceEvents::Never);
let mut app = eframe::create_native("...", options, Box::new(...), &event_loop);
event_loop.run_app(&mut app)?;
```

- `EventLoop::<eframe::UserEvent>` : on précise le **paramètre de type** (le type des messages internes d'eframe) avec la syntaxe « turbofish » `::<...>`.
- `?` dans `main` : `main` renvoie un `eframe::Result`, donc une erreur de winit est convertie automatiquement (`From`) et fait quitter le programme proprement.

## Étape 7 : un global et une partie, plusieurs fenêtres

### Un enum comme identifiant

```rust
pub enum CounterId { Global, Round }
impl CounterId {
    pub const ALL: [CounterId; 2] = [CounterId::Global, CounterId::Round];
}
```

- Un enum **sans donnée** désigne un compteur : impossible d'en citer un qui n'existe pas (un numéro pourrait valoir 7).
- `CounterId::ALL` : une constante pour parcourir tous les compteurs avec `for id in CounterId::ALL`.
- `[T; 2]` : un tableau de taille **fixe**, connue à la compilation (contrairement à `Vec`).

### Des variantes qui portent plusieurs données

```rust
pub enum Command {
    Count(Targets, CounterAction), // quels compteurs + quelle action
    Reset(CounterId),
    ConfirmReset,
    UndoReset,
}
```

- `match` peut viser une variante **et** sa donnée : `Command::Reset(CounterId::Round) => ...`.
- Une **garde** affine un cas : `CounterId::Round if round_copies_global => global`.

### Renvoyer ce qu'il faut faire plutôt que le faire

`Tracker::handle` ne joue pas de son et n'écrit pas de fichier : il renvoie un `Outcome { sound, save, reset_prompt_started }`, et `main.rs` l'exécute. La logique reste pure, donc testable sans haut-parleur ni disque.

- `#[derive(Default)]` + `..Outcome::default()` : on n'écrit que les champs utiles, les autres valent `None` / `false`.
- `Option::take()` : récupère la valeur et laisse `None` à sa place (une annulation ne sert qu'une fois).
- `is_some_and(|id| ...)` : vrai si c'est `Some` **et** que la condition est vraie.
- `if let Some(a) = x && let Some(b) = y { ... }` : deux motifs dans un même `if` (édition 2024).

### Emprunter un élément précis

```rust
fn counter_mut(&mut self, id: CounterId) -> &mut Counter {
    match id {
        CounterId::Global => &mut self.global,
        CounterId::Round => &mut self.round,
    }
}
```

On prête **un champ** de `self` en modification. Tant que cet emprunt vit, on ne peut pas toucher à `self` ailleurs : c'est pourquoi `counter_ui` **clone** le style avant de dessiner, puis note le clic et ne l'exécute (`self.handle(...)`) qu'après le dessin.

### Les durées de vie (`'a`)

```rust
fn effective_style<'a>(id: CounterId, global: &'a DisplayStyle, round: &'a DisplayStyle,
                       copies: bool) -> &'a DisplayStyle
```

La fonction renvoie un **emprunt**. `'a` dit au compilateur à quoi il est emprunté : à l'un des deux styles reçus. Le résultat ne peut donc pas vivre plus longtemps qu'eux. Avec un seul paramètre emprunté, Rust devine seul ; avec deux, il faut le préciser.

### Sauvegarder un type étranger avec serde

- `#[serde(with = "color_hex")]` : quand un type (ici `Color32` d'egui) ne sait pas se sauvegarder, on fournit un module avec deux fonctions, `serialize` et `deserialize`. Les couleurs s'écrivent `"#RRGGBBAA"`.
- `#[serde(rename_all = "lowercase")]` : les variantes d'un enum s'écrivent `"left"`, `"center"`, `"right"`.
- `#[serde(default)]` sur une struct : chaque champ absent prend sa valeur par défaut. L'ancien fichier `{"global_value": 14}` se relit donc sans perte.
- `impl Default` écrit à la main quand le défaut automatique (`false`, 0) ne convient pas (fenêtres affichées par défaut).

### Plusieurs fenêtres avec egui

- `ctx.show_viewport_immediate(id, builder, |ui, _| { ... })` : une fenêtre supplémentaire, dessinée **tout de suite** par la closure. Elle peut emprunter `self` : pas besoin de partager les données entre threads (`Arc<Mutex>`), contrairement aux viewports « deferred ».
- Une fenêtre qu'on cesse d'afficher est fermée par egui ; la ré-afficher la recrée (à une nouvelle position).
- `ctx.input_for(autre_id, |i| ...)` : lire l'état (position, taille) d'une **autre** fenêtre.
- **Fenêtre hôte invisible** : eframe ne redessine une fenêtre racine réduite qu'une fois toutes les 100 ms au plus. La racine est donc un pixel transparent jamais réduit, et toutes les vraies fenêtres en dépendent : réduire la fenêtre principale ne retarde ni l'affichage ni le son.

### Itérateurs, suite

- `enumerate()` : des paires (numéro, élément).
- `std::iter::once(x).chain(autres)` : une suite qui commence par `x`, puis continue avec `autres`.
- `max_by(|a, b| a.total_cmp(b))` : le plus grand élément ; `total_cmp` compare deux `f32` (qui n'ont pas d'ordre « simple » à cause de NaN).
- `fold(depart, |acc, x| ...)` : parcourt la liste en faisant évoluer un résultat (ici, un maximum ou un minimum).
- `all(|r| ...)` : vrai si **tous** les éléments vérifient la condition (le pendant de `any`).
- `?` dans une boucle d'une fonction qui renvoie `Option` : à la première valeur `None`, on sort de la fonction.

## Étape 8a-8b : configuration et raccourcis modifiables

### Les contrôles d'egui

- `ui.add(egui::DragValue::new(&mut x).range(8.0..=300.0))` : un nombre qu'on glisse ou qu'on saisit. Le contrôle reçoit un **emprunt modifiable** (`&mut`) et change la valeur directement.
- `ui.checkbox(&mut b, "…")`, `ui.radio_value(&mut anchor, Anchor::Left, "…")`, `ui.selectable_value(&mut tab, ConfigTab::Counters, "…")` : même principe.
- `.changed()` sur la réponse d'un contrôle : vrai si l'utilisateur vient de le modifier. On le cumule avec `changed |= ...`.
- `egui::ComboBox::from_id_salt(...)` : une liste déroulante. `from_id_salt` donne un identifiant unique, car egui doit distinguer les listes affichées dans la même fenêtre.
- `ui.add_enabled_ui(false, |ui| ...)` : tout ce qui est dessiné dedans est grisé et inactif.
- `color_edit_button_srgba(ui, &mut couleur, Alpha::OnlyBlend)` : le sélecteur de couleur intégré, avec l'opacité.
- Pièges rencontrés :
  - dans une cellule de grille, un texte ne passe pas à la ligne ;
  - la police intégrée n'a pas tous les symboles (➕, ⚠…) ;
  - egui zoome l'interface avec Ctrl + « - » / « + », il a fallu le couper (`options.zoom_with_keyboard = false`).

### Sauvegarder sans écrire 60 fois par seconde

Glisser un `DragValue` change la valeur à chaque image. On note qu'il y a des modifications (`unsaved_changes = true`) et on sauvegarde quand le bouton de la souris est relâché (`!ui.input(|i| i.pointer.any_down())`), ainsi qu'à la fermeture (`on_exit`).

### Conflit de noms

`eframe::App` a déjà une méthode `save`. Notre `self.save()` appelait la sienne, d'où le renommage en `save_state`. Quand un trait et un type ont une méthode du même nom, le compilateur peut choisir celle du trait.

### Renvoyer une demande plutôt qu'agir

Comme `Tracker::handle`, l'éditeur de raccourcis ne modifie rien : il renvoie un `Option<ShortcutRequest>` (« démarrer une capture », « supprimer le n° 3 »…), et `main.rs` l'exécute. Cela évite d'avoir à la fois l'interface et l'appli empruntées en modification.

### `Result` pour un refus

```rust
pub fn apply_capture(...) -> Result<(), CaptureRefused>
```

`Ok(())` : c'est fait ; `Err(CaptureRefused::MainClickNeedsModifier)` : refusé, avec la raison. L'appelant fait un `match` pour afficher le message.

### Petites méthodes utiles

- `Option::filter(|&i| i < len)` : garde la valeur seulement si la condition est vraie.
- `(id != 0).then_some(id)` : `Some(id)` si la condition est vraie, `None` sinon.
- `slice.split_last()` : le dernier élément et le reste (`Option<(&T, &[T])>`).
- `receiver.try_iter()` : tous les messages déjà arrivés dans un channel, sans attendre.
- `#[serde(transparent)]` : une struct à un seul champ (`Key(u16)`) s'écrit comme ce champ (un simple nombre).
- `#[derive(Hash)]` : nécessaire pour servir d'identifiant à egui (`ui.indent(kind, ...)`) ou de clé dans un `HashSet`.

### Piloter un thread qui attend des messages Windows

Le thread du hook dort dans `GetMessageW`. Pour lui envoyer une demande :
1. on la met dans un channel (`Sender<Control>`) ;
2. on le réveille avec `PostThreadMessageW(thread_id, WM_APP, 0, 0)` ;
3. sa boucle reçoit `WM_APP` et lit le channel.

Il faut créer sa file de messages avant de publier son identifiant (`PeekMessageW`), sinon un message envoyé trop tôt se perd.

### Capturer une combinaison

Les touches sont notées dans l'ordre d'appui. Au premier relâchement, la dernière touche enfoncée devient la touche de déclenchement, les autres sont les touches à tenir. Logique pure dans `capture.rs`, testée sans clavier.

## Étape 8c-8e : menu des compteurs, molette, raccourcis dans le menu

### Une petite fenêtre pour le menu

egui ne peut pas dessiner hors de la fenêtre en cours : un menu plus grand qu'un compteur serait coupé. Le menu est donc un viewport « immediate » à part, ouvert sous le compteur, qui se ferme quand il perd le focus (`i.viewport().focused == Some(false)`, après l'avoir eu).

- `Option<CounterMenu>` sert de petit automate : `None` = fermé, `Some(menu)` = ouvert avec son état (valeur saisie, position, taille).
- `ui.rect_contains_pointer(rect)` : la souris est-elle au-dessus ? Le bouton « ••• » n'est dessiné qu'à ce moment-là.
- Un widget ajouté après un autre est « au-dessus » et reçoit le clic à sa place : cliquer « ••• » ne compte pas +1.
- `MenuAction::OpenSettings | MenuAction::EditShortcuts => ...` : un seul bras de `match` pour deux variantes.

### Raw Input, côté Rust

- Une **union** (`RAWINPUT_0`) : plusieurs types rangés au même endroit, selon l'en-tête. Lire un de ses champs demande `unsafe`, car le compilateur ne sait pas lequel est valide ; c'est l'en-tête (`dwType == RIM_TYPEMOUSE`) qui le garantit.
- `buttons.usButtonData as i16` : la rotation de la molette est un nombre signé rangé dans un champ non signé. `as` réinterprète les bits.
- `flags & 0x0400 != 0` : tester un bit avec le « et » bit à bit.
- Fenêtre « message-only » (`HWND_MESSAGE`) : une fenêtre jamais affichée, qui sert juste à recevoir des messages (`WM_INPUT`).

### Logique pure, encore

- `WheelAccumulator` : les fractions de cran s'additionnent (`while total >= 120 { ... }`) ; `signum()` donne le signe pour repérer un changement de sens.
- `RateMonitor` : la fréquence des événements par tranches de 2 s, testée en simulant le temps (`Instant + Duration`).
- `shortcuts_for` : `filter_map` garde les `Some(...)` et écarte les `None` ; `(!liste.is_empty()).then_some(...)` fabrique l'`Option`.
- `.map(|(_, s)| s.len()).sum()` : additionner les longueurs (le type du total est déduit de l'usage).

### Le BOM

Un fichier « UTF-8 avec BOM » commence par le caractère invisible `'\u{feff}'`. `content.strip_prefix('\u{feff}').unwrap_or(&content)` le retire s'il est là. Leçon : un fichier modifiable à la main doit tolérer les habitudes des éditeurs.

### Tester sans gêner le jeu

Les tests manuels pilotés par script vérifient, juste avant chaque clic ou touche simulés, que la fenêtre au premier plan appartient bien au compteur. Sinon, ils s'arrêtent : un clic simulé pourrait partir dans le jeu.
