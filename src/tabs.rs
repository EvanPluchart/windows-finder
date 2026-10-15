//! Onglets : chaque onglet garde son dossier, son historique, sa selection et son mode d'affichage.
//! L'onglet actif vit dans les champs de `Finder` ; les autres sont des instantanes (`Tab`).

use std::path::PathBuf;

use gpui::{
    AnyElement, Context, InteractiveElement, IntoElement, MouseButton, ParentElement, StatefulInteractiveElement, Styled,
    Window, div, prelude::*, px, };

use crate::theme::{rgb, rgba};
use crate::{fs, theme, Finder, ViewMode};

#[derive(Clone)]
pub struct Tab {
    pub cwd: PathBuf,
    pub history: Vec<PathBuf>,
    pub history_pos: usize,
    pub selected: Vec<PathBuf>,
    pub view: ViewMode,
}

impl Tab {
    pub fn at(path: PathBuf, view: ViewMode) -> Self {
        Self { history: vec![path.clone()], cwd: path, history_pos: 0, selected: Vec::new(), view }
    }
}

impl Finder {
    /// Ecrit l'etat de l'onglet actif dans son instantane.
    pub fn save_active_pub(&mut self) {
        self.save_active();
    }

    fn save_active(&mut self) {
        let selected = self.marked.iter().filter_map(|&i| self.entries.get(i)).map(|e| e.path.clone()).collect();
        if let Some(t) = self.tabs.get_mut(self.active_tab) {
            *t = Tab {
                cwd: self.cwd.clone(),
                history: self.history.clone(),
                history_pos: self.history_pos,
                selected,
                view: self.view,
            };
        }
    }

    /// Charge l'instantane de l'onglet actif dans l'etat vivant.
    fn load_active(&mut self, cx: &mut Context<Self>) {
        let Some(t) = self.tabs.get(self.active_tab).cloned() else { return };
        self.cancel_rename_silent();
        self.menu = None;
        self.close_search_silent();
        self.cwd = t.cwd;
        self.history = t.history;
        self.history_pos = t.history_pos;
        self.view = t.view;
        self.reload();
        if !t.selected.is_empty() {
            self.reload_select(&t.selected);
        }
        if self.view == ViewMode::Columns {
            self.set_view(ViewMode::Columns, cx);
        }
        cx.notify();
    }

    pub fn switch_tab(&mut self, idx: usize, cx: &mut Context<Self>) {
        if idx >= self.tabs.len() || idx == self.active_tab {
            return;
        }
        self.save_active();
        self.active_tab = idx;
        self.load_active(cx);
    }

    /// Nouvel onglet sur `path` (le dossier personnel par defaut), qui devient actif.
    pub fn new_tab(&mut self, path: Option<PathBuf>, cx: &mut Context<Self>) {
        self.save_active();
        let path = path.unwrap_or_else(fs::home_dir);
        self.tabs.push(Tab::at(path, self.view));
        self.active_tab = self.tabs.len() - 1;
        self.load_active(cx);
    }

    pub fn close_tab(&mut self, idx: usize, window: &mut Window, cx: &mut Context<Self>) {
        if self.tabs.len() <= 1 {
            window.remove_window();
            return;
        }
        if idx == self.active_tab {
            self.tabs.remove(idx);
            self.active_tab = idx.min(self.tabs.len() - 1);
            self.load_active(cx);
        } else {
            self.save_active();
            self.tabs.remove(idx);
            if idx < self.active_tab {
                self.active_tab -= 1;
            }
            cx.notify();
        }
    }

    pub fn cycle_tab(&mut self, delta: isize, cx: &mut Context<Self>) {
        let n = self.tabs.len() as isize;
        if n > 1 {
            let next = (self.active_tab as isize + delta).rem_euclid(n) as usize;
            self.switch_tab(next, cx);
        }
    }

    /// Barre d'onglets (visible a partir de deux onglets) : bande grise arrondie, onglet actif en pastille blanche.
    pub fn render_tab_bar(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        if self.tabs.len() < 2 {
            return None;
        }
        let tabs = self.tabs.iter().enumerate().map(|(i, tab)| {
            let active = i == self.active_tab;
            let cwd = if active { &self.cwd } else { &tab.cwd };
            let title = fs::display_name(cwd);
            let group = format!("tab-{i}");
            div()
                .id(("tab", i))
                .group(group.clone())
                .relative()
                .flex_1()
                .min_w_0()
                .h_full()
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(7.))
                .text_size(px(13.))
                .px(px(26.))
                .when(active, |d| d.bg(rgb(0xffffff)).text_color(rgb(theme::TEXT)).shadow_sm())
                .when(!active, |d| d.text_color(rgb(theme::TEXT_SECONDARY)))
                .child(crate::views::label(title))
                .child(
                    div()
                        .id(("tab-close", i))
                        .absolute()
                        .left(px(6.))
                        .size(px(16.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(4.))
                        .text_size(px(10.))
                        .text_color(rgb(theme::TEXT_SECONDARY))
                        .invisible()
                        .group_hover(group, |s| s.visible())
                        .hover(|s| s.bg(rgb(0xe2e2e6)))
                        .child("\u{2715}")
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .cursor_pointer().on_click(cx.listener(move |this, _, window, cx| this.close_tab(i, window, cx))),
                )
                .on_mouse_down(
                    MouseButton::Right,
                    cx.listener(move |this, ev: &gpui::MouseDownEvent, _, cx| {
                        this.tab_menu = Some((i, ev.position));
                        cx.notify();
                    }),
                )
                .on_mouse_down(
                    MouseButton::Middle,
                    cx.listener(move |this, _, window, cx| this.close_tab(i, window, cx)),
                )
                .cursor_pointer().on_click(cx.listener(move |this, _, _, cx| this.switch_tab(i, cx)))
        });
        Some(
            div()
                .flex_none()
                .h(px(32.))
                .px(px(12.))
                .pb(px(4.))
                .flex()
                .items_center()
                .gap(px(8.))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .h(px(26.))
                        .p(px(2.))
                        .flex()
                        .rounded(px(9.))
                        .bg(rgb(0xe9e9eb))
                        .children(tabs),
                )
                .child(
                    div()
                        .id("tab-new")
                        .size(px(26.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded_full()
                        .bg(rgb(0xfdfdfd))
                        .border_1()
                        .border_color(rgb(0xe4e5e6))
                        .text_size(px(16.))
                        .text_color(rgb(theme::TEXT))
                        .hover(|s| s.bg(rgb(0xf0f0f0)))
                        .child("+")
                        .cursor_pointer().on_click(cx.listener(|this, _, _, cx| this.new_tab(None, cx))),
                )
                .into_any_element(),
        )
    }
}

impl Finder {
    pub fn toggle_all_tabs(&mut self, cx: &mut Context<Self>) {
        self.all_tabs = !self.all_tabs;
        cx.notify();
    }

    /// Titre et dossier d'un onglet (l'onglet actif utilise l'etat vivant).
    fn tab_label(&self, i: usize) -> (String, String) {
        let cwd = if i == self.active_tab { &self.cwd } else { &self.tabs[i].cwd };
        (fs::display_name(cwd), if fs::is_virtual(cwd) { String::new() } else { cwd.display().to_string() })
    }

    /// Menu contextuel d'un onglet.
    pub fn render_tab_menu(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let (idx, pos) = self.tab_menu?;
        if idx >= self.tabs.len() {
            return None;
        }
        let many = self.tabs.len() > 1;
        let item = |id: &'static str, label: &'static str, enabled: bool, cx: &mut Context<Self>, f: fn(&mut Finder, usize, &mut Window, &mut Context<Finder>)| {
            div()
                .id(id)
                .h(px(26.))
                .px(px(10.))
                .flex()
                .items_center()
                .rounded(px(6.))
                .text_size(px(13.))
                .text_color(rgb(if enabled { theme::TEXT } else { 0xa8a8ae }))
                .when(enabled, |d| d.hover(|s| s.bg(rgb(theme::SELECTION)).text_color(rgb(theme::SELECTION_TEXT))))
                .child(label)
                .when(enabled, |d| {
                    d.cursor_pointer().on_click(cx.listener(move |this, _, window, cx| {
                        this.tab_menu = None;
                        f(this, idx, window, cx);
                        cx.notify();
                    }))
                })
        };
        Some(
            div()
                .id("tabmenu-backdrop")
                .absolute()
                .inset_0()
                .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                    this.tab_menu = None;
                    cx.notify();
                }))
                .on_mouse_down(MouseButton::Right, cx.listener(|this, _, _, cx| {
                    this.tab_menu = None;
                    cx.notify();
                }))
                .child(
                    div()
                        .absolute()
                        .left(pos.x)
                        .top(pos.y)
                        .w(px(260.))
                        .p(px(6.))
                        .rounded(px(10.))
                        .bg(rgba(0xfafafaf2))
                        .border_1()
                        .border_color(rgb(0xd9d9de))
                        .shadow_lg()
                        .occlude()
                        .child(item("tm-new", "Nouvel onglet", true, cx, |t, _, _, cx| t.new_tab(None, cx)))
                        .child(item("tm-dup", "Dupliquer l'onglet", true, cx, |t, i, _, cx| {
                            let p = if i == t.active_tab { t.cwd.clone() } else { t.tabs[i].cwd.clone() };
                            t.new_tab(Some(p), cx)
                        }))
                        .child(item("tm-move", "D\u{e9}placer vers une nouvelle fen\u{ea}tre", many, cx, |t, i, window, cx| {
                            let p = if i == t.active_tab { t.cwd.clone() } else { t.tabs[i].cwd.clone() };
                            cx.defer(move |app| {
                                crate::open_finder_window(app, Some(p));
                            });
                            t.close_tab(i, window, cx);
                        }))
                        .child(item("tm-others", "Fermer les autres onglets", many, cx, |t, i, _, cx| {
                            t.switch_tab(i, cx);
                            let keep = t.tabs[t.active_tab].clone();
                            let _ = keep;
                            t.save_active_pub();
                            let me = t.tabs[t.active_tab].clone();
                            t.tabs = vec![me];
                            t.active_tab = 0;
                            cx.notify();
                        }))
                        .child(item("tm-close", "Fermer l'onglet", true, cx, |t, i, window, cx| t.close_tab(i, window, cx))),
                )
                .into_any_element(),
        )
    }

    /// Vue d'ensemble de tous les onglets (Ctrl+Maj+A) : une carte par onglet.
    pub fn render_all_tabs(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        if !self.all_tabs {
            return None;
        }
        let cards = (0..self.tabs.len()).map(|i| {
            let (title, path) = self.tab_label(i);
            let active = i == self.active_tab;
            div()
                .id(("alltab", i))
                .w(px(220.))
                .h(px(130.))
                .p(px(14.))
                .flex()
                .flex_col()
                .justify_end()
                .gap(px(4.))
                .rounded(px(12.))
                .bg(rgb(0xffffff))
                .border_2()
                .border_color(rgb(if active { theme::SELECTION } else { 0xd9d9de }))
                .hover(|s| s.border_color(rgb(0x6aa6f8)))
                .child(div().text_size(px(15.)).font_weight(gpui::FontWeight::SEMIBOLD).text_color(rgb(theme::TEXT)).child(title))
                .child(div().text_size(px(11.)).text_color(rgb(theme::TEXT_SECONDARY)).child(crate::views::label(path)))
                .cursor_pointer().on_click(cx.listener(move |this, _, _, cx| {
                    this.all_tabs = false;
                    this.switch_tab(i, cx);
                }))
        });
        Some(
            div()
                .id("alltabs-backdrop")
                .absolute()
                .inset_0()
                .bg(rgba(0x00000066))
                .flex()
                .items_center()
                .justify_center()
                .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                    this.all_tabs = false;
                    cx.notify();
                }))
                .child(div().max_w(px(780.)).flex().flex_wrap().justify_center().gap(px(16.)).occlude().children(cards))
                .into_any_element(),
        )
    }
}
