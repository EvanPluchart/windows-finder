//! Page de reglages (Ctrl+,) : General, Apparence, Barre laterale, Barre d'outils, Explorateur, Avance.

use gpui::{
    AnyElement, Context, InteractiveElement, IntoElement, MouseButton, ParentElement, StatefulInteractiveElement, Styled,
    div, prelude::*, px,
};

use crate::theme::{rgb, rgba};
use crate::{layout, prefs, theme, Finder};

const TABS: [&str; 6] = ["G\u{e9}n\u{e9}ral", "Apparence", "Barre lat\u{e9}rale", "Barre d'outils", "Explorateur", "Avanc\u{e9}"];

/// Dossier d'ouverture choisi dans les reglages (valeur de `startdir`).
pub fn start_folder() -> std::path::PathBuf {
    let home = crate::fs::home_dir();
    match layout::get_setting("startdir").as_deref() {
        Some("documents") => home.join(crate::i18n::t("Documents")),
        Some("desktop") => home.join("Desktop"),
        Some("downloads") => home.join("Downloads"),
        Some("last") => layout::get_setting("lastdir").map(std::path::PathBuf::from).filter(|p| p.is_dir()).unwrap_or(home),
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

/// Applique le theme et la couleur d'accentuation choisis : clair, sombre ou systeme.
pub fn apply_theme() {
    theme::set_dark(theme::resolve_dark(layout::get_setting("theme").as_deref()));
    theme::set_accent(&layout::get_setting("accent").unwrap_or_default());
}

impl Finder {
    pub fn toggle_settings(&mut self, cx: &mut Context<Self>) {
        self.settings_open = !self.settings_open;
        if self.settings_open {
            self.explorer_default = crate::explorer::is_default();
            self.ctx_entry = crate::explorer::context_entry();
        }
        cx.notify();
    }

    fn refresh_all_windows(cx: &mut Context<Self>) {
        cx.defer(|app| {
            for w in app.windows() {
                let _ = w.update(app, |_, window, _| window.refresh());
            }
        });
    }

    /// Effets d'un reglage qui vient de changer.
    fn setting_changed(&mut self, key: &str, cx: &mut Context<Self>) {
        match key {
            "theme" | "accent" => {
                apply_theme();
                Self::refresh_all_windows(cx);
            }
            "lang" => {
                crate::i18n::apply();
                self.places = crate::sidebar::default_places();
                self.reload();
                Self::refresh_all_windows(cx);
            }
            "hidden" => {
                self.show_hidden = layout::get_setting("hidden").as_deref() == Some("1");
                self.reload();
            }
            "apps_all" => {
                if self.cwd.to_str() == Some(crate::fs::APPS) {
                    self.reload();
                }
            }
            "pathbar" => self.show_path_bar = layout::get_setting("pathbar").as_deref() != Some("0"),
            "statusbar" => self.show_status_bar = layout::get_setting("statusbar").as_deref() != Some("0"),
            _ => {}
        }
        if key.starts_with("sb_") {
            self.places = crate::sidebar::default_places();
        }
        cx.notify();
    }

    fn flip_setting(&mut self, key: &'static str, default_on: bool, cx: &mut Context<Self>) {
        let on = layout::get_setting(key).map(|v| v == "1").unwrap_or(default_on);
        layout::set_setting(key, if on { "0" } else { "1" });
        self.setting_changed(key, cx);
    }

    fn set_choice(&mut self, key: &'static str, value: &'static str, cx: &mut Context<Self>) {
        layout::set_setting(key, value);
        self.setting_changed(key, cx);
    }

    /// Modifie une valeur par defaut des options d'affichage (mode, tri, dates relatives).
    fn set_default_pref(&mut self, f: impl FnOnce(&mut prefs::Prefs), cx: &mut Context<Self>) {
        f(&mut self.prefs.defaults);
        self.prefs.save();
        cx.notify();
    }

    pub fn render_settings(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        if !self.settings_open {
            return None;
        }
        let tab = self.settings_tab;
        let sep = || div().h(px(1.)).w_full().bg(rgb(0xdcdcdf));
        let header = |t: &'static str| div().text_size(px(12.)).font_weight(gpui::FontWeight::SEMIBOLD).text_color(rgb(theme::TEXT_SECONDARY)).child(t);
        let note = |t: &'static str| div().text_size(px(11.)).text_color(rgb(theme::TEXT_SECONDARY)).child(t);
        let check_ui = |id: &'static str, label: &'static str, on: bool| {
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
                        .flex_none()
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
        };
        // Case a cocher liee a un reglage de settings.txt.
        let check = |id: &'static str, label: &'static str, key: &'static str, default_on: bool, cx: &mut Context<Self>| {
            let on = layout::get_setting(key).map(|v| v == "1").unwrap_or(default_on);
            check_ui(id, label, on).cursor_pointer().on_click(cx.listener(move |this, _, _, cx| this.flip_setting(key, default_on, cx)))
        };
        let pill = |on: bool| {
            div()
                .px(px(12.))
                .h(px(26.))
                .flex()
                .items_center()
                .rounded_full()
                .text_size(px(12.))
                .text_color(rgb(if on { theme::SELECTION_TEXT } else { theme::TEXT }))
                .when(on, |d| d.bg(rgb(theme::SELECTION)))
                .when(!on, |d| d.bg(rgb(0xe9e9ec)))
        };
        // Rangee de boutons exclusifs liee a un reglage de settings.txt.
        let choice = |id: &'static str, key: &'static str, options: Vec<(&'static str, &'static str)>, default: &'static str, cx: &mut Context<Self>| {
            let current = layout::get_setting(key).unwrap_or_else(|| default.to_string());
            div().flex().flex_wrap().gap(px(6.)).children(options.into_iter().enumerate().map(|(i, (value, label))| {
                pill(current == value)
                    .id((id, i))
                    .child(label)
                    .cursor_pointer().on_click(cx.listener(move |this, _, _, cx| this.set_choice(key, value, cx)))
            }))
        };
        let button = |id: &'static str, label: &'static str, cx: &mut Context<Self>, f: fn(&mut Finder, &mut Context<Finder>)| {
            div()
                .id(id)
                .px(px(14.))
                .h(px(28.))
                .flex()
                .items_center()
                .rounded(px(7.))
                .bg(rgb(0xe9e9ec))
                .hover(|s| s.bg(rgb(theme::HOVER)))
                .text_size(px(13.))
                .text_color(rgb(theme::TEXT))
                .child(label)
                .cursor_pointer().on_click(cx.listener(move |this, _, _, cx| f(this, cx)))
        };

        let defaults = &self.prefs.defaults;
        let mut body = div().flex().flex_col().gap(px(12.));
        match tab {
            0 => {
                let view = defaults.view.clone();
                let views: [(&'static str, &'static str); 4] = [("icons", crate::i18n::t("Ic\u{f4}nes")), ("list", crate::i18n::t("Liste")), ("columns", crate::i18n::t("Colonnes")), ("gallery", crate::i18n::t("Galerie"))];
                let sorts: [(&'static str, u8); 5] = [(crate::i18n::t("Nom"), 0), (crate::i18n::t("Type"), 3), (crate::i18n::t("Modifi\u{e9}"), 1), (crate::i18n::t("Cr\u{e9}\u{e9}"), 4), (crate::i18n::t("Taille"), 2)];
                body = body
                    .child(header(crate::i18n::t("Le Finder s'ouvre sur")))
                    .child(choice(
                        "st-start",
                        "startdir",
                        vec![("home", crate::i18n::t("Dossier personnel")), ("documents", crate::i18n::t("Documents")), ("desktop", crate::i18n::t("Bureau")), ("downloads", crate::i18n::t("T\u{e9}l\u{e9}chargements")), ("last", crate::i18n::t("Dernier dossier"))],
                        "home",
                        cx,
                    ))
                    .child(sep())
                    .child(header(crate::i18n::t("Langue")))
                    .child(choice(
                        "st-lang",
                        "lang",
                        vec![("auto", crate::i18n::t("Syst\u{e8}me")), ("fr", "Fran\u{e7}ais"), ("en", "English"), ("es", "Espa\u{f1}ol"), ("de", "Deutsch")],
                        "auto",
                        cx,
                    ))
                    .child(sep())
                    .child(header(crate::i18n::t("Mode d'affichage par d\u{e9}faut")))
                    .child(div().flex().gap(px(6.)).children(views.into_iter().enumerate().map(|(i, (v, label))| {
                        pill(view == v)
                            .id(("st-view", i))
                            .child(label)
                            .cursor_pointer().on_click(cx.listener(move |this, _, _, cx| this.set_default_pref(|p| p.view = v.to_string(), cx)))
                    })))
                    .child(header(crate::i18n::t("Trier par d\u{e9}faut")))
                    .child(div().flex().flex_wrap().gap(px(6.)).children(sorts.into_iter().enumerate().map(|(i, (label, code))| {
                        pill(defaults.sort == code)
                            .id(("st-sort", i))
                            .child(label)
                            .cursor_pointer().on_click(cx.listener(move |this, _, _, cx| this.set_default_pref(|p| p.sort = code, cx)))
                    })))
                    .child(check_ui("st-rel", crate::i18n::t("Dates relatives (\u{ab} Hier \u{e0} 14:03 \u{bb})"), defaults.rel_dates).cursor_pointer().on_click(cx.listener(|this, _, _, cx| {
                        this.set_default_pref(|p| p.rel_dates = !p.rel_dates, cx)
                    })))
                    .child(note(crate::i18n::t("Ces valeurs s'appliquent aux dossiers sans r\u{e9}glage propre (Ctrl+J).")))
                    .child(sep())
                    .child(header(crate::i18n::t("Fichiers")))
                    .child(check("st-hidden", crate::i18n::t("Afficher les fichiers cach\u{e9}s et syst\u{e8}me"), "hidden", false, cx))
                    .child(sep())
                    .child(header(crate::i18n::t("Fen\u{ea}tre")))
                    .child(check("st-pathbar", crate::i18n::t("Afficher la barre de chemin"), "pathbar", true, cx))
                    .child(check("st-statusbar", crate::i18n::t("Afficher la barre d'\u{e9}tat"), "statusbar", true, cx))
                    .child(sep())
                    .child(header(crate::i18n::t("Aper\u{e7}u rapide")))
                    .child(check("st-qlauto", crate::i18n::t("Lire les vid\u{e9}os automatiquement \u{e0} l'ouverture"), "qlauto", true, cx));
            }
            1 => {
                let current = layout::get_setting("accent").unwrap_or_else(|| "blue".into());
                body = body
                    .child(header(crate::i18n::t("Th\u{e8}me")))
                    .child(choice("st-theme", "theme", vec![("system", crate::i18n::t("Syst\u{e8}me")), ("light", crate::i18n::t("Clair")), ("dark", crate::i18n::t("Sombre"))], "system", cx))
                    .child(note(crate::i18n::t("\u{ab} Syst\u{e8}me \u{bb} suit le mode d'applications de Windows et change en direct.")))
                    .child(sep())
                    .child(header(crate::i18n::t("Couleur d'accentuation")))
                    .child(div().flex().gap(px(10.)).children(theme::ACCENTS.iter().enumerate().map(|(i, a)| {
                        let (code, label, light, _, _, _) = *a;
                        let label = crate::i18n::td(label);
                        let on = current == code;
                        div()
                            .id(("st-accent", i))
                            .flex()
                            .flex_col()
                            .items_center()
                            .gap(px(4.))
                            .child(
                                div()
                                    .size(px(26.))
                                    .rounded_full()
                                    .bg(gpui::rgb(light))
                                    .border_2()
                                    .border_color(rgb(if on { 0x262626 } else { 0xffffff }))
                                    .when(on, |d| d.shadow_sm()),
                            )
                            .child(div().text_size(px(10.)).text_color(rgb(theme::TEXT_SECONDARY)).child(label))
                            .cursor_pointer().on_click(cx.listener(move |this, _, _, cx| this.set_choice("accent", code, cx)))
                    })))
                    .child(note(crate::i18n::t("Couleur de la s\u{e9}lection, des ic\u{f4}nes de la barre lat\u{e9}rale et des boutons actifs.")));
            }
            2 => {
                body = body
                    .child(header(crate::i18n::t("Favoris")))
                    .child(check("st-sb-recents", crate::i18n::t("R\u{e9}cents"), "sb_recents", true, cx))
                    .child(check("st-sb-apps", crate::i18n::t("Applications"), "sb_apps", true, cx))
                    .child(check("st-sb-desktop", crate::i18n::t("Bureau"), "sb_desktop", true, cx))
                    .child(check("st-sb-docs", crate::i18n::t("Documents"), "sb_docs", true, cx))
                    .child(check("st-sb-downloads", crate::i18n::t("T\u{e9}l\u{e9}chargements"), "sb_downloads", true, cx))
                    .child(check("st-sb-media", crate::i18n::t("Images, Musique et Vid\u{e9}os"), "sb_media", true, cx))
                    .child(sep())
                    .child(header(crate::i18n::t("Emplacements")))
                    .child(check("st-sb-cloud", "OneDrive", "sb_cloud", true, cx))
                    .child(check("st-sb-home", crate::i18n::t("Dossier personnel"), "sb_home", true, cx))
                    .child(check("st-sb-pc", "Ce PC", "sb_pc", true, cx))
                    .child(check("st-sb-disks", crate::i18n::t("Disques"), "sb_disks", true, cx))
                    .child(check("st-sb-net", crate::i18n::t("R\u{e9}seau et Corbeille"), "sb_net", true, cx))
                    .child(sep())
                    .child(header(crate::i18n::t("\u{c9}tiquettes")))
                    .child(check("st-sb-tags", crate::i18n::t("Afficher les \u{e9}tiquettes"), "sb_tags", true, cx));
            }
            3 => {
                body = body
                    .child(header(crate::i18n::t("Boutons de la barre d'outils")))
                    .child(check("st-tb-views", crate::i18n::t("Modes d'affichage"), "tb_views", true, cx))
                    .child(check("st-tb-group", crate::i18n::t("Regroupement"), "tb_group", true, cx))
                    .child(check("st-tb-share", crate::i18n::t("Partage"), "tb_share", true, cx))
                    .child(check("st-tb-tags", crate::i18n::t("\u{c9}tiquettes"), "tb_tags", true, cx))
                    .child(check("st-tb-more", crate::i18n::t("Actions (\u{2026})"), "tb_more", true, cx))
                    .child(check("st-tb-search", crate::i18n::t("Recherche"), "tb_search", true, cx))
                    .child(note(crate::i18n::t("Astuce : un clic droit sur la barre d'outils ouvre ces r\u{e9}glages.")));
            }
            4 => {
                let is_default = self.explorer_default;
                let ctx_entry = self.ctx_entry;
                body = body
                    .child(header(crate::i18n::t("Gestionnaire de dossiers par d\u{e9}faut")))
                    .child(div().text_size(px(13.)).text_color(rgb(theme::TEXT)).child(if is_default {
                        crate::i18n::t("Windows Finder ouvre les dossiers (double-clic depuis un autre programme, \u{ab} Afficher dans le dossier \u{bb}\u{2026}).")
                    } else {
                        crate::i18n::t("Les dossiers s'ouvrent dans l'Explorateur Windows.")
                    }))
                    .child(if is_default {
                        button("st-ex-off", crate::i18n::t("R\u{e9}tablir l'Explorateur Windows"), cx, |this, cx| {
                            crate::explorer::set_default(false);
                            this.explorer_default = crate::explorer::is_default();
                            cx.notify();
                        })
                        .into_any_element()
                    } else {
                        button("st-ex-on", crate::i18n::t("D\u{e9}finir Windows Finder par d\u{e9}faut"), cx, |this, cx| {
                            if !crate::explorer::set_default(true) {
                                this.show_notice(crate::i18n::t("Impossible de modifier le registre."), cx);
                            }
                            this.explorer_default = crate::explorer::is_default();
                            cx.notify();
                        })
                        .into_any_element()
                    })
                    .child(note(crate::i18n::t("Modifie seulement votre profil (HKEY_CURRENT_USER), sans droits administrateur. Win+E continue d'ouvrir l'Explorateur Windows.")))
                    .child(sep())
                    .child(header(crate::i18n::t("Menu contextuel de l'Explorateur")))
                    .child(check_ui("st-ex-ctx", crate::i18n::t("Ajouter \u{ab} Ouvrir dans Windows Finder \u{bb} au clic droit sur un dossier"), ctx_entry).cursor_pointer().on_click(cx.listener(
                        |this, _, _, cx| {
                            let target = !this.ctx_entry;
                            crate::explorer::set_context_entry(target);
                            this.ctx_entry = crate::explorer::context_entry();
                            cx.notify();
                        },
                    )))
                    .child(note(crate::i18n::t("Fonctionne aussi dans le menu \u{ab} Afficher plus d'options \u{bb} de Windows 11.")));
            }
            _ => {
                body = body
                    .child(header(crate::i18n::t("Applications")))
                    .child(check("st-appsall", crate::i18n::t("Afficher les applications syst\u{e8}me dans \u{ab} Applications \u{bb}"), "apps_all", false, cx))
                    .child(sep())
                    .child(header(crate::i18n::t("Donn\u{e9}es")))
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .gap(px(8.))
                            .child(button("st-reset-views", crate::i18n::t("R\u{e9}initialiser les options d'affichage"), cx, |this, cx| {
                                this.prefs = crate::prefs::Store::default();
                                this.prefs.save();
                                this.layouts.clear();
                                crate::layout::save(&this.layouts);
                                this.reload();
                                this.show_notice(crate::i18n::t("Options d'affichage r\u{e9}initialis\u{e9}es."), cx);
                            }))
                            .child(button("st-reset-all", crate::i18n::t("R\u{e9}initialiser tous les r\u{e9}glages"), cx, |this, cx| {
                                if let Some(f) = std::env::var_os("APPDATA").map(|a| std::path::PathBuf::from(a).join("winfinder").join("settings.txt")) {
                                    let _ = std::fs::remove_file(f);
                                }
                                apply_theme();
                                this.show_sidebar = true;
                                this.show_path_bar = true;
                                this.show_status_bar = true;
                                this.show_hidden = false;
                                this.sidebar_w = theme::SIDEBAR_WIDTH;
                                this.places = crate::sidebar::default_places();
                                this.reload();
                                this.show_notice(crate::i18n::t("R\u{e9}glages r\u{e9}initialis\u{e9}s."), cx);
                            }))
                            .child(button("st-open-data", crate::i18n::t("Ouvrir le dossier des donn\u{e9}es"), cx, |_, _| {
                                if let Some(d) = std::env::var_os("APPDATA").map(|a| std::path::PathBuf::from(a).join("winfinder")) {
                                    let _ = std::fs::create_dir_all(&d);
                                    let _ = std::process::Command::new("explorer.exe").arg(d).spawn();
                                }
                            })),
                    )
                    .child(note(crate::i18n::t("Options d'affichage, \u{e9}tiquettes et r\u{e9}glages sont dans %APPDATA%\\winfinder.")))
                    .child(sep())
                    .child(header(crate::i18n::t("\u{c0} propos")))
                    .child(div().text_size(px(13.)).text_color(rgb(theme::TEXT)).child(crate::i18n::t("Windows Finder 0.1.0 \u{2014} un Finder pour Windows, \u{e9}crit en Rust.")));
            }
        }

        let nav = div().w(px(170.)).flex_none().flex().flex_col().gap(px(2.)).p(px(10.)).children(TABS.iter().enumerate().map(|(i, name)| {
            let name = &crate::i18n::td(name);
            let on = i == tab;
            div()
                .id(("st-tab", i))
                .h(px(30.))
                .px(px(10.))
                .flex()
                .items_center()
                .rounded(px(8.))
                .text_size(px(13.))
                .text_color(rgb(if on { theme::SELECTION_TEXT } else { theme::TEXT }))
                .when(on, |d| d.bg(rgb(theme::SELECTION)))
                .when(!on, |d| d.hover(|s| s.bg(rgb(theme::HOVER))))
                .child(name.clone())
                .cursor_pointer().on_click(cx.listener(move |this, _, _, cx| {
                    this.settings_tab = i;
                    cx.notify();
                }))
        }));

        Some(
            div()
                .id("settings-backdrop")
                .absolute()
                .inset_0()
                .flex()
                .justify_center()
                .items_center()
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _, _, cx| {
                        this.settings_open = false;
                        cx.notify();
                    }),
                )
                .child(
                    div()
                        .w(px(700.))
                        .relative()
                        .h(px(470.))
                        .max_w_full()
                        .max_h_full()
                        .flex()
                        .rounded(px(14.))
                        .bg(rgba(0xf6f6f6fc))
                        .border_1()
                        .border_color(rgb(0xd9d9de))
                        .shadow_lg()
                        .overflow_hidden()
                        .occlude()
                        .child(
                            div()
                                .h_full()
                                .flex()
                                .flex_col()
                                .border_r_1()
                                .border_color(rgb(0xdcdcdf))
                                .child(
                                    div()
                                        .px(px(20.))
                                        .pt(px(18.))
                                        .pb(px(6.))
                                        .text_size(px(15.))
                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                        .text_color(rgb(theme::TEXT))
                                        .child(crate::i18n::t("R\u{e9}glages")),
                                )
                                .child(nav),
                        )
                        .child(
                            div()
                                .id("settings-body")
                                .flex_1()
                                .min_w_0()
                                .h_full()
                                .overflow_y_scroll()
                                .p(px(24.))
                                .child(
                                    div()
                                        .text_size(px(17.))
                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                        .text_color(rgb(theme::TEXT))
                                        .pb(px(14.))
                                        .child(crate::i18n::td(TABS[tab])),
                                )
                                .child(body),
                        )
                        .child(
                            div()
                                .id("settings-close")
                                .absolute()
                                .top(px(12.))
                                .right(px(12.))
                                .size(px(26.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded_full()
                                .cursor_pointer()
                                .bg(rgb(0xe9e9ec))
                                .hover(|s| s.bg(rgb(theme::HOVER)))
                                .text_size(px(12.))
                                .text_color(rgb(theme::TEXT_SECONDARY))
                                .child("\u{2715}")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.settings_open = false;
                                    cx.notify();
                                })),
                        ),
                )
                .into_any_element(),
        )
    }
}
