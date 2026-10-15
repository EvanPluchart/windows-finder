//! Fenetre "Lire les informations" (Ctrl+I) : nom, type, taille, emplacement, dates.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use gpui::{
    AnyElement, Context, InteractiveElement, IntoElement, MouseButton, ParentElement, Styled, div, prelude::*, px, rgb,
    rgba,
};

use crate::fs::{self, Entry};
use crate::theme;
use crate::Finder;

pub struct InfoState {
    pub path: PathBuf,
    pub name: String,
    pub kind: String,
    pub size: String,
    pub location: String,
    pub created: String,
    pub modified: String,
    pub accessed: String,
}

fn format_bytes(n: u64) -> String {
    let f = n as f64;
    let pretty = if f < 1e3 {
        format!("{n} octets")
    } else if f < 1e6 {
        format!("{:.1} Ko", f / 1e3)
    } else if f < 1e9 {
        format!("{:.1} Mo", f / 1e6)
    } else {
        format!("{:.2} Go", f / 1e9)
    };
    if n >= 1000 { format!("{pretty} ({n} octets)") } else { pretty }.replace('.', ",")
}

/// Taille cumulee d'un dossier, plafonnee pour rester instantanee ; `(octets, elements, complet)`.
fn dir_size(root: &Path) -> (u64, u64, bool) {
    let (mut bytes, mut count, mut stack) = (0u64, 0u64, vec![root.to_path_buf()]);
    while let Some(dir) = stack.pop() {
        let Ok(read) = std::fs::read_dir(&dir) else { continue };
        for e in read.flatten() {
            count += 1;
            if count > 50_000 {
                return (bytes, count, false);
            }
            let Ok(meta) = e.metadata() else { continue };
            if meta.is_dir() {
                stack.push(e.path());
            } else {
                bytes += meta.len();
            }
        }
    }
    (bytes, count, true)
}

impl InfoState {
    pub fn for_entry(e: &Entry) -> Self {
        let meta = std::fs::metadata(&e.path).ok();
        let when = |t: Option<std::io::Result<SystemTime>>| fs::format_date(t.and_then(|r| r.ok()));
        let size = if e.is_dir {
            let (bytes, count, complete) = dir_size(&e.path);
            let more = if complete { "" } else { " (partiel)" };
            format!("{}{more}, {count} \u{e9}l\u{e9}ment{}", format_bytes(bytes), if count > 1 { "s" } else { "" })
        } else {
            format_bytes(e.size)
        };
        Self {
            path: e.path.clone(),
            name: e.name.to_string(),
            kind: e.kind.to_string(),
            size,
            location: e.path.parent().map(|p| p.display().to_string()).unwrap_or_default(),
            created: when(meta.as_ref().map(|m| m.created())),
            modified: when(meta.as_ref().map(|m| m.modified())),
            accessed: when(meta.as_ref().map(|m| m.accessed())),
        }
    }
}

impl Finder {
    pub fn show_info(&mut self, cx: &mut Context<Self>) {
        self.commit_rename(cx);
        self.info = self.selected_entry().map(InfoState::for_entry);
        cx.notify();
    }

    pub fn render_info(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let info = self.info.as_ref()?;
        let row = |label: &'static str, value: &str| {
            div()
                .flex()
                .gap(px(12.))
                .text_size(px(13.))
                .child(div().w(px(84.)).flex_none().flex().justify_end().text_color(rgb(theme::TEXT_SECONDARY)).child(label))
                .child(div().flex_1().min_w_0().text_color(rgb(theme::TEXT)).child(value.to_string()))
        };
        let close = |this: &mut Finder, cx: &mut Context<Finder>| {
            this.info = None;
            this.needs_focus = true;
            cx.notify();
        };
        Some(
            div()
                .id("info-backdrop")
                .absolute()
                .inset_0()
                .flex()
                .justify_center()
                .items_start()
                .pt(px(80.))
                .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _, cx| close(this, cx)))
                .child(
                    div()
                        .w(px(340.))
                        .p(px(18.))
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
                                .gap(px(12.))
                                .child(self.icon(&info.path, 48., false))
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .flex()
                                        .flex_col()
                                        .child(
                                            div()
                                                .text_size(px(15.))
                                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                                .text_color(rgb(theme::TEXT))
                                                .child(info.name.clone()),
                                        )
                                        .child(div().text_size(px(12.)).text_color(rgb(theme::TEXT_SECONDARY)).child(info.size.clone())),
                                ),
                        )
                        .child(div().h(px(1.)).w_full().bg(rgb(0xdcdcdf)))
                        .child(row("Type :", &info.kind))
                        .child(row("Taille :", &info.size))
                        .child(row("Emplacement :", &info.location))
                        .child(row("Cr\u{e9}ation :", &info.created))
                        .child(row("Modification :", &info.modified))
                        .child(row("Dernier acc\u{e8}s :", &info.accessed)),
                )
                .into_any_element(),
        )
    }
}
