//! Barre d'outils unifiee (facon Finder) qui sert aussi de barre de titre : capsules, vues, recherche,
//! et boutons de fenetre Windows a droite.

use gpui::{
    AnyElement, Context, Div, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement, Styled,
    MouseButton, div, prelude::*, px, rgb,
};

use crate::theme;
use crate::{Finder, ViewMode};

const FLUENT: &str = "Segoe Fluent Icons";

fn fluent(glyph: &'static str, size: f32) -> Div {
    // Boite carree a hauteur de ligne fixe : le glyphe est centre sur l'axe de la boite, pas sur sa ligne de texte.
    div()
        .font_family(FLUENT)
        .text_size(px(size))
        .size(px(size + 2.))
        .line_height(px(size + 2.))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .child(glyph)
}

/// Capsule blanche a fin contour, comme les groupes de boutons du Finder.
fn capsule() -> Div {
    div()
        .h(px(36.))
        .flex()
        .items_center()
        .p(px(2.))
        .rounded_full()
        .bg(rgb(0xfdfdfd))
        .border_1()
        .border_color(rgb(0xe4e5e6))
        .shadow_sm()
        // Bloque la zone de deplacement placee derriere : sans cela le clic deviendrait un glissement de fenetre.
        .occlude()
}

fn stroke() -> Div {
    div().border_color(rgb(theme::TEXT)).border_1()
}

fn icon_grid() -> AnyElement {
    let sq = || div().size(px(6.)).rounded(px(1.5)).border_color(rgb(theme::TEXT)).border_1();
    div()
        .size(px(16.))
        .flex()
        .flex_wrap()
        .gap(px(2.))
        .content_center()
        .child(sq())
        .child(sq())
        .child(sq())
        .child(sq())
        .into_any_element()
}

fn icon_list() -> AnyElement {
    let row = || {
        div()
            .w(px(16.))
            .h(px(3.))
            .flex()
            .gap(px(2.))
            .child(div().size(px(3.)).rounded_full().bg(rgb(theme::TEXT)))
            .child(div().flex_1().h(px(3.)).rounded(px(1.5)).bg(rgb(theme::TEXT)))
    };
    div().w(px(16.)).flex().flex_col().gap(px(3.)).child(row()).child(row()).child(row()).into_any_element()
}

fn icon_columns() -> AnyElement {
    stroke()
        .w(px(18.))
        .h(px(14.))
        .rounded(px(3.))
        .flex()
        .child(div().flex_1())
        .child(div().w(px(1.)).h_full().bg(rgb(theme::TEXT)))
        .child(div().flex_1())
        .child(div().w(px(1.)).h_full().bg(rgb(theme::TEXT)))
        .child(div().flex_1())
        .into_any_element()
}

fn icon_gallery() -> AnyElement {
    div()
        .w(px(18.))
        .flex()
        .flex_col()
        .gap(px(2.))
        .child(stroke().h(px(10.)).rounded(px(2.)))
        .child(
            div()
                .flex()
                .gap(px(1.5))
                .child(div().flex_1().h(px(3.)).bg(rgb(theme::TEXT)))
                .child(div().flex_1().h(px(3.)).bg(rgb(theme::TEXT)))
                .child(div().flex_1().h(px(3.)).bg(rgb(theme::TEXT)))
                .child(div().flex_1().h(px(3.)).bg(rgb(theme::TEXT))),
        )
        .into_any_element()
}


impl Finder {
    pub fn render_toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let can_back = self.can_back();
        let can_fwd = self.can_forward();
        let dim = |on: bool| rgb(if on { theme::TEXT } else { 0xb4b4b8 });

        let nav = capsule()
            .child(
                div()
                    .id("back")
                    .w(px(36.))
                    .h(px(32.))
                    .rounded_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(dim(can_back))
                    .when(can_back, |d| d.hover(|s| s.bg(rgb(theme::HOVER))))
                    .child(fluent("\u{E76B}", 14.))
                    .on_click(cx.listener(|this, _, _, cx| this.go_back(cx))),
            )
            .child(div().w(px(1.)).h(px(18.)).bg(rgb(0xe4e5e6)))
            .child(
                div()
                    .id("fwd")
                    .w(px(36.))
                    .h(px(32.))
                    .rounded_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(dim(can_fwd))
                    .when(can_fwd, |d| d.hover(|s| s.bg(rgb(theme::HOVER))))
                    .child(fluent("\u{E76C}", 14.))
                    .on_click(cx.listener(|this, _, _, cx| this.go_forward(cx))),
            );

        let seg = |id: &'static str, icon: AnyElement, mode: ViewMode, cx: &mut Context<Self>| {
            let active = self.view == mode;
            div()
                .id(id)
                .w(px(40.))
                .h(px(32.))
                .rounded_full()
                .flex()
                .items_center()
                .justify_center()
                .when(active, |d| d.bg(rgb(0xe6e6e6)))
                .when(!active, |d| d.hover(|s| s.bg(rgb(0xf0f0f0))))
                .child(icon)
                .on_click(cx.listener(move |this, _, _, cx| this.set_view(mode, cx)))
        };
        let views = capsule()
            .child(seg("v-icons", icon_grid(), ViewMode::Icons, cx))
            .child(seg("v-list", icon_list(), ViewMode::List, cx))
            .child(seg("v-cols", icon_columns(), ViewMode::Columns, cx))
            .child(seg("v-gallery", icon_gallery(), ViewMode::Gallery, cx));

        let group = capsule().px(px(10.)).gap(px(4.)).child(icon_grid()).child(fluent("\u{E70D}", 9.));

        let actions = capsule()
            .child(div().w(px(38.)).h(px(32.)).flex().items_center().justify_center().child(fluent("\u{E72D}", 16.)))
            .child(div().w(px(38.)).h(px(32.)).flex().items_center().justify_center().child(fluent("\u{E8EC}", 16.)))
            .child(div().w(px(38.)).h(px(32.)).flex().items_center().justify_center().child(fluent("\u{E712}", 16.)));

        // Recherche : bouton rond ; au clic il devient un champ de saisie (Ctrl+F), Echap le referme.
        let search = match &self.search {
            Some(s) => capsule()
                .w(px(250.))
                .px(px(12.))
                .gap(px(6.))
                .child(div().text_color(rgb(theme::TEXT_SECONDARY)).child(fluent("\u{E721}", 14.)))
                .child(div().flex_1().min_w_0().h(px(26.)).child(s.editor.clone()))
                .child(
                    div()
                        .id("search-clear")
                        .size(px(20.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded_full()
                        .text_color(rgb(theme::TEXT_SECONDARY))
                        .hover(|st| st.bg(rgb(theme::HOVER)))
                        .child(fluent("\u{E711}", 10.))
                        .on_click(cx.listener(|this, _, _, cx| this.close_search(cx))),
                ),
            None => capsule().w(px(40.)).justify_center().child(
                div()
                    .id("search-open")
                    .size_full()
                    .rounded_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(fluent("\u{E721}", 16.))
                    .on_click(cx.listener(|this, _, window, cx| this.open_search(window, cx))),
            ),
        };

        let title = self
            .cwd
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.cwd.display().to_string());

        div()
            .h(px(theme::TOOLBAR_HEIGHT))
            .flex_none()
            .relative()
            .pl(px(16.))
            .pr(px(16.))
            .flex()
            .items_center()
            .gap(px(14.))
            .rounded_tr(px(17.))
            .child(nav)
            .child(
                div()
                    .text_size(px(15.))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child(title),
            )
            .child(div().flex_1().h_full())
            .child(views)
            .child(group)
            .child(actions)
            .child(search)
            .on_mouse_down(MouseButton::Left, |ev, window, _| crate::sidebar::window_drag(ev, window))
    }
}
