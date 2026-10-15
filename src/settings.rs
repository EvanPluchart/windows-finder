//! Reglages (Ctrl+,) : dossier d'ouverture, fichiers caches, elements de la barre laterale.

use gpui::{
    AnyElement, Context, InteractiveElement, IntoElement, MouseButton, ParentElement, StatefulInteractiveElement, Styled,
    div, prelude::*, px,
};

use crate::theme::{rgb, rgba};
use crate::{layout, theme, Finder};

/// Dossier d'ouverture choisi dans les reglages (valeur de `startdir`).
pub fn start_folder() -> std::path::PathBuf {
    let home = crate::fs::home_dir();
    match layout::get_setting("startdir").as_deref() {
        Some("documents") => home.join("Documents"),
        Some("desktop") => home.join("Desktop"),
        Some("downloads") => home.join("Downloads"),
        _ => home,
    }
}

/// Vrai si le bouton de barre d'outils `key` est affiche (tout est affiche par defaut).
pub fn toolbar_on(key: &str) -> bool {
    layout::get_setting(&format!("tb_{key}")).as_deref() != Some("0")
}

/// Vrai si l'element de barre laterale `key` est affiche (tout est affiche par defaut).
pub fn sidebar_on(key: &str) -> bool {
    layout::get_setting(&format!("sb_{key}")).as_deref() != Some("0")
}

impl Finder {
    pub fn toggle_settings(&mut self, cx: &mut Context<Self>) {
        self.settings_open = !self.settings_open;
        cx.notify();
    }

    fn flip_setting(&mut self, key: &str, default_on: bool, cx: &mut Context<Self>) {
        let on = layout::get_setting(key).map(|v| v == "1").unwrap_or(default_on);
        layout::set_setting(key, if on { "0" } else { "1" });
        self.places = crate::sidebar::default_places();
        if key == "hidden" {
            self.show_hidden = !on;
            self.reload();
        }
        if key == "apps_all" && self.cwd.to_str() == Some(crate::fs::APPS) {
            self.reload();
        }
        cx.notify();
    }

    pub fn render_settings(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        if !self.settings_open {
            return None;
        }
        let start = layout::get_setting("startdir").unwrap_or_else(|| "home".into());
        let header = |t: &'static str| div().text_size(px(12.)).font_weight(gpui::FontWeight::SEMIBOLD).text_color(rgb(theme::TEXT_SECONDARY)).child(t);
        let check = |id: &'static str, label: &'static str, key: &'static str, default_on: bool, cx: &mut Context<Self>| {
            let on = layout::get_setting(key).map(|v| v == "1").unwrap_or(default_on);
            div()
                .id(id)
                .flex()
                .items_center()
                .gap(px(8.))
                .text_size(px(13.))
                .text_color(rgb(theme::TEXT))
                .child(
                    div()
                        .size(px(16.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(4.))
                        .text_size(px(11.))
                        .border_1()
                        .border_color(rgb(0xb0b0b6))
                        .when(on, |d| {
                            d.bg(rgb(theme::SELECTION)).text_color(rgb(theme::SELECTION_TEXT)).border_color(rgb(theme::SELECTION)).child("\u{2713}")
                        }),
                )
                .child(label)
                .on_click(cx.listener(move |this, _, _, cx| this.flip_setting(key, default_on, cx)))
        };
        let radio = |id: &'static str, label: &'static str, value: &'static str, current: &str, cx: &mut Context<Self>| {
            let on = current == value;
            div()
                .id(id)
                .flex()
                .items_center()
                .gap(px(8.))
                .text_size(px(13.))
                .text_color(rgb(theme::TEXT))
                .child(
                    div()
                        .size(px(16.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded_full()
                        .border_1()
                        .border_color(rgb(if on { theme::SELECTION } else { 0xb0b0b6 }))
                        .when(on, |d| d.child(div().size(px(8.)).rounded_full().bg(rgb(theme::SELECTION)))),
                )
                .child(label)
                .on_click(cx.listener(move |this, _, _, cx| {
                    layout::set_setting("startdir", value);
                    cx.notify();
                    let _ = this;
                }))
        };
        Some(
            div()
                .id("settings-backdrop")
                .absolute()
                .inset_0()
                .flex()
                .justify_center()
                .items_start()
                .pt(px(70.))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _, _, cx| {
                        this.settings_open = false;
                        cx.notify();
                    }),
                )
                .child(
                    div()
                        .w(px(380.))
                        .p(px(20.))
                        .flex()
                        .flex_col()
                        .gap(px(10.))
                        .rounded(px(12.))
                        .bg(rgba(0xf6f6f6fc))
                        .border_1()
                        .border_color(rgb(0xd9d9de))
                        .shadow_lg()
                        .occlude()
                        .child(div().text_size(px(15.)).font_weight(gpui::FontWeight::SEMIBOLD).text_color(rgb(theme::TEXT)).child("R\u{e9}glages"))
                        .child(header("Le Finder s'ouvre sur"))
                        .child(radio("st-home", "Dossier personnel", "home", &start, cx))
                        .child(radio("st-docs", "Documents", "documents", &start, cx))
                        .child(radio("st-desk", "Bureau", "desktop", &start, cx))
                        .child(radio("st-down", "T\u{e9}l\u{e9}chargements", "downloads", &start, cx))
                        .child(div().h(px(1.)).w_full().bg(rgb(0xdcdcdf)))
                        .child(header("G\u{e9}n\u{e9}ral"))
                        .child(check("st-hidden", "Afficher les fichiers cach\u{e9}s", "hidden", false, cx))
                        .child(check("st-appsall", "Afficher les applications syst\u{e8}me", "apps_all", false, cx))
                        .child(div().h(px(1.)).w_full().bg(rgb(0xdcdcdf)))
                        .child(header("Barre d'outils"))
                        .child(check("st-tb-views", "Modes d'affichage", "tb_views", true, cx))
                        .child(check("st-tb-group", "Regroupement", "tb_group", true, cx))
                        .child(check("st-tb-share", "Partage", "tb_share", true, cx))
                        .child(check("st-tb-tags", "\u{c9}tiquettes", "tb_tags", true, cx))
                        .child(check("st-tb-more", "Actions (\u{2026})", "tb_more", true, cx))
                        .child(check("st-tb-search", "Recherche", "tb_search", true, cx))
                        .child(div().h(px(1.)).w_full().bg(rgb(0xdcdcdf)))
                        .child(header("\u{c9}l\u{e9}ments de la barre lat\u{e9}rale"))
                        .child(check("st-sb-apps", "Applications", "sb_apps", true, cx))
                        .child(check("st-sb-tags", "\u{c9}tiquettes", "sb_tags", true, cx))
                        .child(check("st-sb-net", "R\u{e9}seau et Corbeille", "sb_net", true, cx))
                        .child(div().text_size(px(11.)).text_color(rgb(theme::TEXT_SECONDARY)).child("Le dossier d'ouverture s'applique au prochain lancement.")),
                )
                .into_any_element(),
        )
    }
}
