//! Etiquettes (tags) colorees, comme le Finder : 7 couleurs + noms libres. Enregistrees par chemin dans
//! `%APPDATA%\winfinder\tags.tsv` (suivent les renommages et deplacements faits dans l'application).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use gpui::{
    AnyElement, Context, Entity, Focusable, InteractiveElement, IntoElement, MouseButton, ParentElement, StatefulInteractiveElement,
    Styled, Subscription, Window, div, prelude::*, px,
};

use crate::editor::{Editor, EditorEvent};
use crate::fs::Entry;
use crate::theme::{rgb, rgba};
use crate::{theme, Finder};

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Tag {
    pub name: String,
    /// Couleur de 0 (aucune) a 7, comme macOS : 1 gris, 2 vert, 3 violet, 4 bleu, 5 jaune, 6 rouge, 7 orange.
    pub color: u8,
}

/// Les 7 etiquettes de base : (nom, couleur, 0xRRGGBB).
pub const PALETTE: [(&str, u8, u32); 7] = [
    ("Rouge", 6, 0xff5257),
    ("Orange", 7, 0xff9f0a),
    ("Jaune", 5, 0xffd60a),
    ("Vert", 2, 0x30d158),
    ("Bleu", 4, 0x0a84ff),
    ("Violet", 3, 0xbf5af2),
    ("Gris", 1, 0x98989d),
];

pub fn color_hex(color: u8) -> u32 {
    PALETTE.iter().find(|(_, c, _)| *c == color).map(|(_, _, h)| *h).unwrap_or(0xc6c6cb)
}

type Map = HashMap<PathBuf, Vec<Tag>>;

/// Etiquettes en memoire, et date du fichier quand on l'a lu : un autre processus (une fenetre ouverte depuis
/// l'Explorateur) a pu le modifier ; on le relit avant chaque changement pour ne pas effacer ses etiquettes.
struct Tags {
    map: Map,
    stamp: Option<std::time::SystemTime>,
}

fn store() -> &'static Mutex<Tags> {
    static S: OnceLock<Mutex<Tags>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(Tags { stamp: stamp(), map: load() }))
}

fn file() -> Option<PathBuf> {
    Some(PathBuf::from(std::env::var_os("APPDATA")?).join("winfinder").join("tags.tsv"))
}

fn stamp() -> Option<std::time::SystemTime> {
    std::fs::metadata(file()?).and_then(|m| m.modified()).ok()
}

/// Relit le fichier s'il a change depuis la derniere lecture ou ecriture.
fn refresh(t: &mut Tags) {
    let s = stamp();
    if s != t.stamp {
        t.map = load();
        t.stamp = s;
    }
}

/// Un nom d'etiquette peut contenir `;` ou `%` : ils sont echappes dans le fichier.
fn escape(name: &str) -> String {
    name.replace('%', "%25").replace(';', "%3B").replace('\t', "%09").replace('\n', "%0A").replace('\r', "%0D")
}

fn unescape(name: &str) -> String {
    name.replace("%3B", ";").replace("%09", "\t").replace("%0A", "\n").replace("%0D", "\r").replace("%25", "%")
}

fn load() -> Map {
    let mut m = Map::new();
    let Some(f) = file() else { return m };
    let Ok(bytes) = std::fs::read(&f) else { return m };
    // Fichier abime (modifie a la main dans un autre encodage) : copie de securite avant toute reecriture.
    if std::str::from_utf8(&bytes).is_err() {
        let _ = std::fs::copy(&f, f.with_extension("tsv.bak"));
    }
    let text = String::from_utf8_lossy(&bytes);
    for line in text.trim_start_matches('\u{feff}').lines() {
        let Some((path, rest)) = line.split_once('\t') else { continue };
        let tags: Vec<Tag> = rest
            .split(';')
            .filter_map(|t| t.rsplit_once(':'))
            .map(|(n, c)| Tag { name: unescape(n), color: c.parse().unwrap_or(0) })
            .collect();
        if !tags.is_empty() {
            m.insert(PathBuf::from(path), tags);
        }
    }
    m
}

fn save(t: &mut Tags) {
    let Some(f) = file() else { return };
    let mut out = String::new();
    for (p, tags) in &t.map {
        if tags.is_empty() {
            continue;
        }
        let list: Vec<String> = tags.iter().map(|t| format!("{}:{}", escape(&t.name), t.color)).collect();
        out.push_str(&format!("{}\t{}\n", p.display(), list.join(";")));
    }
    crate::layout::write_settings_file(&f, &out);
    t.stamp = stamp();
}

pub fn get(path: &Path) -> Vec<Tag> {
    store().lock().unwrap_or_else(|e| e.into_inner()).map.get(path).cloned().unwrap_or_default()
}

/// Ajoute `tag` a tous les chemins, ou le retire de tous s'ils l'ont deja tous.
pub fn toggle(paths: &[PathBuf], tag: &Tag) {
    let mut t = store().lock().unwrap_or_else(|e| e.into_inner());
    refresh(&mut t);
    let m = &mut t.map;
    let all = paths.iter().all(|p| m.get(p).map(|t| t.iter().any(|x| x.name == tag.name)).unwrap_or(false));
    for p in paths {
        let list = m.entry(p.clone()).or_default();
        if all {
            list.retain(|x| x.name != tag.name);
        } else if !list.iter().any(|x| x.name == tag.name) {
            list.push(tag.clone());
        }
    }
    m.retain(|_, v| !v.is_empty());
    save(&mut t);
}

/// Les etiquettes suivent un fichier ou un dossier renomme / deplace (et son contenu).
pub fn rename_path(old: &Path, new: &Path) {
    let mut t = store().lock().unwrap_or_else(|e| e.into_inner());
    refresh(&mut t);
    let m = &mut t.map;
    let moved: Vec<(PathBuf, PathBuf)> = m
        .keys()
        .filter(|k| k.starts_with(old))
        .map(|k| (k.clone(), new.join(k.strip_prefix(old).unwrap_or(Path::new("")))))
        .collect();
    if moved.is_empty() {
        return;
    }
    for (from, to) in moved {
        if let Some(tags) = m.remove(&from) {
            m.insert(to, tags);
        }
    }
    save(&mut t);
}

/// Fichiers portant l'etiquette `name` (qui existent encore), comme elements de dossier.
pub fn entries(name: &str) -> Vec<Entry> {
    let mut t = store().lock().unwrap_or_else(|e| e.into_inner());
    refresh(&mut t);
    let mut out: Vec<Entry> = t
        .map
        .iter()
        .filter(|(_, tags)| tags.iter().any(|t| t.name == name))
        .filter_map(|(p, _)| {
            let meta = std::fs::metadata(p).ok()?;
            let is_dir = meta.is_dir();
            let file_name = p.file_name()?.to_string_lossy().into_owned();
            Some(Entry {
                kind: if is_dir { crate::i18n::t("Dossier").into() } else { crate::fs::kind_label(p) },
                name: file_name.into(),
                path: p.clone(),
                is_dir,
                size: if is_dir { 0 } else { meta.len() },
                modified: meta.modified().ok(),
                created: meta.created().ok(),
                depth: 0,
                hint: p.parent().map(|d| crate::fs::display_name(d).into()),
            })
        })
        .collect();
    out.sort_by_key(|e| e.name.to_lowercase());
    out
}

/// Pastilles de couleur d'un element (None s'il n'a pas d'etiquette).
pub fn dots(path: &Path, size: f32) -> Option<AnyElement> {
    let tags = get(path);
    if tags.is_empty() {
        return None;
    }
    Some(
        div()
            .flex()
            .flex_none()
            .items_center()
            .children(tags.into_iter().enumerate().map(|(i, t)| {
                div()
                    .size(px(size))
                    .rounded_full()
                    .bg(rgb(color_hex(t.color)))
                    .border_1()
                    .border_color(rgba(0x00000022))
                    .when(i > 0, |d| d.ml(px(-size * 0.35)))
            }))
            .into_any_element(),
    )
}

pub struct TagPop {
    pub editor: Entity<Editor>,
    _sub: Subscription,
}

impl Finder {
    pub fn toggle_tag_pop(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.tag_pop.take().is_some() {
            self.needs_focus = true;
            cx.notify();
            return;
        }
        if self.selected_paths().is_empty() {
            self.show_notice(crate::i18n::t("S\u{e9}lectionnez d'abord un \u{e9}l\u{e9}ment."), cx);
            return;
        }
        let editor = cx.new(|cx| Editor::new("", true, cx));
        let sub = cx.subscribe_in(&editor, window, |this, ed, ev: &EditorEvent, _, cx| match ev {
            EditorEvent::Commit => {
                let name = ed.read(cx).text().trim().to_string();
                if !name.is_empty() {
                    let paths = this.selected_paths();
                    toggle(&paths, &Tag { name, color: 0 });
                }
                this.tag_pop = None;
                this.needs_focus = true;
                cx.notify();
            }
            EditorEvent::Escape => {
                this.tag_pop = None;
                this.needs_focus = true;
                cx.notify();
            }
            _ => {}
        });
        window.focus(&editor.focus_handle(cx));
        self.tag_pop = Some(TagPop { editor, _sub: sub });
        cx.notify();
    }

    /// Popover "Etiquettes pour ..." : champ + les 7 couleurs (coche = appliquee a tous les elements choisis).
    pub fn render_tag_pop(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let pop = self.tag_pop.as_ref()?;
        let paths = self.selected_paths();
        let title = match paths.as_slice() {
            [one] => crate::i18n::tf(crate::i18n::t("\u{c9}tiquettes pour \u{ab} {0} \u{bb}"), &[("0", (one.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()).to_string())]),
            many => crate::i18n::tf(crate::i18n::t("\u{c9}tiquettes pour {0} \u{e9}l\u{e9}ments"), &[("0", (many.len()).to_string())]),
        };
        let rows = PALETTE.iter().enumerate().map(|(i, (name, color, hex))| {
            let on_all = !paths.is_empty() && paths.iter().all(|p| get(p).iter().any(|t| t.name == *name));
            let on_some = paths.iter().any(|p| get(p).iter().any(|t| t.name == *name));
            let tag = Tag { name: (*name).to_string(), color: *color };
            div()
                .id(("tagrow", i))
                .h(px(26.))
                .px(px(8.))
                .flex()
                .items_center()
                .gap(px(8.))
                .rounded(px(6.))
                .text_size(px(14.))
                .text_color(rgb(theme::TEXT))
                .hover(|s| s.bg(rgb(0xe3e3e6)))
                .child(div().size(px(12.)).rounded_full().bg(rgb(*hex)))
                .child(div().flex_1().child(crate::i18n::td(name)))
                .child(div().text_size(px(12.)).text_color(rgb(theme::TEXT_SECONDARY)).child(if on_all { "\u{2713}" } else if on_some { "\u{2212}" } else { "" }))
                .cursor_pointer().on_click(cx.listener(move |this, _, _, cx| {
                    let paths = this.selected_paths();
                    toggle(&paths, &tag);
                    cx.notify();
                }))
        });
        Some(
            div()
                .id("tagpop-backdrop")
                .absolute()
                .inset_0()
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _, _, cx| {
                        this.tag_pop = None;
                        this.needs_focus = true;
                        cx.notify();
                    }),
                )
                .child(
                    div()
                        .absolute()
                        .top(px(theme::TOOLBAR_HEIGHT + 2.))
                        .right(px(96.))
                        .w(px(250.))
                        .p(px(10.))
                        .flex()
                        .flex_col()
                        .gap(px(6.))
                        .rounded(px(12.))
                        .bg(rgba(0xf6f6f6fc))
                        .border_1()
                        .border_color(rgb(0xd9d9de))
                        .shadow_lg()
                        .occlude()
                        .child(div().text_size(px(12.)).font_weight(gpui::FontWeight::SEMIBOLD).text_color(rgb(theme::TEXT_SECONDARY)).child(title))
                        .child(
                            div()
                                .h(px(26.))
                                .px(px(6.))
                                .rounded(px(6.))
                                .bg(rgb(0xffffff))
                                .border_1()
                                .border_color(rgb(0xb0b0b6))
                                .text_color(rgb(theme::TEXT))
                                .child(pop.editor.clone()),
                        )
                        .child(div().flex().flex_col().children(rows))
                        .child(
                            div()
                                .text_size(px(11.))
                                .text_color(rgb(theme::TEXT_SECONDARY))
                                .child(crate::i18n::t("Saisissez un nom puis Entr\u{e9}e pour une \u{e9}tiquette personnalis\u{e9}e.")),
                        ),
                )
                .into_any_element(),
        )
    }
}
