# Literally A Counter

:gb: [English](README.md) · :fr: **Français**

Un petit compteur pour joueurs et streamers. Les compteurs s'affichent dans de petites fenêtres sans bordure, à l'écran ou en overlay OBS, et on compte avec des raccourcis clavier ou souris **pendant que le jeu garde le focus**.

Exemple : compter les coffres ouverts pendant une partie, et un total sur plusieurs parties, voire plusieurs jours.

## Fonctionnalités

- **Deux compteurs** : un **global** (le total, sauvegardé et repris au démarrage) et un compteur de **session** (repart de 0 à chaque lancement). Chacun peut être affiché ou caché.
- **Raccourcis globaux**, clavier ou souris (boutons et crans de molette), touches seules ou combinaisons. **Les touches et les clics ne sont jamais bloqués** : ils arrivent normalement au jeu, sans délai ajouté.
- +1, −1, +5, −5, reset, annuler le dernier reset. Le reset du global se confirme par une seconde touche, sans boîte de dialogue qui volerait le focus.
- **Format d'affichage libre** avec des variables : `{value}`, `{goal}`, `{remaining}`, `{ratio}`, `{ratio:p}` (ou en français : `{valeur}`, `{objectif}`, `{reste}`). Le texte entre crochets ne s'affiche que s'il y a un objectif : `{value}[ / {goal}]` donne `3 / 10`, ou `3` sans objectif.
- Polices installées, couleurs (objectif atteint / pas atteint), fond transparent, coins arrondis, premier plan.
- **Un son par action** : sons de Windows, sons intégrés, bips ou vos propres fichiers WAV / MP3. Volume, muet, choix de la sortie audio.
- Interface en français et en anglais, d'autres langues par packs.
- Un seul `.exe` d'environ 5,5 Mo, sans installation, consommation CPU négligeable.

Windows uniquement pour l'instant. Linux et macOS sont prévus.

## Téléchargement

Téléchargez `literally_a_counter.exe` sur la page [Releases](../../releases) et lancez-le. Les réglages sont enregistrés dans `%APPDATA%\LiterallyACounter\`.

Vous préférez ne pas lancer un exe tout fait ? [Compilez-le vous-même](#compiler-soi-même) : c'est l'affaire de quelques minutes.

## Raccourcis par défaut

| Action | Raccourci | Compteurs |
|---|---|---|
| +1 | Num + · Ctrl + clic molette · bouton latéral avant | session **et** global |
| −1 | Num − · bouton latéral arrière | session **et** global |
| +5 / −5 | Maj + Num + / Maj + Num − | session **et** global |
| Reset de la session | Num 0 | session |
| Reset du global | Ctrl + Num 0, puis Num Entrée dans les 3 s | global |
| Annuler le dernier reset | Ctrl + Num . | le dernier resetté |

Clic gauche / droit sur une fenêtre de compteur : +1 / −1 sur ce compteur. Au survol d'un compteur, le bouton **•••** ouvre son menu (saisir la valeur, réglages…).

Tout se modifie dans la fenêtre de configuration : appuyez sur le nouveau raccourci pour l'enregistrer.

## Anti-cheat

> [!WARNING]
> L'appli écoute le clavier comme OBS, Discord ou LiveSplit : un hook **passif**, qui lit les touches sans en bloquer ni en envoyer aucune. Elle ne lit ni ne touche jamais le jeu lui-même.
>
> C'est la méthode la plus sûre connue, mais aucun anti-cheat ne donne de garantie : avec les plus stricts (comme **Vanguard**), c'est à vos risques.
>
> **Un avertissement, une exclusion ou un ban d'un anti-cheat ?** [Ouvrez une issue](../../issues) en précisant le jeu et l'anti-cheat, pour que le problème soit corrigé, ou au moins signalé ici pour tout le monde.

<a name="streamer-avec-obs"></a>
<details>
<summary><h2>Streamer avec OBS</h2></summary>

Ajoutez une source **Capture de fenêtre**, choisissez la fenêtre du compteur et réglez la méthode de capture sur **« Moderne (WGC) »** (Windows Graphics Capture ; le libellé varie selon la version d'OBS). Avec les autres méthodes, le fond transparent devient noir.

</details>

<a name="packs-de-langue"></a>
<details>
<summary><h2>Packs de langue</h2></summary>

Le français et l'anglais sont intégrés. Pour ajouter une langue, copiez `lang/en.ftl`, traduisez les textes et enregistrez le fichier sous `lang/xx.ftl` à côté de l'exe (`xx` = code de la langue), avec le nom de la langue dans `language-name`. Elle apparaît dans la liste des langues au lancement suivant. Les textes manquants sont pris en anglais.

</details>

<a name="compiler-soi-même"></a>
<details>
<summary><h2>Compiler soi-même</h2></summary>

Pas besoin de faire confiance à un exe tout fait : l'appli se compile à partir de ce code source avec les outils officiels de Rust.

1. **Installez Rust** depuis [rustup.rs](https://rustup.rs) (`rustup-init.exe`). Quand il le propose, laissez-le installer les **outils de compilation C++ de Visual Studio** : Rust en a besoin sous Windows.
2. **Récupérez le code source** : avec `git clone` et l'adresse donnée par le bouton vert **Code**, ou téléchargez le ZIP depuis ce même bouton et décompressez-le.
3. **Compilez**, dans un terminal ouvert dans le dossier du projet :

   ```
   cargo build --release
   ```

   La première compilation télécharge les dépendances et prend quelques minutes.
4. L'appli est `target\release\literally_a_counter.exe`.

Pour lancer les tests automatiques : `cargo test`.

L'icône est déjà générée (`assets/icon/`). Seulement pour la modifier : `python tools/gen_icon.py assets/icon apercu.png` (nécessite Python et Pillow).

</details>

## Crédits

Sons intégrés : « Interface Sounds » de [Kenney](https://www.kenney.nl) (CC0). Détails dans [CREDITS.md](CREDITS.md).

Notes de version : [CHANGELOG.md](CHANGELOG.md).

## Licence

[GPL-3.0 ou ultérieure](LICENSE) : vous pouvez utiliser, étudier, modifier et partager l'appli ; les versions modifiées que vous diffusez doivent rester sous la même licence, avec leur code source.
