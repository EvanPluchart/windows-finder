//! Operations sur les fichiers : noms uniques, copie recursive, deplacement, corbeille Windows
//! (reversible), creation de dossier, renommage. Aucune suppression definitive n'est proposee.

use std::io;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};

use windows::Win32::UI::Shell::{
    FO_DELETE, FOF_ALLOWUNDO, FOF_NOCONFIRMATION, FOF_NOERRORUI, FOF_SILENT, SHFILEOPSTRUCTW, SHFileOperationW,
};
use windows::core::PCWSTR;

/// "nom", puis "nom 2", "nom 3"... (l'extension reste a la fin pour les fichiers).
pub fn unique_path(dir: &Path, name: &str) -> PathBuf {
    let first = dir.join(name);
    if !first.exists() {
        return first;
    }
    let (stem, ext) = split_name(name, dir.join(name).is_dir());
    for n in 2.. {
        let candidate = dir.join(format!("{stem} {n}{ext}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    unreachable!()
}

/// Separe "rapport.final.txt" en ("rapport.final", ".txt") ; les dossiers n'ont pas d'extension.
pub fn split_name(name: &str, is_dir: bool) -> (String, String) {
    if is_dir {
        return (name.to_string(), String::new());
    }
    match name.rfind('.') {
        Some(i) if i > 0 => (name[..i].to_string(), name[i..].to_string()),
        _ => (name.to_string(), String::new()),
    }
}

/// Nom d'une copie dans le meme dossier : "x copie.txt", "x copie 2.txt"...
pub fn copy_name(name: &str, is_dir: bool) -> String {
    let (stem, ext) = split_name(name, is_dir);
    format!("{stem} copie{ext}")
}

pub fn copy_recursive(src: &Path, dst: &Path) -> io::Result<()> {
    if src.is_dir() {
        // Copier un dossier dans lui-meme bouclerait a l'infini.
        if dst.starts_with(src) {
            return Err(io::Error::other("Impossible de copier un dossier dans lui-m\u{ea}me"));
        }
        std::fs::create_dir_all(dst)?;
        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            copy_recursive(&entry.path(), &dst.join(entry.file_name()))?;
        }
        Ok(())
    } else {
        std::fs::copy(src, dst).map(|_| ())
    }
}

/// Deplace : renommage direct, sinon (autre volume) copie puis suppression de la source.
pub fn move_path(src: &Path, dst: &Path) -> io::Result<()> {
    if dst.starts_with(src) && src.is_dir() {
        return Err(io::Error::other("Impossible de d\u{e9}placer un dossier dans lui-m\u{ea}me"));
    }
    match std::fs::rename(src, dst) {
        Ok(()) => Ok(()),
        Err(_) => {
            copy_recursive(src, dst)?;
            if src.is_dir() { std::fs::remove_dir_all(src) } else { std::fs::remove_file(src) }
        }
    }
}

/// Envoie les elements a la corbeille Windows (restaurables), sans confirmation ni fenetre.
pub fn trash(paths: &[PathBuf]) -> Result<(), String> {
    if paths.is_empty() {
        return Ok(());
    }
    let mut from: Vec<u16> = Vec::new();
    for p in paths {
        from.extend(p.as_os_str().encode_wide());
        from.push(0);
    }
    from.push(0); // double terminaison nulle exigee par l'API
    let mut op = SHFILEOPSTRUCTW {
        wFunc: FO_DELETE,
        pFrom: PCWSTR(from.as_ptr()),
        fFlags: (FOF_ALLOWUNDO | FOF_NOCONFIRMATION | FOF_SILENT | FOF_NOERRORUI).0 as _,
        ..Default::default()
    };
    let code = unsafe { SHFileOperationW(&mut op) };
    if code != 0 || op.fAnyOperationsAborted.as_bool() {
        return Err(format!("La corbeille a refus\u{e9} l'op\u{e9}ration (code {code})"));
    }
    Ok(())
}

pub fn create_folder(dir: &Path) -> io::Result<PathBuf> {
    let path = unique_path(dir, "dossier sans titre");
    std::fs::create_dir(&path)?;
    Ok(path)
}

/// Verifie qu'un nom est valide sous Windows.
pub fn validate_name(name: &str) -> Result<(), String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Le nom ne peut pas \u{ea}tre vide".into());
    }
    if let Some(c) = name.chars().find(|c| "<>:\"/\\|?*".contains(*c) || c.is_control()) {
        return Err(format!("Le caract\u{e8}re \u{ab} {c} \u{bb} n'est pas autoris\u{e9} dans un nom"));
    }
    if name.ends_with('.') || name.ends_with(' ') {
        return Err("Le nom ne peut pas finir par un point ou une espace".into());
    }
    Ok(())
}

pub fn rename(path: &Path, new_name: &str) -> Result<PathBuf, String> {
    validate_name(new_name)?;
    let new_name = new_name.trim();
    let target = path.with_file_name(new_name);
    if target == path {
        return Ok(target);
    }
    // Sous Windows la casse seule peut changer ("a" -> "A") : exists() serait vrai, on l'autorise.
    let only_case = target.to_string_lossy().to_lowercase() == path.to_string_lossy().to_lowercase();
    if target.exists() && !only_case {
        return Err(format!("\u{ab} {new_name} \u{bb} existe d\u{e9}j\u{e0} dans ce dossier"));
    }
    std::fs::rename(path, &target).map_err(|e| format!("Renommage impossible : {e}"))?;
    Ok(target)
}
