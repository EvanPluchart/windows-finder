//! Boutons de fenetre a la macOS : trois pastilles (symboles au survol) et popover "Move & Resize" au survol du vert.

use gpui::{
    AnyElement, Context, InteractiveElement, IntoElement, MouseButton, ParentElement, StatefulInteractiveElement, Styled,
    Window, div, prelude::*, px, };
use crate::theme::{rgb, rgba};
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow};
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, SW_RESTORE, SWP_NOACTIVATE, SWP_NOZORDER, SetWindowPos, ShowWindow,
};

use crate::{theme, Finder};

const SYMBOLS: &str = "Segoe UI Symbol";

/// Symbole affiche dans la pastille au survol (couleur sombre de la teinte de la pastille).
fn mark(kind: &'static str) -> AnyElement {
    match kind {
        // Croix fine.
        "close" => div()
            .font_family(SYMBOLS)
            .text_size(px(8.))
            .line_height(px(8.))
            .font_weight(gpui::FontWeight::BOLD)
            .text_color(rgb(0x4d0000))
            .child("\u{2715}")
            .into_any_element(),
        // Trait.
        "min" => div().w(px(8.)).h(px(1.6)).rounded_full().bg(rgb(0x5a3a00)).into_any_element(),
        // Deux triangles opposes (agrandir / remplir).
        _ => div()
            .relative()
            .size(px(8.))
            .font_family(SYMBOLS)
            .text_size(px(7.))
            .line_height(px(7.))
            .text_color(rgb(0x0a4d10))
            .child(div().absolute().left(px(-0.5)).top(px(-0.5)).child("\u{25E4}"))
            .child(div().absolute().right(px(-0.5)).bottom(px(-0.5)).child("\u{25E2}"))
            .into_any_element(),
    }
}

fn dot(
    id: &'static str,
    color: u32,
    edge: u32,
    icon: &'static str,
    on_click: impl Fn(&mut Window, &mut gpui::App) + 'static,
) -> gpui::Stateful<gpui::Div> {
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
        .child(div().invisible().group_hover("traffic", |s| s.visible()).child(mark(icon)))
        .cursor_pointer().on_click(move |_, window, cx| on_click(window, cx))
}

/// Fenetre Windows de cette fenetre GPUI. Pas GetForegroundWindow : une autre application a pu passer au premier
/// plan entre le clic et l'action, et c'est elle qui aurait ete deplacee ou redimensionnee.
fn hwnd_of(window: &Window) -> HWND {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    match HasWindowHandle::window_handle(window).map(|h| h.as_raw()) {
        Ok(RawWindowHandle::Win32(h)) => HWND(h.hwnd.get() as *mut _),
        _ => unsafe { GetForegroundWindow() },
    }
}

/// Remet la fenetre a sa taille normale, puis la place dans la zone de travail du moniteur (fractions 0..1).
pub fn place(window: &Window, fx: f32, fy: f32, fw: f32, fh: f32) {
    let hwnd = hwnd_of(window);
    unsafe {
        let mon = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        let mut mi = MONITORINFO { cbSize: size_of::<MONITORINFO>() as u32, ..Default::default() };
        if !GetMonitorInfoW(mon, &mut mi).as_bool() {
            return;
        }
        let w: RECT = mi.rcWork;
        let (ww, wh) = ((w.right - w.left) as f32, (w.bottom - w.top) as f32);
        let _ = ShowWindow(hwnd, SW_RESTORE);
        let _ = SetWindowPos(
            hwnd,
            None,
            w.left + (fx * ww) as i32,
            w.top + (fy * wh) as i32,
            (fw * ww) as i32,
            (fh * wh) as i32,
            SWP_NOZORDER | SWP_NOACTIVATE,
        );
    }
}

/// Icone de disposition : cadre arrondi avec la zone occupee (fractions) en plein, le reste en gris si `gray`.
fn layout_icon(cell: (f32, f32, f32, f32), gray_rest: bool, filled_gray: bool) -> impl IntoElement {
    let dark = if filled_gray { 0x9a9aa0 } else { 0x262626 };
    let (x, y, w, h) = cell;
    div()
        .relative()
        .w(px(30.))
        .h(px(22.))
        .rounded(px(5.))
        .border_2()
        .border_color(rgb(dark))
        .overflow_hidden()
        .when(gray_rest, |d| d.bg(rgb(0xc6c6cb)))
        .child(div().absolute().left(px(x * 26.)).top(px(y * 18.)).w(px(w * 26.)).h(px(h * 18.)).bg(rgb(dark)))
}

impl Finder {
    /// Ouvre le popover quand la souris est sur la pastille verte, le garde ouvert tant qu'elle reste dessus ou dedans.
    pub fn update_zoom_hover(&mut self, pos: gpui::Point<gpui::Pixels>, cx: &mut Context<Self>) {
        let (x, y) = (f32::from(pos.x), f32::from(pos.y));
        let on_dot = (64.0..=80.0).contains(&x) && (18.0..=34.0).contains(&y);
        let in_pop = (8.0..=260.0).contains(&x) && (24.0..=260.0).contains(&y);
        let open = on_dot || (self.zoom_dot_hover && in_pop);
        if open != self.zoom_dot_hover {
            self.zoom_dot_hover = open;
            cx.notify();
        }
    }

    /// Pastilles rouge / jaune / verte ; le survol du vert ouvre le popover.
    pub fn traffic_lights(&self, cx: &mut Context<Self>) -> AnyElement {
        let green = dot("tl-zoom", 0x28c840, 0x1aab29, "zoom", |window, _| {
            if window.is_maximized() {
                unsafe {
                    let _ = ShowWindow(hwnd_of(window), SW_RESTORE);
                }
            } else {
                window.zoom_window();
            }
        })
        ;
        div()
            .id("traffic")
            .group("traffic")
            .on_mouse_move(cx.listener(|this, ev: &gpui::MouseMoveEvent, _, cx| this.update_zoom_hover(ev.position, cx)))
            .flex()
            .items_center()
            .gap(px(9.))
            // Les pastilles gardent le clic : la zone de deplacement derriere est bloquee.
            .occlude()
            .child(dot("tl-close", 0xff5f57, 0xe0443e, "close", |window, _| window.remove_window()))
            .child(dot("tl-min", 0xfebc2e, 0xdea123, "min", |window, _| window.minimize_window()))
            .child(green)
            .into_any_element()
    }

    /// Popover "Deplacer et redimensionner" / "Remplir et organiser" / "Plein ecran".
    pub fn render_zoom_menu(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        if !self.zoom_dot_hover {
            return None;
        }
        let tile = |id: &'static str, cell: (f32, f32, f32, f32), gray_rest: bool, filled_gray: bool, target: (f32, f32, f32, f32), cx: &mut Context<Self>| {
            div()
                .id(id)
                .w(px(46.))
                .h(px(36.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(8.))
                .hover(|s| s.bg(rgb(0xe4e4e8)))
                .child(layout_icon(cell, gray_rest, filled_gray))
                .cursor_pointer().on_click(cx.listener(move |this, _, window, cx| {
                    place(window, target.0, target.1, target.2, target.3);
                    this.zoom_pop_hover = false;
                    this.zoom_dot_hover = false;
                    cx.notify();
                }))
        };
        let header = |t: &'static str| {
            div().text_size(px(12.)).font_weight(gpui::FontWeight::SEMIBOLD).text_color(rgb(0xa0a0a6)).child(t)
        };
        let row = || div().flex().justify_between().px(px(4.));
        let rule = || div().h(px(1.)).w_full().bg(rgb(0xe2e2e6));
        Some(
            // Zone de survol continue depuis la pastille (marge haute) jusqu'au popover.
            div()
                .id("zoom-pop")
                .absolute()
                .left(px(8.))
                .top(px(30.))
                .pt(px(10.))
                .child(
                    div()
                        .w(px(250.))
                        .p(px(12.))
                        .flex()
                        .flex_col()
                        .gap(px(8.))
                        .rounded(px(14.))
                        .bg(rgba(0xfafafaf4))
                        .border_1()
                        .border_color(rgb(0xdadade))
                        .shadow_lg()
                        .occlude()
                        .child(header(crate::i18n::t("D\u{e9}placer et redimensionner")))
                        .child(
                            row()
                                .child(tile("z-left", (0.08, 0.1, 0.4, 0.8), false, false, (0., 0., 0.5, 1.), cx))
                                .child(tile("z-right", (0.52, 0.1, 0.4, 0.8), false, false, (0.5, 0., 0.5, 1.), cx))
                                .child(tile("z-top", (0.08, 0.1, 0.84, 0.4), false, false, (0., 0., 1., 0.5), cx))
                                .child(tile("z-bottom", (0.08, 0.5, 0.84, 0.4), false, false, (0., 0.5, 1., 0.5), cx)),
                        )
                        .child(rule())
                        .child(header(crate::i18n::t("Remplir et organiser")))
                        .child(
                            row()
                                .child(tile("z-fill", (0.08, 0.1, 0.84, 0.8), false, false, (0., 0., 1., 1.), cx))
                                .child(tile("z-23", (0.08, 0.1, 0.5, 0.8), true, true, (0., 0., 0.66, 1.), cx))
                                .child(tile("z-tl", (0.08, 0.1, 0.4, 0.4), true, true, (0., 0., 0.5, 0.5), cx))
                                .child(tile("z-br", (0.52, 0.5, 0.4, 0.4), true, false, (0.5, 0.5, 0.5, 0.5), cx)),
                        )
                        .child(rule())
                        .child(
                            div()
                                .id("z-full")
                                .h(px(30.))
                                .px(px(6.))
                                .flex()
                                .items_center()
                                .justify_between()
                                .rounded(px(8.))
                                .text_size(px(14.))
                                .text_color(rgb(theme::TEXT))
                                .hover(|s| s.bg(rgb(0xe4e4e8)))
                                .child(crate::i18n::t("Plein \u{e9}cran"))
                                .child(div().text_color(rgb(0x6e6e73)).child("\u{203A}"))
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(|this, _, window, cx| {
                                        window.toggle_fullscreen();
                                        this.zoom_pop_hover = false;
                                        this.zoom_dot_hover = false;
                                        cx.notify();
                                    }),
                                ),
                        ),
                )
                .into_any_element(),
        )
    }
}
