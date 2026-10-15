//! Petit editeur de texte (monospace) : curseur, selection (clavier + souris), copier/couper/coller,
//! defilement. Sert a l'edition des .txt et des cellules CSV dans l'apercu rapide.
//! Limites actuelles : pas d'annulation (Ctrl+Z), pas de retour a la ligne automatique, pas d'IME.

use std::cell::Cell;
use std::rc::Rc;

use gpui::{
    App, Bounds, ClipboardItem, Context, EventEmitter, FocusHandle, Focusable, InteractiveElement, IntoElement,
    KeyDownEvent, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement, Pixels, Point, Render,
    ScrollHandle, StatefulInteractiveElement, Styled, Window, canvas, div, font, point, prelude::*, px, rgb, rgba,
};

const FONT: &str = "Cascadia Mono";

pub enum EditorEvent {
    /// Echap : quitter l'edition.
    Escape,
    /// Le contenu a change.
    Changed,
    /// Ctrl+S.
    Save,
    /// Entree en mode une ligne (validation).
    Commit,
    /// Tab (true = avec Maj), utile pour naviguer entre cellules.
    Tab(bool),
    /// Fleche haut (-1) ou bas (1) en mode une ligne : sert a naviguer dans une liste de suggestions.
    Arrow(i32),
}

type Pos = (usize, usize); // (ligne, colonne en caracteres)

pub struct Editor {
    lines: Vec<String>,
    cursor: Pos,
    anchor: Option<Pos>,
    pub dirty: bool,
    single_line: bool,
    focus: FocusHandle,
    scroll: ScrollHandle,
    bounds: Rc<Cell<Bounds<Pixels>>>,
    advance: Pixels,
    font_size: f32,
    line_height: f32,
    dragging: bool,
    undo: Vec<(Vec<String>, Pos)>,
    redo: Vec<(Vec<String>, Pos)>,
    last_typing: bool,
    placeholder: String,
    pad_x: f32,
    pad_y: f32,
}

impl EventEmitter<EditorEvent> for Editor {}

impl Focusable for Editor {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus.clone()
    }
}

fn byte_idx(s: &str, col: usize) -> usize {
    s.char_indices().nth(col).map(|(i, _)| i).unwrap_or(s.len())
}

fn char_len(s: &str) -> usize {
    s.chars().count()
}

impl Editor {
    pub fn new(text: &str, single_line: bool, cx: &mut Context<Self>) -> Self {
        let text = text.replace("\r\n", "\n").replace('\t', "    ");
        let mut lines: Vec<String> = if single_line {
            vec![text.replace('\n', " ")]
        } else {
            text.split('\n').map(str::to_owned).collect()
        };
        if lines.is_empty() {
            lines.push(String::new());
        }
        let end = (lines.len() - 1, char_len(&lines[lines.len() - 1]));
        Self {
            cursor: if single_line { end } else { (0, 0) },
            anchor: if single_line { Some((0, 0)) } else { None },
            lines,
            dirty: false,
            single_line,
            focus: cx.focus_handle(),
            scroll: ScrollHandle::new(),
            bounds: Rc::new(Cell::new(Bounds::default())),
            advance: px(7.8),
            font_size: if single_line { 13.0 } else { 13.0 },
            line_height: 20.0,
            dragging: false,
            undo: Vec::new(),
            redo: Vec::new(),
            last_typing: false,
            placeholder: String::new(),
            // Mode une ligne (cellule) : marge fine ; sinon marge confortable de zone de texte.
            pad_x: if single_line { 8.0 } else { 12.0 },
            pad_y: if single_line { 3.0 } else { 12.0 },
        }
    }

    /// Selectionne les colonnes [from, to) de la premiere ligne (renommage : nom sans extension).
    pub fn select_range(&mut self, from: usize, to: usize) {
        self.anchor = Some((0, from));
        self.cursor = (0, to);
    }

    /// Remplace tout le contenu (curseur a la fin, sans selection).
    pub fn set_text(&mut self, text: &str, cx: &mut Context<Self>) {
        let text = text.replace("\r\n", "\n").replace('\t', "    ");
        let text = if self.single_line { text.replace('\n', " ") } else { text };
        self.lines = text.split('\n').map(str::to_owned).collect();
        let last = self.lines.len() - 1;
        self.cursor = (last, char_len(&self.lines[last]));
        self.anchor = None;
        cx.notify();
    }

    pub fn set_placeholder(&mut self, text: &str) {
        self.placeholder = text.to_string();
    }

    pub fn text(&self) -> String {
        self.lines.join("\n")
    }

    pub fn is_focused(&self, window: &Window) -> bool {
        self.focus.is_focused(window)
    }

    fn ordered_selection(&self) -> Option<(Pos, Pos)> {
        let a = self.anchor?;
        if a == self.cursor {
            return None;
        }
        Some(if a < self.cursor { (a, self.cursor) } else { (self.cursor, a) })
    }

    fn selected_text(&self) -> String {
        let Some(((r1, c1), (r2, c2))) = self.ordered_selection() else { return String::new() };
        if r1 == r2 {
            let l = &self.lines[r1];
            return l[byte_idx(l, c1)..byte_idx(l, c2)].to_string();
        }
        let mut out = self.lines[r1][byte_idx(&self.lines[r1], c1)..].to_string();
        for r in r1 + 1..r2 {
            out.push('\n');
            out.push_str(&self.lines[r]);
        }
        out.push('\n');
        out.push_str(&self.lines[r2][..byte_idx(&self.lines[r2], c2)]);
        out
    }

    fn delete_selection(&mut self) -> bool {
        let Some(((r1, c1), (r2, c2))) = self.ordered_selection() else {
            self.anchor = None;
            return false;
        };
        let tail = self.lines[r2][byte_idx(&self.lines[r2], c2)..].to_string();
        let head_end = byte_idx(&self.lines[r1], c1);
        self.lines[r1].truncate(head_end);
        self.lines[r1].push_str(&tail);
        self.lines.drain(r1 + 1..=r2);
        self.cursor = (r1, c1);
        self.anchor = None;
        true
    }

    fn insert_str(&mut self, s: &str) {
        self.delete_selection();
        let s = s.replace("\r\n", "\n").replace('\t', "    ");
        let s = if self.single_line { s.replace('\n', " ") } else { s };
        let (r, c) = self.cursor;
        let at = byte_idx(&self.lines[r], c);
        let tail = self.lines[r].split_off(at);
        let mut parts = s.split('\n');
        self.lines[r].push_str(parts.next().unwrap_or(""));
        let mut row = r;
        let mut col = c + char_len(&s.split('\n').next().unwrap_or(""));
        for part in parts {
            row += 1;
            self.lines.insert(row, part.to_string());
            col = char_len(part);
        }
        self.lines[row].push_str(&tail);
        self.cursor = (row, col);
        self.dirty = true;
    }

    fn move_to(&mut self, pos: Pos, extend: bool) {
        if extend {
            self.anchor.get_or_insert(self.cursor);
        } else {
            self.anchor = None;
        }
        self.cursor = pos;
    }

    fn clamp(&self, (r, c): Pos) -> Pos {
        let r = r.min(self.lines.len() - 1);
        (r, c.min(char_len(&self.lines[r])))
    }

    fn word_left(&self) -> Pos {
        let (r, c) = self.cursor;
        if c == 0 {
            return if r > 0 { (r - 1, char_len(&self.lines[r - 1])) } else { (0, 0) };
        }
        let chars: Vec<char> = self.lines[r].chars().collect();
        let mut i = c;
        while i > 0 && chars[i - 1].is_whitespace() {
            i -= 1;
        }
        while i > 0 && !chars[i - 1].is_whitespace() {
            i -= 1;
        }
        (r, i)
    }

    fn word_right(&self) -> Pos {
        let (r, c) = self.cursor;
        let chars: Vec<char> = self.lines[r].chars().collect();
        if c >= chars.len() {
            return if r + 1 < self.lines.len() { (r + 1, 0) } else { (r, c) };
        }
        let mut i = c;
        while i < chars.len() && !chars[i].is_whitespace() {
            i += 1;
        }
        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }
        (r, i)
    }

    fn reveal_cursor(&self) {
        let vp = self.scroll.bounds();
        if vp.size.width <= px(0.) {
            return;
        }
        let mut off = self.scroll.offset();
        let cy = px(self.pad_y + self.cursor.0 as f32 * self.line_height);
        let cx = px(self.pad_x) + self.advance * self.cursor.1 as f32;
        let lh = px(self.line_height);
        if cy + off.y < px(0.) {
            off.y = -cy;
        } else if cy + lh + off.y > vp.size.height {
            off.y = vp.size.height - cy - lh;
        }
        if cx + off.x < px(0.) {
            off.x = -cx + px(self.pad_x);
        } else if cx + px(8.) + off.x > vp.size.width {
            off.x = vp.size.width - cx - px(self.pad_x);
        }
        self.scroll.set_offset(point(off.x.min(px(0.)), off.y.min(px(0.))));
    }

    fn pos_from_point(&self, p: Point<Pixels>) -> Pos {
        let origin = self.bounds.get().origin;
        let off = self.scroll.offset();
        let x = f32::from(p.x - origin.x - off.x) - self.pad_x;
        let y = f32::from(p.y - origin.y - off.y) - self.pad_y;
        let row = (y / self.line_height).floor().max(0.0) as usize;
        let col = (x / f32::from(self.advance)).round().max(0.0) as usize;
        self.clamp((row, col))
    }

    /// Entree clavier avec historique : Ctrl+Z annule, Ctrl+Y ou Ctrl+Maj+Z retablit. La frappe continue
    /// est regroupee en une seule etape d annulation.
    fn on_key(&mut self, ev: &KeyDownEvent, cx: &mut Context<Self>) {
        let ks = &ev.keystroke;
        let m = ks.modifiers;
        let key = ks.key.as_str();
        if m.control && key == "z" && !m.shift {
            self.undo_step(cx);
            return;
        }
        if m.control && (key == "y" || (key == "z" && m.shift)) {
            self.redo_step(cx);
            return;
        }
        let before = (self.lines.clone(), self.cursor);
        let typing = key == "space" || (ks.key_char.as_deref().map(|c| !c.chars().all(char::is_control)).unwrap_or(false) && !(m.control && !m.alt));
        self.on_key_inner(ev, cx);
        if self.lines != before.0 {
            if !(typing && self.last_typing) {
                self.undo.push(before);
                if self.undo.len() > 500 {
                    self.undo.remove(0);
                }
            }
            self.last_typing = typing;
            self.redo.clear();
        } else if !matches!(key, "left" | "right" | "up" | "down" | "home" | "end") {
            // rien a faire
        } else {
            self.last_typing = false;
        }
    }

    fn undo_step(&mut self, cx: &mut Context<Self>) {
        if let Some((lines, cursor)) = self.undo.pop() {
            self.redo.push((std::mem::replace(&mut self.lines, lines), self.cursor));
            self.cursor = cursor;
            self.anchor = None;
            self.last_typing = false;
            self.dirty = true;
            cx.emit(EditorEvent::Changed);
            self.reveal_cursor();
            cx.notify();
        }
    }

    fn redo_step(&mut self, cx: &mut Context<Self>) {
        if let Some((lines, cursor)) = self.redo.pop() {
            self.undo.push((std::mem::replace(&mut self.lines, lines), self.cursor));
            self.cursor = cursor;
            self.anchor = None;
            self.last_typing = false;
            self.dirty = true;
            cx.emit(EditorEvent::Changed);
            self.reveal_cursor();
            cx.notify();
        }
    }

    fn on_key_inner(&mut self, ev: &KeyDownEvent, cx: &mut Context<Self>) {
        let ks = &ev.keystroke;
        let m = ks.modifiers;
        let extend = m.shift;
        let (r, c) = self.cursor;
        match ks.key.as_str() {
            "escape" => {
                cx.emit(EditorEvent::Escape);
                return;
            }
            "enter" => {
                if self.single_line {
                    cx.emit(EditorEvent::Commit);
                    return;
                }
                self.insert_str("\n");
                cx.emit(EditorEvent::Changed);
            }
            "tab" => {
                if self.single_line {
                    cx.emit(EditorEvent::Tab(m.shift));
                    return;
                }
                self.insert_str("    ");
                cx.emit(EditorEvent::Changed);
            }
            "backspace" => {
                if !self.delete_selection() {
                    if c > 0 {
                        let at = byte_idx(&self.lines[r], c - 1);
                        self.lines[r].remove(at);
                        self.cursor = (r, c - 1);
                    } else if r > 0 {
                        let line = self.lines.remove(r);
                        let prev = char_len(&self.lines[r - 1]);
                        self.lines[r - 1].push_str(&line);
                        self.cursor = (r - 1, prev);
                    } else {
                        return;
                    }
                }
                self.dirty = true;
                cx.emit(EditorEvent::Changed);
            }
            "delete" => {
                if !self.delete_selection() {
                    if c < char_len(&self.lines[r]) {
                        let at = byte_idx(&self.lines[r], c);
                        self.lines[r].remove(at);
                    } else if r + 1 < self.lines.len() {
                        let next = self.lines.remove(r + 1);
                        self.lines[r].push_str(&next);
                    } else {
                        return;
                    }
                }
                self.dirty = true;
                cx.emit(EditorEvent::Changed);
            }
            "left" => {
                if !extend && self.ordered_selection().is_some() {
                    let (start, _) = self.ordered_selection().unwrap();
                    self.move_to(start, false);
                } else {
                    let to = if m.control {
                        self.word_left()
                    } else if c > 0 {
                        (r, c - 1)
                    } else if r > 0 {
                        (r - 1, char_len(&self.lines[r - 1]))
                    } else {
                        (0, 0)
                    };
                    self.move_to(to, extend);
                }
            }
            "right" => {
                if !extend && self.ordered_selection().is_some() {
                    let (_, end) = self.ordered_selection().unwrap();
                    self.move_to(end, false);
                } else {
                    let to = if m.control {
                        self.word_right()
                    } else if c < char_len(&self.lines[r]) {
                        (r, c + 1)
                    } else if r + 1 < self.lines.len() {
                        (r + 1, 0)
                    } else {
                        (r, c)
                    };
                    self.move_to(to, extend);
                }
            }
            "up" if self.single_line => {
                cx.emit(EditorEvent::Arrow(-1));
                return;
            }
            "down" if self.single_line => {
                cx.emit(EditorEvent::Arrow(1));
                return;
            }
            "up" if !self.single_line => {
                let to = if r > 0 { self.clamp((r - 1, c)) } else { (0, 0) };
                self.move_to(to, extend);
            }
            "down" if !self.single_line => {
                let to = if r + 1 < self.lines.len() { self.clamp((r + 1, c)) } else { (r, char_len(&self.lines[r])) };
                self.move_to(to, extend);
            }
            "home" => {
                let to = if m.control { (0, 0) } else { (r, 0) };
                self.move_to(to, extend);
            }
            "end" => {
                let to = if m.control {
                    (self.lines.len() - 1, char_len(&self.lines[self.lines.len() - 1]))
                } else {
                    (r, char_len(&self.lines[r]))
                };
                self.move_to(to, extend);
            }
            "pageup" if !self.single_line => self.move_to(self.clamp((r.saturating_sub(15), c)), extend),
            "pagedown" if !self.single_line => self.move_to(self.clamp((r + 15, c)), extend),
            // Espace : certains evenements clavier n'ont pas de key_char pour cette touche, on l'insere nous-memes.
            "space" if !m.control || m.alt => {
                self.insert_str(" ");
                cx.emit(EditorEvent::Changed);
            }
            "a" if m.control => {
                self.anchor = Some((0, 0));
                self.cursor = (self.lines.len() - 1, char_len(&self.lines[self.lines.len() - 1]));
            }
            "c" if m.control => {
                let t = self.selected_text();
                if !t.is_empty() {
                    cx.write_to_clipboard(ClipboardItem::new_string(t));
                }
                return;
            }
            "x" if m.control => {
                let t = self.selected_text();
                if !t.is_empty() {
                    cx.write_to_clipboard(ClipboardItem::new_string(t));
                    self.delete_selection();
                    self.dirty = true;
                    cx.emit(EditorEvent::Changed);
                }
            }
            "v" if m.control => {
                if let Some(t) = cx.read_from_clipboard().and_then(|c| c.text()) {
                    self.insert_str(&t);
                    cx.emit(EditorEvent::Changed);
                }
            }
            "s" if m.control => {
                cx.emit(EditorEvent::Save);
                return;
            }
            _ => {
                // Caracteres imprimables ; AltGr (ctrl+alt) produit aussi des caracteres (~ # { [ ...).
                match &ks.key_char {
                    Some(ch) if !(m.control && !m.alt) && !ch.chars().all(char::is_control) => {
                        self.insert_str(ch);
                        cx.emit(EditorEvent::Changed);
                    }
                    _ => return,
                }
            }
        }
        self.reveal_cursor();
        cx.notify();
    }
}

impl Render for Editor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let fnt = font(FONT);
        let ts = window.text_system().clone();
        let id = ts.resolve_font(&fnt);
        self.advance = ts.advance(id, px(self.font_size), '0').map(|s| s.width).unwrap_or(px(7.8));
        let (adv, lh) = (self.advance, px(self.line_height));
        let focused = self.focus.is_focused(window);
        let selection = self.ordered_selection();

        let max_cols = self.lines.iter().map(|l| char_len(l)).max().unwrap_or(0);
        let content_w = adv * (max_cols as f32 + 2.0) + px(self.pad_x * 2.0);
        let content_h = lh * self.lines.len() as f32 + px(self.pad_y * 2.0);

        // Surbrillance de la selection : un rectangle par ligne touchee.
        let mut highlights = Vec::new();
        if let Some(((r1, c1), (r2, c2))) = selection {
            for r in r1..=r2 {
                let len = char_len(&self.lines[r]);
                let (from, to) = (if r == r1 { c1 } else { 0 }, if r == r2 { c2 } else { len + 1 });
                highlights.push(
                    div()
                        .absolute()
                        .top(px(self.pad_y) + lh * r as f32)
                        .left(px(self.pad_x) + adv * from as f32)
                        .w(adv * (to - from) as f32)
                        .h(lh)
                        .bg(rgba(0x0064e12e)),
                );
            }
        }

        let lines = self.lines.iter().map(|l| div().h(lh).whitespace_nowrap().child(l.clone()));
        let bounds_cell = self.bounds.clone();

        div()
            .relative()
            .size_full()
            .child(
                canvas(move |b, _, _| bounds_cell.set(b), |_, _, _, _| {}).absolute().size_full(),
            )
            .child(
                div()
                    .id("editor")
                    .track_focus(&self.focus)
                    .key_context("Editor")
                    .size_full()
                    .overflow_scroll()
                    .track_scroll(&self.scroll)
                    .font_family(FONT)
                    .text_size(px(self.font_size))
                    .cursor_text()
                    .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _, cx| this.on_key(ev, cx)))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, ev: &MouseDownEvent, window, cx| {
                            window.focus(&this.focus);
                            let pos = this.pos_from_point(ev.position);
                            if ev.click_count >= 2 {
                                // Double-clic : selectionne la ligne (mode une ligne : tout).
                                this.anchor = Some((pos.0, 0));
                                this.cursor = (pos.0, char_len(&this.lines[pos.0]));
                            } else {
                                this.move_to(pos, ev.modifiers.shift);
                                this.dragging = true;
                            }
                            cx.notify();
                        }),
                    )
                    .on_mouse_move(cx.listener(|this, ev: &MouseMoveEvent, _, cx| {
                        if this.dragging && ev.pressed_button == Some(MouseButton::Left) {
                            let pos = this.pos_from_point(ev.position);
                            this.move_to(pos, true);
                            this.reveal_cursor();
                            cx.notify();
                        }
                    }))
                    .on_mouse_up(
                        MouseButton::Left,
                        cx.listener(|this, _: &MouseUpEvent, _, _| this.dragging = false),
                    )
                    .on_mouse_up_out(
                        MouseButton::Left,
                        cx.listener(|this, _: &MouseUpEvent, _, _| this.dragging = false),
                    )
                    .child(
                        div()
                            .relative()
                            .min_w(content_w)
                            .h(content_h)
                            .pt(px(self.pad_y))
                            .px(px(self.pad_x))
                            .children(highlights)
                            .child(div().flex().flex_col().children(lines))
                            .when(!self.placeholder.is_empty() && self.lines.len() == 1 && self.lines[0].is_empty(), |d| {
                                d.child(
                                    div()
                                        .absolute()
                                        .top(px(self.pad_y))
                                        .left(px(self.pad_x))
                                        .h(lh)
                                        .text_color(rgb(0x9a9aa0))
                                        .child(self.placeholder.clone()),
                                )
                            })
                            .when(focused, |d| {
                                d.child(
                                    div()
                                        .absolute()
                                        .top(px(self.pad_y) + lh * self.cursor.0 as f32)
                                        .left(px(self.pad_x) + adv * self.cursor.1 as f32)
                                        .w(px(1.5))
                                        .h(lh)
                                        .bg(rgb(0x0064e1)),
                                )
                            }),
                    ),
            )
    }
}
