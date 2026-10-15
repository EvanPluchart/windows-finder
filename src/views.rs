use std::ops::Range;

use gpui::{
    AnyElement, ClickEvent, Context, InteractiveElement, IntoElement, MouseButton, MouseDownEvent, ParentElement,
    SharedString, StatefulInteractiveElement, Styled, div, prelude::*, px, uniform_list,
};
use crate::theme::rgb;

use crate::fs::{self, Entry};
use crate::theme;
use crate::Finder;

/// Ligne affichee dans la liste : titre de groupe ou element (indice dans `entries`).
pub enum Row {
    Header(String),
    Item(usize),
}

pub(crate) fn label(text: impl Into<SharedString>) -> gpui::Div {
    div().overflow_hidden().whitespace_nowrap().text_ellipsis().child(text.into())
}

/// Marge horizontale des lignes (liste, colonnes) par rapport aux bords du contenu.
const ROW_MARGIN: f32 = 6.0;
/// Colonne "Nom" : retrait du texte = padding (8) + icone (16) + espace (8).
const NAME_INDENT: f32 = 50.0;
const COL_DATE: f32 = 160.0;
const COL_SIZE: f32 = 90.0;
const COL_KIND: f32 = 120.0;

impl Finder {
    /// Nom de l'element, ou champ de saisie si l'element est en cours de renommage.
    fn name_element(&self, e: &Entry) -> AnyElement {
        if let Some(r) = self.rename.as_ref().filter(|r| r.path == e.path) {
            return div()
                .flex_1()
                .min_w_0()
                .h(px(18.))
                .rounded(px(4.))
                .bg(rgb(0xffffff))
                .border_1()
                .border_color(rgb(0x6aa6f8))
                .text_color(rgb(theme::TEXT))
                .child(r.editor.clone())
                .into_any_element();
        }
        label(e.name.clone()).flex_1().min_w_0().into_any_element()
    }

    /// Clic gauche sur un element : Ctrl/Maj pour la selection multiple, double-clic pour ouvrir.
    fn on_item_click(&mut self, i: usize, ev: &ClickEvent, cx: &mut Context<Self>) {
        let mods = ev.modifiers();
        self.click_select(i, mods, cx);
        if ev.click_count() >= 2 && !mods.control && !mods.shift {
            self.open_selection(cx);
        }
    }

    /// Clic droit sur un element : on le selectionne s'il ne l'est pas, puis menu contextuel.
    fn on_item_context(&mut self, i: usize, ev: &MouseDownEvent, cx: &mut Context<Self>) {
        if !self.marked.contains(&i) {
            self.select(i, cx);
        }
        self.open_menu(ev.position, true, cx);
        cx.stop_propagation();
    }

    /// Clic sur le fond du dossier : desselectionne ; clic droit : menu du dossier.
    fn background_handlers<E: InteractiveElement>(&self, el: E, cx: &mut Context<Self>) -> E {
        el.on_mouse_down(
            MouseButton::Left,
            cx.listener(|this, _: &MouseDownEvent, _, cx| {
                this.commit_rename(cx);
                if this.view != crate::ViewMode::Columns && !this.marked.is_empty() {
                    this.marked.clear();
                    this.selected = None;
                    cx.notify();
                }
            }),
        )
        .on_mouse_down(
            MouseButton::Right,
            cx.listener(|this, ev: &MouseDownEvent, _, cx| this.open_menu(ev.position, false, cx)),
        )
    }

    /// Lignes de la liste, avec un titre avant chaque groupe quand le regroupement est actif.
    pub fn display_rows(&self) -> Vec<Row> {
        let g = fs::group();
        if g == 0 {
            return (0..self.entries.len()).map(Row::Item).collect();
        }
        let mut out = Vec::with_capacity(self.entries.len() + 8);
        let mut last: Option<String> = None;
        for (i, e) in self.entries.iter().enumerate() {
            let title = fs::group_of(e, g).1;
            if last.as_deref() != Some(title.as_str()) {
                out.push(Row::Header(title.clone()));
                last = Some(title);
            }
            out.push(Row::Item(i));
        }
        out
    }

    fn render_header_row(&self, title: &str) -> AnyElement {
        div()
            .w_full()
            .h(px(theme::LIST_ROW_HEIGHT))
            .px(px(ROW_MARGIN + 8.))
            .flex_none()
            .flex()
            .items_center()
            .text_size(px(12.))
            .font_weight(gpui::FontWeight::SEMIBOLD)
            .text_color(rgb(theme::TEXT_SECONDARY))
            .child(title.to_string())
            .into_any_element()
    }

    pub fn render_list(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let (cur, asc) = crate::fs::sort();
        // En-tete cliquable : fleche sur la colonne triee, second clic = ordre inverse.
        let head = |text: &'static str, key: crate::fs::SortKey, cx: &mut Context<Self>| {
            let active = cur == key;
            div()
                .id(text)
                .flex()
                .items_center()
                .gap(px(4.))
                .text_size(px(12.))
                .text_color(rgb(if active { theme::TEXT } else { theme::TEXT_SECONDARY }))
                .when(key == crate::fs::SortKey::Name, |d| d.justify_between().pr(px(10.)))
                .child(text)
                .when(active, |d| {
                    d.child(
                        div().font_family("Segoe Fluent Icons").text_size(px(8.)).child(if asc { "\u{E70E}" } else { "\u{E70D}" }),
                    )
                })
                .on_click(cx.listener(move |this, _, _, cx| this.sort_by(key, cx)))
        };
        use crate::fs::SortKey;
        let pr = self.eff();
        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .child(
                // Memes marges et memes largeurs que les lignes : les colonnes restent alignees.
                div()
                    .h(px(theme::LIST_HEADER_HEIGHT))
                    .flex_none()
                    .relative()
                    .px(px(ROW_MARGIN))
                    .flex()
                    .items_center()
                    .border_b_1()
                    .border_color(rgb(theme::SEPARATOR))
                    .child(head("Nom", SortKey::Name, cx).flex_1().min_w_0().pl(px(NAME_INDENT)))
                    .children({
                        // Filets verticaux entre les colonnes, de la droite vers la gauche.
                        let mut off = ROW_MARGIN;
                        let mut seps = Vec::new();
                        for (on, w) in [(pr.col_kind, COL_KIND), (pr.col_size, COL_SIZE), (pr.col_created, COL_DATE), (pr.col_modified, COL_DATE)] {
                            if on {
                                off += w;
                                seps.push(div().absolute().right(px(off)).top(px(7.)).w(px(1.)).h(px(14.)).bg(rgb(theme::SEPARATOR)));
                            }
                        }
                        seps
                    })
                    .when(pr.col_modified, |d| d.child(head("Date de modification", SortKey::Modified, cx).w(px(COL_DATE)).flex_none().px_2()))
                    .when(pr.col_created, |d| d.child(head("Date de création", SortKey::Created, cx).w(px(COL_DATE)).flex_none().px_2()))
                    .when(pr.col_size, |d| d.child(head("Taille", SortKey::Size, cx).w(px(COL_SIZE)).flex_none().px_2().justify_end()))
                    .when(pr.col_kind, |d| d.child(head("Type", SortKey::Kind, cx).w(px(COL_KIND)).flex_none().px_2())),
            )
            .child(self.background_handlers(
                div().flex_1().min_h_0().on_drop(cx.listener(|this, p: &gpui::ExternalPaths, window, cx| {
                    let cwd = this.cwd.clone();
                    this.drop_external(p.paths().to_vec(), cwd, window.modifiers().control, cx);
                })).child(
                    uniform_list(
                        "list",
                        self.display_rows().len().max(80),
                        cx.processor(|this, range: Range<usize>, _window, cx| {
                            let rows = this.display_rows();
                            let n_items = this.entries.len();
                            range
                                .map(|ri| match rows.get(ri) {
                                    Some(Row::Header(t)) => this.render_header_row(t),
                                    Some(Row::Item(i)) => this.render_row(*i, cx),
                                    None => this.render_row(n_items + (ri - rows.len()), cx),
                                })
                                .collect::<Vec<_>>()
                        }),
                        
                    )
                    .track_scroll(self.list_scroll.clone())
                    .size_full(),
                ),
                cx,
            ))
    }

    fn render_row(&self, i: usize, cx: &mut Context<Self>) -> AnyElement {
        let Some(e) = self.entries.get(i) else {
            // Lignes vides : le zebrage continue sous le dernier element, comme sur macOS.
            return div()
                .w_full()
                .h(px(theme::LIST_ROW_HEIGHT))
                .px(px(ROW_MARGIN))
                .flex_none()
                .child(div().size_full().rounded(px(8.)).when(i % 2 == 1, |d| d.bg(rgb(theme::ALT_ROW))))
                .into_any_element();
        };
        let selected = self.marked.contains(&i);
        let (fg, sub) = if selected {
            (theme::SELECTION_TEXT, theme::SELECTION_TEXT)
        } else {
            (theme::TEXT, theme::TEXT_SECONDARY)
        };
        let pr = self.eff();
        let row = div()
            .id(("row", i))
            .size_full()
            .text_size(px(pr.text as f32))
            .rounded(px(8.))
            .flex()
            .items_center()
            .text_color(rgb(fg))
            .when(self.is_cut(&e.path), |d| d.opacity(0.45))
            .when(!selected && i % 2 == 1, |d| d.bg(rgb(theme::ALT_ROW)))
            .when(selected, |d| d.bg(rgb(theme::SELECTION)))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .px_2()
                    .flex()
                    .gap_2()
                    .items_center()
                    .pl(px(8. + e.depth as f32 * 16.))
                    // Triangle de depliage (dossiers) ou espace equivalent.
                    .child(
                        div()
                            .id(("disc", i))
                            .w(px(10.))
                            .flex_none()
                            .flex()
                            .justify_center()
                            .text_size(px(9.))
                            .text_color(rgb(sub))
                            .child(if e.is_dir && self.view == crate::ViewMode::List && self.search_query().is_none() {
                                if self.expanded.contains(&e.path) { "\u{25BE}" } else { "\u{25B8}" }
                            } else {
                                ""
                            })
                            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                            .on_click(cx.listener(move |this, _, _, cx| {
                                cx.stop_propagation();
                                this.toggle_expand(i, cx);
                            })),
                    )
                    .child(self.icon(&e.path, 16.0, false))
                    .child(self.name_element(e))
                    .children(crate::tags::dots(&e.path, 9.))
                    .children(e.hint.clone().map(|h| {
                        div().flex_none().max_w(px(220.)).text_size(px(11.)).text_color(rgb(sub)).child(label(h))
                    })),
            )
            .when(pr.col_modified, |d| d.child(div().w(px(COL_DATE)).flex_none().px_2().text_color(rgb(sub)).child(fs::format_date_pref(e.modified, pr.rel_dates))))
            .when(pr.col_created, |d| d.child(div().w(px(COL_DATE)).flex_none().px_2().text_color(rgb(sub)).child(fs::format_date_pref(e.created, pr.rel_dates))))
            .when(pr.col_size, |d| {
                d.child(div().w(px(COL_SIZE)).flex_none().px_2().flex().justify_end().text_color(rgb(sub)).child(fs::format_size(e)))
            })
            .when(pr.col_kind, |d| d.child(div().w(px(COL_KIND)).flex_none().px_2().text_color(rgb(sub)).child(label(e.kind.clone()))))
            .on_drag(crate::dragdrop::DragPaths::new(self, e, self.marked.contains(&i)), |d, off, _, cx| cx.new(|_| d.view(off)))
            .when(e.is_dir, |d| {
                let target = e.path.clone();
                d.drag_over::<crate::dragdrop::DragPaths>(|s, _, _, _| s.bg(rgb(0xcfe2fb))).on_drop(cx.listener(
                    move |this, p: &crate::dragdrop::DragPaths, window, cx| {
                        this.drop_into(p, target.clone(), window.modifiers().control, cx);
                        cx.stop_propagation();
                    },
                ))
            })
            .when(e.is_dir, |d| {
                let folder = e.path.clone();
                d.on_drag_move::<crate::dragdrop::DragPaths>(cx.listener(move |this, ev: &gpui::DragMoveEvent<crate::dragdrop::DragPaths>, _, cx| {
                    this.spring_tick(folder.clone(), ev.bounds.contains(&ev.event.position), cx)
                }))
            })
            .when(e.is_dir, |d| {
                let dest = e.path.clone();
                d.on_drop(cx.listener(move |this, p: &gpui::ExternalPaths, window, cx| {
                    this.drop_external(p.paths().to_vec(), dest.clone(), window.modifiers().control, cx);
                    cx.stop_propagation();
                }))
            })
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_mouse_down(MouseButton::Right, cx.listener(move |this, ev, _, cx| this.on_item_context(i, ev, cx)))
            .on_click(cx.listener(move |this, ev: &ClickEvent, _, cx| this.on_item_click(i, ev, cx)));
        div().w_full().h(px(theme::LIST_ROW_HEIGHT)).px(px(ROW_MARGIN)).flex_none().child(row).into_any_element()
    }

    pub fn render_icons(&self, window: &gpui::Window, cx: &mut Context<Self>) -> impl IntoElement {
        use crate::dragdrop::DragPaths;
        use crate::layout::PAD;
        let (cell_w, cell_h) = self.cell();
        let pr = self.eff();
        let icon_px = pr.icon_size as f32;
        let width = f32::from(window.viewport_size().width);
        let cols = self.icon_cols(width);
        let positions = self.icon_positions(cols);
        let headers = self.icon_headers(cols);
        let content_h = positions.iter().map(|p| p.1).fold(0.0f32, f32::max) + cell_h + 2.0 * PAD;
        let cells = self.entries.iter().enumerate().map(|(i, e)| {
            let selected = self.marked.contains(&i);
            let renaming = self.rename.as_ref().map(|r| r.path == e.path).unwrap_or(false);
            let (x, y) = positions[i];
            let drag = DragPaths::new(self, e, selected);
            let target = e.path.clone();
            div()
                .id(("icon", i))
                .group(format!("icell-{i}"))
                .absolute()
                .left(px(x + PAD))
                .top(px(y + PAD))
                .w(px(cell_w))
                .h(px(cell_h))
                .pt(px(8.))
                .flex()
                .flex_col()
                .items_center()
                .gap(px(6.))
                .when(self.is_cut(&e.path), |d| d.opacity(0.45))
                // Boite de l'icone : carre arrondi gris quand l'element est selectionne.
                .child(
                    div()
                        .size(px(icon_px + 12.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(9.))
                        .when(selected, |d| d.bg(rgb(0xe6e6e6)))
                        .group(format!("ibox-{i}"))
                        .when(e.is_dir, |d| d.group_drag_over::<DragPaths>(format!("icell-{i}"), |s| s.bg(rgb(0xcfe2fb))))
                        .child(self.icon(&e.path, icon_px, true)),
                )
                .children(crate::tags::dots(&e.path, 10.))
                // Nom sur 2 lignes maximum, centre ; pastille bleue quand selectionne.
                .child(if renaming {
                    div().w(px(cell_w - 16.)).h(px(22.)).flex().child(self.name_element(e)).into_any_element()
                } else {
                    div()
                        .max_w(px(cell_w - 12.))
                        .px(px(6.))
                        .py(px(1.))
                        .rounded(px(5.))
                        .text_size(px(pr.text as f32))
                        .text_center()
                        .line_clamp(2)
                        .when(selected, |d| d.bg(rgb(theme::SELECTION)).text_color(rgb(theme::SELECTION_TEXT)))
                        .child(e.name.clone())
                        .into_any_element()
                })
                .when(pr.item_info, |d| {
                    d.child(
                        div().text_size(px((pr.text as f32 - 2.0).max(9.0))).text_color(rgb(theme::TEXT_SECONDARY)).child(if e.is_dir { SharedString::from("Dossier") } else { SharedString::from(fs::format_size(e)) }),
                    )
                })
                .on_drag(drag, |d, off, _, cx| cx.new(|_| d.view(off)))
                .when(e.is_dir, |d| {
                    d.on_drop(cx.listener(move |this, p: &DragPaths, window, cx| {
                        this.drop_into(p, target.clone(), window.modifiers().control, cx);
                        cx.stop_propagation();
                    }))
                })
                .when(e.is_dir, |d| {
                    let folder = e.path.clone();
                    d.on_drag_move::<DragPaths>(cx.listener(move |this, ev: &gpui::DragMoveEvent<DragPaths>, _, cx| {
                        this.spring_tick(folder.clone(), ev.bounds.contains(&ev.event.position), cx)
                    }))
                })
                .when(e.is_dir, |d| {
                    let dest = e.path.clone();
                    d.on_drop(cx.listener(move |this, p: &gpui::ExternalPaths, window, cx| {
                        this.drop_external(p.paths().to_vec(), dest.clone(), window.modifiers().control, cx);
                        cx.stop_propagation();
                    }))
                })
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_mouse_down(MouseButton::Right, cx.listener(move |this, ev, _, cx| this.on_item_context(i, ev, cx)))
                .on_click(cx.listener(move |this, ev: &ClickEvent, _, cx| this.on_item_click(i, ev, cx)))
        });
        // Rectangle de selection a la souris.
        let band = self.band.filter(|b| { let r = b.rect(); r.2 >= 3.0 || r.3 >= 3.0 }).map(|b| {
            let (x, y, w, h) = b.rect();
            div().absolute().left(px(x)).top(px(y)).w(px(w)).h(px(h)).rounded(px(3.)).bg(crate::theme::rgba(0x0064e11f)).border_1().border_color(crate::theme::rgba(0x0064e199))
        });
        self.background_handlers(
            div()
                .id("icon-grid")
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .track_scroll(&self.icon_scroll)
                .on_drop(cx.listener(|this, p: &gpui::ExternalPaths, window, cx| {
                    let cwd = this.cwd.clone();
                    this.drop_external(p.paths().to_vec(), cwd, window.modifiers().control, cx);
                }))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, ev: &MouseDownEvent, _, _| this.band_start(ev.position)),
                )
                .on_mouse_move(cx.listener(move |this, ev: &gpui::MouseMoveEvent, _, cx| {
                    if ev.pressed_button == Some(MouseButton::Left) {
                        this.band_move(ev.position, width, cx);
                    }
                }))
                .on_mouse_up(MouseButton::Left, cx.listener(|this, _, _, cx| this.band_end(cx)))
                .on_mouse_up_out(MouseButton::Left, cx.listener(|this, _, _, cx| this.band_end(cx)))
                .on_drop(cx.listener(move |this, p: &DragPaths, window, cx| {
                    this.drop_free(p, window.mouse_position(), width, cx)
                }))
                .child(
                    div()
                        .relative()
                        .w_full()
                        .h(px(content_h))
                        .children(headers.into_iter().map(|(y, title)| {
                            div()
                                .absolute()
                                .left(px(PAD))
                                .top(px(y + PAD))
                                .h(px(24.))
                                .flex()
                                .items_center()
                                .text_size(px(13.))
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .text_color(rgb(theme::TEXT_SECONDARY))
                                .child(title)
                        }))
                        .children(cells)
                        .children(band),
                ),
            cx,
        )
    }

    pub fn render_columns(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let cols = self.columns.iter().enumerate().map(|(c, col)| {
            let rows = col.entries.iter().enumerate().map(|(i, e)| {
                let selected = col.selected == Some(i);
                let row = div()
                    .id(("crow", c * 100_000 + i))
                    .size_full()
                    .px(px(8.))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .rounded(px(6.))
                    .when(self.is_cut(&e.path), |d| d.opacity(0.45))
                    .when(selected, |d| d.bg(rgb(theme::SELECTION)).text_color(rgb(theme::SELECTION_TEXT)))
                    .on_drag(crate::dragdrop::DragPaths::new(self, e, false), |d, off, _, cx| cx.new(|_| d.view(off)))
                    .when(e.is_dir, |d| {
                        let target = e.path.clone();
                        d.drag_over::<crate::dragdrop::DragPaths>(|s, _, _, _| s.bg(rgb(0xcfe2fb))).on_drop(cx.listener(
                            move |this, p: &crate::dragdrop::DragPaths, window, cx| {
                                this.drop_into(p, target.clone(), window.modifiers().control, cx);
                                cx.stop_propagation();
                            },
                        ))
                    })
                    .child(self.icon(&e.path, 16.0, false))
                    .child(self.name_element(e))
                    .when(e.is_dir, |d| {
                        d.child(
                            div()
                                .text_color(rgb(if selected { theme::SELECTION_TEXT } else { 0x9a9aa0 }))
                                .child("\u{203A}"),
                        )
                    })
                    .on_mouse_down(
                        MouseButton::Right,
                        cx.listener(move |this, ev: &MouseDownEvent, _, cx| {
                            this.select_in_column(c, i, cx);
                            this.open_menu(ev.position, true, cx);
                            cx.stop_propagation();
                        }),
                    )
                    .on_click(cx.listener(move |this, ev: &ClickEvent, _, cx| {
                        this.select_in_column(c, i, cx);
                        if ev.click_count() >= 2 {
                            this.open_selection(cx);
                        }
                    }));
                div().w_full().h(px(theme::COLUMN_ROW_HEIGHT)).flex_none().px(px(ROW_MARGIN)).child(row)
            });
            div()
                .id(("col", c))
                .w(px(theme::COLUMN_WIDTH))
                .flex_none()
                .h_full()
                .overflow_y_scroll()
                .py(px(4.))
                .border_r_1()
                .border_color(rgb(theme::SEPARATOR))
                .children(rows)
        });
        let show_preview = self.eff().preview;
        let preview = self.selected_entry().filter(|e| !e.is_dir && show_preview).map(|e| self.render_preview_pane(e));
        div().id("columns").flex_1().min_h_0().flex().overflow_x_scroll().children(cols).children(preview)
    }

    /// Volet d'apercu de la vue colonnes : miniature, nom, type/taille, informations (312 pt).
    fn render_preview_pane(&self, e: &Entry) -> impl IntoElement {
        div()
            .w(px(theme::PREVIEW_PANE_WIDTH))
            .flex_none()
            .h_full()
            .p(px(14.))
            .flex()
            .flex_col()
            .gap(px(10.))
            .child(
                div()
                    .h(px(250.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(self.icon(&e.path, 220.0, true)),
            )
            .child(div().text_size(px(15.)).font_weight(gpui::FontWeight::SEMIBOLD).child(label(e.name.clone())))
            .child(
                div()
                    .text_color(rgb(theme::TEXT_SECONDARY))
                    .child(format!("{} \u{2014} {}", e.kind, fs::format_size(e))),
            )
            .child(div().pt(px(10.)).font_weight(gpui::FontWeight::SEMIBOLD).child("Informations"))
            .child(info_row("Modifi\u{e9}", fs::format_date(e.modified)))
    }

    pub fn render_gallery(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let thumbs = self.entries.iter().enumerate().map(|(i, e)| {
            let selected = self.selected == Some(i);
            // Anneau gris de 3 pt autour de la vignette selectionnee, avec un lisere blanc.
            div()
                .id(("thumb", i))
                .size(px(56.))
                .mr(px(6.))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(7.))
                .border_2()
                .border_color(rgb(if selected { 0xbdbdbd } else { theme::BG }))
                .child(self.icon(&e.path, 44.0, true))
                .on_mouse_down(
                    MouseButton::Right,
                    cx.listener(move |this, ev: &MouseDownEvent, _, cx| this.on_item_context(i, ev, cx)),
                )
                .on_click(cx.listener(move |this, _, _, cx| this.select(i, cx)))
        });

        // Apercu central : cadre a fin contour 403x261 (texte en monospace, sinon miniature).
        let main = match self.selected_entry() {
            Some(e) => {
                let body = if e.is_text() {
                    div()
                        .size_full()
                        .p(px(14.))
                        .overflow_hidden()
                        .font_family("Cascadia Mono")
                        .text_size(px(13.))
                        .child(fs::read_text_preview(&e.path))
                        .into_any_element()
                } else {
                    div()
                        .size_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(self.icon(&e.path, 240.0, true))
                        .into_any_element()
                };
                div().flex_1().min_w_0().flex().items_center().justify_center().p(px(32.)).child(
                    div()
                        .w_full()
                        .h_full()
                        .max_w(px(560.))
                        .max_h(px(360.))
                        .rounded(px(10.))
                        .border_1()
                        .border_color(rgb(0xe3e3e7))
                        .overflow_hidden()
                        .child(body),
                )
            }
            None => div().flex_1(),
        };

        let show_inspector = self.eff().preview;
        let inspector = self.selected_entry().filter(|_| show_inspector).map(|e| {
            div()
                .w(px(250.))
                .flex_none()
                .h_full()
                .p(px(14.))
                .flex()
                .flex_col()
                .gap(px(10.))
                .bg(rgb(0xf1f1f4))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(12.))
                        .child(self.icon(&e.path, 48.0, true))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .child(
                                    div()
                                        .text_size(px(16.))
                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                        .line_clamp(2)
                                        .child(e.name.clone()),
                                )
                                .child(
                                    div()
                                        .text_color(rgb(theme::TEXT_SECONDARY))
                                        .child(format!("{} \u{2014} {}", e.kind, fs::format_size(e))),
                                ),
                        ),
                )
                .child(div().pt(px(10.)).font_weight(gpui::FontWeight::SEMIBOLD).child("Informations"))
                .child(info_row("Modifi\u{e9}", fs::format_date(e.modified)))
        });

        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .child(div().flex_1().min_h_0().flex().child(main).children(inspector))
            .child(
                div()
                    .id("strip")
                    .h(px(68.))
                    .flex_none()
                    .px(px(10.))
                    .flex()
                    .items_center()
                    .overflow_x_scroll()
                    .border_t_1()
                    .border_color(rgb(theme::SEPARATOR))
                    .children(thumbs),
            )
    }
}

/// Ligne "titre ... valeur" avec filet de separation (volets d'informations).
fn info_row(title: &'static str, value: String) -> impl IntoElement {
    div()
        .py(px(5.))
        .flex()
        .justify_between()
        .border_b_1()
        .border_color(rgb(0xdcdce0))
        .text_size(px(13.))
        .child(div().text_color(rgb(theme::TEXT_SECONDARY)).child(title))
        .child(value)
}
