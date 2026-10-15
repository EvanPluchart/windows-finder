//! Couches superposees du Finder : menu contextuel (clic droit), "Aller au dossier" (Ctrl+Maj+G),
//! recherche recursive (Ctrl+F) et pastille d'information.

use std::path::{Path, PathBuf};

use gpui::{
    AnyElement, Context, Entity, Focusable, InteractiveElement, IntoElement, MouseButton, ParentElement, Pixels, Point,
    StatefulInteractiveElement, Styled, Subscription, Window, div, prelude::*, px, };
use crate::theme::{rgb, rgba};

use crate::editor::{Editor, EditorEvent};
use crate::theme;
use crate::{Finder, fs};

// ---------------------------------------------------------------------------------------------
// Menu contextuel
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    Open,
    QuickLook,
    Copy,
    Cut,
    Paste,
    Duplicate,
    Rename,
    Info,
    Trash,
    NewFolder,
    Refresh,
    CleanUp,
    Uninstall,
    OpenInTab,
    Tags,
    Compress,
    Extract,
}

pub struct MenuState {
    pub pos: Point<Pixels>,
    /// Clic droit sur un element (sinon sur le fond du dossier).
    pub on_item: bool,
}

impl Finder {
    pub fn open_menu(&mut self, pos: Point<Pixels>, on_item: bool, cx: &mut Context<Self>) {
        self.commit_rename(cx);
        self.menu = Some(MenuState { pos, on_item });
        cx.notify();
    }

    /// Entrees du menu : (libelle, action, activee). `None` = separateur.
    fn menu_items(&self, on_item: bool) -> Vec<Option<(&'static str, MenuAction, bool)>> {
        let can_paste = self.clipboard.is_some();
        let single = self.selected_paths().len() == 1;
        if crate::fs::is_virtual(&self.cwd) {
            let apps = self.cwd.to_str() == Some(crate::fs::APPS);
            return if on_item {
                let mut v = vec![Some(("Ouvrir", MenuAction::Open, true)), Some(("Lire les informations", MenuAction::Info, single))];
                if apps {
                    v.push(None);
                    v.push(Some(("D\u{e9}sinstaller\u{2026}", MenuAction::Uninstall, single)));
                }
                v
            } else {
                vec![Some(("Actualiser", MenuAction::Refresh, true))]
            };
        }
        if on_item {
            vec![
                Some(("Ouvrir", MenuAction::Open, true)),
                Some(("Ouvrir dans un nouvel onglet", MenuAction::OpenInTab, single && self.selected_entry().map(|e| e.is_dir).unwrap_or(false))),
                Some(("Aper\u{e7}u rapide", MenuAction::QuickLook, true)),
                None,
                Some(("Copier", MenuAction::Copy, true)),
                Some(("Couper", MenuAction::Cut, true)),
                Some(("Coller", MenuAction::Paste, can_paste)),
                Some(("Dupliquer", MenuAction::Duplicate, true)),
                None,
                Some((if single { "Renommer" } else { "Renommer les éléments…" }, MenuAction::Rename, true)),
                Some(("Lire les informations", MenuAction::Info, single)),
                Some(("\u{c9}tiquettes\u{2026}", MenuAction::Tags, true)),
                Some(("Compresser", MenuAction::Compress, true)),
                Some((
                    "Extraire ici",
                    MenuAction::Extract,
                    single && self.selected_entry().map(|e| crate::fs::extension(&e.path) == "zip").unwrap_or(false),
                )),
                Some(("Mettre \u{e0} la corbeille", MenuAction::Trash, true)),
                None,
                Some(("Nouveau dossier", MenuAction::NewFolder, true)),
            ]
        } else {
            vec![
                Some(("Nouveau dossier", MenuAction::NewFolder, true)),
                None,
                Some(("Coller", MenuAction::Paste, can_paste)),
                Some(("Actualiser", MenuAction::Refresh, true)),
                Some(("Ranger les icônes", MenuAction::CleanUp, self.view == crate::ViewMode::Icons && self.layouts.contains_key(&self.cwd))),
            ]
        }
    }

    fn run_menu(&mut self, action: MenuAction, window: &mut Window, cx: &mut Context<Self>) {
        self.menu = None;
        match action {
            MenuAction::Open => self.open_selection(cx),
            MenuAction::QuickLook => self.toggle_quick_look(cx),
            MenuAction::Copy => self.copy_selection(false, cx),
            MenuAction::Cut => self.copy_selection(true, cx),
            MenuAction::Paste => self.paste(cx),
            MenuAction::Duplicate => self.duplicate(cx),
            MenuAction::Rename => self.begin_rename(window, cx),
            MenuAction::Info => self.show_info(window, cx),
            MenuAction::Tags => self.toggle_tag_pop(window, cx),
            MenuAction::Compress => self.compress_selection(cx),
            MenuAction::Extract => self.extract_selection(cx),
            MenuAction::Trash => self.trash_selection(cx),
            MenuAction::NewFolder => self.new_folder(window, cx),
            MenuAction::Refresh => self.refresh(cx),
            MenuAction::CleanUp => self.clean_up(cx),
            MenuAction::Uninstall => self.ask_uninstall(cx),
            MenuAction::OpenInTab => {
                if let Some(p) = self.selected_entry().filter(|e| e.is_dir).map(|e| e.path.clone()) {
                    self.new_tab(Some(p), cx);
                }
            }
        }
        cx.notify();
    }

    pub fn render_menu(&self, window: &Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        let state = self.menu.as_ref()?;
        let items = self.menu_items(state.on_item);
        let height: f32 = items.iter().map(|i| if i.is_some() { 26.0 } else { 9.0 }).sum::<f32>() + 12.0;
        // Reste dans la fenetre meme pres des bords.
        let vp = window.viewport_size();
        let x = f32::from(state.pos.x).min(f32::from(vp.width) - 230.0).max(4.0);
        let y = f32::from(state.pos.y).min(f32::from(vp.height) - height - 4.0).max(4.0);

        let rows = items.into_iter().enumerate().map(|(i, item)| match item {
            None => div().h(px(9.)).flex().items_center().child(div().h(px(1.)).w_full().bg(rgb(0xd9d9de))).into_any_element(),
            Some((label, action, enabled)) => div()
                .id(("menu-item", i))
                .h(px(26.))
                .px(px(10.))
                .flex()
                .items_center()
                .rounded(px(6.))
                .text_size(px(13.))
                .text_color(rgb(if enabled { theme::TEXT } else { 0xa8a8ae }))
                .when(enabled, |d| d.hover(|s| s.bg(rgb(theme::SELECTION)).text_color(rgb(0xfffffe))))
                .child(label)
                .when(enabled, |d| d.on_click(cx.listener(move |this, _, window, cx| this.run_menu(action, window, cx))))
                .into_any_element(),
        });

        let close = |this: &mut Finder, cx: &mut Context<Finder>| {
            this.menu = None;
            cx.notify();
        };
        Some(
            div()
                .id("menu-backdrop")
                .absolute()
                .inset_0()
                .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _, cx| close(this, cx)))
                .on_mouse_down(MouseButton::Right, cx.listener(move |this, _, _, cx| close(this, cx)))
                .child(
                    div()
                        .absolute()
                        .left(px(x))
                        .top(px(y))
                        .w(px(220.))
                        .p(px(6.))
                        .rounded(px(10.))
                        .bg(rgba(0xfafafaf2))
                        .border_1()
                        .border_color(rgb(0xd9d9de))
                        .shadow_lg()
                        .occlude()
                        .children(rows),
                )
                .into_any_element(),
        )
    }

    // -----------------------------------------------------------------------------------------
    // Pastille d'information
    // -----------------------------------------------------------------------------------------

    pub fn render_notice(&self) -> Option<AnyElement> {
        let (_, text) = self.notice.as_ref()?;
        Some(
            div()
                .absolute()
                .bottom(px(64.))
                .left_0()
                .right_0()
                .flex()
                .justify_center()
                .child(
                    div()
                        .max_w(px(560.))
                        .px(px(16.))
                        .py(px(8.))
                        .rounded_full()
                        .bg(rgba(0x262625ee))
                        .text_color(rgb(0xfffffe))
                        .text_size(px(13.))
                        .shadow_lg()
                        .child(text.clone()),
                )
                .into_any_element(),
        )
    }

    // -----------------------------------------------------------------------------------------
    // Fermeture de la couche la plus haute (Echap)
    // -----------------------------------------------------------------------------------------

    pub fn dismiss(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.menu.take().is_some() {
            cx.notify();
            return;
        }
        if self.conflict.is_some() {
            self.resolve_conflict(crate::transfer::Resolution::Stop, cx);
            return;
        }
        if self.bulk.take().is_some() {
            self.needs_focus = true;
            cx.notify();
            return;
        }
        if self.tag_pop.take().is_some() {
            self.needs_focus = true;
            cx.notify();
            return;
        }
        if self.view_opts {
            self.view_opts = false;
            cx.notify();
            return;
        }
        if self.confirm.take().is_some() {
            cx.notify();
            return;
        }
        if self.info.take().is_some() {
            self.needs_focus = true;
            cx.notify();
            return;
        }
        if self.goto.take().is_some() {
            self.needs_focus = true;
            cx.notify();
            return;
        }
        if self.rename.is_some() {
            self.cancel_rename_silent();
            cx.notify();
            return;
        }
        if self.search.is_some() {
            self.close_search(cx);
            return;
        }
        self.close_quick_look(cx);
    }

    /// Vrai pendant une saisie dans un champ du Finder : les raccourcis de navigation se desactivent.
    pub fn text_input_active(&self, window: &Window, cx: &gpui::App) -> bool {
        self.rename.is_some()
            || self.tag_pop.is_some()
            || self.bulk.is_some()
            || self.info.is_some()
            || self.goto.is_some()
            || self.search.as_ref().map(|s| s.editor.read(cx).is_focused(window)).unwrap_or(false)
    }
}

// ---------------------------------------------------------------------------------------------
// Aller au dossier
// ---------------------------------------------------------------------------------------------

pub struct GoTo {
    pub editor: Entity<Editor>,
    _sub: Subscription,
    pub suggestions: Vec<PathBuf>,
    pub sel: usize,
}

/// Interprete la saisie : "~" = dossier personnel, "/" accepte comme separateur.
fn expand_path(text: &str) -> PathBuf {
    let t = text.trim().trim_matches('"').replace('/', "\\");
    if let Some(rest) = t.strip_prefix('~') {
        let home = fs::home_dir();
        return PathBuf::from(format!("{}{}", home.display(), rest));
    }
    PathBuf::from(t)
}

/// Sous-dossiers qui completent la saisie (max 8).
fn suggestions_for(text: &str) -> Vec<PathBuf> {
    let expanded = expand_path(text);
    let raw = text.trim();
    let (dir, prefix): (PathBuf, String) = if raw.ends_with('\\') || raw.ends_with('/') {
        (expanded, String::new())
    } else {
        let parent = expanded.parent().map(Path::to_path_buf).unwrap_or_default();
        let prefix = expanded.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
        (parent, prefix)
    };
    if dir.as_os_str().is_empty() {
        return Vec::new();
    }
    let mut found: Vec<PathBuf> = std::fs::read_dir(&dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .filter(|e| {
            let n = e.file_name().to_string_lossy().to_lowercase();
            !n.starts_with('.') && n.starts_with(&prefix)
        })
        .map(|e| e.path())
        .collect();
    found.sort_by_key(|p| p.to_string_lossy().to_lowercase());
    found.truncate(8);
    found
}

impl Finder {
    pub fn open_goto(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.commit_rename(cx);
        self.menu = None;
        let start = format!("{}\\", self.cwd.display().to_string().trim_end_matches('\\'));
        let editor = cx.new(|cx| Editor::new(&start, true, cx));
        let sub = cx.subscribe_in(&editor, window, |this, _, ev: &EditorEvent, _, cx| match ev {
            EditorEvent::Commit => this.goto_commit(cx),
            EditorEvent::Escape => {
                this.goto = None;
                this.needs_focus = true;
                cx.notify();
            }
            EditorEvent::Changed => this.goto_refresh(cx),
            EditorEvent::Tab(_) => this.goto_complete(cx),
            EditorEvent::Arrow(d) => {
                if let Some(g) = &mut this.goto {
                    let n = g.suggestions.len().max(1) as i32;
                    g.sel = ((g.sel as i32 + d).rem_euclid(n)) as usize;
                }
                cx.notify();
            }
            EditorEvent::Save => {}
        });
        window.focus(&editor.focus_handle(cx));
        let suggestions = suggestions_for(&start);
        self.goto = Some(GoTo { editor, _sub: sub, suggestions, sel: 0 });
        cx.notify();
    }

    fn goto_refresh(&mut self, cx: &mut Context<Self>) {
        if let Some(g) = &mut self.goto {
            g.suggestions = suggestions_for(&g.editor.read(cx).text());
            g.sel = 0;
        }
        cx.notify();
    }

    /// Tab : complete avec la suggestion choisie et ajoute un separateur.
    fn goto_complete(&mut self, cx: &mut Context<Self>) {
        let Some(g) = &self.goto else { return };
        let Some(choice) = g.suggestions.get(g.sel).cloned() else { return };
        let text = format!("{}\\", choice.display());
        g.editor.update(cx, |e, cx| e.set_text(&text, cx));
        self.goto_refresh(cx);
    }

    fn goto_commit(&mut self, cx: &mut Context<Self>) {
        let Some(g) = self.goto.take() else { return };
        self.needs_focus = true;
        let text = g.editor.read(cx).text();
        let target = expand_path(&text);
        if target.is_dir() {
            self.navigate(target, true, cx);
        } else if target.is_file() {
            // Un fichier : on ouvre son dossier et on le selectionne.
            if let Some(parent) = target.parent().map(Path::to_path_buf) {
                self.navigate(parent, true, cx);
                self.reload_select(&[target]);
            }
        } else {
            self.show_notice(format!("Dossier introuvable : {}", text.trim()), cx);
        }
        cx.notify();
    }

    pub fn render_goto(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let g = self.goto.as_ref()?;
        let rows = g.suggestions.iter().enumerate().map(|(i, p)| {
            let selected = i == g.sel;
            let target = p.clone();
            div()
                .id(("goto-sug", i))
                .h(px(24.))
                .px(px(8.))
                .flex()
                .items_center()
                .rounded(px(5.))
                .text_size(px(13.))
                .when(selected, |d| d.bg(rgb(theme::SELECTION)).text_color(rgb(0xfffffe)))
                .child(p.display().to_string())
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.goto = None;
                    this.needs_focus = true;
                    this.navigate(target.clone(), true, cx);
                }))
        });
        Some(
            div()
                .id("goto-backdrop")
                .absolute()
                .inset_0()
                .flex()
                .justify_center()
                .items_start()
                .pt(px(90.))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _, _, cx| {
                        this.goto = None;
                        this.needs_focus = true;
                        cx.notify();
                    }),
                )
                .child(
                    div()
                        .w(px(480.))
                        .p(px(16.))
                        .flex()
                        .flex_col()
                        .gap(px(10.))
                        .rounded(px(14.))
                        .bg(rgba(0xf8f8faf6))
                        .border_1()
                        .border_color(rgb(0xd9d9de))
                        .shadow_lg()
                        .occlude()
                        .child(div().text_size(px(13.)).child("Aller au dossier :"))
                        .child(
                            div()
                                .h(px(30.))
                                .rounded(px(6.))
                                .bg(rgb(0xffffff))
                                .border_2()
                                .border_color(rgb(0x6aa6f8))
                                .child(g.editor.clone()),
                        )
                        .children((!g.suggestions.is_empty()).then(|| div().flex().flex_col().children(rows)))
                        .child(
                            div()
                                .flex()
                                .justify_end()
                                .gap(px(8.))
                                .child(
                                    div()
                                        .id("goto-cancel")
                                        .px(px(14.))
                                        .h(px(26.))
                                        .flex()
                                        .items_center()
                                        .rounded(px(7.))
                                        .bg(rgb(0xffffff))
                                        .border_1()
                                        .border_color(rgb(0xd0d0d6))
                                        .child("Annuler")
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.goto = None;
                                            this.needs_focus = true;
                                            cx.notify();
                                        })),
                                )
                                .child(
                                    div()
                                        .id("goto-go")
                                        .px(px(18.))
                                        .h(px(26.))
                                        .flex()
                                        .items_center()
                                        .rounded(px(7.))
                                        .bg(rgb(theme::SELECTION))
                                        .text_color(rgb(0xfffffe))
                                        .child("Aller")
                                        .on_click(cx.listener(|this, _, _, cx| this.goto_commit(cx))),
                                ),
                        ),
                )
                .into_any_element(),
        )
    }
}

// ---------------------------------------------------------------------------------------------
// Recherche
// ---------------------------------------------------------------------------------------------

pub struct Search {
    pub editor: Entity<Editor>,
    _sub: Subscription,
    pub query: String,
}

impl Finder {
    /// Requete en cours (non vide), si une recherche est active.
    pub fn search_query(&self) -> Option<&str> {
        self.search.as_ref().map(|s| s.query.as_str()).filter(|q| !q.is_empty())
    }

    pub fn open_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.commit_rename(cx);
        self.menu = None;
        if let Some(s) = &self.search {
            window.focus(&s.editor.focus_handle(cx));
            return;
        }
        let editor = cx.new(|cx| {
            let mut e = Editor::new("", true, cx);
            e.set_placeholder("Rechercher");
            e
        });
        let sub = cx.subscribe_in(&editor, window, |this, _, ev: &EditorEvent, _, cx| match ev {
            EditorEvent::Changed => this.run_search(cx),
            EditorEvent::Escape => this.close_search(cx),
            // Entree : on garde les resultats et on rend le clavier a la liste.
            EditorEvent::Commit => {
                this.needs_focus = true;
                cx.notify();
            }
            _ => {}
        });
        window.focus(&editor.focus_handle(cx));
        self.search = Some(Search { editor, _sub: sub, query: String::new() });
        cx.notify();
    }

    /// Ferme le champ sans relire le dossier (la navigation le fait deja).
    pub fn close_search_silent(&mut self) {
        if self.search.take().is_some() {
            self.search_gen += 1;
            self.needs_focus = true;
        }
    }

    pub fn close_search(&mut self, cx: &mut Context<Self>) {
        if self.search.is_none() {
            return;
        }
        self.close_search_silent();
        let keep = self.selected_paths();
        self.reload_select(&keep);
        if self.marked.is_empty() && !self.entries.is_empty() {
            self.set_single(0);
            self.rebuild_columns();
        }
        cx.notify();
    }

    /// Recherche recursive dans le dossier courant, sur un thread d'arriere-plan (les frappes
    /// plus anciennes sont ignorees grace au compteur de generation).
    pub fn run_search(&mut self, cx: &mut Context<Self>) {
        let Some(s) = &mut self.search else { return };
        let query = s.editor.read(cx).text().trim().to_string();
        s.query = query.clone();
        self.search_gen += 1;
        let generation = self.search_gen;
        if query.is_empty() {
            self.entries = fs::read_dir(&self.cwd, self.show_hidden);
            self.clear_selection();
            self.rebuild_columns();
            cx.notify();
            return;
        }
        let root = self.cwd.clone();
        let hidden = self.show_hidden;
        cx.spawn(async move |this, cx| {
            let results = cx.background_executor().spawn(async move { fs::search(&root, &query, hidden, 3000) }).await;
            this.update(cx, |f, cx| {
                if f.search_gen == generation && f.search.is_some() {
                    f.entries = results;
                    f.clear_selection();
                    if !f.entries.is_empty() {
                        f.set_single(0);
                    }
                    f.rebuild_columns();
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
    }
}
