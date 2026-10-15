//! Presse-papiers Windows pour les fichiers (CF_HDROP + "Preferred DropEffect") : copier/couper dans le Finder
//! et coller dans l'Explorateur, et inversement.

use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::PathBuf;

use windows::Win32::Foundation::{GlobalFree, HANDLE, HGLOBAL, HWND};
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard, RegisterClipboardFormatW,
    SetClipboardData,
};
use windows::Win32::System::Memory::{GMEM_MOVEABLE, GMEM_ZEROINIT, GlobalAlloc, GlobalLock, GlobalUnlock};
use windows::Win32::UI::Shell::{DragQueryFileW, HDROP};
use windows::core::w;

const CF_HDROP: u32 = 15;
const DROPEFFECT_COPY: u32 = 1;
const DROPEFFECT_MOVE: u32 = 2;

/// Ouvre le presse-papiers (quelques essais : un autre programme peut le tenir une fraction de seconde).
fn open() -> bool {
    for _ in 0..10 {
        if unsafe { OpenClipboard(Some(HWND::default())) }.is_ok() {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(15));
    }
    false
}

unsafe fn put(format: u32, bytes: &[u8]) -> bool {
    unsafe {
        let Ok(mem) = GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, bytes.len()) else { return false };
        let ptr = GlobalLock(mem) as *mut u8;
        if ptr.is_null() {
            let _ = GlobalFree(Some(mem));
            return false;
        }
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len());
        let _ = GlobalUnlock(mem);
        if SetClipboardData(format, Some(HANDLE(mem.0))).is_err() {
            let _ = GlobalFree(Some(mem));
            return false;
        }
        true
    }
}

/// Place des fichiers dans le presse-papiers Windows (`cut` = deplacer au collage).
pub fn set_files(paths: &[PathBuf], cut: bool) {
    // DROPFILES : pFiles = 20, pt = (0, 0), fNC = 0, fWide = 1, puis les chemins UTF-16 separes par NUL, double NUL final.
    let mut bytes: Vec<u8> = Vec::new();
    for v in [20u32, 0, 0, 0, 1] {
        bytes.extend_from_slice(&v.to_le_bytes());
    }
    for p in paths {
        for u in p.as_os_str().encode_wide().chain(std::iter::once(0)) {
            bytes.extend_from_slice(&u.to_le_bytes());
        }
    }
    bytes.extend_from_slice(&0u16.to_le_bytes());
    if !open() {
        return;
    }
    unsafe {
        let _ = EmptyClipboard();
        if put(CF_HDROP, &bytes) {
            let effect = RegisterClipboardFormatW(w!("Preferred DropEffect"));
            let e = if cut { DROPEFFECT_MOVE } else { DROPEFFECT_COPY };
            put(effect, &e.to_le_bytes());
        }
        let _ = CloseClipboard();
    }
}

/// Fichiers du presse-papiers Windows, et `true` s'ils ont ete coupes.
pub fn get_files() -> Option<(Vec<PathBuf>, bool)> {
    unsafe {
        if IsClipboardFormatAvailable(CF_HDROP).is_err() || !open() {
            return None;
        }
        let mut out = Vec::new();
        let mut cut = false;
        if let Ok(h) = GetClipboardData(CF_HDROP) {
            let drop = HDROP(h.0);
            let count = DragQueryFileW(drop, 0xFFFF_FFFF, None);
            for i in 0..count {
                let len = DragQueryFileW(drop, i, None) as usize;
                let mut buf = vec![0u16; len + 1];
                let n = DragQueryFileW(drop, i, Some(&mut buf)) as usize;
                out.push(PathBuf::from(std::ffi::OsString::from_wide(&buf[..n])));
            }
        }
        let effect = RegisterClipboardFormatW(w!("Preferred DropEffect"));
        if let Ok(h) = GetClipboardData(effect) {
            let mem = HGLOBAL(h.0);
            let p = GlobalLock(mem) as *const u32;
            if !p.is_null() {
                cut = *p & DROPEFFECT_MOVE != 0;
                let _ = GlobalUnlock(mem);
            }
        }
        let _ = CloseClipboard();
        if out.is_empty() { None } else { Some((out, cut)) }
    }
}

/// Vide le presse-papiers (apres un couper-coller, comme l'Explorateur).
pub fn clear() {
    if open() {
        unsafe {
            let _ = EmptyClipboard();
            let _ = CloseClipboard();
        }
    }
}
