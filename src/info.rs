//! Fenetre "Lire les informations" (Ctrl+I) : en-tete (icone, nom, taille, etiquettes) et sections repliables
//! General, Plus d'infos, Nom et extension, Ouvrir avec, Apercu, Permissions.

use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use gpui::{
    AnyElement, Context, Entity, Focusable, InteractiveElement, IntoElement, MouseButton, ParentElement, Styled,
    StatefulInteractiveElement, Subscription, Window, div, prelude::*, px,
};
use windows::Win32::Storage::FileSystem::{
    FILE_ATTRIBUTE_HIDDEN, FILE_ATTRIBUTE_READONLY, FILE_FLAGS_AND_ATTRIBUTES, GetFileAttributesW, SetFileAttributesW,
};
use windows::Win32::UI::Shell::{ASSOCF_NONE, ASSOCSTR_FRIENDLYAPPNAME, AssocQueryStringW};
use windows::core::{PCWSTR, PWSTR};

use crate::editor::{Editor, EditorEvent};
use crate::fs::{self, Entry};
use crate::theme::{rgb, rgba};
use crate::{ops, theme, Finder};

pub struct InfoState {
    pub path: PathBuf,
    pub name: String,
    pub kind: String,
    pub size: String,
    pub size_short: String,
    pub location: String,
    pub created: String,
    pub modified: String,
    pub accessed: String,
    pub dims: Option<(u32, u32)>,
    pub open_with: Option<String>,
    pub locked: bool,
    pub hidden: bool,
    pub is_dir: bool,
    pub name_ed: Entity<Editor>,
    _sub: Subscription,
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
    pretty.replace('.', ",")
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

fn wide(path: &Path) -> Vec<u16> {
    path.as_os_str().encode_wide().chain(std::iter::once(0)).collect()
}

fn attributes(path: &Path) -> u32 {
    unsafe { GetFileAttributesW(PCWSTR(wide(path).as_ptr())) }
}

fn set_attribute(path: &Path, flag: u32, on: bool) {
    let cur = attributes(path);
    if cur == u32::MAX {
        return;
    }
    let next = if on { cur | flag } else { cur & !flag };
    unsafe {
        let _ = SetFileAttributesW(PCWSTR(wide(path).as_ptr()), FILE_FLAGS_AND_ATTRIBUTES(next));
    }
}

/// Application par defaut associee a l'extension (nom affichable), s'il y en a une.
fn default_app(path: &Path) -> Option<String> {
    let ext = format!(".{}", path.extension()?.to_string_lossy());
    let w: Vec<u16> = ext.encode_utf16().chain(std::iter::once(0)).collect();
    let mut buf = [0u16; 260];
    let mut len = buf.len() as u32;
    unsafe {
        AssocQueryStringW(ASSOCF_NONE, ASSOCSTR_FRIENDLYAPPNAME, PCWSTR(w.as_ptr()), PCWSTR::null(), Some(PWSTR(buf.as_mut_ptr())), &mut len)
            .ok()
            .ok()?;
    }
    let s = String::from_utf16_lossy(&buf[..(len as usize).saturating_sub(1)]);
    (!s.is_empty()).then_some(s)
}

impl Finder {
    pub fn show_info(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.commit_rename(cx);
        let Some(e) = self.selected_entry().cloned() else { return };
        self.info = Some(self.build_info(&e, window, cx));
        cx.notify();
    }

    fn build_info(&mut self, e: &Entry, window: &mut Window, cx: &mut Context<Self>) -> InfoState {
        let meta = std::fs::metadata(&e.path).ok();
        let when = |t: Option<std::io::Result<SystemTime>>| fs::format_date(t.and_then(|r| r.ok()));
        let (size, size_short) = if e.is_dir {
            let (bytes, count, complete) = dir_size(&e.path);
            let more = if complete { "" } else { " (partiel)" };
            let n = format!("{count} \u{e9}l\u{e9}ment{}", if count > 1 { "s" } else { "" });
            (format!("{}{more}, {n}", format_bytes(bytes)), format_bytes(bytes))
        } else {
            let on_disk = e.size.div_ceil(4096) * 4096;
            let full = if e.size >= 1000 {
                format!("{} ({} octets, {} sur le disque)", format_bytes(e.size), e.size, format_bytes(on_disk))
            } else {
                format!("{} ({} sur le disque)", format_bytes(e.size), format_bytes(on_disk))
            };
            (full, format_bytes(e.size))
        };
        let attrs = attributes(&e.path);
        let name_ed = cx.new(|cx| Editor::new(&e.name, true, cx));
        let path = e.path.clone();
        let sub = cx.subscribe_in(&name_ed, window, move |this, ed, ev: &EditorEvent, window, cx| match ev {
            EditorEvent::Commit => {
                let new_name = ed.read(cx).text();
                match ops::rename(&path, &new_name) {
                    Ok(new_path) => {
                        if new_path != path {
                            this.record(crate::undo::Op::Rename { from: path.clone(), to: new_path.clone() });
                        }
                        this.reload_select(&[new_path]);
                        this.show_info(window, cx);
                    }
                    Err(err) => this.show_notice(err, cx),
                }
            }
            EditorEvent::Escape => {
                this.info = None;
                this.needs_focus = true;
                cx.notify();
            }
            _ => {}
        });
        InfoState {
            path: e.path.clone(),
            name: e.name.to_string(),
            kind: e.kind.to_string(),
            size,
            size_short,
            location: e.path.parent().map(|p| p.display().to_string()).unwrap_or_default(),
            created: when(meta.as_ref().map(|m| m.created())),
            modified: when(meta.as_ref().map(|m| m.modified())),
            accessed: when(meta.as_ref().map(|m| m.accessed())),
            dims: if e.is_image() { image::image_dimensions(&e.path).ok() } else { None },
            open_with: if e.is_dir { None } else { default_app(&e.path) },
            locked: attrs != u32::MAX && attrs & FILE_ATTRIBUTE_READONLY.0 != 0,
            hidden: attrs != u32::MAX && attrs & FILE_ATTRIBUTE_HIDDEN.0 != 0,
            is_dir: e.is_dir,
            name_ed,
            _sub: sub,
        }
    }

    pub fn render_info(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let info = self.info.as_ref()?;
        let open = self.info_open;
        let row = |label: &'static str, value: &str| {
            div()
                .flex()
                .gap(px(12.))
                .text_size(px(13.))
                .child(div().w(px(96.)).flex_none().flex().justify_end().text_color(rgb(theme::TEXT_SECONDARY)).child(label))
                .child(div().flex_1().min_w_0().text_color(rgb(theme::TEXT)).child(value.to_string()))
        };
        let section = |idx: usize, title: &'static str, body: AnyElement, cx: &mut Context<Self>| {
            div()
                .flex()
                .flex_col()
                .gap(px(8.))
                .child(
                    div()
                        .id(("info-sec", idx))
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .text_size(px(12.))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(rgb(theme::TEXT_SECONDARY))
                        .child(if open[idx] { "\u{25BE}" } else { "\u{25B8}" })
                        .child(title)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.info_open[idx] = !this.info_open[idx];
                            cx.notify();
                        })),
                )
                .when(open[idx], |d| d.child(body))
        };
        let check = |id: &'static str, label: &'static str, on: bool, flag: u32, cx: &mut Context<Self>| {
            div()
                .id(id)
                .flex()
                .items_center()
                .gap(px(8.))
                .pl(px(108.))
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
                .on_click(cx.listener(move |this, _, window, cx| {
                    if let Some(i) = this.info.as_ref() {
                        set_attribute(&i.path, flag, !on);
                    }
                    this.show_info(window, cx);
                }))
        };

        let general = div()
            .flex()
            .flex_col()
            .gap(px(6.))
            .child(row("Type :", &info.kind))
            .child(row("Taille :", &info.size))
            .child(row("Emplacement :", &info.location))
            .child(row("Cr\u{e9}ation :", &info.created))
            .child(row("Modification :", &info.modified))
            .child(row("Dernier acc\u{e8}s :", &info.accessed))
            .child(check("info-lock", "Verrouill\u{e9} (lecture seule)", info.locked, FILE_ATTRIBUTE_READONLY.0, cx))
            .child(check("info-hide", "Masqu\u{e9}", info.hidden, FILE_ATTRIBUTE_HIDDEN.0, cx));

        let more = div().flex().flex_col().gap(px(6.)).child(match info.dims {
            Some((w, h)) => row("Dimensions :", &format!("{w} \u{d7} {h} px")).into_any_element(),
            None => div()
                .pl(px(108.))
                .text_size(px(13.))
                .text_color(rgb(theme::TEXT_SECONDARY))
                .child("Aucune information suppl\u{e9}mentaire")
                .into_any_element(),
        });

        let ed = info.name_ed.clone();
        let name_body = div()
            .id("info-name")
            .h(px(26.))
            .px(px(6.))
            .rounded(px(6.))
            .bg(rgb(0xffffff))
            .border_1()
            .border_color(rgb(0xb0b0b6))
            .text_color(rgb(theme::TEXT))
            .child(info.name_ed.clone())
            .on_mouse_down(MouseButton::Left, cx.listener(move |_, _, window, cx| window.focus(&ed.focus_handle(cx))))
            .into_any_element();

        let open_body = div()
            .pl(px(108.))
            .text_size(px(13.))
            .text_color(rgb(theme::TEXT))
            .child(info.open_with.clone().unwrap_or_else(|| "Aucune application associ\u{e9}e".to_string()))
            .into_any_element();

        let preview = div().flex().justify_center().child(self.icon(&info.path, 160., true)).into_any_element();

        let perm = if info.locked { "Vous pouvez seulement lire cet \u{e9}l\u{e9}ment." } else { "Vous pouvez lire et modifier cet \u{e9}l\u{e9}ment." };
        let perm_body = div().pl(px(108.)).text_size(px(13.)).text_color(rgb(theme::TEXT)).child(perm).into_any_element();

        let tag_chips = crate::tags::get(&info.path);
        Some(
            div()
                .id("info-backdrop")
                .absolute()
                .inset_0()
                .flex()
                .justify_center()
                .items_start()
                .pt(px(36.))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _, _, cx| {
                        this.info = None;
                        this.needs_focus = true;
                        cx.notify();
                    }),
                )
                .child(
                    div()
                        .id("info-panel")
                        .w(px(400.))
                        .max_h(px(560.))
                        .overflow_y_scroll()
                        .p(px(18.))
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
                                        .child(
                                            div()
                                                .text_size(px(12.))
                                                .text_color(rgb(theme::TEXT_SECONDARY))
                                                .child(format!("Modifi\u{e9} : {}", info.modified)),
                                        )
                                        .child(div().flex().items_center().gap(px(6.)).mt(px(2.)).children(tag_chips.into_iter().map(|t| {
                                            div()
                                                .flex()
                                                .items_center()
                                                .gap(px(4.))
                                                .px(px(6.))
                                                .rounded_full()
                                                .bg(rgb(0xe9e9ec))
                                                .text_size(px(11.))
                                                .text_color(rgb(theme::TEXT))
                                                .child(div().size(px(8.)).rounded_full().bg(rgb(crate::tags::color_hex(t.color))))
                                                .child(t.name)
                                        }))),
                                )
                                .child(div().text_size(px(13.)).text_color(rgb(theme::TEXT_SECONDARY)).child(info.size_short.clone())),
                        )
                        .child(div().h(px(1.)).w_full().bg(rgb(0xdcdcdf)))
                        .child(section(0, "G\u{e9}n\u{e9}ral", general.into_any_element(), cx))
                        .child(section(1, "Plus d'infos", more.into_any_element(), cx))
                        .child(section(2, "Nom et extension", name_body, cx))
                        .when(!info.is_dir, |d| d.child(section(3, "Ouvrir avec", open_body, cx)))
                        .child(section(4, "Aper\u{e7}u", preview, cx))
                        .child(section(5, "Permissions", perm_body, cx)),
                )
                .into_any_element(),
        )
    }
}
