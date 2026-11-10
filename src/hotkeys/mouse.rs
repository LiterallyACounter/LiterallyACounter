//! Boutons de souris par INTERROGATION, et non par écoute des événements.
//!
//! Pourquoi : Windows ne permet pas d'écouter seulement les clics. Tout moyen d'écouter la souris
//! reçoit aussi CHAQUE mouvement (jusqu'à 8 000 par seconde avec une souris gaming), et un hook
//! souris se place même sur le trajet de ces mouvements, au risque de retarder le jeu.
//!
//! À la place, on demande régulièrement à Windows « quels boutons sont enfoncés ? ». On compare
//! avec la fois précédente, et chaque changement devient un appui ou un relâchement, transmis au
//! même moteur de règles que le clavier. Coût fixe et minime, indépendant de la souris, et l'appli
//! n'est jamais sur le trajet des clics ni des mouvements.

use std::collections::HashSet;

use super::Key;

/// Ce qui a changé pour un bouton depuis l'interrogation précédente.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonChange {
    Pressed(Key),
    Released(Key),
}

#[derive(Debug, Default)]
pub struct MousePoller {
    /// Les boutons enfoncés lors de la dernière interrogation.
    down: HashSet<Key>,
}

impl MousePoller {
    /// Interroge l'état des boutons `buttons` (via `is_down`) et renvoie les changements.
    /// `is_down` est passée en paramètre : les tests simulent ainsi la souris.
    pub fn poll(&mut self, buttons: &[Key], is_down: impl Fn(Key) -> bool) -> Vec<ButtonChange> {
        let mut changes = Vec::new();
        for &button in buttons {
            let now_down = is_down(button);
            let was_down = self.down.contains(&button);
            // `match` sur un tuple de deux booléens : les 4 cas possibles.
            match (was_down, now_down) {
                (false, true) => {
                    self.down.insert(button);
                    changes.push(ButtonChange::Pressed(button));
                }
                (true, false) => {
                    self.down.remove(&button);
                    changes.push(ButtonChange::Released(button));
                }
                // Pas de changement (toujours relâché, ou toujours enfoncé).
                _ => {}
            }
        }
        changes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BUTTONS: [Key; 2] = [Key::MOUSE_MIDDLE, Key::MOUSE_X2];

    #[test]
    fn nothing_changes_while_released() {
        let mut poller = MousePoller::default();
        assert_eq!(poller.poll(&BUTTONS, |_| false), vec![]);
    }

    #[test]
    fn press_is_reported_once_then_release() {
        let mut poller = MousePoller::default();
        let middle_down = |key: Key| key == Key::MOUSE_MIDDLE;
        assert_eq!(
            poller.poll(&BUTTONS, middle_down),
            vec![ButtonChange::Pressed(Key::MOUSE_MIDDLE)]
        );
        // Toujours enfoncé : pas de nouvel appui (pas de répétition).
        assert_eq!(poller.poll(&BUTTONS, middle_down), vec![]);
        assert_eq!(
            poller.poll(&BUTTONS, |_| false),
            vec![ButtonChange::Released(Key::MOUSE_MIDDLE)]
        );
    }

    #[test]
    fn only_watched_buttons_are_reported() {
        // Le clic gauche n'est pas surveillé : il est ignoré même s'il est enfoncé.
        let mut poller = MousePoller::default();
        assert_eq!(poller.poll(&BUTTONS, |key| key == Key::MOUSE_LEFT), vec![]);
    }

    #[test]
    fn several_buttons_at_once() {
        let mut poller = MousePoller::default();
        assert_eq!(
            poller.poll(&BUTTONS, |_| true),
            vec![
                ButtonChange::Pressed(Key::MOUSE_MIDDLE),
                ButtonChange::Pressed(Key::MOUSE_X2)
            ]
        );
    }
}
