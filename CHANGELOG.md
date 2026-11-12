# Notes de version

## v1.0.0 — première version complète (Windows)

Literally A Counter affiche des compteurs dans de petites fenêtres sans bordure, à l'écran ou en overlay de stream (OBS), et les fait avancer par des raccourcis clavier ou souris, même quand le jeu a le focus.

### Compteurs

- **Deux compteurs** : un compteur **global** (le total, sauvegardé et repris au démarrage) et un compteur de **session** (repart de 0 à chaque lancement). Chacun peut être affiché ou caché.
- Opérations : +1, −1, +5, −5, reset, annuler le dernier reset. Un compteur ne descend jamais sous 0.
- **Reset du global en deux touches**, sans boîte de dialogue qui volerait le focus au jeu : la fenêtre affiche « Reset ? », une touche de validation confirme dans les 3 secondes.
- **Objectif** facultatif pour chaque compteur, avec une couleur « pas encore atteint » et une couleur « atteint ».
- Clic gauche / droit sur une fenêtre de compteur : +1 / −1 sur ce compteur. Un bouton « ••• » au survol ouvre son menu (saisie de la valeur, reset, premier plan, ses raccourcis…).

### Affichage

- **Format libre** avec des variables, en anglais ou en français : `{value}`, `{goal}`, `{remaining}`, `{ratio}`, `{ratio:p}` (pourcentage). Une partie entre crochets ne s'affiche que s'il y a un objectif : `{value}[ / {goal}]` donne `3 / 10`, ou `3` sans objectif.
- **Polices installées** sur l'ordinateur, avec aperçu ; taille, marge, coins arrondis, couleurs, fond transparent possible.
- La fenêtre s'adapte au texte, avec un ancrage au centre, à gauche ou à droite. Premier plan par défaut, désactivable.
- La session peut **copier l'affichage du global** (activé par défaut) ; ses propres réglages sont gardés pour plus tard.
- Capture OBS : source « Capture de fenêtre », méthode **« Moderne (WGC) »**.

### Raccourcis

- Touches seules ou combinaisons (« Ctrl + Num + »), définies en appuyant dessus. Par défaut : Num + / Num − pour +1 / −1, Maj + Num + / Maj + Num − pour +5 / −5.
- **Souris** : tous les boutons, et les crans de molette (haut, bas, gauche, droite). Clic gauche et droit seulement avec Ctrl, Maj ou Alt, pour ne pas compter à chaque tir.
- Chaque raccourci « compter » vise les deux compteurs, le global seul ou la session seule.
- **Les touches et les clics ne sont jamais bloqués** : ils arrivent normalement au jeu, sans délai ajouté. Si aucun raccourci n'utilise la souris, l'appli ne l'écoute pas.
- Deux méthodes d'écoute de la souris (Raw Input par défaut, ou interrogation), avec un avertissement si une souris très rapide coûte trop de CPU.

### Son

- Un son par action : sons de Windows, sons embarqués (pack « Interface Sounds » de Kenney, CC0), bips générés ou **fichiers importés** (WAV, MP3). Chaque son peut être coupé.
- Volume général, **muet général**, choix de la sortie audio (avec repli sur une autre sortie si elle est occupée).
- **Son prioritaire** : quand un appui fait compter les deux compteurs, on n'entend que celui du compteur choisi (la session par défaut).

### Interface

- Fenêtre de configuration en quatre onglets : Compteurs, Raccourcis, Son et tonalités, Infos. Sections repliables, bulles d'aide, réglages appliqués en direct et sauvegardés.
- **Français et anglais**, détectés au premier lancement ; packs de langue supplémentaires possibles (`lang/xx.ftl` à côté de l'exe).
- Réglages sauvegardés dans `%APPDATA%\LiterallyACounter\`.

### Technique

- Un seul fichier `.exe` d'environ 5,5 Mo, sans installation.
- Consommation négligeable : pas de lag ni de perte de FPS mesurés en jeu.
- Pour l'instant **Windows uniquement** ; Linux et Mac sont prévus.
