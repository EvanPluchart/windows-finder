//! Tableau CSV editable pour l'apercu rapide : clic = editer la cellule, Entree/Tab = cellule
//! suivante, Echap = annuler la saisie, Ctrl+S = enregistrer, boutons pour ajouter ligne/colonne.

use std::path::PathBuf;

use gpui::{
    App, Context, Entity, EventEmitter, FocusHandle, Focusable, InteractiveElement, IntoElement, ParentElement, Render,
    StatefulInteractiveElement, Styled, Subscription, Window, div, prelude::*, px, };
use crate::theme::{rgb};

use crate::editor::{Editor, EditorEvent};
use crate::theme;

const COL_W: f32 = 150.0;
const ROW_H: f32 = 30.0;
const GUTTER_W: f32 = 40.0;

pub enum CsvEvent {
    /// La saisie s'est arretee : le parent peut reprendre le focus.
    StoppedEditing,
    Changed,
}

struct CellEdit {
    row: usize,
    col: usize,
    editor: Entity<Editor>,
    _sub: Subscription,
}

pub struct CsvTable {
    path: PathBuf,
    rows: Vec<Vec<String>>,
    pub dirty: bool,
    pub error: Option<String>,
    editing: Option<CellEdit>,
    focus: FocusHandle,
}

impl EventEmitter<CsvEvent> for CsvTable {}

impl Focusable for CsvTable {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl CsvTable {
    pub fn new(path: PathBuf, cx: &mut Context<Self>) -> Self {
        let (rows, error) = match Self::load(&path) {
            Ok(r) => (r, None),
            Err(e) => (vec![vec![String::new()]], Some(crate::i18n::tf(crate::i18n::t("Lecture impossible : {e}"), &[("e", (e).to_string())]))),
        };
        Self { path, rows, dirty: false, error, editing: None, focus: cx.focus_handle() }
    }

    fn load(path: &PathBuf) -> Result<Vec<Vec<String>>, String> {
        let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
        let text = String::from_utf8_lossy(&bytes);
        let text = text.trim_start_matches('\u{feff}');
        // Separateur : ';' si la premiere ligne en contient plus que de ',' (CSV francais).
        let first = text.lines().next().unwrap_or("");
        let delim = if first.matches(';').count() > first.matches(',').count() { b';' } else { b',' };
        let mut reader = csv::ReaderBuilder::new().has_headers(false).flexible(true).delimiter(delim).from_reader(text.as_bytes());
        let mut rows: Vec<Vec<String>> = reader
            .records()
            .map(|r| r.map(|rec| rec.iter().map(str::to_owned).collect()).map_err(|e| e.to_string()))
            .collect::<Result<_, _>>()?;
        if rows.is_empty() {
            rows.push(vec![String::new()]);
        }
        let width = rows.iter().map(Vec::len).max().unwrap_or(1);
        for r in &mut rows {
            r.resize(width, String::new());
        }
        Ok(rows)
    }

    pub fn is_editing(&self) -> bool {
        self.editing.is_some()
    }

    pub fn save(&mut self, cx: &mut Context<Self>) {
        self.commit_edit(cx);
        let first = std::fs::read_to_string(&self.path).unwrap_or_default();
        let first = first.lines().next().unwrap_or("").to_owned();
        let delim = if first.matches(';').count() > first.matches(',').count() { b';' } else { b',' };
        let mut writer = csv::WriterBuilder::new().delimiter(delim).from_writer(Vec::new());
        for row in &self.rows {
            if writer.write_record(row).is_err() {
                self.error = Some(crate::i18n::t("Ecriture impossible").into());
                return;
            }
        }
        match writer.into_inner().map_err(|e| e.to_string()).and_then(|b| std::fs::write(&self.path, b).map_err(|e| e.to_string())) {
            Ok(()) => {
                self.dirty = false;
                self.error = None;
            }
            Err(e) => self.error = Some(crate::i18n::tf(crate::i18n::t("Enregistrement impossible : {e}"), &[("e", (e).to_string())])),
        }
        cx.notify();
    }

    /// Applique la saisie en cours a la cellule.
    fn commit_edit(&mut self, cx: &mut Context<Self>) {
        if let Some(edit) = self.editing.take() {
            let value = edit.editor.read(cx).text();
            if self.rows[edit.row][edit.col] != value {
                self.rows[edit.row][edit.col] = value;
                self.dirty = true;
                cx.emit(CsvEvent::Changed);
            }
        }
    }

    fn start_edit(&mut self, row: usize, col: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.commit_edit(cx);
        let text = self.rows[row][col].clone();
        let editor = cx.new(|cx| Editor::new(&text, true, cx));
        let sub = cx.subscribe_in(&editor, window, |this, _, ev: &EditorEvent, window, cx| {
            let Some((r, c)) = this.editing.as_ref().map(|e| (e.row, e.col)) else { return };
            let (rows, cols) = (this.rows.len(), this.rows[0].len());
            match ev {
                EditorEvent::Commit => {
                    // Entree : cellule du dessous (ou on s'arrete en bas du tableau).
                    if r + 1 < rows {
                        this.start_edit(r + 1, c, window, cx);
                    } else {
                        this.stop_editing(true, window, cx);
                    }
                }
                EditorEvent::Tab(back) => {
                    let (nr, nc) = if *back {
                        if c > 0 { (r, c - 1) } else if r > 0 { (r - 1, cols - 1) } else { (r, c) }
                    } else if c + 1 < cols {
                        (r, c + 1)
                    } else if r + 1 < rows {
                        (r + 1, 0)
                    } else {
                        (r, c)
                    };
                    this.start_edit(nr, nc, window, cx);
                }
                EditorEvent::Escape => this.stop_editing(false, window, cx),
                EditorEvent::Save => this.save(cx),
                EditorEvent::Changed | EditorEvent::Arrow(_) => {}
            }
        });
        window.focus(&editor.read(cx).focus_handle(cx));
        self.editing = Some(CellEdit { row, col, editor, _sub: sub });
        cx.notify();
    }

    fn stop_editing(&mut self, apply: bool, window: &mut Window, cx: &mut Context<Self>) {
        if apply {
            self.commit_edit(cx);
        } else {
            self.editing = None;
        }
        window.focus(&self.focus);
        cx.emit(CsvEvent::StoppedEditing);
        cx.notify();
    }

    fn add_row(&mut self, cx: &mut Context<Self>) {
        let width = self.rows[0].len();
        self.rows.push(vec![String::new(); width]);
        self.dirty = true;
        cx.notify();
    }

    fn add_col(&mut self, cx: &mut Context<Self>) {
        for r in &mut self.rows {
            r.push(String::new());
        }
        self.dirty = true;
        cx.notify();
    }
}

fn pill(label: &'static str) -> gpui::Stateful<gpui::Div> {
    div()
        .id(label)
        .h(px(24.))
        .px(px(12.))
        .flex()
        .items_center()
        .rounded_full()
        .bg(rgb(0xe9e9ee))
        .hover(|s| s.bg(rgb(0xdedee4)))
        .text_size(px(12.))
        .child(label)
}

impl Render for CsvTable {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let cols = self.rows[0].len();
        let editing = self.editing.as_ref().map(|e| (e.row, e.col, e.editor.clone()));

        let header_gutter = div().w(px(GUTTER_W)).flex_none().h(px(ROW_H)).border_b_1().border_r_1().border_color(rgb(0xe3e3e7)).bg(rgb(0xf4f4f6));
        let rows = self.rows.iter().enumerate().map(|(r, row)| {
            let cells = row.iter().enumerate().map(|(c, text)| {
                let is_editing = editing.as_ref().map(|(er, ec, _)| *er == r && *ec == c).unwrap_or(false);
                let base = div()
                    .id(("cell", r * 10_000 + c))
                    .w(px(COL_W))
                    .flex_none()
                    .h(px(ROW_H))
                    .border_b_1()
                    .border_r_1()
                    .border_color(rgb(0xe3e3e7));
                if is_editing {
                    base.border_2()
                        .border_color(rgb(theme::SELECTION))
                        .bg(rgb(0xffffff))
                        .child(editing.as_ref().unwrap().2.clone())
                } else {
                    base.px(px(8.))
                        .flex()
                        .items_center()
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_ellipsis()
                        .when(r == 0, |d| d.font_weight(gpui::FontWeight::SEMIBOLD).bg(rgb(0xf4f4f6)))
                        .hover(|s| s.bg(rgb(0xf0f6ff)))
                        .child(text.clone())
                        .cursor_pointer().on_click(cx.listener(move |this, _, window, cx| this.start_edit(r, c, window, cx)))
                }
            });
            div()
                .flex()
                .child(
                    div()
                        .w(px(GUTTER_W))
                        .flex_none()
                        .h(px(ROW_H))
                        .flex()
                        .items_center()
                        .justify_center()
                        .border_b_1()
                        .border_r_1()
                        .border_color(rgb(0xe3e3e7))
                        .bg(rgb(0xf4f4f6))
                        .text_size(px(11.))
                        .text_color(rgb(theme::TEXT_SECONDARY))
                        .child((r + 1).to_string()),
                )
                .children(cells)
        });

        let col_letters = (0..cols).map(|c| {
            let mut name = String::new();
            let mut n = c + 1;
            while n > 0 {
                n -= 1;
                name.insert(0, (b'A' + (n % 26) as u8) as char);
                n /= 26;
            }
            div()
                .w(px(COL_W))
                .flex_none()
                .h(px(ROW_H))
                .flex()
                .items_center()
                .justify_center()
                .border_b_1()
                .border_r_1()
                .border_color(rgb(0xe3e3e7))
                .bg(rgb(0xf4f4f6))
                .text_size(px(11.))
                .text_color(rgb(theme::TEXT_SECONDARY))
                .child(name)
        });

        div()
            .track_focus(&self.focus)
            .key_context("CsvTable")
            .size_full()
            .flex()
            .flex_col()
            .child(
                div()
                    .id("csv-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_scroll()
                    .child(div().flex().child(header_gutter).children(col_letters))
                    .children(rows),
            )
            .child(
                div()
                    .h(px(36.))
                    .flex_none()
                    .px(px(10.))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .border_t_1()
                    .border_color(rgb(0xe3e3e7))
                    .child(pill(crate::i18n::t("+ Ligne")).cursor_pointer().on_click(cx.listener(|this, _, _, cx| this.add_row(cx))))
                    .child(pill(crate::i18n::t("+ Colonne")).cursor_pointer().on_click(cx.listener(|this, _, _, cx| this.add_col(cx))))
                    .child(div().flex_1())
                    .when_some(self.error.clone(), |d, e| d.child(div().text_size(px(12.)).text_color(rgb(0xc42b1c)).child(e)))
                    .child(
                        pill(crate::i18n::t("Enregistrer"))
                            .when(self.dirty, |d| d.bg(rgb(theme::SELECTION)).text_color(rgb(0xfffffe)))
                            .cursor_pointer().on_click(cx.listener(|this, _, _, cx| this.save(cx))),
                    ),
            )
    }
}
