//! « Plus d'options… » : le menu contextuel natif de l'Explorateur Windows (IContextMenu) pour la selection.
//! Donne acces a toutes les extensions du shell (Envoyer vers, 7-Zip, Git, Proprietes, Restaurer les versions...).
//! Le menu tourne dans un thread dedie (fenetre cachee + boucle de messages) pour ne pas bloquer ni perturber l'interface.

use std::cell::RefCell;
use std::os::windows::ffi::OsStrExt;
use std::path::PathBuf;

use gpui::{Context, Window};
use windows::core::{Interface, PCSTR, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Shell::{
    BHID_SFUIObject, IContextMenu, IContextMenu2, IContextMenu3, IShellItemArray, SHCreateShellItemArrayFromIDLists,
    SHParseDisplayName, CMF_CANRENAME, CMF_NORMAL, CMINVOKECOMMANDINFOEX,
};
use windows::Win32::UI::Shell::Common::ITEMIDLIST;
use windows::Win32::UI::WindowsAndMessaging::{
    CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu, DestroyWindow, GetCursorPos, RegisterClassW,
    SetForegroundWindow, TrackPopupMenuEx, HMENU, SW_SHOWNORMAL, TPM_RETURNCMD, TPM_RIGHTBUTTON, WM_DRAWITEM,
    WM_INITMENUPOPUP, WM_MEASUREITEM, WM_MENUCHAR, WNDCLASSW, WS_POPUP,
};

use crate::Finder;

thread_local! {
    pub static MENU3: RefCell<Option<IContextMenu3>> = const { RefCell::new(None) };
    pub static MENU2: RefCell<Option<IContextMenu2>> = const { RefCell::new(None) };
}

pub unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if matches!(msg, WM_INITMENUPOPUP | WM_DRAWITEM | WM_MEASUREITEM | WM_MENUCHAR) {
        if let Some(m3) = MENU3.with(|m| m.borrow().clone()) {
            let mut result = LRESULT(0);
            if unsafe { m3.HandleMenuMsg2(msg, wp, lp, Some(&mut result)) }.is_ok() {
                return result;
            }
        } else if let Some(m2) = MENU2.with(|m| m.borrow().clone()) {
            if unsafe { m2.HandleMenuMsg(msg, wp, lp) }.is_ok() {
                return LRESULT(0);
            }
        }
    }
    unsafe { DefWindowProcW(hwnd, msg, wp, lp) }
}

/// Affiche le menu natif pour `paths` (a appeler hors du thread d'interface) ; renvoie vrai si une commande a ete lancee.
fn run_menu(paths: Vec<PathBuf>) -> bool {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let done = run_menu_inner(&paths);
        CoUninitialize();
        done
    }
}

pub unsafe fn context_menu(paths: &[PathBuf]) -> Option<IContextMenu> {
    let mut pidls: Vec<*mut ITEMIDLIST> = Vec::new();
    for p in paths {
        let norm = PathBuf::from(p.to_string_lossy().replace('/', "\\"));
        let wide: Vec<u16> = norm.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
        let mut pidl: *mut ITEMIDLIST = std::ptr::null_mut();
        if unsafe { SHParseDisplayName(PCWSTR(wide.as_ptr()), None, &mut pidl, 0, None) }.is_ok() && !pidl.is_null() {
            pidls.push(pidl);
        }
    }
    if pidls.is_empty() {
        return None;
    }
    let consts: Vec<*const ITEMIDLIST> = pidls.iter().map(|p| *p as *const ITEMIDLIST).collect();
    let array = unsafe { SHCreateShellItemArrayFromIDLists(&consts) };
    // Le tableau copie les listes d'identifiants : on libere les notres dans tous les cas.
    for p in pidls {
        unsafe { windows::Win32::System::Com::CoTaskMemFree(Some(p as *const _)) };
    }
    let array: IShellItemArray = array.ok()?;
    unsafe { array.BindToHandler(None, &BHID_SFUIObject) }.ok()
}

unsafe fn run_menu_inner(paths: &[PathBuf]) -> bool {
    let Some(menu) = (unsafe { context_menu(paths) }) else { return false };
    MENU3.with(|m| *m.borrow_mut() = menu.cast::<IContextMenu3>().ok());
    MENU2.with(|m| *m.borrow_mut() = menu.cast::<IContextMenu2>().ok());

    let class = windows::core::w!("WindowsFinderShellMenu");
    let hinst = unsafe { GetModuleHandleW(None) }.map(|h| h.into()).unwrap_or_default();
    let wc = WNDCLASSW { lpfnWndProc: Some(wndproc), hInstance: hinst, lpszClassName: class, ..Default::default() };
    unsafe { RegisterClassW(&wc) };
    let hwnd = unsafe { CreateWindowExW(Default::default(), class, windows::core::w!(""), WS_POPUP, 0, 0, 0, 0, None, None, Some(hinst), None) }
        .unwrap_or_default();
    let hmenu: HMENU = match unsafe { CreatePopupMenu() } {
        Ok(h) => h,
        Err(_) => return false,
    };
    let mut ran = false;
    if unsafe { menu.QueryContextMenu(hmenu, 0, 1, 0x7FFF, CMF_NORMAL | CMF_CANRENAME) }.is_ok() {
        let mut pt = POINT::default();
        let _ = unsafe { GetCursorPos(&mut pt) };
        let _ = unsafe { SetForegroundWindow(hwnd) };
        let cmd = unsafe { TrackPopupMenuEx(hmenu, (TPM_RETURNCMD | TPM_RIGHTBUTTON).0, pt.x, pt.y, hwnd, None) }.0;
        if cmd > 0 {
            let info = CMINVOKECOMMANDINFOEX {
                cbSize: size_of::<CMINVOKECOMMANDINFOEX>() as u32,
                hwnd,
                lpVerb: PCSTR((cmd as usize - 1) as *const u8),
                nShow: SW_SHOWNORMAL.0,
                ptInvoke: pt,
                ..Default::default()
            };
            ran = unsafe { menu.InvokeCommand(&info as *const _ as *const _) }.is_ok();
        }
    }
    let _ = unsafe { DestroyMenu(hmenu) };
    if hwnd.0 != std::ptr::null_mut() {
        let _ = unsafe { DestroyWindow(hwnd) };
    }
    MENU3.with(|m| *m.borrow_mut() = None);
    MENU2.with(|m| *m.borrow_mut() = None);
    ran
}

impl Finder {
    /// Menu contextuel natif de Windows pour la selection (ou le dossier courant).
    pub fn native_menu(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let mut paths = self.selected_paths();
        if paths.is_empty() {
            if crate::fs::is_virtual(&self.cwd) {
                return;
            }
            paths.push(self.cwd.clone());
        }
        let keep = self.selected_paths();
        cx.spawn(async move |this, cx| {
            let ran = cx.background_executor().spawn(async move { run_menu(paths) }).await;
            this.update(cx, |f, cx| {
                if ran {
                    f.reload_select(&keep);
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
}
