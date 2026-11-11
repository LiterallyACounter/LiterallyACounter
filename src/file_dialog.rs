//! La fenêtre « Ouvrir » standard de Windows, pour choisir un fichier son à importer.
//!
//! Elle est ouverte dans un THREAD à part : tant qu'elle est affichée, la fonction de Windows
//! ne rend pas la main. Dans le thread de l'interface, les compteurs seraient figés pendant
//! ce temps. Le résultat revient par un channel, et on réveille l'interface.

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};

use eframe::egui;

/// Ouvre la fenêtre « Ouvrir » dans un thread. Le channel reçoit le fichier choisi, ou `None`
/// si l'utilisateur a annulé. `title` et `filter_name` sont les textes traduits affichés.
pub fn pick_sound_file(
    ctx: &egui::Context,
    title: String,
    filter_name: String,
) -> Receiver<Option<PathBuf>> {
    let (sender, receiver) = mpsc::channel();
    let ctx = ctx.clone();
    std::thread::spawn(move || {
        let _ = sender.send(open_dialog(&title, &filter_name));
        // Réveille l'interface pour qu'elle lise le résultat.
        ctx.request_repaint();
    });
    receiver
}

#[cfg(windows)]
fn open_dialog(title: &str, filter_name: &str) -> Option<PathBuf> {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;

    use windows_sys::Win32::UI::Controls::Dialogs::{
        GetOpenFileNameW, OFN_FILEMUSTEXIST, OFN_NOCHANGEDIR, OFN_PATHMUSTEXIST, OPENFILENAMEW,
    };

    // Les textes pour Windows : UTF-16, terminés par un 0.
    let wide = |text: &str| -> Vec<u16> { text.encode_utf16().chain(std::iter::once(0)).collect() };
    let title = wide(title);
    // Le filtre : des paires « nom affiché \0 motifs \0 », et un \0 final en plus.
    let filter: Vec<u16> = format!("{filter_name} (*.wav, *.mp3)\0*.wav;*.mp3\0\0")
        .encode_utf16()
        .collect();
    // La zone où Windows écrit le chemin choisi.
    let mut file = vec![0u16; 1024];

    let mut options = OPENFILENAMEW {
        lStructSize: size_of::<OPENFILENAMEW>() as u32,
        lpstrFilter: filter.as_ptr(),
        lpstrFile: file.as_mut_ptr(),
        nMaxFile: file.len() as u32,
        lpstrTitle: title.as_ptr(),
        // Le fichier doit exister ; ne change pas le dossier courant de l'appli.
        Flags: OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR,
        // `..zeroed()` : tous les autres champs à zéro (valeurs par défaut de Windows).
        // SÉCURITÉ : OPENFILENAMEW est une simple struct C, un contenu à zéro est valide.
        ..unsafe { std::mem::zeroed() }
    };
    // SÉCURITÉ : toutes les zones pointées (`filter`, `file`, `title`) vivent jusqu'à la fin
    // de la fonction, et leurs tailles sont exactes.
    if unsafe { GetOpenFileNameW(&mut options) } == 0 {
        return None; // annulé (ou erreur)
    }
    // Le chemin s'arrête au premier 0.
    let length = file.iter().position(|&c| c == 0).unwrap_or(file.len());
    Some(PathBuf::from(OsString::from_wide(&file[..length])))
}

#[cfg(not(windows))]
fn open_dialog(_title: &str, _filter_name: &str) -> Option<PathBuf> {
    None
}
