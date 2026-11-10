//! Branchement Windows : un « hook clavier bas niveau » (WH_KEYBOARD_LL), et l'interrogation
//! régulière des boutons de souris (voir `mouse.rs`).
//!
//! Principe du clavier :
//! - Windows appelle notre fonction `hook_proc` à CHAQUE frappe, quelle que soit l'appli au premier plan.
//! - On regarde la touche, on prévient l'appli si un raccourci correspond, puis on PASSE LA MAIN
//!   (`CallNextHookEx`) : la touche continue son chemin jusqu'au jeu, rien n'est bloqué (contrainte C3).
//! - Ce traitement doit être ULTRA RAPIDE : Windows désactive en silence un hook trop lent,
//!   et un hook lent ferait ramer tout le système (contrainte C4).
//!
//! Le hook vit dans un THREAD dédié, qui tourne en parallèle de l'interface. L'appli le pilote
//! (nouvelle liste de raccourcis, capture d'un raccourci) avec une `HotkeyHandle` : la demande
//! part dans un channel, puis un message Windows (`WM_APP`) réveille le thread pour qu'il la lise.
//!
//! Ce fichier contient du code `unsafe` : des appels directs à l'API Windows (écrite en C).
//! Le compilateur Rust ne peut pas vérifier ce que fait du code C, donc on lui dit
//! « je garantis moi-même que c'est correct » avec un bloc `unsafe { ... }`.
//! On garde ces blocs aussi petits que possible, chacun avec un commentaire `SÉCURITÉ :`.

use std::cell::RefCell;
use std::ptr::{null, null_mut};
use std::sync::mpsc::{self, Receiver, Sender};

use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetMessageW, HC_ACTION, KBDLLHOOKSTRUCT, KillTimer, LLKHF_EXTENDED, MSG,
    PM_NOREMOVE, PeekMessageW, PostThreadMessageW, SetTimer, SetWindowsHookExW, WH_KEYBOARD_LL,
    WM_APP, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP, WM_TIMER,
};

use super::capture::{Capture, CaptureStep};
use super::matcher::Matcher;
use super::mouse::{ButtonChange, MousePoller};
use super::{ALL_MOUSE_BUTTONS, Control, HotkeyEvent, Key, Shortcut, mouse_keys_used};

/// Intervalle d'interrogation des boutons de souris, en millisecondes.
/// (Le minuteur de Windows a une précision d'environ 15,6 ms : inutile de demander moins.)
const MOUSE_POLL_MS: u32 = 15;

/// Le moyen de piloter le thread des raccourcis depuis l'appli.
pub struct HotkeyHandle {
    /// Bout « envoi » du channel des demandes.
    controls: Sender<Control>,
    /// L'identifiant Windows du thread du hook, pour le réveiller.
    thread_id: u32,
}

impl HotkeyHandle {
    /// Envoie une demande au thread des raccourcis.
    pub fn send(&self, control: Control) {
        if self.controls.send(control).is_ok() {
            // Réveille le thread : sa boucle de messages recevra WM_APP et lira la demande.
            // SÉCURITÉ : simple envoi d'un message sans donnée à un thread de notre programme.
            unsafe { PostThreadMessageW(self.thread_id, WM_APP, 0, 0) };
        }
    }
}

/// L'état dont `hook_proc` a besoin.
struct HookState {
    matcher: Matcher,
    /// Les boutons de souris utilisés par les raccourcis.
    mouse_buttons: Vec<Key>,
    mouse_poller: MousePoller,
    /// Le minuteur d'interrogation de la souris, s'il tourne : son identifiant, attribué par
    /// Windows (sans fenêtre, c'est Windows qui le choisit) et nécessaire pour l'arrêter.
    mouse_timer: Option<usize>,
    /// Capture en cours (`None` : pas de capture, les raccourcis sont actifs).
    capture: Option<Capture>,
    /// Les demandes de l'appli.
    controls: Receiver<Control>,
    /// La fonction à appeler pour chaque événement (raccourci déclenché, capture terminée…).
    /// `Box<dyn Fn(...)>` : « n'importe quelle fonction de cette forme », rangée dans le tas.
    on_event: Box<dyn Fn(HotkeyEvent)>,
}

impl HookState {
    /// Les boutons à interroger : ceux des raccourcis, ou TOUS pendant une capture.
    fn watched_buttons(&self) -> &[Key] {
        if self.capture.is_some() {
            &ALL_MOUSE_BUTTONS
        } else {
            &self.mouse_buttons
        }
    }

    /// Démarre ou arrête le minuteur de la souris selon les besoins : si aucun bouton n'est
    /// à surveiller, l'appli n'interroge pas du tout la souris.
    fn update_mouse_timer(&mut self) {
        let needed = !self.watched_buttons().is_empty();
        match self.mouse_timer {
            None if needed => {
                // SÉCURITÉ : minuteur rattaché au thread (fenêtre nulle, pas de fonction de rappel).
                let id = unsafe { SetTimer(null_mut(), 0, MOUSE_POLL_MS, None) };
                // 0 = échec : on laisse `None` (on réessaiera au prochain changement).
                self.mouse_timer = (id != 0).then_some(id);
            }
            Some(id) if !needed => {
                // SÉCURITÉ : arrêt d'un minuteur de ce thread, avec l'identifiant donné par Windows.
                unsafe { KillTimer(null_mut(), id) };
                self.mouse_timer = None;
            }
            _ => {} // déjà dans le bon état
        }
    }
}

// Problème : Windows appelle `hook_proc` sans nous laisser lui passer de paramètre.
// Où ranger le matcher ? Dans une variable propre au thread du hook (`thread_local!`).
// `RefCell` permet de la modifier même à travers une référence partagée, avec une vérification
// à l'exécution (et non à la compilation) qu'on ne la modifie pas à deux endroits à la fois.
// `Option` : `None` tant que le hook n'est pas démarré.
thread_local! {
    static STATE: RefCell<Option<HookState>> = const { RefCell::new(None) };
}

/// Démarre l'écoute du clavier dans un thread dédié, et renvoie le moyen de le piloter
/// (`None` si le thread n'a pas pu démarrer).
///
/// `on_event` est appelée DEPUIS LE THREAD DU HOOK. Elle doit donc être rapide et pouvoir être
/// envoyée à un autre thread :
/// - `Send` : « peut être transférée à un autre thread en toute sécurité » ;
/// - `'static` : « ne contient pas de référence vers une donnée qui pourrait disparaître avant elle ».
///   Le compilateur vérifie ces deux garanties : c'est ce qui rend les threads sûrs en Rust.
pub fn start(
    shortcuts: Vec<Shortcut>,
    on_event: impl Fn(HotkeyEvent) + Send + 'static,
) -> Option<HotkeyHandle> {
    let (controls_sender, controls) = mpsc::channel();
    // Un second channel, juste pour récupérer l'identifiant du thread une fois démarré.
    let (id_sender, id_receiver) = mpsc::channel();

    // `move` : la closure PREND POSSESSION de ce qu'elle utilise (elle l'emporte dans le thread).
    let spawn_result = std::thread::Builder::new()
        .name("hotkeys".into())
        .spawn(move || {
            STATE.with(|state| {
                let mut hook_state = HookState {
                    mouse_buttons: mouse_keys_used(&shortcuts),
                    matcher: Matcher::new(shortcuts),
                    mouse_poller: MousePoller::default(),
                    mouse_timer: None,
                    capture: None,
                    controls,
                    on_event: Box::new(on_event),
                };
                // Souris : SEULEMENT si un raccourci l'utilise. Le minuteur nous enverra alors
                // un message WM_TIMER toutes les ~15 ms, traité dans la boucle de messages.
                hook_state.update_mouse_timer();
                *state.borrow_mut() = Some(hook_state);
            });

            // SÉCURITÉ : `hook_proc` a exactement la signature attendue par Windows ;
            // `GetModuleHandleW(null())` renvoie le module de notre propre exe.
            let hook = unsafe {
                SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_proc), GetModuleHandleW(null()), 0)
            };
            if hook.is_null() {
                eprintln!("Impossible d'installer le hook clavier : les raccourcis sont inactifs.");
            }

            // SÉCURITÉ : `MSG` est une simple struct C, un contenu tout à zéro est valide.
            let mut msg: MSG = unsafe { std::mem::zeroed() };
            // Windows ne crée la file de messages du thread qu'au premier appel d'une fonction
            // de messages : on la crée maintenant, pour que `PostThreadMessageW` ne se perde pas.
            // SÉCURITÉ : `msg` est une struct valide, prêtée en écriture le temps de l'appel.
            unsafe { PeekMessageW(&mut msg, null_mut(), 0, 0, PM_NOREMOVE) };
            // SÉCURITÉ : simple lecture de l'identifiant du thread courant.
            let _ = id_sender.send(unsafe { GetCurrentThreadId() });

            // Boucle de messages : un hook bas niveau n'est appelé que si son thread « écoute »
            // les messages Windows. `GetMessageW` attend (sans consommer de CPU) le message suivant.
            // SÉCURITÉ : `msg` est une struct valide, qu'on prête en écriture le temps de l'appel.
            while unsafe { GetMessageW(&mut msg, null_mut(), 0, 0) } > 0 {
                match msg.message {
                    WM_TIMER => poll_mouse(),
                    WM_APP => apply_controls(),
                    _ => {}
                }
            }
        });

    if let Err(error) = spawn_result {
        eprintln!("Impossible de démarrer le thread des raccourcis : {error}");
        return None;
    }
    // On attend que le thread donne son identifiant (immédiat en pratique).
    let thread_id = id_receiver.recv().ok()?;
    Some(HotkeyHandle {
        controls: controls_sender,
        thread_id,
    })
}

/// Lit et applique toutes les demandes en attente de l'appli.
fn apply_controls() {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        let Some(state) = state.as_mut() else {
            return;
        };
        // `try_iter` : toutes les demandes déjà arrivées, sans attendre.
        // `collect` d'abord, car `state` est modifié pendant le traitement.
        let controls: Vec<Control> = state.controls.try_iter().collect();
        for control in controls {
            match control {
                Control::SetShortcuts(shortcuts) => {
                    state.mouse_buttons = mouse_keys_used(&shortcuts);
                    state.matcher = Matcher::new(shortcuts);
                }
                Control::StartCapture => state.capture = Some(Capture::default()),
                Control::CancelCapture => state.capture = None,
            }
        }
        state.update_mouse_timer();
    });
}

/// Appelée par Windows à chaque frappe.
///
/// `unsafe extern "system" fn` : une fonction appelable depuis du code C
/// avec la convention d'appel de Windows (« system »).
unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // Windows demande de ne traiter que les `code == HC_ACTION` et de transmettre le reste tel quel.
    if code == HC_ACTION as i32 {
        // SÉCURITÉ : pour WH_KEYBOARD_LL, Windows garantit que `lparam` pointe vers un
        // KBDLLHOOKSTRUCT valide pendant toute la durée de l'appel.
        let info = unsafe { &*(lparam as *const KBDLLHOOKSTRUCT) };
        let extended = info.flags & LLKHF_EXTENDED != 0;
        let key = normalize(info.vkCode as u16, extended);

        // `wparam` indique le type d'événement. Les variantes « SYS » arrivent quand Alt est tenu.
        match wparam as u32 {
            WM_KEYDOWN | WM_SYSKEYDOWN => with_state(|state| key_down(state, key)),
            WM_KEYUP | WM_SYSKEYUP => with_state(|state| key_up(state, key)),
            _ => {} // `_` = « tous les autres cas » : rien à faire
        }
    }

    // On transmet TOUJOURS la frappe au suivant : elle arrive normalement au jeu.
    // SÉCURITÉ : on retransmet exactement les paramètres reçus de Windows.
    unsafe { CallNextHookEx(null_mut(), code, wparam, lparam) }
}

/// Exécute `action` sur l'état du hook, s'il est démarré.
/// `impl FnOnce(&mut HookState)` : une fonction appelée une seule fois, qui reçoit l'état.
fn with_state(action: impl FnOnce(&mut HookState)) {
    STATE.with(|state| {
        // `if let Some(x) = ...` : n'exécute le bloc que si l'Option contient une valeur.
        if let Some(state) = state.borrow_mut().as_mut() {
            action(state);
        }
    });
}

/// Appelée à chaque tic du minuteur souris : un bouton qui vient d'être enfoncé est traité
/// exactement comme une touche du clavier (mêmes règles, combinaisons possibles avec Ctrl…).
fn poll_mouse() {
    with_state(|state| {
        // `to_vec()` : une copie de la liste, car `state` est modifié dans la boucle.
        let buttons = state.watched_buttons().to_vec();
        for change in state.mouse_poller.poll(&buttons, is_key_down) {
            match change {
                ButtonChange::Pressed(button) => key_down(state, button),
                ButtonChange::Released(button) => key_up(state, button),
            }
        }
    });
}

/// Traite l'appui d'une touche (ou d'un bouton de souris).
fn key_down(state: &mut HookState, key: Key) {
    state.matcher.forget_released(is_key_down);
    // Le matcher suit toujours les touches enfoncées, même pendant une capture.
    let commands = state.matcher.key_down(key);
    if let Some(capture) = &mut state.capture {
        // Capture en cours : les raccourcis sont suspendus, la touche sert à la capture.
        let step = capture.key_down(key);
        finish_capture(state, step);
        return;
    }
    for command in commands {
        (state.on_event)(HotkeyEvent::Command(command));
    }
}

/// Traite le relâchement d'une touche (ou d'un bouton de souris).
fn key_up(state: &mut HookState, key: Key) {
    state.matcher.key_up(key);
    if let Some(capture) = &mut state.capture {
        let step = capture.key_up(key);
        finish_capture(state, step);
    }
}

/// Si la capture est terminée (ou annulée), prévient l'appli et réactive les raccourcis.
fn finish_capture(state: &mut HookState, step: CaptureStep) {
    let event = match step {
        CaptureStep::Pending => return,
        CaptureStep::Done { held, trigger } => HotkeyEvent::Captured { held, trigger },
        CaptureStep::Cancelled => HotkeyEvent::CaptureCancelled,
    };
    state.capture = None;
    state.update_mouse_timer();
    (state.on_event)(event);
}

/// Demande à Windows si une touche est physiquement enfoncée.
fn is_key_down(key: Key) -> bool {
    // Notre code maison « Num Entrée » n'existe pas pour Windows : on demande l'état d'Entrée.
    let code = if key == Key::NUMPAD_ENTER {
        Key::ENTER.0
    } else {
        key.0
    };
    // SÉCURITÉ : simple lecture d'état, n'importe quel code de touche est accepté.
    let state = unsafe { GetAsyncKeyState(i32::from(code)) };
    // Le bit de poids fort (0x8000) vaut 1 si la touche est enfoncée.
    (state as u16) & 0x8000 != 0
}

/// Windows distingue Ctrl gauche / droit, Shift gauche / droit, etc.
/// On les ramène à une seule touche « Ctrl », « Shift »… pour que « Ctrl + Num + »
/// marche avec les deux Ctrl.
/// `extended` : indicateur « touche étendue » de Windows, qui distingue Num Entrée d'Entrée.
fn normalize(vk: u16, extended: bool) -> Key {
    match vk {
        0x0D if extended => Key::NUMPAD_ENTER,
        0xA0 | 0xA1 => Key::SHIFT,   // VK_LSHIFT, VK_RSHIFT
        0xA2 | 0xA3 => Key::CONTROL, // VK_LCONTROL, VK_RCONTROL
        0xA4 | 0xA5 => Key::ALT,     // VK_LMENU, VK_RMENU
        other => Key(other),
    }
}
