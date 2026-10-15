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
    OpenWith,
    Reveal,
    MakeAlias,
    ShowOriginal,
    PutBack,
    EmptyTrash,
    Extract,
    CopyPath,
    Settings,
    GoTo,
    NewTextFile,
    Terminal,
    Pin,
    Wallpaper,
    RotateLeft,
    RotateRight,
    RunAdmin,
    MoreOptions,
}

pub struct MenuState {
    pub pos: Point<Pixels>,
    /// Clic droit sur un element (sinon sur le fond du dossier).
    pub on_item: bool,
    /// 0 = clic droit, 1 = bouton Partager, 2 = bouton Actions.
    pub kind: u8,
}

impl Finder {
    pub fn open_menu(&mut self, pos: Point<Pixels>, on_item: bool, cx: &mut Context<Self>) {
        self.commit_rename(cx);
        self.menu = Some(MenuState { pos, on_item, kind: 0 });
        cx.notify();
    }

    pub fn open_toolbar_menu(&mut self, pos: Point<Pixels>, kind: u8, cx: &mut Context<Self>) {
        self.commit_rename(cx);
        self.menu = Some(MenuState { pos, on_item: false, kind });
        cx.notify();
    }

    /// Entrees du menu : (libelle, action, activee). `None` = separateur.
    fn menu_items(&self, on_item: bool) -> Vec<Option<(&'static str, MenuAction, bool)>> {
        let kind = self.menu.as_ref().map(|m| m.kind).unwrap_or(0);
        if kind == 1 {
            let any = !self.selected_paths().is_empty() && !crate::fs::is_virtual(&self.cwd);
            let single = self.selected_paths().len() == 1;
            return vec![
                Some((crate::i18n::t("Copier le chemin"), MenuAction::CopyPath, any)),
                Some((crate::i18n::t("Compresser"), MenuAction::Compress, any)),
                Some((crate::i18n::t("Cr\u{e9}er un raccourci"), MenuAction::MakeAlias, any)),
                None,
                Some((crate::i18n::t("Ouvrir avec\u{2026}"), MenuAction::OpenWith, single && self.selected_entry().map(|e| !e.is_dir).unwrap_or(false))),
                Some((crate::i18n::t("Afficher dans l'Explorateur"), MenuAction::Reveal, single)),
            ];
        }
        if kind == 2 {
            return vec![
                Some((crate::i18n::t("Nouveau dossier"), MenuAction::NewFolder, !crate::fs::is_virtual(&self.cwd))),
                Some((crate::i18n::t("Lire les informations"), MenuAction::Info, self.selected_paths().len() == 1)),
                Some((crate::i18n::t("Actualiser"), MenuAction::Refresh, true)),
                Some((crate::i18n::t("Aller au dossier\u{2026}"), MenuAction::GoTo, true)),
                None,
                Some((crate::i18n::t("R\u{e9}glages\u{2026}"), MenuAction::Settings, true)),
            ];
        }
        let can_paste = self.clipboard.is_some();
        let single = self.selected_paths().len() == 1;
        if self.in_trash() {
            return if on_item {
                vec![
                    Some((crate::i18n::t("Remettre \u{e0} sa place"), MenuAction::PutBack, true)),
                    Some((crate::i18n::t("Lire les informations"), MenuAction::Info, single)),
                    None,
                    Some((crate::i18n::t("Vider la corbeille\u{2026}"), MenuAction::EmptyTrash, true)),
                ]
            } else {
                vec![Some((crate::i18n::t("Vider la corbeille\u{2026}"), MenuAction::EmptyTrash, true)), Some((crate::i18n::t("Actualiser"), MenuAction::Refresh, true))]
            };
        }
        if crate::fs::is_virtual(&self.cwd) {
            let apps = self.cwd.to_str() == Some(crate::fs::APPS);
            return if on_item {
                let mut v = vec![Some((crate::i18n::t("Ouvrir"), MenuAction::Open, true)), Some((crate::i18n::t("Lire les informations"), MenuAction::Info, single))];
                if apps {
                    v.push(None);
                    v.push(Some((crate::i18n::t("D\u{e9}sinstaller\u{2026}"), MenuAction::Uninstall, single)));
                }
                v
            } else {
                vec![Some((crate::i18n::t("Actualiser"), MenuAction::Refresh, true))]
            };
        }
        if on_item {
            vec![
                Some((crate::i18n::t("Ouvrir"), MenuAction::Open, true)),
                Some((crate::i18n::t("Ouvrir dans un nouvel onglet"), MenuAction::OpenInTab, single && self.selected_entry().map(|e| e.is_dir).unwrap_or(false))),
                Some((crate::i18n::t("Aper\u{e7}u rapide"), MenuAction::QuickLook, true)),
                None,
                Some((crate::i18n::t("Copier"), MenuAction::Copy, true)),
                Some((crate::i18n::t("Couper"), MenuAction::Cut, true)),
                Some((crate::i18n::t("Coller"), MenuAction::Paste, can_paste)),
                Some((crate::i18n::t("Dupliquer"), MenuAction::Duplicate, true)),
                None,
                Some((if single { crate::i18n::t("Renommer") } else { crate::i18n::t("Renommer les éléments…") }, MenuAction::Rename, true)),
                Some((crate::i18n::t("Lire les informations"), MenuAction::Info, single)),
                Some((crate::i18n::t("\u{c9}tiquettes\u{2026}"), MenuAction::Tags, true)),
                Some((crate::i18n::t("Compresser"), MenuAction::Compress, true)),
                Some((crate::i18n::t("Cr\u{e9}er un raccourci"), MenuAction::MakeAlias, true)),
                Some((crate::i18n::t("Ouvrir avec\u{2026}"), MenuAction::OpenWith, single && self.selected_entry().map(|e| !e.is_dir).unwrap_or(false))),
                Some((crate::i18n::t("Afficher dans l'Explorateur"), MenuAction::Reveal, single)),
                Some((
                    crate::i18n::t("Afficher l'original"),
                    MenuAction::ShowOriginal,
                    single && self.selected_entry().map(|e| crate::fs::extension(&e.path) == "lnk").unwrap_or(false),
                )),
                Some((
                    crate::i18n::t("Extraire ici"),
                    MenuAction::Extract,
                    single && self.selected_entry().map(|e| crate::archive::is_archive(&e.path)).unwrap_or(false),
                )),
                Some((crate::i18n::t("Ouvrir dans le Terminal"), MenuAction::Terminal, single)),
                Some((crate::i18n::t("Ajouter \u{e0} la barre lat\u{e9}rale"), MenuAction::Pin, single && self.selected_entry().map(|e| e.is_dir).unwrap_or(false))),
                Some((crate::i18n::t("D\u{e9}finir comme fond d'\u{e9}cran"), MenuAction::Wallpaper, single && self.selected_entry().map(|e| matches!(crate::fs::extension(&e.path).as_str(), "png" | "jpg" | "jpeg" | "bmp")).unwrap_or(false))),
                Some((crate::i18n::t("Pivoter \u{e0} gauche"), MenuAction::RotateLeft, self.selected_entry().map(|e| matches!(crate::fs::extension(&e.path).as_str(), "png" | "jpg" | "jpeg" | "bmp" | "gif" | "tif" | "tiff")).unwrap_or(false))),
                Some((crate::i18n::t("Pivoter \u{e0} droite"), MenuAction::RotateRight, self.selected_entry().map(|e| matches!(crate::fs::extension(&e.path).as_str(), "png" | "jpg" | "jpeg" | "bmp" | "gif" | "tif" | "tiff")).unwrap_or(false))),
                Some((crate::i18n::t("Ex\u{e9}cuter en tant qu'administrateur"), MenuAction::RunAdmin, single && self.selected_entry().map(|e| matches!(crate::fs::extension(&e.path).as_str(), "exe" | "bat" | "cmd" | "msi" | "ps1")).unwrap_or(false))),
                Some((crate::i18n::t("Plus d'options\u{2026}"), MenuAction::MoreOptions, true)),
                Some((crate::i18n::t("Mettre \u{e0} la corbeille"), MenuAction::Trash, true)),
                None,
                Some((crate::i18n::t("Nouveau dossier"), MenuAction::NewFolder, true)),
            ]
        } else {
            vec![
                Some((crate::i18n::t("Nouveau dossier"), MenuAction::NewFolder, true)),
                Some((crate::i18n::t("Nouveau fichier texte"), MenuAction::NewTextFile, true)),
                Some((crate::i18n::t("Ouvrir dans le Terminal"), MenuAction::Terminal, true)),
                None,
                Some((crate::i18n::t("Coller"), MenuAction::Paste, can_paste)),
                Some((crate::i18n::t("Actualiser"), MenuAction::Refresh, true)),
                Some((crate::i18n::t("Ranger les icônes"), MenuAction::CleanUp, self.view == crate::ViewMode::Icons && self.layouts.contains_key(&self.cwd))),
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
            MenuAction::OpenWith => self.open_with(),
            MenuAction::Reveal => self.reveal_in_explorer(),
            MenuAction::MakeAlias => self.make_alias(cx),
            MenuAction::ShowOriginal => self.show_original(cx),
            MenuAction::PutBack => self.put_back(cx),
            MenuAction::EmptyTrash => self.ask_empty_trash(cx),
            MenuAction::Extract => self.extract_selection(cx),
            MenuAction::CopyPath => {
                let text = self.selected_paths().iter().map(|p| p.display().to_string()).collect::<Vec<_>>().join("
");
                if !text.is_empty() {
                    cx.write_to_clipboard(gpui::ClipboardItem::new_string(text));
                    self.show_notice(crate::i18n::t("Chemin copi\u{e9}"), cx);
                }
            }
            MenuAction::Settings => self.toggle_settings(cx),
            MenuAction::NewTextFile => self.new_text_file(cx),
            MenuAction::Terminal => self.open_terminal(),
            MenuAction::Pin => self.pin_selection(cx),
            MenuAction::Wallpaper => self.set_wallpaper(cx),
            MenuAction::RotateLeft => self.rotate_images(false, cx),
            MenuAction::RotateRight => self.rotate_images(true, cx),
            MenuAction::RunAdmin => self.run_as_admin(),
            MenuAction::MoreOptions => self.native_menu(window, cx),
            MenuAction::GoTo => self.open_goto(window, cx),
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
                .when(enabled, |d| d.cursor_pointer().on_click(cx.listener(move |this, _, window, cx| this.run_menu(action, window, cx))))
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
        if self.tab_menu.take().is_some() {
            cx.notify();
            return;
        }
        if self.all_tabs {
            self.all_tabs = false;
            cx.notify();
            return;
        }
        if self.settings_open {
            self.settings_open = false;
            cx.notify();
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
        self.open_goto_with(window, cx, None);
    }

    /// "Aller au dossier" prerempli ("Se connecter au serveur" : `\\\\`).
    pub fn open_goto_with(&mut self, window: &mut Window, cx: &mut Context<Self>, preset: Option<&str>) {
        self.commit_rename(cx);
        self.menu = None;
        let start = preset.map(str::to_owned).unwrap_or_else(|| format!("{}\\", self.cwd.display().to_string().trim_end_matches('\\')));
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
            self.show_notice(crate::i18n::tf(crate::i18n::t("Dossier introuvable : {0}"), &[("0", (text.trim()).to_string())]), cx);
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
                .cursor_pointer().on_click(cx.listener(move |this, _, _, cx| {
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
                        .child(div().text_size(px(13.)).child(crate::i18n::t("Aller au dossier :")))
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
                                        .child(crate::i18n::t("Annuler"))
                                        .cursor_pointer().on_click(cx.listener(|this, _, _, cx| {
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
                                        .child(crate::i18n::t("Aller"))
                                        .cursor_pointer().on_click(cx.listener(|this, _, _, cx| this.goto_commit(cx))),
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
            e.set_placeholder(crate::i18n::t("Rechercher"));
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
            self.entries = self.load_entries();
            self.clear_selection();
            self.rebuild_columns();
            cx.notify();
            return;
        }
        let root = self.cwd.clone();
        let hidden = self.show_hidden;
        let (content, kind) = (self.search_content, self.search_kind);
        cx.spawn(async move |this, cx| {
            let results = cx.background_executor().spawn(async move { fs::search(&root, &query, hidden, 3000, content, kind) }).await;
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

impl Finder {
    /// Barre de criteres sous la barre d'outils pendant une recherche : portee du texte et type d'element.
    pub fn render_search_bar(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        self.search.as_ref()?;
        let chip = |id: (&'static str, usize), label: &'static str, on: bool, cx: &mut Context<Self>, f: fn(&mut Finder)| {
            div()
                .id(id)
                .px(px(10.))
                .h(px(22.))
                .flex()
                .items_center()
                .rounded_full()
                .text_size(px(12.))
                .text_color(rgb(if on { theme::SELECTION_TEXT } else { theme::TEXT }))
                .when(on, |d| d.bg(rgb(theme::SELECTION)))
                .when(!on, |d| d.bg(rgb(0xe9e9ec)).hover(|s| s.bg(rgb(0xe0e0e4))))
                .child(label)
                .cursor_pointer().on_click(cx.listener(move |this, _, _, cx| {
                    f(this);
                    this.run_search(cx);
                    cx.notify();
                }))
        };
        let (content, kind) = (self.search_content, self.search_kind);
        let label = |t: &'static str| div().text_size(px(12.)).text_color(rgb(theme::TEXT_SECONDARY)).child(t);
        Some(
            div()
                .flex_none()
                .h(px(32.))
                .px(px(14.))
                .flex()
                .items_center()
                .gap(px(6.))
                .child(label(crate::i18n::t("Chercher")))
                .child(chip(("sb-name", 0), crate::i18n::t("Noms"), !content, cx, |f| f.search_content = false))
                .child(chip(("sb-name", 1), crate::i18n::t("Noms et contenu"), content, cx, |f| f.search_content = true))
                .child(div().w(px(10.)))
                .child(label(crate::i18n::t("Type")))
                .child(chip(("sb-kind", 0), crate::i18n::t("Tous"), kind == 0, cx, |f| f.search_kind = 0))
                .child(chip(("sb-kind", 1), crate::i18n::t("Images"), kind == 1, cx, |f| f.search_kind = 1))
                .child(chip(("sb-kind", 2), crate::i18n::t("Documents"), kind == 2, cx, |f| f.search_kind = 2))
                .child(chip(("sb-kind", 3), crate::i18n::t("Dossiers"), kind == 3, cx, |f| f.search_kind = 3))
                .child(chip(("sb-kind", 4), crate::i18n::t("Audio / vid\u{e9}o"), kind == 4, cx, |f| f.search_kind = 4))
                .into_any_element(),
        )
    }
}
