//! Renommage groupe ("Renommer N elements...") : remplacer du texte, ajouter du texte, ou appliquer un format
//! (nom + numero / compteur / date), avec apercu.

use std::path::PathBuf;

use gpui::{
    AnyElement, Context, Entity, Focusable, InteractiveElement, IntoElement, MouseButton, ParentElement, Styled,
    StatefulInteractiveElement, Subscription, Window, div, prelude::*, px,
};

use crate::editor::{Editor, EditorEvent};
use crate::ops;
use crate::theme::{rgb, rgba};
use crate::{theme, Finder};

pub struct Bulk {
    paths: Vec<PathBuf>,
    /// 0 = remplacer le texte, 1 = ajouter du texte, 2 = format.
    mode: u8,
    a: Entity<Editor>,
    b: Entity<Editor>,
    /// Ajouter apres le nom (sinon avant) ; format : 0 index, 1 compteur, 2 date.
    after: bool,
    kind: u8,
    _subs: Vec<Subscription>,
}

impl Bulk {
    /// Nouveaux noms (dans l'ordre), calcules d'apres les champs.
    fn names(&self, cx: &gpui::App) -> Vec<String> {
        let a = self.a.read(cx).text();
        let b = self.b.read(cx).text();
        let start: u32 = if self.mode == 2 { b.trim().parse().unwrap_or(1) } else { 1 };
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        self.paths
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                let (stem, ext) = ops::split_name(&name, p.is_dir());
                let new_stem = match self.mode {
                    0 if !a.is_empty() => stem.replace(&a, &b),
                    0 => stem,
                    1 => {
                        if self.after {
                            format!("{stem}{a}")
                        } else {
                            format!("{a}{stem}")
                        }
                    }
                    _ => {
                        let base = if a.trim().is_empty() { "Sans titre".to_string() } else { a.trim().to_string() };
                        let n = start + i as u32;
                        match self.kind {
                            0 => format!("{base} {n}"),
                            1 => format!("{base} {n:04}"),
                            _ => format!("{base} {today}{}", if self.paths.len() > 1 { format!(" {n}") } else { String::new() }),
                        }
                    }
                };
                format!("{new_stem}{ext}")
            })
            .collect()
    }
}

impl Finder {
    pub fn open_bulk_rename(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.read_only_place(cx) {
            return;
        }
        let paths = self.selected_paths();
        if paths.len() < 2 {
            return;
        }
        let a = cx.new(|cx| Editor::new("", true, cx));
        let b = cx.new(|cx| Editor::new("", true, cx));
        let mut subs = Vec::new();
        for ed in [&a, &b] {
            subs.push(cx.subscribe_in(ed, window, |this, _, ev: &EditorEvent, _, cx| match ev {
                EditorEvent::Commit => this.apply_bulk(cx),
                EditorEvent::Escape => {
                    this.bulk = None;
                    this.needs_focus = true;
                    cx.notify();
                }
                EditorEvent::Changed => cx.notify(),
                _ => {}
            }));
        }
        window.focus(&a.focus_handle(cx));
        self.bulk = Some(Bulk { paths, mode: 0, a, b, after: true, kind: 0, _subs: subs });
        cx.notify();
    }

    pub fn apply_bulk(&mut self, cx: &mut Context<Self>) {
        let Some(bulk) = self.bulk.take() else { return };
        self.needs_focus = true;
        let names = bulk.names(cx);
        let mut done = Vec::new();
        let mut errors = Vec::new();
        // Deux passes (noms temporaires) : "a -> b" et "b -> c" dans le meme lot ne se marchent pas dessus.
        let mut staged = Vec::new();
        for (i, (p, n)) in bulk.paths.iter().zip(&names).enumerate() {
            let tmp = p.with_file_name(format!(".winfinder-rename-{i}"));
            match std::fs::rename(p, &tmp) {
                Ok(()) => staged.push((p.clone(), tmp, n.clone())),
                Err(e) => errors.push(format!("{} : {e}", p.display())),
            }
        }
        for (orig, tmp, name) in staged {
            match ops::rename(&tmp, &name) {
                Ok(target) => {
                    crate::tags::rename_path(&orig, &target);
                    done.push(target);
                }
                Err(e) => {
                    let _ = std::fs::rename(&tmp, &orig);
                    errors.push(format!("{name} : {e}"));
                }
            }
        }
        self.reload_select(&done);
        if let Some(e) = errors.first() {
            self.show_notice(format!("Renommage incomplet \u{2014} {e}"), cx);
        } else {
            self.show_notice(format!("{} \u{e9}l\u{e9}ments renomm\u{e9}s", done.len()), cx);
        }
        cx.notify();
    }

    pub fn render_bulk(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let bulk = self.bulk.as_ref()?;
        let names = bulk.names(cx);
        let first_old = bulk.paths[0].file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let preview = format!("{first_old}  \u{2192}  {}", names.first().cloned().unwrap_or_default());
        let mode = bulk.mode;
        let after = bulk.after;
        let kind = bulk.kind;

        let seg = |id: &'static str, items: Vec<(&'static str, bool)>, cx: &mut Context<Self>, set: fn(&mut Bulk, usize)| {
            div().flex().p(px(2.)).rounded(px(8.)).bg(rgb(0xe9e9ec)).children(items.into_iter().enumerate().map(|(i, (label, on))| {
                div()
                    .id((id, i))
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
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if let Some(b) = this.bulk.as_mut() {
                            set(b, i);
                        }
                        cx.notify();
                    }))
            }))
        };
        let field = |id: &'static str, label: &'static str, ed: Entity<Editor>, cx: &mut Context<Self>| {
            div()
                .flex()
                .flex_col()
                .gap(px(4.))
                .child(div().text_size(px(12.)).text_color(rgb(theme::TEXT_SECONDARY)).child(label))
                .child(
                    div()
                        .id(id)
                        .h(px(26.))
                        .px(px(6.))
                        .rounded(px(6.))
                        .bg(rgb(0xffffff))
                        .border_1()
                        .border_color(rgb(0xb0b0b6))
                        .text_color(rgb(theme::TEXT))
                        .child(ed.clone())
                        .on_mouse_down(MouseButton::Left, cx.listener(move |_, _, window, cx| {
                            window.focus(&ed.focus_handle(cx));
                        })),
                )
        };
        let button = |id: &'static str, text: &'static str, primary: bool| {
            div()
                .id(id)
                .px(px(14.))
                .h(px(28.))
                .flex()
                .items_center()
                .rounded(px(7.))
                .text_size(px(13.))
                .when(primary, |d| d.bg(rgb(theme::SELECTION)).text_color(rgb(theme::SELECTION_TEXT)))
                .when(!primary, |d| d.bg(rgb(0xe9e9ec)).text_color(rgb(theme::TEXT)))
                .child(text)
        };

        let mut form = div().flex().flex_col().gap(px(10.)).child(seg(
            "bk-mode",
            vec![("Remplacer", mode == 0), ("Ajouter", mode == 1), ("Format", mode == 2)],
            cx,
            |b, i| b.mode = i as u8,
        ));
        match mode {
            0 => {
                form = form
                    .child(field("bk-a", "Rechercher", bulk.a.clone(), cx))
                    .child(field("bk-b", "Remplacer par", bulk.b.clone(), cx));
            }
            1 => {
                form = form.child(field("bk-a", "Texte \u{e0} ajouter", bulk.a.clone(), cx)).child(seg(
                    "bk-pos",
                    vec![("Avant le nom", !after), ("Apr\u{e8}s le nom", after)],
                    cx,
                    |b, i| b.after = i == 1,
                ));
            }
            _ => {
                form = form
                    .child(seg(
                        "bk-kind",
                        vec![("Nom + index", kind == 0), ("Nom + compteur", kind == 1), ("Nom + date", kind == 2)],
                        cx,
                        |b, i| b.kind = i as u8,
                    ))
                    .child(field("bk-a", "Nom personnalis\u{e9}", bulk.a.clone(), cx))
                    .child(field("bk-b", "Commencer \u{e0} (num\u{e9}ro)", bulk.b.clone(), cx));
            }
        }

        Some(
            div()
                .id("bulk-backdrop")
                .absolute()
                .inset_0()
                .flex()
                .justify_center()
                .items_start()
                .pt(px(80.))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(
                    div()
                        .w(px(400.))
                        .p(px(20.))
                        .flex()
                        .flex_col()
                        .gap(px(12.))
                        .rounded(px(12.))
                        .bg(rgba(0xf6f6f6fc))
                        .border_1()
                        .border_color(rgb(0xd9d9de))
                        .shadow_lg()
                        .occlude()
                        .child(
                            div()
                                .text_size(px(15.))
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .text_color(rgb(theme::TEXT))
                                .child(format!("Renommer {} \u{e9}l\u{e9}ments", bulk.paths.len())),
                        )
                        .child(form)
                        .child(div().text_size(px(12.)).text_color(rgb(theme::TEXT_SECONDARY)).child(format!("Exemple : {preview}")))
                        .child(
                            div()
                                .flex()
                                .justify_end()
                                .gap(px(8.))
                                .child(button("bk-cancel", "Annuler", false).on_click(cx.listener(|this, _, _, cx| {
                                    this.bulk = None;
                                    this.needs_focus = true;
                                    cx.notify();
                                })))
                                .child(button("bk-ok", "Renommer", true).on_click(cx.listener(|this, _, _, cx| this.apply_bulk(cx)))),
                        ),
                )
                .into_any_element(),
        )
    }
}
