//! Branchement Windows : un « hook clavier bas niveau » (WH_KEYBOARD_LL).
//!
//! Principe :
//! - Windows appelle notre fonction `hook_proc` à CHAQUE frappe, quelle que soit l'appli au premier plan.
//! - On regarde la touche, on prévient l'appli si un raccourci correspond, puis on PASSE LA MAIN
//!   (`CallNextHookEx`) : la touche continue son chemin jusqu'au jeu, rien n'est bloqué (contrainte C3).
//! - Ce traitement doit être ULTRA RAPIDE : Windows désactive en silence un hook trop lent,
//!   et un hook lent ferait ramer tout le système (contrainte C4).
//!
//! Le hook vit dans un THREAD dédié, qui tourne en parallèle de l'interface.
//!
//! Ce fichier contient du code `unsafe` : des appels directs à l'API Windows (écrite en C).
//! Le compilateur Rust ne peut pas vérifier ce que fait du code C, donc on lui dit
//! « je garantis moi-même que c'est correct » avec un bloc `unsafe { ... }`.
//! On garde ces blocs aussi petits que possible, chacun avec un commentaire `SÉCURITÉ :`.

use std::cell::RefCell;
use std::ptr::{null, null_mut};

use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetMessageW, HC_ACTION, KBDLLHOOKSTRUCT, LLKHF_EXTENDED, MSG,
    SetWindowsHookExW, WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

use super::matcher::Matcher;
use super::{Key, Shortcut};
use crate::command::Command;

/// L'état dont `hook_proc` a besoin.
struct HookState {
    matcher: Matcher,
    /// La fonction à appeler pour chaque commande déclenchée.
    /// `Box<dyn Fn(...)>` : « n'importe quelle fonction de cette forme », rangée dans le tas.
    on_command: Box<dyn Fn(Command)>,
}

// Problème : Windows appelle `hook_proc` sans nous laisser lui passer de paramètre.
// Où ranger le matcher ? Dans une variable propre au thread du hook (`thread_local!`).
// `RefCell` permet de la modifier même à travers une référence partagée, avec une vérification
// à l'exécution (et non à la compilation) qu'on ne la modifie pas à deux endroits à la fois.
// `Option` : `None` tant que le hook n'est pas démarré.
thread_local! {
    static STATE: RefCell<Option<HookState>> = const { RefCell::new(None) };
}

/// Démarre l'écoute du clavier dans un thread dédié.
///
/// `on_command` est appelée pour chaque commande déclenchée, DEPUIS LE THREAD DU HOOK.
/// Elle doit donc être rapide et pouvoir être envoyée à un autre thread :
/// - `Send` : « peut être transférée à un autre thread en toute sécurité » ;
/// - `'static` : « ne contient pas de référence vers une donnée qui pourrait disparaître avant elle ».
///   Le compilateur vérifie ces deux garanties : c'est ce qui rend les threads sûrs en Rust.
pub fn start(shortcuts: Vec<Shortcut>, on_command: impl Fn(Command) + Send + 'static) {
    // `move` : la closure PREND POSSESSION de `shortcuts` et `on_command` (elle les emporte dans le thread).
    let spawn_result = std::thread::Builder::new()
        .name("hotkeys".into())
        .spawn(move || {
            STATE.with(|state| {
                *state.borrow_mut() = Some(HookState {
                    matcher: Matcher::new(shortcuts),
                    on_command: Box::new(on_command),
                });
            });

            // SÉCURITÉ : `hook_proc` a exactement la signature attendue par Windows ;
            // `GetModuleHandleW(null())` renvoie le module de notre propre exe.
            let hook = unsafe {
                SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_proc), GetModuleHandleW(null()), 0)
            };
            if hook.is_null() {
                eprintln!("Impossible d'installer le hook clavier : les raccourcis sont inactifs.");
                return;
            }

            // Boucle de messages : un hook bas niveau n'est appelé que si son thread « écoute »
            // les messages Windows. `GetMessageW` attend (sans consommer de CPU) le message suivant.
            // SÉCURITÉ : `MSG` est une simple struct C, un contenu tout à zéro est valide.
            let mut msg: MSG = unsafe { std::mem::zeroed() };
            // SÉCURITÉ : `msg` est une struct valide, qu'on prête en écriture le temps de l'appel.
            while unsafe { GetMessageW(&mut msg, null_mut(), 0, 0) } > 0 {}
        });

    if let Err(error) = spawn_result {
        eprintln!("Impossible de démarrer le thread des raccourcis : {error}");
    }
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
            WM_KEYDOWN | WM_SYSKEYDOWN => on_key_down(key),
            WM_KEYUP | WM_SYSKEYUP => on_key_up(key),
            _ => {} // `_` = « tous les autres cas » : rien à faire
        }
    }

    // On transmet TOUJOURS la frappe au suivant : elle arrive normalement au jeu.
    // SÉCURITÉ : on retransmet exactement les paramètres reçus de Windows.
    unsafe { CallNextHookEx(null_mut(), code, wparam, lparam) }
}

fn on_key_down(key: Key) {
    STATE.with(|state| {
        // `if let Some(x) = ...` : n'exécute le bloc que si l'Option contient une valeur.
        if let Some(state) = state.borrow_mut().as_mut() {
            state.matcher.forget_released(is_key_down);
            for command in state.matcher.key_down(key) {
                (state.on_command)(command);
            }
        }
    });
}

fn on_key_up(key: Key) {
    STATE.with(|state| {
        if let Some(state) = state.borrow_mut().as_mut() {
            state.matcher.key_up(key);
        }
    });
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
