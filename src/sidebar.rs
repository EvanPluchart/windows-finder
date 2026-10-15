//! Barre laterale flottante (sections Favoris / Emplacements, icones bleues au trait) et
//! barre inferieure (chemin + etat), d'apres les captures du Finder.

use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};

use gpui::{
    Context, InteractiveElement, IntoElement, ParentElement, SharedString, StatefulInteractiveElement, Styled, div,
    prelude::*, px, rgb,
};
use windows::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
use windows::core::PCWSTR;

use crate::theme;
use crate::views::label;
use crate::Finder;

const FLUENT: &str = "Segoe Fluent Icons";
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
    let mut places = vec![Place::Header("Favoris".into())];
    let recent = std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join("Microsoft/Windows/Recent"));
    if let Some(r) = recent.filter(|r| r.is_dir()) {
        places.push(item("R\u{e9}cents", "\u{E823}", r));
    }
    places.push(item("Accueil", "\u{E80F}", home.clone()));
    places.push(item("Bureau", "\u{E7F4}", home.join("Desktop")));
    places.push(item("Documents", "\u{E8A5}", home.join("Documents")));
    places.push(item("T\u{e9}l\u{e9}chargements", "\u{E896}", home.join("Downloads")));
    places.push(item("Images", "\u{E91B}", home.join("Pictures")));
    places.push(item("Musique", "\u{E8D6}", home.join("Music")));
    places.push(item("Vid\u{e9}os", "\u{E714}", home.join("Videos")));

    places.push(Place::Header("Emplacements".into()));
    let onedrive = home.join("OneDrive");
    if onedrive.is_dir() {
        places.push(item("OneDrive", "\u{E753}", onedrive));
    }
    for letter in b'C'..=b'Z' {
        let root = PathBuf::from(format!("{}:\\", letter as char));
        if root.exists() {
            places.push(item(&format!("Disque ({}:)", letter as char), "\u{EDA2}", root));
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
    format!("{:.2} Go", bytes as f64 / 1e9).replace('.', ",")
}

/// Un bouton de fenetre macOS : pastille coloree, symbole visible au survol du groupe.
fn traffic_dot(
    id: &'static str,
    color: u32,
    edge: u32,
    glyph: &'static str,
    on_click: impl Fn(&mut gpui::Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .size(px(14.))
        .rounded_full()
        .bg(rgb(color))
        .border_1()
        .border_color(rgb(edge))
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(10.))
        .font_weight(gpui::FontWeight::BOLD)
        .text_color(rgb(0x4d0000))
        .child(div().invisible().group_hover("traffic", |s| s.visible()).child(glyph))
        .on_click(move |_, window, cx| on_click(window, cx))
}

/// Feu tricolore macOS (fermer, reduire, agrandir) a la place des boutons Windows.
fn traffic_lights() -> impl IntoElement {
    div()
        .group("traffic")
        .flex()
        .items_center()
        .gap(px(9.))
        // Les pastilles gardent le clic : la zone de deplacement derriere est bloquee.
        .occlude()
        .child(traffic_dot("tl-close", 0xff5f57, 0xe0443e, "\u{d7}", |window, _| window.remove_window()))
        .child(traffic_dot("tl-min", 0xfebc2e, 0xdea123, "\u{2212}", |window, _| window.minimize_window()))
        .child(traffic_dot("tl-zoom", 0x28c840, 0x1aab29, "+", |window, _| {
            if window.is_maximized() {
                // GPUI ne sait que maximiser : on restaure via l'API Windows (la fenetre cliquee est au premier plan).
                use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, SW_RESTORE, ShowWindow};
                unsafe {
                    let _ = ShowWindow(GetForegroundWindow(), SW_RESTORE);
                }
            } else {
                window.zoom_window();
            }
        }))
}

impl Finder {
    pub fn render_sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let rows = self.places.iter().enumerate().map(|(i, place)| match place {
            Place::Header(title) => div()
                .px(px(10.))
                .pt(px(14.))
                .pb(px(4.))
                .text_size(px(12.))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(rgb(0x8e8e93))
                .child(title.clone())
                .into_any_element(),
            Place::Item { name, glyph, path } => {
                let active = *path == self.cwd;
                let target = path.clone();
                div()
                    .id(("place", i))
                    .h(px(30.))
                    .px(px(10.))
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .rounded(px(9.))
                    .text_size(px(14.))
                    .when(active, |d| d.bg(rgb(0xdcdcdf)))
                    .when(!active, |d| d.hover(|s| s.bg(rgb(0xe3e3e6))))
                    .child(
                        div()
                            .w(px(20.))
                            .flex()
                            .justify_center()
                            .font_family(FLUENT)
                            .text_size(px(16.))
                            .text_color(rgb(ACCENT))
                            .child(*glyph),
                    )
                    .child(label(name.clone()).flex_1().min_w_0())
                    .drag_over::<crate::dragdrop::DragPaths>(|s, _, _, _| s.bg(rgb(0xc4daf9)))
                    .on_drop(cx.listener({
                        let target = target.clone();
                        move |this, p: &crate::dragdrop::DragPaths, window, cx| {
                            this.drop_into(p, target.clone(), window.modifiers().control, cx)
                        }
                    }))
                    .drag_over::<crate::dragdrop::DragPaths>(|s, _, _, _| s.bg(rgb(0xc4daf9)))
                .on_drop(cx.listener({
                    let target = target.clone();
                    move |this, p: &crate::dragdrop::DragPaths, window, cx| {
                        this.drop_into(p, target.clone(), window.modifiers().control, cx)
                    }
                }))
                .on_click(cx.listener(move |this, _, _, cx| this.navigate(target.clone(), true, cx)))
                    .into_any_element()
            }
        });

        // Barre laterale collee aux bords, sur toute la hauteur (mesure sur le Finder macOS 27 : fond #EDEDED,
        // filet #D3D4D8 a droite). Les coins arrondis de la fenetre la decoupent. Rangee du haut de 52 pt,
        // comme la barre d'outils, pour que boutons de fenetre et capsules soient sur le meme axe.
        div()
            .w(px(theme::SIDEBAR_WIDTH))
            .flex_none()
            .h_full()
            .flex()
            .flex_col()
            .rounded_tl(px(17.))
            .rounded_bl(px(17.))
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
                    .child(traffic_lights())
                    .on_mouse_down(gpui::MouseButton::Left, |ev, window, _| window_drag(ev, window)),
            )
            .child(div().id("sidebar").flex_1().min_h_0().overflow_y_scroll().px(px(10.)).pb(px(8.)).children(rows))
    }

    /// Barre de chemin cliquable + ligne d'etat ("N elements, X Go disponible(s)").
    pub fn render_bottom_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut crumbs: Vec<(String, PathBuf)> = Vec::new();
        let mut acc = PathBuf::new();
        for comp in self.cwd.components() {
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
                .child(div().font_family(FLUENT).text_size(px(12.)).text_color(rgb(ACCENT)).child("\u{E8B7}"))
                .child(name)
                .when(i != last, |d| d.child(div().text_color(rgb(0xb0b0b6)).child("\u{203A}")))
                .on_click(cx.listener(move |this, _, _, cx| this.navigate(target.clone(), true, cx)))
        });

        let total = self.entries.len();
        let free = free_space(&self.cwd).map(|b| format!(", {} disponible(s)", format_gb(b))).unwrap_or_default();
        let status = match self.selected_entry() {
            _ if self.search_query().is_some() => {
                format!("{total} r\u{e9}sultat{}", if total > 1 { "s" } else { "" })
            }
            Some(_) => {
                let k = self.marked.len().max(1);
                format!("{k} sur {total} s\u{e9}lectionn\u{e9}{}{free}", if k > 1 { "s" } else { "" })
            }
            None => format!("{total} \u{e9}l\u{e9}ment{}{free}", if total > 1 { "s" } else { "" }),
        };

        div()
            .flex_none()
            .flex()
            .flex_col()
            .text_size(px(12.))
            .text_color(rgb(theme::TEXT_SECONDARY))
            .child(
                div()
                    .h(px(26.))
                    .px(px(12.))
                    .flex()
                    .items_center()
                    .border_t_1()
                    .border_color(rgb(theme::SEPARATOR))
                    .children(path_bar),
            )
            .child(
                div()
                    .h(px(26.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .border_t_1()
                    .border_color(rgb(theme::SEPARATOR))
                    .child(status),
            )
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
