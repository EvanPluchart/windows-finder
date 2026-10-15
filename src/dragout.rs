//! Glisser-deposer avec l'Explorateur Windows : on emporte des fichiers hors de la fenetre (drag OLE natif) ;
//! le depot de fichiers venant de l'Explorateur est gere par `drop_external`.

use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};

use gpui::Context;
use windows::Win32::Foundation::{DRAGDROP_S_CANCEL, DRAGDROP_S_DROP, DRAGDROP_S_USEDEFAULTCURSORS, E_FAIL, S_OK};
use windows::Win32::System::Com::{CoTaskMemFree, IDataObject};
use windows::Win32::System::Ole::{
    DROPEFFECT, DROPEFFECT_COPY, DROPEFFECT_LINK,
    DROPEFFECT_MOVE, DoDragDrop, IDropSource, IDropSource_Impl, OleInitialize,
};
use windows::Win32::System::SystemServices::MODIFIERKEYS_FLAGS;
use windows::Win32::UI::Shell::Common::ITEMIDLIST;
use windows::Win32::UI::Shell::{ILFindLastID, SHCreateDataObject, SHParseDisplayName};
use windows::core::{BOOL, HRESULT, PCWSTR, implement};

#[implement(IDropSource)]
struct Source;

impl IDropSource_Impl for Source_Impl {
    fn QueryContinueDrag(&self, escape: BOOL, keys: MODIFIERKEYS_FLAGS) -> HRESULT {
        if escape.as_bool() {
            DRAGDROP_S_CANCEL
        } else if keys.0 & 0x0001 == 0 {
            // Bouton gauche relache : on depose.
            DRAGDROP_S_DROP
        } else {
            S_OK
        }
    }

    fn GiveFeedback(&self, _effect: DROPEFFECT) -> HRESULT {
        DRAGDROP_S_USEDEFAULTCURSORS
    }
}

/// Lance un glisser-deposer natif des fichiers `paths` ; bloque jusqu'au depot ou a l'annulation (Echap).
/// Retourne l'effet obtenu (copie, deplacement...) ou None si annule.
pub fn start(paths: &[PathBuf]) -> Option<DROPEFFECT> {
    unsafe {
        let _ = OleInitialize(None);
        // Les elements doivent partager un dossier parent : SHCreateDataObject attend (dossier, elements relatifs).
        let Some(parent) = paths.first().and_then(|p| p.parent()).map(|p| p.to_path_buf()) else { return None };
        let w = |p: &Path| -> Vec<u16> { p.as_os_str().encode_wide().chain(std::iter::once(0)).collect() };
        let mut parent_pidl: *mut ITEMIDLIST = std::ptr::null_mut();
        let pw = w(&parent);
        if SHParseDisplayName(PCWSTR(pw.as_ptr()), None, &mut parent_pidl, 0, None).is_err() || parent_pidl.is_null() {
            return None;
        }
        let mut pidls: Vec<*mut ITEMIDLIST> = Vec::new();
        let mut rel: Vec<*const ITEMIDLIST> = Vec::new();
        for p in paths.iter().filter(|p| p.parent() == Some(parent.as_path())) {
            let pw = w(p);
            let mut pidl: *mut ITEMIDLIST = std::ptr::null_mut();
            if SHParseDisplayName(PCWSTR(pw.as_ptr()), None, &mut pidl, 0, None).is_ok() && !pidl.is_null() {
                rel.push(ILFindLastID(pidl) as *const ITEMIDLIST);
                pidls.push(pidl);
            }
        }
        if rel.is_empty() {
            CoTaskMemFree(Some(parent_pidl as *const _));
            return None;
        }
        let data: windows::core::Result<IDataObject> = SHCreateDataObject(Some(parent_pidl as *const ITEMIDLIST), Some(&rel), None);
        let source: IDropSource = Source.into();
        let mut effect = DROPEFFECT(0);
        let result = match &data {
            Ok(d) => Some(DoDragDrop(d, &source, DROPEFFECT(DROPEFFECT_COPY.0 | DROPEFFECT_MOVE.0 | DROPEFFECT_LINK.0), &mut effect)),
            Err(_) => None,
        };
        for p in pidls {
            CoTaskMemFree(Some(p as *const _));
        }
        CoTaskMemFree(Some(parent_pidl as *const _));
        let _ = E_FAIL;
        match result {
            Some(hr) if hr == DRAGDROP_S_DROP => Some(effect),
            _ => None,
        }
    }
}

fn same_drive(a: &Path, b: &Path) -> bool {
    let root = |p: &Path| p.components().next().map(|c| c.as_os_str().to_ascii_lowercase());
    root(a) == root(b)
}

impl crate::Finder {
    /// Fichiers venant de l'Explorateur (ou d'une autre application) deposes sur `target`.
    pub fn drop_external(&mut self, paths: Vec<PathBuf>, target: PathBuf, copy: bool, cx: &mut Context<Self>) {
        if crate::fs::is_virtual(&target) || paths.is_empty() {
            return;
        }
        let other_drive = paths.iter().any(|s| !same_drive(s, &target));
        let mode = if copy || other_drive { crate::transfer::Mode::Copy } else { crate::transfer::Mode::Move };
        self.start_transfer(paths, target, mode, cx);
    }

    /// Fin d'un glisser natif vers l'exterieur : on relit le dossier (les elements ont pu etre deplaces).
    pub fn after_drag_out(&mut self, effect: Option<DROPEFFECT>, cx: &mut Context<Self>) {
        // L'effet retourne vaut souvent "aucun" meme apres un deplacement (le destinataire deplace lui-meme) : on relit toujours.
        let _ = effect;
        self.reload_select(&[]);
        cx.notify();
    }
}
