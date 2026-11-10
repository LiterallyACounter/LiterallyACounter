//! Capture d'un raccourci « à l'appui », logique pure (testable sans clavier).
//!
//! Règle : on note les touches dans l'ordre où elles sont enfoncées. Au PREMIER relâchement,
//! la combinaison est terminée : la dernière touche enfoncée est la déclencheuse, les autres
//! sont les touches à tenir. Exemples :
//! - Num + enfoncé puis relâché → « Num + » ;
//! - Ctrl, puis Num +, puis relâchement → « Ctrl + Num + » ;
//! - Z, puis Num +, puis relâchement → « Z + Num + ».
//!
//! Échap annule la capture (il ne peut donc pas devenir un raccourci).

use super::Key;

/// Où en est la capture après une touche.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureStep {
    /// Pas encore terminée.
    Pending,
    /// Terminée : la combinaison capturée.
    Done { held: Vec<Key>, trigger: Key },
    /// Annulée avec Échap.
    Cancelled,
}

#[derive(Debug, Default)]
pub struct Capture {
    /// Les touches enfoncées depuis le début de la capture, dans l'ordre.
    pressed: Vec<Key>,
}

impl Capture {
    pub fn key_down(&mut self, key: Key) -> CaptureStep {
        if key == Key::ESCAPE {
            return CaptureStep::Cancelled;
        }
        // Une touche maintenue se répète : on ne la note qu'une fois.
        if !self.pressed.contains(&key) {
            self.pressed.push(key);
        }
        CaptureStep::Pending
    }

    pub fn key_up(&mut self, key: Key) -> CaptureStep {
        // Une touche enfoncée AVANT le début de la capture (et relâchée pendant) ne compte pas.
        if !self.pressed.contains(&key) {
            return CaptureStep::Pending;
        }
        // `split_last` : le dernier élément et le reste, ou `None` si la liste est vide
        // (impossible ici, puisque `key` est dedans).
        match self.pressed.split_last() {
            Some((&trigger, held)) => CaptureStep::Done {
                held: held.to_vec(),
                trigger,
            },
            None => CaptureStep::Pending,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const Z: Key = Key(0x5A);

    fn done(held: &[Key], trigger: Key) -> CaptureStep {
        CaptureStep::Done {
            held: held.to_vec(),
            trigger,
        }
    }

    #[test]
    fn single_key() {
        let mut c = Capture::default();
        assert_eq!(c.key_down(Key::NUMPAD_ADD), CaptureStep::Pending);
        assert_eq!(c.key_up(Key::NUMPAD_ADD), done(&[], Key::NUMPAD_ADD));
    }

    #[test]
    fn modifier_then_key() {
        let mut c = Capture::default();
        c.key_down(Key::CONTROL);
        c.key_down(Key::NUMPAD_ADD);
        // Relâcher Ctrl en premier donne le même résultat : c'est l'ordre d'appui qui compte.
        assert_eq!(
            c.key_up(Key::CONTROL),
            done(&[Key::CONTROL], Key::NUMPAD_ADD)
        );
    }

    #[test]
    fn any_key_can_be_held() {
        let mut c = Capture::default();
        c.key_down(Z);
        c.key_down(Key::NUMPAD_ADD);
        assert_eq!(c.key_up(Key::NUMPAD_ADD), done(&[Z], Key::NUMPAD_ADD));
    }

    #[test]
    fn mouse_buttons_too() {
        let mut c = Capture::default();
        c.key_down(Key::CONTROL);
        c.key_down(Key::MOUSE_MIDDLE);
        assert_eq!(
            c.key_up(Key::MOUSE_MIDDLE),
            done(&[Key::CONTROL], Key::MOUSE_MIDDLE)
        );
    }

    #[test]
    fn repeat_is_ignored() {
        let mut c = Capture::default();
        c.key_down(Key::CONTROL);
        c.key_down(Key::CONTROL);
        c.key_down(Key::NUMPAD_0);
        assert_eq!(
            c.key_up(Key::NUMPAD_0),
            done(&[Key::CONTROL], Key::NUMPAD_0)
        );
    }

    #[test]
    fn release_of_a_key_held_before_capture_is_ignored() {
        let mut c = Capture::default();
        assert_eq!(c.key_up(Key::NUMPAD_ADD), CaptureStep::Pending);
    }

    #[test]
    fn escape_cancels() {
        let mut c = Capture::default();
        c.key_down(Key::CONTROL);
        assert_eq!(c.key_down(Key::ESCAPE), CaptureStep::Cancelled);
    }
}
