//! Barre d'outils unifiee (facon Finder) qui sert aussi de barre de titre : capsules, vues, recherche,
//! et boutons de fenetre Windows a droite.

use gpui::{
    AnyElement, Context, Div, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement, Styled,
    MouseButton, div, prelude::*, px, };
use crate::theme::{rgb};

use crate::symbols::sym;
use crate::theme;
use crate::{Finder, ViewMode};



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

fn icon_grid() -> AnyElement {
    sym("square.grid.2x2-medium", 14., theme::TEXT)
}

fn icon_list() -> AnyElement {
    sym("list.bullet-medium", 14., theme::TEXT)
}

fn icon_columns() -> AnyElement {
    sym("rectangle.split.3x1-medium", 14., theme::TEXT)
}

fn icon_gallery() -> AnyElement {
    sym("square.grid.3x1.below.line.grid.1x2-medium", 14., theme::TEXT)
}

impl Finder {
    pub fn render_toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        // Sans barre laterale, les pastilles de fenetre passent dans la barre d'outils.
        let lights = if self.show_sidebar { None } else { Some(self.traffic_lights(cx)) };
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
                    .child(sym("chevron.left-medium", 12., if can_back { theme::TEXT } else { 0xb4b4b8 }))
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
                    .child(sym("chevron.right-medium", 12., if can_fwd { theme::TEXT } else { 0xb4b4b8 }))
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

        let group = capsule().id("group-btn").px(px(10.)).gap(px(4.)).child(sym("square.grid.3x2-medium", 14., theme::TEXT)).child(sym("chevron.down-medium", 8., theme::TEXT_SECONDARY)).on_click(cx.listener(|this, _, _, cx| this.toggle_view_options(cx)));

        let on = crate::settings::toolbar_on;
        let share_btn = div().w(px(38.)).h(px(32.)).flex().items_center().justify_center().child(sym("square.and.arrow.up-medium", 14., theme::TEXT));
        let tag_btn = div()
            .id("tag-btn")
            .w(px(38.))
            .h(px(32.))
            .flex()
            .items_center()
            .justify_center()
            .rounded_full()
            .hover(|s| s.bg(rgb(theme::HOVER)))
            .child(sym("tag-medium", 14., theme::TEXT))
            .on_click(cx.listener(|this, _, window, cx| this.toggle_tag_pop(window, cx)));
        let more_btn = div().w(px(38.)).h(px(32.)).flex().items_center().justify_center().child(sym("ellipsis-medium", 14., theme::TEXT));
        let actions = capsule()
            .when(on("share"), |d| d.child(share_btn))
            .when(on("tags"), |d| d.child(tag_btn))
            .when(on("more"), |d| d.child(more_btn));
        let show_actions = on("share") || on("tags") || on("more");


        // Recherche : bouton rond ; au clic il devient un champ de saisie (Ctrl+F), Echap le referme.
        let search = match &self.search {
            Some(s) => capsule()
                .w(px(250.))
                .px(px(12.))
                .gap(px(6.))
                .child(div().text_color(rgb(theme::TEXT_SECONDARY)).child(sym("magnifyingglass-medium", 11., theme::TEXT_SECONDARY)))
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
                        .child(sym("xmark-medium", 8., theme::TEXT_SECONDARY))
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
                    .child(sym("magnifyingglass-medium", 14., theme::TEXT))
                    .on_click(cx.listener(|this, _, window, cx| this.open_search(window, cx))),
            ),
        };

        let title = crate::fs::display_name(&self.cwd);

        div()
            .h(px(theme::TOOLBAR_HEIGHT))
            .flex_none()
            .relative()
            .pl(px(if self.show_sidebar { 16. } else { 104. }))
            .pr(px(16.))
            .flex()
            .items_center()
            .gap(px(14.))
            .rounded_tr(px(14.))
            .children(lights.map(|l| div().absolute().left(px(19.)).top(px(19.)).child(l)))
            .child(nav)
            .child(
                div()
                    .text_size(px(15.))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child(title),
            )
            .child(div().flex_1().h_full())
            .children(on("views").then(|| views))
            .children(on("group").then(|| group))
            .children(show_actions.then(|| actions))
            .children((on("search") || self.search.is_some()).then(|| search))
            // Clic droit sur la barre d'outils : personnaliser (reglages).
            .on_mouse_down(MouseButton::Right, cx.listener(|this, _, _, cx| this.toggle_settings(cx)))
            .on_mouse_down(MouseButton::Left, |ev, window, _| crate::sidebar::window_drag(ev, window))
    }
}
