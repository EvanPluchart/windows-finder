//! Barre laterale flottante (sections Favoris / Emplacements, icones bleues au trait) et
//! barre inferieure (chemin + etat), d'apres les captures du Finder.

use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};

use gpui::{
    Context, InteractiveElement, IntoElement, MouseButton, ParentElement, SharedString, StatefulInteractiveElement, Styled, div,
    prelude::*, px, };
use crate::theme::{rgb};
use windows::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
use windows::core::PCWSTR;

use crate::layout;
use crate::theme;
use crate::views::label;
use crate::Finder;

/// Bleu des icones de la barre laterale (accent du Finder).
const ACCENT: u32 = 0x2f7df4;

#[derive(Clone)]
pub enum Place {
    Header(SharedString),
    Item { name: SharedString, glyph: &'static str, path: PathBuf },
}

pub fn default_places() -> Vec<Place> {
    let home = crate::fs::home_dir();
    let item = |name: &str, glyph: &'static str, path: PathBuf| Place::Item { name: name.to_string().into(), glyph, path };
    let mut places = vec![Place::Header(crate::i18n::t("Favoris").into())];
    let recent = std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join("Microsoft/Windows/Recent"));
    if let Some(r) = recent.filter(|r| r.is_dir() && crate::settings::sidebar_on("recents")) {
        places.push(item(crate::i18n::t("R\u{e9}cents"), "clock", r));
    }
    if crate::settings::sidebar_on("apps") {
        places.push(item(crate::i18n::t("Applications"), "applications", PathBuf::from(crate::fs::APPS)));
    }
    let on = crate::settings::sidebar_on;
    if on("desktop") {
        places.push(item(crate::i18n::t("Bureau"), "menubar.dock.rectangle", home.join("Desktop")));
    }
    if on("docs") {
        places.push(item(crate::i18n::t("Documents"), "doc", home.join(crate::i18n::t("Documents"))));
    }
    if on("downloads") {
        places.push(item(crate::i18n::t("T\u{e9}l\u{e9}chargements"), "arrow.down.circle", home.join("Downloads")));
    }
    if on("media") {
        places.push(item(crate::i18n::t("Images"), "photo", home.join("Pictures")));
        places.push(item(crate::i18n::t("Musique"), "music.note", home.join("Music")));
        places.push(item(crate::i18n::t("Vid\u{e9}os"), "video", home.join("Videos")));
    }

    for p in crate::extras::pins() {
        let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| p.display().to_string());
        places.push(item(&name, "folder", p));
    }

    places.push(Place::Header(crate::i18n::t("Emplacements").into()));
    let onedrive = home.join("OneDrive");
    if onedrive.is_dir() && on("cloud") {
        places.push(item("OneDrive", "icloud", onedrive));
    }
    let user = home.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| crate::i18n::t("Accueil").into());
    if on("home") {
        places.push(item(&user, "house", home.clone()));
    }
    let pc = std::env::var("COMPUTERNAME").unwrap_or_else(|_| "Ce PC".into());
    if on("pc") {
        places.push(item(&pc, "laptopcomputer", PathBuf::from(crate::fs::PC)));
    }
    for letter in b'A'..=b'Z' {
        let root = PathBuf::from(format!("{}:\\", letter as char));
        if root.exists() && on("disks") {
            let (label, removable) = drive_info(&root);
            places.push(item(&label, if removable { "externaldrive" } else { "internaldrive" }, root));
        }
    }
    if on("wsl") {
        for d in crate::netplaces::wsl_distros() {
            places.push(item(d, "folder", crate::netplaces::wsl_path(d)));
        }
    }
    if crate::settings::sidebar_on("net") {
        places.push(item(crate::i18n::t("R\u{e9}seau"), "globe", PathBuf::from("::net")));
        places.push(item(crate::i18n::t("Corbeille"), "trash", PathBuf::from("::trash")));
    }

    if crate::settings::sidebar_on("tags") {
        places.push(Place::Header(crate::i18n::t("\u{c9}tiquettes").into()));
        for (name, _, hex) in crate::tags::PALETTE {
            let glyph: &'static str = Box::leak(format!("dot:{hex:06x}").into_boxed_str());
            places.push(item(&crate::i18n::td(name), glyph, PathBuf::from(format!("::tag:{name}"))));
        }
    }
    places
}

/// Espace libre du volume contenant `path`, en octets.
fn free_space(path: &Path) -> Option<u64> {
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    let mut avail = 0u64;
    unsafe { GetDiskFreeSpaceExW(PCWSTR(wide.as_ptr()), Some(&mut avail), None, None).ok()? };
    Some(avail)
}

fn format_gb(bytes: u64) -> String {
    crate::i18n::size(bytes as f64 / 1e9, 2, 2)
}

impl Finder {
    pub fn render_sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let lights = self.traffic_lights(cx);
        let rows = self.places.iter().enumerate().map(|(i, place)| match place {
            Place::Header(title) => div()
                .px(px(10.))
                .pt(px(14.))
                .pb(px(4.))
                .text_size(px(12.))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(rgb(0x878787))
                .child(title.clone())
                .into_any_element(),
            Place::Item { name, glyph, path } => {
                let active = *path == self.cwd;
                let target = path.clone();
                div()
                    .id(("place", i))
                    .h(px(32.))
                    .px(px(10.))
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .rounded(px(9.))
                    .text_size(px(14.))
                    .when(active, |d| d.bg(rgb(0xdcdcdf)))
                    .when(!active, |d| d.hover(|s| s.bg(rgb(0xe3e3e6))))
                    .child(
                        div().w(px(22.)).flex().justify_center().child(if let Some(hex) = glyph.strip_prefix("dot:") {
                            div()
                                .size(px(12.))
                                .rounded_full()
                                .bg(rgb(u32::from_str_radix(hex, 16).unwrap_or(0x98989d)))
                                .into_any_element()
                        } else {
                            crate::symbols::sym(
                                glyph,
                                if *glyph == "applications" { 8.5 } else { 13. },
                                if matches!(*glyph, "house" | "laptopcomputer" | "internaldrive" | "externaldrive" | "globe" | "trash") {
                                    0x6e6e73
                                } else {
                                    ACCENT
                                },
                            )
                        }),
                    )
                    .child(label(name.clone()).flex_1().min_w_0())
                    .when(*glyph == "externaldrive", |d| {
                        let root = path.clone();
                        d.child(
                            div()
                                .id(("eject", i))
                                .size(px(18.))
                                .flex_none()
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded(px(4.))
                                .hover(|s| s.bg(rgb(0xd0d0d6)))
                                .child(crate::symbols::sym("eject.fill", 7., 0x8e8e93))
                                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                                .cursor_pointer().on_click(cx.listener(move |this, _, _, cx| {
                                    cx.stop_propagation();
                                    this.eject_drive(root.clone(), cx);
                                })),
                        )
                    })
                    .drag_over::<crate::dragdrop::DragPaths>(|s, _, _, _| s.bg(rgb(0xc4daf9)))
                    .on_drop(cx.listener({
                        let target = target.clone();
                        move |this, p: &crate::dragdrop::DragPaths, window, cx| {
                            this.drop_into(p, target.clone(), window.modifiers().control, cx)
                        }
                    }))
                    .on_drop(cx.listener({
                        let target = target.clone();
                        move |this, p: &gpui::ExternalPaths, window, cx| {
                            this.drop_external(p.paths().to_vec(), target.clone(), window.modifiers().control, cx)
                        }
                    }))
                    .cursor_pointer().on_click(cx.listener(move |this, _, _, cx| this.open_place(target.clone(), cx)))
                    .on_mouse_down(MouseButton::Right, cx.listener({
                        let target = path.clone();
                        move |this, _, _, cx| {
                            if crate::extras::pins().contains(&target) {
                                this.unpin(&target, cx);
                                this.show_notice(crate::i18n::t("Retir\u{e9} de la barre lat\u{e9}rale"), cx);
                            }
                        }
                    }))
                    .on_mouse_down(MouseButton::Middle, cx.listener({
                        let target = path.clone();
                        move |this, _, _, cx| {
                            if !matches!(target.to_str(), Some("::net") | Some("::trash")) {
                                this.new_tab(Some(target.clone()), cx);
                            }
                        }
                    }))
                    .into_any_element()
            }
        });

        // Barre laterale collee aux bords, sur toute la hauteur (mesure sur le Finder macOS 27 : fond #EDEDED,
        // filet #D3D4D8 a droite). Les coins arrondis de la fenetre la decoupent. Rangee du haut de 52 pt,
        // comme la barre d'outils, pour que boutons de fenetre et capsules soient sur le meme axe.
        div()
            .w(px(self.sidebar_w))
            .relative()
            .flex_none()
            .h_full()
            .flex()
            .flex_col()
            .rounded_tl(px(14.))
            .rounded_bl(px(14.))
            .bg(rgb(0xededed))
            .border_r_1()
            .border_color(rgb(0xd3d4d8))
            .child(
                div()
                    .h(px(theme::TOOLBAR_HEIGHT))
                    .flex_none()
                    .pl(px(19.))
                    .flex()
                    .items_center()
                    .child(lights)
                    .on_mouse_down(gpui::MouseButton::Left, |ev, window, _| window_drag(ev, window)),
            )
            .child(div().id("sidebar").flex_1().min_h_0().overflow_y_scroll().px(px(10.)).pb(px(8.)).children(rows))
            // Poignee de redimensionnement sur le bord droit (glisser ; double-clic = masquer la barre laterale).
            .child(
                div()
                    .id("sidebar-resize")
                    .absolute()
                    .top_0()
                    .bottom_0()
                    .right(px(-3.))
                    .w(px(7.))
                    .cursor(gpui::CursorStyle::ResizeLeftRight)
                    .on_drag(SidebarResize, |_, _, _, cx| cx.new(|_| SidebarResize))
                    .cursor_pointer().on_click(cx.listener(|this, ev: &gpui::ClickEvent, _, cx| {
                        if ev.click_count() >= 2 {
                            this.show_sidebar = false;
                            layout::set_setting("sidebar", "0");
                            cx.notify();
                        }
                    })),
            )
    }

    /// Barre de chemin cliquable + ligne d'etat ("N elements, X Go disponible(s)").
    pub fn render_bottom_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut crumbs: Vec<(String, PathBuf)> = Vec::new();
        let mut acc = PathBuf::new();
        if crate::fs::is_virtual(&self.cwd) {
            crumbs.push((crate::fs::display_name(&self.cwd), self.cwd.clone()));
        }
        for comp in self.cwd.components().filter(|_| !crate::fs::is_virtual(&self.cwd)) {
            acc.push(comp);
            let name = match comp {
                std::path::Component::Prefix(p) => p.as_os_str().to_string_lossy().into_owned(),
                std::path::Component::RootDir => continue,
                other => other.as_os_str().to_string_lossy().into_owned(),
            };
            let target = if matches!(comp, std::path::Component::Prefix(_)) { acc.join("\\") } else { acc.clone() };
            crumbs.push((name, target));
        }
        let last = crumbs.len().saturating_sub(1);
        let path_bar = crumbs.into_iter().enumerate().map(|(i, (name, target))| {
            div()
                .id(("crumb", i))
                .flex()
                .items_center()
                .gap(px(4.))
                .px(px(4.))
                .rounded(px(5.))
                .hover(|s| s.bg(rgb(0xececf0)))
                .when(i == last, |d| d.text_color(rgb(theme::TEXT)))
                .child(crate::symbols::sym("folder", 10.5, ACCENT))
                .child(name)
                .when(i != last, |d| d.child(div().text_color(rgb(0xb0b0b6)).child("\u{203A}")))
                .cursor_pointer().on_click(cx.listener(move |this, _, _, cx| this.navigate(target.clone(), true, cx)))
        });

        let total = self.entries.len();
        let free = free_space(&self.cwd).map(|b| crate::i18n::tf(crate::i18n::t(", {0} disponible(s)"), &[("0", (format_gb(b)).to_string())])).unwrap_or_default();
        let status = match self.selected_entry() {
            _ if self.search_query().is_some() => {
                crate::i18n::tf(crate::i18n::t("{total} r\u{e9}sultat{0}"), &[("total", (total).to_string()), ("0", (if total > 1 { "s" } else { "" }).to_string())])
            }
            Some(_) => {
                let k = self.marked.len().max(1);
                crate::i18n::tf(crate::i18n::t("{k} sur {total} s\u{e9}lectionn\u{e9}{0}{free}"), &[("k", (k).to_string()), ("total", (total).to_string()), ("0", (if k > 1 { "s" } else { "" }).to_string()), ("free", (free).to_string())])
            }
            None => crate::i18n::tf(crate::i18n::t("{total} \u{e9}l\u{e9}ment{0}{free}"), &[("total", (total).to_string()), ("0", (if total > 1 { "s" } else { "" }).to_string()), ("free", (free).to_string())]),
        };

        div()
            .flex_none()
            .flex()
            .flex_col()
            .text_size(px(12.))
            .text_color(rgb(theme::TEXT_SECONDARY))
            .when(self.show_path_bar, |d| {
                d.child(
                    div()
                        .h(px(28.))
                        .px(px(12.))
                        .flex()
                        .items_center()
                        .border_t_1()
                        .border_color(rgb(theme::SEPARATOR))
                        .children(path_bar),
                )
            })
            .when(self.show_status_bar, |d| {
                d.child(
                    div()
                        .h(px(28.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .border_t_1()
                        .border_color(rgb(theme::SEPARATOR))
                        .child(status),
                )
            })
    }
}

/// Deplacement de la fenetre par glisser depuis le fond de la barre d'outils / du haut de la barre laterale
/// (comme la barre de titre du Finder) ; double-clic = agrandir / restaurer. Declenche par un vrai clic seulement :
/// la fenetre cliquee est alors au premier plan.
pub fn window_drag(ev: &gpui::MouseDownEvent, window: &mut gpui::Window) {
    use windows::Win32::Foundation::{LPARAM, WPARAM};
    use windows::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, HTCAPTION, PostMessageW, SW_RESTORE, ShowWindow, WM_NCLBUTTONDOWN,
    };
    if ev.click_count >= 2 {
        if window.is_maximized() {
            unsafe {
                let _ = ShowWindow(GetForegroundWindow(), SW_RESTORE);
            }
        } else {
            window.zoom_window();
        }
        return;
    }
    unsafe {
        let hwnd = GetForegroundWindow();
        let _ = ReleaseCapture();
        let _ = PostMessageW(Some(hwnd), WM_NCLBUTTONDOWN, WPARAM(HTCAPTION as usize), LPARAM(0));
    }
}

/// Nom affiche d'un disque ("Disque local (C:)") et `true` s'il est amovible.
pub fn drive_info(root: &Path) -> (String, bool) {
    use windows::Win32::Storage::FileSystem::{GetDriveTypeW, GetVolumeInformationW};
    let wide: Vec<u16> = root.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    let letter = root.to_string_lossy().chars().next().unwrap_or('C');
    let mut label = [0u16; 64];
    let ok = unsafe { GetVolumeInformationW(PCWSTR(wide.as_ptr()), Some(&mut label), None, None, None, None).is_ok() };
    let kind = unsafe { GetDriveTypeW(PCWSTR(wide.as_ptr())) };
    let removable = kind == 2 || kind == 5;
    let text = String::from_utf16_lossy(&label).trim_end_matches('\0').to_string();
    let name = if ok && !text.is_empty() { text } else { crate::i18n::t("Disque local").to_string() };
    (format!("{name} ({letter}:)"), removable)
}

impl Finder {
    /// Ejecte un disque amovible (verbe "Eject" du shell Windows).
    pub fn eject_drive(&mut self, root: PathBuf, cx: &mut Context<Self>) {
        use std::os::windows::process::CommandExt;
        let letter = root.to_string_lossy().chars().next().unwrap_or('E');
        let script = format!("(New-Object -ComObject Shell.Application).Namespace(17).ParseName('{letter}:').InvokeVerb('Eject')");
        let _ = std::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .creation_flags(0x0800_0000)
            .spawn();
        self.show_notice(crate::i18n::tf(crate::i18n::t("\u{c9}jection du disque {letter}: demand\u{e9}e"), &[("letter", (letter).to_string())]), cx);
        // La liste des disques est relue peu apres.
        self.places = default_places();
        cx.notify();
    }
}

/// Valeur portee par le glisser de la poignee de la barre laterale (apercu vide).
pub struct SidebarResize;

impl gpui::Render for SidebarResize {
    fn render(&mut self, _: &mut gpui::Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size(px(0.))
    }
}
