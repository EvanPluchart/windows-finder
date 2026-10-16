//! Sous-menu « Plus d'options » : les entrees du menu natif de Windows (IContextMenu), redessinees dans le menu du Finder.
//! Un thread dedie (apartment STA) liste les entrees, puis garde le menu vivant pour lancer la commande choisie.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::time::{Duration, Instant};

use gpui::Context;
use windows::core::{Interface, PCSTR, PWSTR};
use windows::Win32::Foundation::{LPARAM, POINT, WPARAM};
use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Shell::{IContextMenu, IContextMenu2, IContextMenu3, CMF_CANRENAME, CMF_NORMAL, CMINVOKECOMMANDINFOEX};
use windows::Win32::UI::WindowsAndMessaging::{
    CreatePopupMenu, CreateWindowExW, DestroyMenu, DestroyWindow, DispatchMessageW, GetCursorPos, GetMenuItemCount,
    GetMenuItemInfoW, PeekMessageW, RegisterClassW, TranslateMessage, HMENU, MENUITEMINFOW, MFS_DISABLED, MFS_GRAYED,
    MFT_SEPARATOR, MIIM_FTYPE, MIIM_ID, MIIM_STATE, MIIM_STRING, MIIM_SUBMENU, MSG, PM_REMOVE, SW_SHOWNORMAL,
    WM_INITMENUPOPUP, WNDCLASSW, WS_POPUP,
};

use crate::shellmenu::{context_menu, wndproc, MENU2, MENU3};
use crate::Finder;

#[derive(Clone)]
pub struct ShellItem {
    pub id: u32,
    pub label: String,
    pub enabled: bool,
    pub sep: bool,
    pub children: Vec<ShellItem>,
}

pub struct ShellMore {
    token: u64,
    tx: Sender<u32>,
    done: Option<Receiver<bool>>,
    /// `None` tant que la liste se charge.
    pub items: Option<Vec<ShellItem>>,
    /// Indices des sous-menus ouverts (un par volet supplementaire).
    pub path: Vec<usize>,
}

static TOKEN: AtomicU64 = AtomicU64::new(1);

fn clean_label(raw: &str) -> String {
    let head = raw.split('\t').next().unwrap_or("");
    let mut out = String::new();
    let mut it = head.chars().peekable();
    while let Some(c) = it.next() {
        if c == '&' {
            if it.peek() == Some(&'&') {
                out.push('&');
                it.next();
            }
        } else {
            out.push(c);
        }
    }
    out.trim().to_string()
}

unsafe fn read_menu(menu: &IContextMenu, hmenu: HMENU, depth: u32) -> Vec<ShellItem> {
    let mut out: Vec<ShellItem> = Vec::new();
    let n = unsafe { GetMenuItemCount(Some(hmenu)) };
    for i in 0..n.max(0) {
        let mut mii = MENUITEMINFOW { cbSize: size_of::<MENUITEMINFOW>() as u32, fMask: MIIM_FTYPE | MIIM_STATE | MIIM_ID | MIIM_SUBMENU, ..Default::default() };
        if unsafe { GetMenuItemInfoW(hmenu, i as u32, true, &mut mii) }.is_err() {
            continue;
        }
        if mii.fType.0 & MFT_SEPARATOR.0 != 0 {
            if out.last().map(|l| !l.sep).unwrap_or(false) {
                out.push(ShellItem { id: 0, label: String::new(), enabled: false, sep: true, children: Vec::new() });
            }
            continue;
        }
        // Libelle : un premier appel donne la longueur, le second remplit le tampon.
        let mut s = MENUITEMINFOW { cbSize: size_of::<MENUITEMINFOW>() as u32, fMask: MIIM_STRING, ..Default::default() };
        let _ = unsafe { GetMenuItemInfoW(hmenu, i as u32, true, &mut s) };
        let mut buf = vec![0u16; s.cch as usize + 2];
        s.dwTypeData = PWSTR(buf.as_mut_ptr());
        s.cch += 1;
        let _ = unsafe { GetMenuItemInfoW(hmenu, i as u32, true, &mut s) };
        let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        let label = clean_label(&String::from_utf16_lossy(&buf[..len]));
        if label.is_empty() {
            continue;
        }
        let enabled = mii.fState.0 & (MFS_DISABLED.0 | MFS_GRAYED.0) == 0;
        let mut children = Vec::new();
        if !mii.hSubMenu.is_invalid() && depth < 2 {
            if let Ok(m2) = menu.cast::<IContextMenu2>() {
                let _ = unsafe { m2.HandleMenuMsg(WM_INITMENUPOPUP, WPARAM(mii.hSubMenu.0 as usize), LPARAM(i as isize)) };
            }
            children = unsafe { read_menu(menu, mii.hSubMenu, depth + 1) };
            if children.is_empty() {
                continue;
            }
        }
        out.push(ShellItem { id: mii.wID, label, enabled, sep: false, children });
    }
    while out.last().map(|l| l.sep).unwrap_or(false) {
        out.pop();
    }
    out
}

fn pump_for(ms: u64) {
    let end = Instant::now() + Duration::from_millis(ms);
    while Instant::now() < end {
        let mut msg = MSG::default();
        while unsafe { PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE) }.as_bool() {
            let _ = unsafe { TranslateMessage(&msg) };
            unsafe { DispatchMessageW(&msg) };
        }
        std::thread::sleep(Duration::from_millis(15));
    }
}

/// Thread dedie : liste les entrees du menu natif, puis attend (60 s max) qu'on en lance une.
fn start_session(paths: Vec<PathBuf>) -> (Sender<u32>, Receiver<Vec<ShellItem>>, Receiver<bool>) {
    let (cmd_tx, cmd_rx) = channel::<u32>();
    let (items_tx, items_rx) = channel::<Vec<ShellItem>>();
    let (done_tx, done_rx) = channel::<bool>();
    std::thread::spawn(move || unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        'session: {
            let Some(menu) = context_menu(&paths) else {
                let _ = items_tx.send(Vec::new());
                break 'session;
            };
            MENU3.with(|m| *m.borrow_mut() = menu.cast::<IContextMenu3>().ok());
            MENU2.with(|m| *m.borrow_mut() = menu.cast::<IContextMenu2>().ok());
            let class = windows::core::w!("WindowsFinderShellMenu");
            let hinst = GetModuleHandleW(None).map(|h| h.into()).unwrap_or_default();
            let wc = WNDCLASSW { lpfnWndProc: Some(wndproc), hInstance: hinst, lpszClassName: class, ..Default::default() };
            RegisterClassW(&wc);
            let hwnd = CreateWindowExW(Default::default(), class, windows::core::w!(""), WS_POPUP, 0, 0, 0, 0, None, None, Some(hinst), None)
                .unwrap_or_default();
            let Ok(hmenu) = CreatePopupMenu() else {
                let _ = items_tx.send(Vec::new());
                break 'session;
            };
            if menu.QueryContextMenu(hmenu, 0, 1, 0x7FFF, CMF_NORMAL | CMF_CANRENAME).is_ok() {
                let _ = items_tx.send(read_menu(&menu, hmenu, 0));
            } else {
                let _ = items_tx.send(Vec::new());
            }
            let start = Instant::now();
            loop {
                pump_for(20);
                match cmd_rx.try_recv() {
                    Ok(id) => {
                        let mut pt = POINT::default();
                        let _ = GetCursorPos(&mut pt);
                        let info = CMINVOKECOMMANDINFOEX {
                            cbSize: size_of::<CMINVOKECOMMANDINFOEX>() as u32,
                            hwnd,
                            lpVerb: PCSTR((id as usize - 1) as *const u8),
                            nShow: SW_SHOWNORMAL.0,
                            ptInvoke: pt,
                            ..Default::default()
                        };
                        let ok = menu.InvokeCommand(&info as *const _ as *const _).is_ok();
                        let _ = done_tx.send(ok);
                        pump_for(1500);
                        break;
                    }
                    Err(TryRecvError::Disconnected) => break,
                    Err(TryRecvError::Empty) => {
                        if start.elapsed() > Duration::from_secs(60) {
                            break;
                        }
                    }
                }
            }
            let _ = DestroyMenu(hmenu);
            if !hwnd.0.is_null() {
                let _ = DestroyWindow(hwnd);
            }
        }
        MENU3.with(|m| *m.borrow_mut() = None);
        MENU2.with(|m| *m.borrow_mut() = None);
        CoUninitialize();
    });
    (cmd_tx, items_rx, done_rx)
}

impl Finder {
    /// Ouvre (une seule fois) le volet des options natives a droite du menu contextuel.
    pub fn open_shell_more(&mut self, cx: &mut Context<Self>) {
        if self.shell_more.is_some() {
            return;
        }
        let mut paths = self.selected_paths();
        if paths.is_empty() {
            if crate::fs::is_virtual(&self.cwd) {
                return;
            }
            paths.push(self.cwd.clone());
        }
        let (tx, items_rx, done_rx) = start_session(paths);
        let token = TOKEN.fetch_add(1, Ordering::Relaxed);
        self.shell_more = Some(ShellMore { token, tx, done: Some(done_rx), items: None, path: Vec::new() });
        cx.spawn(async move |this, cx| {
            let items = cx.background_executor().spawn(async move { items_rx.recv().ok() }).await;
            this.update(cx, |f, cx| {
                if let Some(m) = f.shell_more.as_mut().filter(|m| m.token == token) {
                    m.items = Some(items.unwrap_or_default());
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    /// Lance une commande du menu natif puis ferme le menu.
    pub fn shell_invoke(&mut self, id: u32, cx: &mut Context<Self>) {
        self.menu = None;
        let Some(mut m) = self.shell_more.take() else { return };
        let keep = self.selected_paths();
        let _ = m.tx.send(id);
        let Some(done) = m.done.take() else { return };
        cx.spawn(async move |this, cx| {
            let ran = cx.background_executor().spawn(async move { done.recv().unwrap_or(false) }).await;
            this.update(cx, |f, cx| {
                if ran {
                    f.reload_select(&keep);
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }
}
