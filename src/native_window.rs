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

    /// Cherche une fenêtre de NOTRE programme d'après son titre. Sert pour les fenêtres
    /// supplémentaires (« viewports ») d'egui, dont eframe ne fournit pas l'identifiant.
    /// Le titre doit donc être unique dans l'appli.
    pub fn find_by_title(title: &str) -> Option<Self> {
        #[cfg(windows)]
        return windows::find_own_window(title).map(|hwnd| Self { hwnd });
        #[cfg(not(windows))]
        {
            let _ = title;
            None
        }
    }

    /// Cache la fenêtre de la barre des tâches ET d'Alt + Tab (fenêtre « outil »).
    pub fn hide_from_task_switcher(&self) {
        #[cfg(windows)]
        windows::hide_from_task_switcher(self.hwnd);
    }

    /// Exclut la fenêtre de toutes les captures (OBS, Discord, captures d'écran) : elle reste
    /// visible à l'écran, mais apparaît vide (ou absente) dans les captures.
    /// Windows 10 2004 et plus récent ; sans effet sur les versions plus anciennes.
    pub fn exclude_from_capture(&self) {
        #[cfg(windows)]
        windows::exclude_from_capture(self.hwnd);
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
        FindWindowExW, GWL_EXSTYLE, GWL_STYLE, GetWindowLongW, GetWindowThreadProcessId,
        STYLESTRUCT, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOCOPYBITS, SWP_NOMOVE, SWP_NOSIZE,
        SWP_NOZORDER, SetWindowDisplayAffinity, SetWindowLongW, SetWindowPos,
        WDA_EXCLUDEFROMCAPTURE, WM_NCACTIVATE, WM_NCCALCSIZE, WM_NCPAINT, WM_STYLECHANGING,
        WS_CAPTION, WS_EX_APPWINDOW, WS_EX_TOOLWINDOW, WS_POPUP, WS_THICKFRAME,
    };

    /// Identifiant de notre « sous-classe » (n'importe quel nombre unique pour nous).
    const SUBCLASS_ID: usize = 1;
    /// Identifiant de la sous-classe qui garde la fenêtre hôte hors d'Alt + Tab.
    const TOOL_WINDOW_SUBCLASS_ID: usize = 2;

    /// Messages internes non documentés de Windows, absents de `windows-sys`.
    const WM_NCUAHDRAWCAPTION: u32 = 0x00AE;
    const WM_NCUAHDRAWFRAME: u32 = 0x00AF;

    pub fn find_own_window(title: &str) -> Option<isize> {
        // Windows attend un texte UTF-16 terminé par un 0.
        // `encode_utf16` : les caractères en UTF-16 ; `chain(once(0))` : on ajoute le 0 final.
        let title: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
        let mut previous: HWND = null_mut();
        // `loop` : boucle infinie, dont on sort avec `return`.
        loop {
            // Fenêtre suivante portant ce titre, après `previous` (toutes applis confondues).
            // SÉCURITÉ : `title` est un texte UTF-16 valide terminé par 0, vivant pendant l'appel.
            let hwnd = unsafe { FindWindowExW(null_mut(), previous, null_mut(), title.as_ptr()) };
            if hwnd.is_null() {
                return None;
            }
            // On vérifie qu'elle appartient bien à NOTRE programme (une autre instance de l'appli
            // ouverte en même temps aurait les mêmes titres).
            let mut process_id = 0;
            // SÉCURITÉ : `hwnd` vient d'être renvoyé par Windows ; `process_id` est modifiable.
            unsafe { GetWindowThreadProcessId(hwnd, &mut process_id) };
            if process_id == std::process::id() {
                return Some(hwnd as isize);
            }
            previous = hwnd;
        }
    }

    pub fn exclude_from_capture(hwnd: isize) {
        // SÉCURITÉ : simple réglage d'une fenêtre valide de notre programme.
        unsafe { SetWindowDisplayAffinity(hwnd as HWND, WDA_EXCLUDEFROMCAPTURE) };
    }

    pub fn hide_from_task_switcher(hwnd: isize) {
        let hwnd = hwnd as HWND;
        // winit réécrit le style étendu après la création de la fenêtre (« fenêtre d'appli »,
        // visible dans Alt + Tab). Comme pour la barre de titre, une sous-classe corrige chaque
        // changement de style avant qu'il ne s'applique (voir `tool_window_proc`).
        // SÉCURITÉ : `tool_window_proc` a la signature attendue ; lecture puis écriture du style
        // étendu d'une fenêtre valide.
        unsafe {
            SetWindowSubclass(hwnd, Some(tool_window_proc), TOOL_WINDOW_SUBCLASS_ID, 0);
            let style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
            SetWindowLongW(hwnd, GWL_EXSTYLE, as_tool_window(style) as i32);
        }
    }

    /// Sous-classe de la fenêtre hôte : garde le style « fenêtre outil ».
    unsafe extern "system" fn tool_window_proc(
        hwnd: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
        _subclass_id: usize,
        _ref_data: usize,
    ) -> LRESULT {
        if message == WM_STYLECHANGING && wparam as i32 == GWL_EXSTYLE {
            // SÉCURITÉ : pour ce message, Windows garantit que `lparam` pointe vers un
            // STYLESTRUCT valide et modifiable pendant l'appel.
            let change = unsafe { &mut *(lparam as *mut STYLESTRUCT) };
            change.styleNew = as_tool_window(change.styleNew);
        }
        // SÉCURITÉ : on retransmet exactement les paramètres reçus.
        unsafe { DefSubclassProc(hwnd, message, wparam, lparam) }
    }

    /// « Fenêtre outil » : absente de la barre des tâches et d'Alt + Tab.
    fn as_tool_window(style: u32) -> u32 {
        (style | WS_EX_TOOLWINDOW) & !WS_EX_APPWINDOW
    }

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
        // 4) Pas de barre de titre, jamais : winit donne à la fenêtre le STYLE « barre de titre »
        //    (WS_CAPTION, pour son ombre). Tant que ce style existe, Windows peut dessiner la barre
        //    (titre, ✕…) un instant, par ex. au premier affichage. On retire le style lui-même.
        // SÉCURITÉ : `subclass_proc` a la signature attendue, et la fenêtre est valide.
        unsafe {
            SetWindowSubclass(hwnd, Some(subclass_proc), SUBCLASS_ID, 0);
            let style = GetWindowLongW(hwnd, GWL_STYLE) as u32;
            SetWindowLongW(hwnd, GWL_STYLE, without_frame(style) as i32);
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
            // winit pourrait REMETTRE le style « barre de titre » lors d'un changement d'état de
            // la fenêtre. Windows nous prévient avant tout changement de style (WM_STYLECHANGING) :
            // on retire alors le cadre du nouveau style, avant qu'il ne s'applique.
            WM_STYLECHANGING if wparam as i32 == GWL_STYLE => {
                // SÉCURITÉ : pour ce message, Windows garantit que `lparam` pointe vers un
                // STYLESTRUCT valide et modifiable pendant l'appel.
                let change = unsafe { &mut *(lparam as *mut STYLESTRUCT) };
                change.styleNew = without_frame(change.styleNew);
            }
            _ => {}
        }
        // Tous les autres messages : on laisse passer, comme si on n'était pas là.
        // SÉCURITÉ : on retransmet exactement les paramètres reçus.
        unsafe { DefSubclassProc(hwnd, message, wparam, lparam) }
    }

    /// Retire d'un style de fenêtre la barre de titre et le cadre redimensionnable,
    /// et en fait une fenêtre « popup » (sans cadre). Un style est un ensemble de bits :
    /// `& !X` met les bits de X à 0, `| Y` met ceux de Y à 1.
    fn without_frame(style: u32) -> u32 {
        (style & !(WS_CAPTION | WS_THICKFRAME)) | WS_POPUP
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
