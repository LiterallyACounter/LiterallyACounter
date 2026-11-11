# Literally A Counter : traduction « Français ».
# Format Fluent (https://projectfluent.org) : `identifiant = texte`.
# `{ $nom }` est un emplacement rempli par l'appli (ne pas le traduire).

## Langue
language-name = Français
# Nombres : séparateur décimal, et format du pourcentage (`{ $percent }` = le nombre).
number-decimal-separator = ,
number-percent = { $percent } %

## Interface
# Choix de la langue de l'interface.
language-label = Langue
# Demande de confirmation du reset. Emplacement : `{key}` (touche de validation).
reset-prompt = Reset ? ({ $key } pour valider)
# Titre de la fenêtre du compteur global (visible dans la barre des tâches et OBS).
global-window-title = Literally A Counter : global
# Titre de la fenêtre du compteur de partie.
round-window-title = Literally A Counter : partie
# Titre de la fenêtre de configuration.
config-window-title = Literally A Counter : configuration
# Onglet des compteurs.
tab-counters = Compteurs
# Onglet des raccourcis.
tab-shortcuts = Raccourcis
# Bouton pour quitter l'appli.
quit = Quitter
# Titre de la section du global.
global-counter = Compteur global
# Titre de la section de la partie.
round-counter = Compteur de partie
# Case « afficher la fenêtre ».
show-window = Afficher la fenêtre
# Objectif d'un compteur.
goal = Objectif (0 = aucun)
# Option de copie de l'affichage.
round-copies-global = Copier l'affichage du global
# Champ du format.
template = Format
# Aide du format.
template-help = Variables : {"{"}value{"}"}, {"{"}goal{"}"}, {"{"}remaining{"}"}, {"{"}ratio{"}"}, {"{"}ratio:p{"}"}. Entre crochets : affiché seulement s'il y a un objectif.
# Taille de police.
font-size = Taille du texte
# Marge autour du texte.
margin = Marge
# Rayon des coins.
corner-radius = Arrondi des coins
# Ancrage.
anchor = Quand la largeur change
# Ancrage à gauche.
anchor-left = Bord gauche fixe
# Ancrage au centre.
anchor-center = Centre fixe
# Ancrage à droite.
anchor-right = Bord droit fixe
# Premier plan.
always-on-top = Toujours au premier plan
# Couleur de fond.
background = Fond
# Fond transparent.
transparent-background = Transparent
# Couleur du texte sans objectif.
plain-text-color = Texte (sans objectif)
# Couleur objectif pas atteint.
goal-pending-color = Objectif pas atteint
# Couleur objectif atteint.
goal-reached-color = Objectif atteint
# Couleur des alertes.
alert-color = Alertes (« Reset ? »)
# Compteurs visés : les deux.
target-both = global et partie
# Compteurs visés : le global.
target-global = global
# Compteurs visés : la partie.
target-round = partie
# Action reset du global.
action-reset-global = Reset du global
# Action reset de la partie.
action-reset-round = Reset de la partie
# Action valider le reset.
action-confirm-reset = Valider le reset
# Action annuler le reset.
action-undo-reset = Annuler le dernier reset
# Aide de l'onglet raccourcis.
shortcuts-help = Clique sur un raccourci pour le changer, ou sur + pour en ajouter. Appuie sur les touches voulues : la combinaison est enregistrée au premier relâchement.
# Capture en cours.
press-shortcut = Appuie sur la combinaison… (Échap pour annuler)
# Refus d'un clic gauche/droit seul.
main-click-needs-modifier = Clic gauche ou droit : ajoute Ctrl, Maj ou Alt, sinon il se déclencherait à chaque tir ou visée en jeu.
# Avertissement de conflit.
shortcut-conflict = Cette combinaison sert aussi à une autre action : les deux se déclencheront ensemble.
# Bouton de remise à zéro des raccourcis.
restore-default-shortcuts = Rétablir les raccourcis par défaut
# Infobulle du bouton ＋.
add-shortcut = Ajouter un raccourci
# Infobulle du bouton ✕.
remove-shortcut = Supprimer ce raccourci
# Ligne +1.
action-add1 = +1
# Ligne −1.
action-subtract1 = −1
# Ligne +5.
action-add5 = +5
# Ligne −5.
action-subtract5 = −5
# Titre de la fenêtre du menu d'un compteur.
menu-window-title = Literally A Counter : menu
# Champ de saisie de la valeur.
value = Valeur
# Bouton de validation.
apply = Valider
# Bouton de reset dans le menu.
reset-action = Reset
# Bouton pour cacher la fenêtre.
hide-window = Cacher la fenêtre
# Bouton pour ouvrir la configuration.
open-settings = Configuration…
# Rappel du menu dans la configuration.
menu-hint = Le menu d'un compteur (saisie de la valeur, reset…) s'ouvre avec le bouton ••• qui apparaît quand la souris le survole.
# Choix de la méthode d'écoute de la souris.
mouse-method-label = Écoute de la souris :
# Option Raw Input.
mouse-raw-input = Raw Input (recommandé)
# Option interrogation.
mouse-polling = Interrogation (plus économe, sans crans de molette)
# Raccourci de molette inactif en mode interrogation.
wheel-needs-raw-input = Inactif : les crans de molette demandent l'écoute Raw Input.
# Bandeau de protection. Emplacement : `{rate}` (événements par seconde).
mouse-rate-warning = Ta souris envoie { $rate } événements par seconde à l'appli, ce qui peut coûter du processeur. L'interrogation est plus économe, mais sans crans de molette.
# Bouton du bandeau.
switch-to-polling = Passer à l'interrogation
# Bouton du menu vers l'onglet Raccourcis.
edit-shortcuts = Modifier…
# Titre de la mini-fenêtre du bouton « ••• » (jamais visible).
menu-button-window-title = Literally A Counter : bouton du menu
# Choix de la police.
font = Police
# Police par défaut.
default-font = Police par défaut
# Champ de recherche de police.
search-font = Rechercher…
# Onglet du son.
tab-sound = Son et tonalités
# Choix de la sortie audio.
audio-output = Sortie audio
# Sortie par défaut de Windows.
default-output = Par défaut de Windows
# Sortie choisie mais débranchée.
output-missing = Sortie introuvable : celle par défaut est utilisée
# Volume général.
volume = Volume
# Aucune sortie audio.
no-audio = Aucune sortie audio disponible : l'appli fonctionne sans son.
# La sortie voulue refuse de s'ouvrir. Emplacement : `{name}` (sortie réellement utilisée).
output-unavailable = Cette sortie n'a pas pu être ouverte (elle est peut-être réservée par une autre appli, comme Voicemeeter) : le son passe par « { $name } ».
# Sortie choisie qui refuse de s'ouvrir. Emplacement : `{name}`.
output-refused = « { $name } » n'a pas pu être ouverte (elle est peut-être réservée par une autre appli, comme Voicemeeter) : la sortie précédente est conservée.
# Titre de la section des sons.
sounds = Sons
# Emplacement de son : +.
slot-increment = + (ajout)
# Emplacement de son : −.
slot-decrement = − (retrait)
# Emplacement de son : alerte.
slot-reset-prompt = Alerte « Reset ? »
# Emplacement de son : reset fait.
slot-reset-done = Reset fait
# Emplacement de son : reset annulé.
slot-reset-undone = Reset annulé
# Bouton pour écouter un son.
listen = Écouter
# Entrée de liste pour importer.
import-file = Importer un fichier…
# Titre de groupe.
windows-sounds = Sons de Windows
# Titre de groupe.
beeps = Bips
# Titre de groupe.
imported-sounds = Sons importés
# Bip grave.
beep-low = Bip grave
# Bip moyen.
beep-mid = Bip moyen
# Bip aigu.
beep-high = Bip aigu
# Bip très aigu.
beep-very-high = Bip très aigu
# Nom d'un son importé. Emplacement : `{name}`.
imported-sound = { $name } (importé)
# Titre de la fenêtre Ouvrir.
import-title = Importer un son
# Nom du filtre de fichiers.
sound-files = Sons
# Import raté : fichier illisible. Emplacement : `{name}`.
import-unreadable = « { $name } » n'a pas pu être importé : format non pris en charge ou fichier abîmé (formats acceptés : WAV, MP3).
# Import raté : copie impossible. Emplacement : `{name}`.
import-copy-failed = « { $name } » n'a pas pu être copié dans le dossier des sons.
# Titre de groupe : sons embarqués.
embedded-sounds = Sons de l'appli (Kenney, CC0)

# Onglet des informations.
tab-info = Infos
# Section repliable : affichage d'un compteur.
display-section = Affichage
# Section repliable : réglages avancés.
advanced-section = Avancé
# Titre des couleurs du texte.
colors-section = Couleurs du texte
# Bouton : rétablir l'affichage par défaut.
reset-display = Rétablir l'affichage par défaut
# Case : la partie utilise les sons du global.
round-uses-global-sounds = Utiliser les sons du global
# Bulle de l'icône haut-parleur.
mute-tooltip = Couper ou réactiver ce son
# Bulle d'aide du format.
tooltip-format = Le texte affiché. Les variables entre accolades sont remplacées par la valeur, l'objectif…
# Bulle d'aide de l'ancrage.
tooltip-anchor = Le côté qui reste fixe quand le nombre s'allonge (par ex. de 9 à 10).
# Bulle d'aide du premier plan.
tooltip-always-on-top = Garde le compteur au-dessus des autres fenêtres, jeu compris.
# Bulle d'aide du fond transparent.
tooltip-transparent = Seul le texte est visible : idéal pour OBS.
# Bulle d'aide de la copie de l'affichage.
tooltip-copy-display = La partie prend le format, la police et les couleurs du global (pas l'objectif).
# Bulle d'aide de Raw Input.
tooltip-raw-input = Reçoit tous les événements de la souris : aucun clic manqué, crans de molette disponibles.
# Bulle d'aide de l'interrogation.
tooltip-polling = Vérifie les boutons toutes les ~15 ms : un peu plus économe, mais sans crans de molette.
# Version de l'appli. Emplacement : `{version}`.
app-version = Version { $version }
# Bouton : ouvrir le dossier de l'appli.
open-app-folder = Ouvrir le dossier de l'appli
# Bulle du bouton du dossier.
open-app-folder-tooltip = Contient la sauvegarde des réglages et les sons importés.
# Titre du mode d'emploi.
guide-title = Mode d'emploi
# Mode d'emploi : compter.
guide-count = Compter : appuie sur les raccourcis (onglet Raccourcis), même quand le jeu a le focus. Clic gauche / droit sur un compteur : +1 / −1.
# Mode d'emploi : reset.
guide-reset = Reset : celui de la partie est immédiat ; celui du global affiche « Reset ? », à valider dans les 3 secondes. Une erreur ? « Annuler le dernier reset ».
# Mode d'emploi : déplacer, cacher.
guide-move = Déplacer un compteur : glisse-le avec la souris. Échap le cache (comme décocher « Afficher la fenêtre »).
# Mode d'emploi : stream.
guide-stream = Stream : dans OBS, ajoute une « Capture de fenêtre » par compteur. Le bouton ••• et son menu n'apparaissent jamais dans la capture.
# Crédits.
credits = Sons de l'appli : « Interface Sounds » de Kenney (www.kenney.nl), licence CC0.

## Noms des touches (Ctrl, Alt, Num +… sont identiques partout et restent dans le code)
key-shift = Maj
key-enter = Entrée
key-numpad-enter = Num Entrée
key-mouse-left = Clic gauche
key-mouse-right = Clic droit
key-mouse-middle = Clic molette
key-mouse-x1 = Souris X1 (arrière)
key-mouse-x2 = Souris X2 (avant)
key-wheel-up = Molette haut
key-wheel-down = Molette bas
key-wheel-left = Molette gauche
key-wheel-right = Molette droite
key-unknown = Touche { $code }
