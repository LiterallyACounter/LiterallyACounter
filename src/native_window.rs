//! Accès direct à la fenêtre du système, quand egui / winit ne suffisent pas.
//!
//! 1) Décorations : sous Windows 11, plusieurs décorations restent visibles autour d'une
//!    fenêtre sans bordure, et sautent aux yeux avec un fond transparent (« effet glace ») :
//!    - un fin LISERÉ et des COINS ARRONDIS, dessinés par le gestionnaire de fenêtres (DWM) ;
//!    - une OMBRE PORTÉE et une LIGNE SOMBRE d'1 pixel en haut. egui demande toujours une ombre
//!      pour les fenêtres sans bordure, et winit l'obtient en décalant la zone de dessin d'1 pixel
//!      (son code le reconnaît : « This leads to a small black 1px border on the top »).
//!
//! 2) Redimensionnement : egui ne sait que « changer la taille » PUIS « déplacer », en deux fois,
//!    et Windows recopie l'ancienne image dans la fenêtre agrandie : un contour fantôme apparaît
//!    un instant. On fait les deux d'un coup, sans recopie.

use raw_window_handle::{HasWindowHandle, RawWindowHandle};

/// La fenêtre du système (sous Windows : son identifiant `HWND`).
pub struct NativeWindow {
    #[cfg(windows)]
    hwnd: isize,
}

impl NativeWindow {
    /// Récupère la fenêtre du système. `None` si ce n'est pas une fenêtre prise en charge
    /// (hors Windows pour l'instant) : on se rabat alors sur les fonctions d'egui.
    ///
    /// `window: &impl HasWindowHandle` : « n'importe quoi qui sait donner l'identifiant de sa
    /// fenêtre » (ici, le contexte de création d'eframe).
    pub fn from_window(window: &impl HasWindowHandle) -> Option<Self> {
        let handle = window.window_handle().ok()?;
        // Un `RawWindowHandle` est un enum : une variante par système (Win32, X11, Wayland, AppKit…).
        match handle.as_raw() {
            #[cfg(windows)]
            RawWindowHandle::Win32(win32) => Some(Self {
                hwnd: win32.hwnd.get(),
            }),
            _ => None,
        }
    }

    /// Retire liseré, coins arrondis, ombre et ligne du haut ajoutés par Windows / winit.
    pub fn remove_decorations(&self) {
        #[cfg(windows)]
        windows::remove_decorations(self.hwnd);
    }

    /// Place et dimensionne la fenêtre EN UNE SEULE FOIS, en pixels physiques.
    pub fn set_rect(&self, left: i32, top: i32, width: i32, height: i32) {
        #[cfg(windows)]
        windows::set_rect(self.hwnd, left, top, width, height);
        // Hors Windows, les paramètres ne servent pas : on le dit au compilateur.
        #[cfg(not(windows))]
        let _ = (left, top, width, height);
    }
}

#[cfg(windows)]
mod windows {
    use std::ffi::c_void;
    use std::ptr::null_mut;

    use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows_sys::Win32::Graphics::Dwm::{
        DWMNCRP_DISABLED, DWMWA_BORDER_COLOR, DWMWA_COLOR_NONE, DWMWA_NCRENDERING_POLICY,
        DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND, DwmSetWindowAttribute,
    };
    use windows_sys::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOCOPYBITS, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER,
        SetWindowPos, WM_NCACTIVATE, WM_NCCALCSIZE, WM_NCPAINT,
    };

    /// Identifiant de notre « sous-classe » (n'importe quel nombre unique pour nous).
    const SUBCLASS_ID: usize = 1;

    /// Messages internes non documentés de Windows, absents de `windows-sys`.
    const WM_NCUAHDRAWCAPTION: u32 = 0x00AE;
    const WM_NCUAHDRAWFRAME: u32 = 0x00AF;

    pub fn remove_decorations(hwnd: isize) {
        let hwnd = hwnd as HWND;

        // 1) Pas de liseré, coins carrés (les coins arrondis sont dessinés par nous, dans le fond).
        set_attribute(hwnd, DWMWA_BORDER_COLOR as u32, DWMWA_COLOR_NONE);
        set_attribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE as u32,
            DWMWCP_DONOTROUND as u32,
        );
        // 2) Pas d'ombre : on interdit au DWM de dessiner autour de la fenêtre.
        set_attribute(
            hwnd,
            DWMWA_NCRENDERING_POLICY as u32,
            DWMNCRP_DISABLED as u32,
        );

        // 3) Pas de ligne en haut : on intercepte le message où Windows demande
        //    « quelle partie de la fenêtre est la zone de dessin ? » (WM_NCCALCSIZE),
        //    pour répondre « toute la fenêtre », avant que winit n'ajoute son décalage.
        //    Une « sous-classe » insère notre fonction devant celle de winit.
        // SÉCURITÉ : `subclass_proc` a la signature attendue, et la fenêtre est valide.
        unsafe {
            SetWindowSubclass(hwnd, Some(subclass_proc), SUBCLASS_ID, 0);
            // Force Windows à recalculer le cadre tout de suite (sans bouger ni redimensionner).
            SetWindowPos(
                hwnd,
                null_mut(),
                0,
                0,
                0,
                0,
                SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
            );
        }
    }

    pub fn set_rect(hwnd: isize, left: i32, top: i32, width: i32, height: i32) {
        // SÉCURITÉ : simple demande de placement d'une fenêtre valide.
        // - SWP_NOZORDER : ne change pas l'ordre des fenêtres (on reste au premier plan) ;
        // - SWP_NOACTIVATE : ne vole pas le focus au jeu ;
        // - SWP_NOCOPYBITS : ne recopie PAS l'ancienne image dans la nouvelle fenêtre.
        unsafe {
            SetWindowPos(
                hwnd as HWND,
                null_mut(),
                left,
                top,
                width,
                height,
                SWP_NOZORDER | SWP_NOACTIVATE | SWP_NOCOPYBITS,
            );
        }
    }

    /// Appelée par Windows pour chaque message destiné à la fenêtre, AVANT winit.
    unsafe extern "system" fn subclass_proc(
        hwnd: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
        _subclass_id: usize,
        _ref_data: usize,
    ) -> LRESULT {
        match message {
            // `wparam != 0` : Windows demande la zone de dessin. Répondre 0 sans rien modifier
            // signifie « la zone de dessin occupe toute la fenêtre » (aucune bordure).
            WM_NCCALCSIZE if wparam != 0 => return 0,
            // winit garde le style « barre de titre » (WS_CAPTION) pour l'ombre. Comme on a
            // désactivé le dessin des bordures par Windows 11, l'ancien moteur de Windows
            // redessinerait une barre de titre classique (avec ✕, □, _) à chaque redimensionnement
            // ou changement de focus. On bloque ces dessins :
            // - WM_NCPAINT : « dessine la bordure » → on ne dessine rien ;
            WM_NCPAINT => return 0,
            // - WM_NCACTIVATE : « la fenêtre (dés)active, redessine la barre de titre ».
            //   On ne peut pas l'ignorer (Windows y met à jour son état interne, nécessaire au
            //   déplacement de la fenêtre) : on le transmet avec `lparam = -1`, la valeur prévue
            //   par Windows pour dire « traite-le, mais sans redessiner la bordure ».
            // SÉCURITÉ : on retransmet le message reçu, seul `lparam` change (valeur documentée).
            WM_NCACTIVATE => return unsafe { DefSubclassProc(hwnd, message, wparam, -1) },
            // - deux messages internes non documentés de Windows, qui dessinent aussi
            //   la barre de titre et le cadre (WM_NCUAHDRAWCAPTION / WM_NCUAHDRAWFRAME).
            WM_NCUAHDRAWCAPTION | WM_NCUAHDRAWFRAME => return 0,
            _ => {}
        }
        // Tous les autres messages : on laisse passer, comme si on n'était pas là.
        // SÉCURITÉ : on retransmet exactement les paramètres reçus.
        unsafe { DefSubclassProc(hwnd, message, wparam, lparam) }
    }

    /// Règle un attribut DWM de type entier. Les erreurs sont ignorées : sur Windows 10,
    /// certains attributs n'existent pas (et il n'y a alors rien à retirer).
    fn set_attribute(hwnd: HWND, attribute: u32, value: u32) {
        // SÉCURITÉ : `value` vit pendant tout l'appel, et on indique sa taille exacte.
        unsafe {
            DwmSetWindowAttribute(
                hwnd,
                attribute,
                &value as *const u32 as *const c_void,
                size_of::<u32>() as u32,
            );
        }
    }
}
