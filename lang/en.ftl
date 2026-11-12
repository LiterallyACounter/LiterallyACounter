# Literally A Counter : traduction « English ».
# Format Fluent (https://projectfluent.org) : `identifiant = texte`.
# `{ $nom }` est un emplacement rempli par l'appli (ne pas le traduire).

## Langue
language-name = English
# Nombres : séparateur décimal, et format du pourcentage (`{ $percent }` = le nombre).
number-decimal-separator = .
number-percent = { $percent }%

## Interface
# Choix de la langue de l'interface.
language-label = Language
# Demande de confirmation du reset. Emplacement : `{key}` (touche de validation).
reset-prompt = Reset? ({ $key } to confirm)
# Titre de la fenêtre du compteur global (visible dans la barre des tâches et OBS).
global-window-title = Literally A Counter: total
# Titre de la fenêtre du compteur de partie.
round-window-title = Literally A Counter: session
# Titre de la fenêtre de configuration.
config-window-title = Literally A Counter: settings
# Onglet des compteurs.
tab-counters = Counters
# Onglet des raccourcis.
tab-shortcuts = Shortcuts
# Bouton pour quitter l'appli.
quit = Quit
# Titre de la section du global.
global-counter = Total counter
# Titre de la section de la partie.
round-counter = Session counter
# Case « afficher la fenêtre ».
show-window = Show the window
# Objectif d'un compteur.
goal = Goal (0 = none)
# Option de copie de l'affichage.
round-copies-global = Use the total's display
# Champ du format.
template = Format
# Aide du format.
template-help = Variables: {"{"}value{"}"}, {"{"}goal{"}"}, {"{"}remaining{"}"}, {"{"}ratio{"}"}, {"{"}ratio:p{"}"}. Inside brackets: shown only when there is a goal.
# Taille de police.
font-size = Text size
# Marge autour du texte.
margin = Margin
# Rayon des coins.
corner-radius = Corner radius
# Ancrage.
anchor = When the width changes
# Ancrage à gauche.
anchor-left = Left edge fixed
# Ancrage au centre.
anchor-center = Center fixed
# Ancrage à droite.
anchor-right = Right edge fixed
# Premier plan.
always-on-top = Always on top
# Couleur de fond.
background = Background
# Fond transparent.
transparent-background = Transparent
# Couleur du texte sans objectif.
plain-text-color = Text (no goal)
# Couleur objectif pas atteint.
goal-pending-color = Goal not reached
# Couleur objectif atteint.
goal-reached-color = Goal reached
# Couleur des alertes.
alert-color = Alerts ("Reset?")
# Compteurs visés : les deux.
target-both = total and session
# Compteurs visés : le global.
target-global = total
# Compteurs visés : la partie.
target-round = session
# Action reset du global.
action-reset-global = Reset the total
# Action reset de la partie.
action-reset-round = Reset the session
# Action valider le reset.
action-confirm-reset = Confirm the reset
# Action annuler le reset.
action-undo-reset = Undo the last reset
# Aide de l'onglet raccourcis.
shortcuts-help = Click a shortcut to change it, or + to add one. Press the keys you want: the combination is saved when you release the first key.
# Capture en cours.
press-shortcut = Press the combination… (Esc to cancel)
# Refus d'un clic gauche/droit seul.
main-click-needs-modifier = Left or right click: add Ctrl, Shift or Alt, otherwise it would trigger on every shot or aim in game.
# Avertissement de conflit.
shortcut-conflict = This combination is also used by another action: both will trigger together.
# Bouton de remise à zéro des raccourcis.
restore-default-shortcuts = Restore default shortcuts
# Infobulle du bouton ＋.
add-shortcut = Add a shortcut
# Infobulle du bouton ✕.
remove-shortcut = Remove this shortcut
# Ligne +1.
action-add1 = +1
# Ligne −1.
action-subtract1 = −1
# Ligne +5.
action-add5 = +5
# Ligne −5.
action-subtract5 = −5
# Titre de la fenêtre du menu d'un compteur.
menu-window-title = Literally A Counter: menu
# Champ de saisie de la valeur.
value = Value
# Bouton de validation.
apply = Apply
# Bouton de reset dans le menu.
reset-action = Reset
# Bouton pour cacher la fenêtre.
hide-window = Hide the window
# Bouton pour ouvrir la configuration.
open-settings = Settings…
# Rappel du menu dans la configuration.
menu-hint = A counter's menu (set value, reset…) opens with the ••• button shown when the mouse hovers it.
# Choix de la méthode d'écoute de la souris.
mouse-method-label = Mouse listening:
# Option Raw Input.
mouse-raw-input = Raw Input (recommended)
# Option interrogation.
mouse-polling = Polling (lighter, no wheel notches)
# Raccourci de molette inactif en mode interrogation.
wheel-needs-raw-input = Inactive: wheel notches need Raw Input listening.
# Bandeau de protection. Emplacement : `{rate}` (événements par seconde).
mouse-rate-warning = Your mouse sends { $rate } events per second to the app, which may cost CPU. Polling is lighter, but has no wheel notches.
# Bouton du bandeau.
switch-to-polling = Switch to polling
# Bouton du menu vers l'onglet Raccourcis.
edit-shortcuts = Edit…
# Titre de la mini-fenêtre du bouton « ••• » (jamais visible).
menu-button-window-title = Literally A Counter: menu button
# Choix de la police.
font = Font
# Police par défaut.
default-font = Default font
# Champ de recherche de police.
search-font = Search…
# Onglet du son.
tab-sound = Sound & tones
# Choix de la sortie audio.
audio-output = Audio output
# Sortie par défaut de Windows.
default-output = Windows default
# Sortie choisie mais débranchée.
output-missing = Output not found: using the default one
# Volume général.
volume = Volume
# Aucune sortie audio.
no-audio = No audio output available: the app works without sound.
# La sortie voulue refuse de s'ouvrir. Emplacement : `{name}` (sortie réellement utilisée).
output-unavailable = This output could not be opened (another app, such as audio software or a virtual mixer, may be using it exclusively): sound goes to "{ $name }".
# Sortie choisie qui refuse de s'ouvrir. Emplacement : `{name}`.
output-refused = '{ $name }' could not be opened (another app, such as audio software or a virtual mixer, may be using it exclusively): the previous output is kept.
# Titre de la section des sons.
sounds = Sounds
# Emplacement de son : +.
slot-increment = + (add)
# Emplacement de son : −.
slot-decrement = − (remove)
# Emplacement de son : alerte.
slot-reset-prompt = Reset? alert
# Emplacement de son : reset fait.
slot-reset-done = Reset done
# Emplacement de son : reset annulé.
slot-reset-undone = Reset undone
# Bouton pour écouter un son.
listen = Listen
# Entrée de liste pour importer.
import-file = Import a file…
# Titre de groupe.
windows-sounds = Windows sounds
# Titre de groupe.
beeps = Beeps
# Titre de groupe.
imported-sounds = Imported sounds
# Bip grave.
beep-low = Low beep
# Bip moyen.
beep-mid = Medium beep
# Bip aigu.
beep-high = High beep
# Bip très aigu.
beep-very-high = Very high beep
# Nom d'un son importé. Emplacement : `{name}`.
imported-sound = { $name } (imported)
# Titre de la fenêtre Ouvrir.
import-title = Import a sound
# Nom du filtre de fichiers.
sound-files = Sounds
# Import raté : fichier illisible. Emplacement : `{name}`.
import-unreadable = '{ $name }' could not be imported: unsupported format or damaged file (accepted formats: WAV, MP3).
# Import raté : copie impossible. Emplacement : `{name}`.
import-copy-failed = '{ $name }' could not be copied to the sounds folder.
# Titre de groupe : sons embarqués.
embedded-sounds = App sounds (Kenney, CC0)

# Onglet des informations.
tab-info = About
# Section repliable : affichage d'un compteur.
display-section = Display
# Section repliable : réglages avancés.
advanced-section = Advanced
# Titre des couleurs du texte.
colors-section = Text colours
# Bouton : rétablir l'affichage par défaut.
reset-display = Restore default display
# Case : la partie utilise les sons du global.
round-uses-global-sounds = Use the total's sounds
# Bulle de l'icône haut-parleur.
mute-tooltip = Mute or unmute this sound
# Bulle du muet général.
mute-all-tooltip = Mute or unmute all sounds (moving the volume unmutes)
# Libellé : son prioritaire.
sound-priority = Priority sound
# Bulle du son prioritaire.
sound-priority-tooltip = When a shortcut counts on both counters, only one sound plays: the one of the counter chosen here.
# Bouton : priorité au global.
priority-global = Total
# Bouton : priorité à la session.
priority-round = Session
# Bulle d'aide du format.
tooltip-format = The displayed text. Variables in braces are replaced by the value, the goal…
# Bulle d'aide de l'ancrage.
tooltip-anchor = The side that stays fixed when the number gets longer (e.g. from 9 to 10).
# Bulle d'aide du premier plan.
tooltip-always-on-top = Keeps the counter above other windows, the game included.
# Bulle d'aide du fond transparent.
tooltip-transparent = Only the text is visible: ideal for OBS.
# Bulle d'aide de la copie de l'affichage.
tooltip-copy-display = The session takes the total's format, font and colours (not the goal).
# Bulle d'aide de Raw Input.
tooltip-raw-input = Receives every mouse event: no missed click, wheel notches available.
# Bulle d'aide de l'interrogation.
tooltip-polling = Checks the buttons every ~15 ms: a bit lighter, but no wheel notches.
# Version de l'appli. Emplacement : `{version}`.
app-version = Version { $version }
# Bouton : ouvrir le dossier de l'appli.
open-app-folder = Open the app folder
# Bulle du bouton du dossier.
open-app-folder-tooltip = Holds the saved settings and the imported sounds.
# Titre du mode d'emploi.
guide-title = How to use
# Mode d'emploi : compter.
guide-count = Counting: press the shortcuts (Shortcuts tab), even while the game has the focus. Left / right click on a counter: +1 / −1.
# Mode d'emploi : reset.
guide-reset = Reset: the session's is immediate; the total's shows "Reset?", to confirm within 3 seconds. A mistake? "Undo the last reset".
# Mode d'emploi : déplacer, cacher.
guide-move = Moving a counter: drag it with the mouse. Esc hides it (like unticking "Show the window").
# Mode d'emploi : stream.
guide-stream = Streaming: in OBS, add one "Window Capture" per counter. The ••• button and its menu never show in the capture.
# Crédits.
credits = App sounds: "Interface Sounds" by Kenney (www.kenney.nl), CC0 licence.

## Noms des touches (Ctrl, Alt, Num +… sont identiques partout et restent dans le code)
key-shift = Shift
key-enter = Enter
key-numpad-enter = Num Enter
key-mouse-left = Left click
key-mouse-right = Right click
key-mouse-middle = Middle click
key-mouse-x1 = Mouse X1 (back)
key-mouse-x2 = Mouse X2 (forward)
key-wheel-up = Wheel up
key-wheel-down = Wheel down
key-wheel-left = Wheel left
key-wheel-right = Wheel right
key-unknown = Key { $code }
