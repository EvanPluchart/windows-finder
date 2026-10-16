//! Options d'affichage par dossier (Ctrl+J) : mode, tri, taille des icones, espacement, taille du texte,
//! colonnes de la liste, dates relatives. Memorisees par dossier dans `%APPDATA%\winfinder\folders.tsv`,
//! avec des valeurs par defaut modifiables ("Utiliser par defaut").

use std::collections::HashMap;
use std::path::PathBuf;

use gpui::{
    AnyElement, Context, InteractiveElement, IntoElement, MouseButton, ParentElement, StatefulInteractiveElement, Styled,
    div, prelude::*, px,
};

use crate::fs::SortKey;
use crate::theme::{rgb, rgba};
use crate::{theme, Finder, ViewMode};

#[derive(Clone, PartialEq, Debug)]
pub struct Prefs {
    pub view: String,
    pub sort: u8,
    pub group: u8,
    pub asc: bool,
    pub icon_size: u16,
    pub spacing: u8,
    pub text: u8,
    pub item_info: bool,
    pub rel_dates: bool,
    pub preview: bool,
    pub col_modified: bool,
    pub col_created: bool,
    pub col_size: bool,
    pub col_kind: bool,
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            view: "icons".into(),
            sort: 0,
            group: 0,
            asc: true,
            icon_size: 64,
            spacing: 0,
            text: 13,
            item_info: false,
            rel_dates: true,
            preview: true,
            col_modified: true,
            col_created: false,
            col_size: true,
            col_kind: true,
        }
    }
}

impl Prefs {
    fn encode(&self) -> String {
        format!(
            "view={};sort={};grp={};asc={};icon={};space={};text={};info={};rel={};pv={};cm={};cc={};cs={};ck={}",
            self.view, self.sort, self.group, self.asc as u8, self.icon_size, self.spacing, self.text, self.item_info as u8,
            self.rel_dates as u8, self.preview as u8, self.col_modified as u8, self.col_created as u8, self.col_size as u8, self.col_kind as u8
        )
    }

    fn decode(s: &str) -> Self {
        let mut p = Self::default();
        for kv in s.split(';') {
            let Some((k, v)) = kv.split_once('=') else { continue };
            let b = v == "1";
            match k {
                "view" => p.view = v.to_string(),
                "sort" => p.sort = v.parse().unwrap_or(0),
                "grp" => p.group = v.parse().unwrap_or(0),
                "asc" => p.asc = b,
                // Bornes des curseurs des options d'affichage : un fichier modifie a la main ne peut pas
                // demander une miniature de 65 535 px.
                "icon" => p.icon_size = v.parse().unwrap_or(64u16).clamp(16, 256),
                "space" => p.spacing = v.parse().unwrap_or(0u8).min(3),
                "text" => p.text = v.parse().unwrap_or(13u8).clamp(9, 20),
                "info" => p.item_info = b,
                "rel" => p.rel_dates = b,
                "pv" => p.preview = b,
                "cm" => p.col_modified = b,
                "cc" => p.col_created = b,
                "cs" => p.col_size = b,
                "ck" => p.col_kind = b,
                _ => {}
            }
        }
        p
    }

    pub fn view_mode(&self) -> ViewMode {
        match self.view.as_str() {
            "list" => ViewMode::List,
            "columns" => ViewMode::Columns,
            "gallery" => ViewMode::Gallery,
            _ => ViewMode::Icons,
        }
    }

    pub fn sort_key(&self) -> SortKey {
        match self.sort {
            1 => SortKey::Modified,
            2 => SortKey::Size,
            3 => SortKey::Kind,
            4 => SortKey::Created,
            _ => SortKey::Name,
        }
    }
}

pub fn sort_code(k: SortKey) -> u8 {
    match k {
        SortKey::Name => 0,
        SortKey::Modified => 1,
        SortKey::Size => 2,
        SortKey::Kind => 3,
        SortKey::Created => 4,
    }
}

pub fn view_name(v: ViewMode) -> &'static str {
    match v {
        ViewMode::Icons => "icons",
        ViewMode::List => "list",
        ViewMode::Columns => "columns",
        ViewMode::Gallery => "gallery",
    }
}

#[derive(Default)]
pub struct Store {
    pub defaults: Prefs,
    pub folders: HashMap<PathBuf, Prefs>,
    /// Le fichier tel qu'on l'a lu ou ecrit la derniere fois (valeurs encodees). Chaque fenetre a sa copie des
    /// options : a l'enregistrement, seules nos modifications depuis ce point sont reportees sur la version du
    /// disque, pour ne pas effacer celles d'une autre fenetre ou d'un autre processus.
    base: (String, HashMap<PathBuf, String>),
}

fn file() -> Option<PathBuf> {
    Some(PathBuf::from(std::env::var_os("APPDATA")?).join("winfinder").join("folders.tsv"))
}

/// Contenu brut de folders.tsv : options par defaut (`*`) et par dossier, encodees.
fn read_raw() -> (Option<String>, HashMap<PathBuf, String>) {
    let mut defaults = None;
    let mut folders = HashMap::new();
    let text = file().and_then(|f| std::fs::read_to_string(f).ok()).unwrap_or_default();
    for line in text.trim_start_matches('\u{feff}').lines() {
        let Some((path, rest)) = line.split_once('\t') else { continue };
        if path == "*" {
            defaults = Some(rest.to_string());
        } else {
            folders.insert(PathBuf::from(path), rest.to_string());
        }
    }
    (defaults, folders)
}

impl Store {
    pub fn load() -> Self {
        let mut s = Store::default();
        // Ancien reglage global du mode d'affichage (settings.txt).
        if let Some(v) = crate::layout::load_view() {
            s.defaults.view = v;
        }
        let (defaults, folders) = read_raw();
        if let Some(d) = &defaults {
            s.defaults = Prefs::decode(d);
        }
        s.folders = folders.iter().map(|(p, v)| (p.clone(), Prefs::decode(v))).collect();
        s.base = (s.defaults.encode(), folders);
        s
    }

    /// Enregistre nos changements par-dessus la version actuelle du fichier, puis adopte le resultat.
    pub fn save(&mut self) {
        let (disk_defaults, mut disk) = read_raw();
        let (base_defaults, base) = std::mem::take(&mut self.base);
        let ours = self.defaults.encode();
        let defaults = if ours != base_defaults { ours } else { disk_defaults.unwrap_or(base_defaults) };
        let keys: std::collections::HashSet<PathBuf> = self.folders.keys().chain(base.keys()).cloned().collect();
        for k in keys {
            let mine = self.folders.get(&k).map(Prefs::encode);
            if mine.as_ref() != base.get(&k) {
                match mine {
                    Some(v) => disk.insert(k, v),
                    None => disk.remove(&k),
                };
            }
        }
        self.write(defaults, disk);
    }

    /// Remplace tout le fichier par nos options (reinitialisation).
    pub fn save_replace(&mut self) {
        let folders = self.folders.iter().map(|(p, v)| (p.clone(), v.encode())).collect();
        self.write(self.defaults.encode(), folders);
    }

    fn write(&mut self, defaults: String, folders: HashMap<PathBuf, String>) {
        let mut out = format!("*\t{defaults}\n");
        for (p, prefs) in &folders {
            out.push_str(&format!("{}\t{}\n", p.display(), prefs));
        }
        if let Some(f) = file() {
            crate::layout::write_settings_file(&f, &out);
        }
        self.defaults = Prefs::decode(&defaults);
        self.folders = folders.iter().map(|(p, v)| (p.clone(), Prefs::decode(v))).collect();
        self.base = (defaults, folders);
    }
}

impl Finder {
    /// Options effectives du dossier courant : les siennes, sinon les valeurs par defaut.
    pub fn eff(&self) -> Prefs {
        self.prefs.folders.get(&self.cwd).cloned().unwrap_or_else(|| self.prefs.defaults.clone())
    }

    /// Applique les options du dossier courant (mode, tri) ; appelee a chaque lecture du dossier.
    pub fn apply_folder_prefs(&mut self) {
        let p = self.eff();
        if !self.view_forced {
            self.view = p.view_mode();
        }
        crate::fs::set_sort(p.sort_key(), p.asc);
        crate::fs::set_group(p.group);
    }

    /// Modifie une option du dossier courant, l'enregistre et relit le dossier si le tri change.
    pub fn set_pref(&mut self, f: impl FnOnce(&mut Prefs), cx: &mut Context<Self>) {
        let before = self.eff();
        let mut p = before.clone();
        f(&mut p);
        if p == before {
            return;
        }
        let resort = (p.sort, p.asc, p.group) != (before.sort, before.asc, before.group);
        self.prefs.folders.insert(self.cwd.clone(), p.clone());
        self.prefs.save();
        if resort {
            crate::fs::set_sort(p.sort_key(), p.asc);
            crate::fs::set_group(p.group);
            self.resort_in_place();
        }
        cx.notify();
    }

    /// Nouveau mode d'affichage pour ce dossier (et par defaut pour les prochains dossiers).
    pub fn remember_view(&mut self, v: ViewMode) {
        let mut p = self.eff();
        p.view = view_name(v).to_string();
        self.prefs.folders.insert(self.cwd.clone(), p.clone());
        self.prefs.defaults.view = p.view;
        self.prefs.save();
    }

    pub fn remember_sort(&mut self) {
        let (k, asc) = crate::fs::sort();
        let mut p = self.eff();
        p.sort = sort_code(k);
        p.asc = asc;
        self.prefs.folders.insert(self.cwd.clone(), p);
        self.prefs.save();
    }

    /// Dimensions d'une cellule de la vue Icones : (largeur, hauteur).
    pub fn cell(&self) -> (f32, f32) {
        let p = self.eff();
        let extra = p.spacing as f32 * 10.0;
        let text = (p.text as f32 - 13.0) * 2.0;
        (p.icon_size as f32 + 64.0 + extra, p.icon_size as f32 + 70.0 + extra + text.max(0.0) + if p.item_info { 16.0 } else { 0.0 })
    }

    pub fn toggle_view_options(&mut self, cx: &mut Context<Self>) {
        self.view_opts = !self.view_opts;
        cx.notify();
    }

    /// Panneau flottant "Options d'affichage" (Ctrl+J), adapte au mode courant.
    pub fn render_view_options(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        if !self.view_opts {
            return None;
        }
        let p = self.eff();
        let view = self.view;
        let name = crate::fs::display_name(&self.cwd);

        let header = |t: &'static str| div().text_size(px(12.)).text_color(rgb(theme::TEXT_SECONDARY)).child(t);
        let rule = || div().h(px(1.)).w_full().bg(rgb(0xe2e2e6));
        let check = |id: &'static str, label: &'static str, on: bool, cx: &mut Context<Self>, f: fn(&mut Prefs)| {
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
                        .when(on, |d| d.bg(rgb(theme::SELECTION)).text_color(rgb(theme::SELECTION_TEXT)).border_color(rgb(theme::SELECTION)).child("\u{2713}")),
                )
                .child(label)
                .cursor_pointer().on_click(cx.listener(move |this, _, _, cx| this.set_pref(f, cx)))
        };
        // Reglette a crans : cliquer un cran choisit la valeur.
        let slider = |id: &'static str, stops: Vec<(u32, bool)>, cx: &mut Context<Self>, setter: fn(&mut Prefs, u32)| {
            div().flex().items_center().h(px(22.)).children(stops.into_iter().enumerate().map(|(i, (value, current))| {
                div()
                    .id((id, i))
                    .flex_1()
                    .h_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .when(current, |d| d.size(px(14.)).rounded_full().bg(rgb(0xffffff)).border_1().border_color(rgb(0xb0b0b6)).shadow_sm())
                            .when(!current, |d| d.size(px(4.)).rounded_full().bg(rgb(0xb0b0b6))),
                    )
                    .cursor_pointer().on_click(cx.listener(move |this, _, _, cx| this.set_pref(|p| setter(p, value), cx)))
            }))
        };
        let seg = |id: &'static str, items: Vec<(&'static str, bool, u8)>, cx: &mut Context<Self>| {
            div().flex().p(px(2.)).rounded(px(8.)).bg(rgb(0xe9e9ec)).children(items.into_iter().map(|(label, on, code)| {
                div()
                    .id((id, code as usize))
                    .flex_1()
                    .h(px(24.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(6.))
                    .text_size(px(12.))
                    .text_color(rgb(theme::TEXT))
                    .when(on, |d| d.bg(rgb(0xffffff)).shadow_sm())
                    .child(label)
                    .cursor_pointer().on_click(cx.listener(move |this, _, _, cx| this.set_pref(|p| p.sort = code, cx)))
            }))
        };

        let group_seg = {
            let items: Vec<(&'static str, bool, u8)> = vec![
                (crate::i18n::t("Aucun"), p.group == 0, 0),
                (crate::i18n::t("Nom"), p.group == 1, 1),
                (crate::i18n::t("Type"), p.group == 2, 2),
                (crate::i18n::t("Modif."), p.group == 3, 3),
                (crate::i18n::t("Cr\u{e9}\u{e9}"), p.group == 4, 4),
                (crate::i18n::t("Taille"), p.group == 5, 5),
            ];
            div().flex().p(px(2.)).rounded(px(8.)).bg(rgb(0xe9e9ec)).children(items.into_iter().map(|(label, on, code)| {
                div()
                    .id(("vo-grp", code as usize))
                    .flex_1()
                    .h(px(24.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(6.))
                    .text_size(px(11.))
                    .text_color(rgb(theme::TEXT))
                    .when(on, |d| d.bg(rgb(0xffffff)).shadow_sm())
                    .child(label)
                    .cursor_pointer().on_click(cx.listener(move |this, _, _, cx| this.set_pref(|p| p.group = code, cx)))
            }))
        };
        let sort_seg = seg(
            "vo-sort",
            vec![
                (crate::i18n::t("Nom"), p.sort == 0, 0),
                (crate::i18n::t("Type"), p.sort == 3, 3),
                (crate::i18n::t("Modif."), p.sort == 1, 1),
                (crate::i18n::t("Cr\u{e9}\u{e9}"), p.sort == 4, 4),
                (crate::i18n::t("Taille"), p.sort == 2, 2),
            ],
            cx,
        );
        let asc_check = check("vo-asc", crate::i18n::t("Ordre croissant"), p.asc, cx, |p| p.asc = !p.asc);

        let mut body = div().flex().flex_col().gap(px(10.)).child(header(crate::i18n::t("Regrouper par"))).child(group_seg).child(header(crate::i18n::t("Trier par"))).child(sort_seg).child(asc_check).child(rule());

        match view {
            ViewMode::Icons => {
                let sizes = [32u32, 48, 64, 80, 96, 128];
                let spaces = [0u32, 1, 2, 3];
                let texts = [10u32, 11, 12, 13, 14, 16];
                body = body
                    .child(header(crate::i18n::t("Taille des ic\u{f4}nes")))
                    .child(div().text_size(px(13.)).text_color(rgb(theme::TEXT)).child(format!("{0}\u{d7}{0}", p.icon_size)))
                    .child(slider("vo-size", sizes.iter().map(|&s| (s, s == p.icon_size as u32)).collect(), cx, |p, v| p.icon_size = v as u16))
                    .child(header(crate::i18n::t("Espacement de la grille")))
                    .child(slider("vo-space", spaces.iter().map(|&s| (s, s == p.spacing as u32)).collect(), cx, |p, v| p.spacing = v as u8))
                    .child(header(crate::i18n::t("Taille du texte")))
                    .child(slider("vo-text", texts.iter().map(|&s| (s, s == p.text as u32)).collect(), cx, |p, v| p.text = v as u8))
                    .child(rule())
                    .child(check("vo-info", crate::i18n::t("Afficher les infos des \u{e9}l\u{e9}ments"), p.item_info, cx, |p| p.item_info = !p.item_info));
            }
            ViewMode::List => {
                let texts = [11u32, 12, 13, 14, 16];
                body = body
                    .child(header(crate::i18n::t("Taille du texte")))
                    .child(slider("vo-text", texts.iter().map(|&s| (s, s == p.text as u32)).collect(), cx, |p, v| p.text = v as u8))
                    .child(header(crate::i18n::t("Afficher les colonnes")))
                    .child(check("vo-cm", crate::i18n::t("Date de modification"), p.col_modified, cx, |p| p.col_modified = !p.col_modified))
                    .child(check("vo-cc", crate::i18n::t("Date de cr\u{e9}ation"), p.col_created, cx, |p| p.col_created = !p.col_created))
                    .child(check("vo-cs", crate::i18n::t("Taille"), p.col_size, cx, |p| p.col_size = !p.col_size))
                    .child(check("vo-ck", crate::i18n::t("Type"), p.col_kind, cx, |p| p.col_kind = !p.col_kind))
                    .child(rule())
                    .child(check("vo-rel", crate::i18n::t("Utiliser les dates relatives"), p.rel_dates, cx, |p| p.rel_dates = !p.rel_dates));
            }
            _ => {
                let texts = [11u32, 12, 13, 14, 16];
                body = body
                    .child(header(crate::i18n::t("Taille du texte")))
                    .child(slider("vo-text", texts.iter().map(|&s| (s, s == p.text as u32)).collect(), cx, |p, v| p.text = v as u8))
                    .child(rule())
                    .child(check("vo-preview", crate::i18n::t("Afficher le volet d'aper\u{e7}u"), p.preview, cx, |p| p.preview = !p.preview));
            }
        }

        let button = |id: &'static str, text: &'static str| {
            div()
                .id(id)
                .h(px(28.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(7.))
                .bg(rgb(0xe9e9ec))
                .text_size(px(13.))
                .text_color(rgb(theme::TEXT))
                .hover(|s| s.bg(rgb(0xe0e0e4)))
                .child(text)
        };
        let panel = div()
            .absolute()
            .top(px(theme::TOOLBAR_HEIGHT + 4.))
            .right(px(14.))
            .w(px(250.))
            .p(px(14.))
            .flex()
            .flex_col()
            .gap(px(10.))
            .rounded(px(12.))
            .bg(rgba(0xf6f6f6fc))
            .border_1()
            .border_color(rgb(0xd9d9de))
            .shadow_lg()
            .occlude()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_size(px(13.))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(rgb(theme::TEXT))
                            .child(crate::i18n::tf(crate::i18n::t("Options \u{2014} {name}"), &[("name", (name).to_string())])),
                    )
                    .child(
                        div()
                            .id("vo-close")
                            .size(px(18.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_full()
                            .text_size(px(10.))
                            .text_color(rgb(theme::TEXT_SECONDARY))
                            .hover(|s| s.bg(rgb(0xe3e3e6)))
                            .child("\u{2715}")
                            .cursor_pointer().on_click(cx.listener(|this, _, _, cx| this.toggle_view_options(cx))),
                    ),
            )
            .child(body)
            .child(rule())
            .child(
                div()
                    .flex()
                    .gap(px(8.))
                    .child(
                        button("vo-default", crate::i18n::t("Par d\u{e9}faut"))
                            .flex_1()
                            .cursor_pointer().on_click(cx.listener(|this, _, _, cx| {
                                this.prefs.defaults = this.eff();
                                this.prefs.save();
                                this.show_notice(crate::i18n::t("Options enregistr\u{e9}es comme valeurs par d\u{e9}faut."), cx);
                            })),
                    )
                    .child(
                        button("vo-restore", crate::i18n::t("Restaurer"))
                            .flex_1()
                            .cursor_pointer().on_click(cx.listener(|this, _, _, cx| {
                                this.prefs.folders.remove(&this.cwd);
                                this.prefs.save();
                                this.apply_folder_prefs();
                                this.resort_in_place();
                                this.rebuild_columns();
                                cx.notify();
                            })),
                    ),
            );
        // Le panneau garde le clic ; le fond reste utilisable (panneau flottant, pas modal).
        let _ = MouseButton::Left;
        Some(panel.into_any_element())
    }
}
