mod csvview;
mod apps;
mod editor;
mod selection;
mod prefs;
mod bulk;
mod archive;
mod settings;
mod symbols;
mod tree;
mod trash;
mod undo;
mod tags;
mod transfer;
mod tabs;
mod traffic;
mod uninstall;
mod dragdrop;
mod dragout;
mod fileops;
mod fs;
mod icons;
mod info;
mod layout;
mod ops;
mod pdf;
mod video;
mod overlays;
mod quicklook;
mod sidebar;
mod syscb;
mod theme;
mod toolbar;
mod views;

use std::collections::BTreeSet;
use std::path::PathBuf;

use gpui::{
    App, Application, Bounds, Context, FocusHandle, Focusable, KeyBinding, Modifiers, UniformListScrollHandle, Window,
    WindowBounds, WindowOptions, actions, div, prelude::*, px, size,
};
use crate::theme::rgb;

use fs::Entry;

actions!(
    finder,
    [
        MoveUp, MoveDown, MoveLeft, MoveRight, ExtendUp, ExtendDown, OpenSelection, GoParent, GoBack, GoForward,
        ToggleQuickLook, CloseQuickLook, ViewIcons, ViewList, ViewColumns, ViewGallery, ToggleHidden,
        NewFolder, RenameSelection, TrashSelection, CopySelection, CutSelection, PasteSelection, DuplicateSelection,
        SelectAll, GoToFolder, FocusSearch, Refresh, Dismiss, GetInfo, ShowAllTabs, ConnectServer, NewWindow, UndoOp, RedoOp, OpenSettings, ToggleSidebar, TogglePathBar, ToggleStatusBar, ViewOptions, NewTab, CloseTab, NextTab, PrevTab,
    ]
);

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Icons,
    List,
    Columns,
    Gallery,
}

pub struct Column {
    pub entries: Vec<Entry>,
    pub selected: Option<usize>,
}

pub struct Finder {
    pub cwd: PathBuf,
    pub history: Vec<PathBuf>,
    pub history_pos: usize,
    pub entries: Vec<Entry>,
    /// Element "principal" (dernier clique) : sert a l'apercu, a la barre d'etat et a la navigation clavier.
    pub selected: Option<usize>,
    /// Ensemble des elements selectionnes (sélection multiple).
    pub marked: BTreeSet<usize>,
    pub anchor: Option<usize>,
    pub columns: Vec<Column>,
    pub view: ViewMode,
    pub ql: Option<gpui::WindowHandle<quicklook::QuickLook>>,
    pub show_hidden: bool,
    pub places: Vec<sidebar::Place>,
    pub focus: FocusHandle,
    pub list_scroll: UniformListScrollHandle,
    pub icons: icons::IconCache,
    // Operations sur fichiers et fenetres superposees (voir fileops.rs et overlays.rs).
    pub clipboard: Option<fileops::Clip>,
    pub rename: Option<fileops::Rename>,
    pub menu: Option<overlays::MenuState>,
    pub goto: Option<overlays::GoTo>,
    pub info: Option<info::InfoState>,
    pub confirm: Option<uninstall::Confirm>,
    pub zoom_dot_hover: bool,
    pub expanded: std::collections::HashSet<PathBuf>,
    pub tab_menu: Option<(usize, gpui::Point<gpui::Pixels>)>,
    pub all_tabs: bool,
    pub spring_target: Option<PathBuf>,
    pub search_content: bool,
    pub search_kind: u8,
    pub sidebar_w: f32,
    pub undo_stack: Vec<undo::Op>,
    pub redo_stack: Vec<undo::Op>,
    pub settings_open: bool,
    pub show_sidebar: bool,
    pub show_path_bar: bool,
    pub show_status_bar: bool,
    pub info_open: [bool; 6],
    pub bulk: Option<bulk::Bulk>,
    pub tag_pop: Option<tags::TagPop>,
    pub prefs: prefs::Store,
    pub view_opts: bool,
    pub view_forced: bool,
    pub job: Option<transfer::Job>,
    pub conflict: Option<transfer::Conflict>,
    pub tabs: Vec<tabs::Tab>,
    pub active_tab: usize,
    pub type_buf: String,
    pub type_at: Option<std::time::Instant>,
    pub band: Option<selection::Band>,
    pub zoom_pop_hover: bool,
    pub layouts: layout::Layouts,
    pub icon_scroll: gpui::ScrollHandle,
    pub search: Option<overlays::Search>,
    pub search_gen: u64,
    pub notice: Option<(u64, String)>,
    pub notice_id: u64,
    /// Rendre le focus a la fenetre principale au prochain rendu (apres une saisie en ligne).
    pub needs_focus: bool,
}

impl Finder {
    /// Fenetre sur `start` (sinon : argument de la ligne de commande, puis dossier d'ouverture des reglages).
    pub fn new_at(cx: &mut Context<Self>, start: Option<PathBuf>) -> Self {
        let (done_tx, done_rx) = flume::unbounded::<()>();
        let icons = icons::IconCache::new(done_tx);
        // Repeint quand des icones arrivent ; les signaux rapproches sont regroupes.
        cx.spawn(async move |this, cx| {
            while done_rx.recv_async().await.is_ok() {
                cx.background_executor().timer(std::time::Duration::from_millis(40)).await;
                while done_rx.try_recv().is_ok() {}
                if this.update(cx, |_, cx| cx.notify()).is_err() {
                    break;
                }
            }
        })
        .detach();
        let start = start
            .or_else(|| std::env::args_os().nth(1).map(PathBuf::from).filter(|p| p.is_dir() || fs::is_virtual(p)))
            .unwrap_or_else(settings::start_folder);
        // Test : --sort=name|date|size|kind, suffixe ":desc" pour l'ordre inverse.
        if let Some(s) = std::env::args().find_map(|a| a.strip_prefix("--sort=").map(str::to_owned)) {
            let (k, d) = s.split_once(':').unwrap_or((&s, ""));
            let key = match k {
                "date" => fs::SortKey::Modified,
                "size" => fs::SortKey::Size,
                "kind" => fs::SortKey::Kind,
                _ => fs::SortKey::Name,
            };
            fs::set_sort(key, d != "desc");
        }
        let mut f = Self {
            cwd: start.clone(),
            history: vec![start],
            history_pos: 0,
            entries: vec![],
            selected: None,
            marked: BTreeSet::new(),
            anchor: None,
            columns: vec![],
            // --view=... (tests) > dernier mode choisi > mosaique par defaut.
            view: match std::env::args()
                .find_map(|a| a.strip_prefix("--view=").map(str::to_owned))
                .or_else(layout::load_view)
                .as_deref()
            {
                Some("list") => ViewMode::List,
                Some("columns") => ViewMode::Columns,
                Some("gallery") => ViewMode::Gallery,
                _ => ViewMode::Icons,
            },
            ql: None,
            show_hidden: layout::get_setting("hidden").as_deref() == Some("1"),
            places: sidebar::default_places(),
            focus: cx.focus_handle(),
            list_scroll: UniformListScrollHandle::new(),
            icons,
            clipboard: None,
            rename: None,
            menu: None,
            goto: None,
            info: None,
            confirm: None,
            zoom_dot_hover: false,
            expanded: std::collections::HashSet::new(),
            tab_menu: None,
            all_tabs: false,
            spring_target: None,
            search_content: false,
            search_kind: 0,
            sidebar_w: layout::get_setting("sidebar_w").and_then(|v| v.parse().ok()).map(|w: f32| w.clamp(135.0, 400.0)).unwrap_or(theme::SIDEBAR_WIDTH),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            settings_open: false,
            show_sidebar: layout::get_setting("sidebar").as_deref() != Some("0"),
            show_path_bar: layout::get_setting("pathbar").as_deref() != Some("0"),
            show_status_bar: layout::get_setting("statusbar").as_deref() != Some("0"),
            info_open: [true, true, true, true, true, false],
            bulk: None,
            tag_pop: None,
            prefs: prefs::Store::load(),
            view_opts: false,
            view_forced: std::env::args().any(|a| a.starts_with("--view=")),
            job: None,
            conflict: None,
            tabs: Vec::new(),
            active_tab: 0,
            type_buf: String::new(),
            type_at: None,
            band: None,
            zoom_pop_hover: false,
            layouts: layout::load(),
            icon_scroll: gpui::ScrollHandle::new(),
            search: None,
            search_gen: 0,
            notice: None,
            notice_id: 0,
            needs_focus: false,
        };
        f.reload();
        f.tabs = vec![tabs::Tab::at(f.cwd.clone(), f.view)];
        // `--select=<nom>` preselectionne un element (tests automatiques).
        if let Some(name) = std::env::args().find_map(|a| a.strip_prefix("--select=").map(str::to_owned)) {
            if let Some(ix) = f.entries.iter().position(|e| e.name.as_ref() == name) {
                f.set_single(ix);
                f.rebuild_columns();
            }
        }
        f
    }

    /// Relit le dossier courant ; selectionne le premier element.
    pub fn reload(&mut self) {
        self.apply_folder_prefs();
        self.entries = self.load_entries();
        match self.entries.is_empty() {
            true => self.clear_selection(),
            false => self.set_single(0),
        }
        self.rebuild_columns();
    }

    /// Relit le dossier et selectionne les elements dont le chemin est dans `paths`.
    pub fn reload_select(&mut self, paths: &[PathBuf]) {
        self.entries = self.load_entries();
        self.marked = self.entries.iter().enumerate().filter(|(_, e)| paths.contains(&e.path)).map(|(i, _)| i).collect();
        self.selected = self.marked.iter().next_back().copied();
        self.anchor = self.marked.iter().next().copied();
        self.rebuild_columns();
    }

    pub fn clear_selection(&mut self) {
        self.selected = None;
        self.marked.clear();
        self.anchor = None;
    }

    /// Selection d'un seul element, sans notification ni defilement.
    pub fn set_single(&mut self, ix: usize) {
        self.selected = Some(ix);
        self.marked.clear();
        self.marked.insert(ix);
        self.anchor = Some(ix);
    }

    pub fn rebuild_columns(&mut self) {
        self.columns = vec![Column { entries: self.entries.clone(), selected: self.selected }];
    }

    pub fn navigate(&mut self, path: PathBuf, record: bool, cx: &mut Context<Self>) {
        if !(fs::is_virtual(&path) || path.is_dir()) {
            return;
        }
        self.cancel_rename_silent();
        self.menu = None;
        self.close_search_silent();
        if record {
            self.history.truncate(self.history_pos + 1);
            self.history.push(path.clone());
            self.history_pos = self.history.len() - 1;
        }
        self.cwd = path;
        self.reload();
        cx.notify();
    }

    /// Clic sur un emplacement de la barre laterale : dossier, emplacement virtuel, ou fenetre de l'Explorateur
    /// pour le reseau et la corbeille.
    pub fn open_place(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        let shell = match path.to_str() {
            Some("::net") => Some("shell:NetworkPlacesFolder"),
            _ => None,
        };
        match shell {
            Some(target) => {
                let _ = std::process::Command::new("explorer.exe").arg(target).spawn();
            }
            None => self.navigate(path, true, cx),
        }
    }

    pub fn go_back(&mut self, cx: &mut Context<Self>) {
        if self.history_pos > 0 {
            self.history_pos -= 1;
            let p = self.history[self.history_pos].clone();
            self.navigate(p, false, cx);
        }
    }

    pub fn go_forward(&mut self, cx: &mut Context<Self>) {
        if self.history_pos + 1 < self.history.len() {
            self.history_pos += 1;
            let p = self.history[self.history_pos].clone();
            self.navigate(p, false, cx);
        }
    }

    fn go_parent(&mut self, cx: &mut Context<Self>) {
        if let Some(p) = self.cwd.parent().map(|p| p.to_path_buf()) {
            // On reselectionne le dossier qu'on quitte, comme le Finder.
            let from = self.cwd.clone();
            self.navigate(p, true, cx);
            self.reload_select(&[from]);
        }
    }

    pub fn can_back(&self) -> bool {
        self.history_pos > 0
    }

    pub fn can_forward(&self) -> bool {
        self.history_pos + 1 < self.history.len()
    }

    /// Entree actuellement selectionnee (en vue colonnes : la plus profonde).
    pub fn selected_entry(&self) -> Option<&Entry> {
        if self.view == ViewMode::Columns {
            let c = self.active_column();
            let col = self.columns.get(c)?;
            return col.entries.get(col.selected?);
        }
        self.entries.get(self.selected?)
    }

    fn active_column(&self) -> usize {
        self.columns.iter().rposition(|c| c.selected.is_some()).unwrap_or(0)
    }

    /// Chemins des elements selectionnes (ceux du Finder, ou celui de la colonne active).
    pub fn selected_paths(&self) -> Vec<PathBuf> {
        if self.view == ViewMode::Columns {
            return self.selected_entry().map(|e| vec![e.path.clone()]).unwrap_or_default();
        }
        self.marked.iter().filter_map(|&i| self.entries.get(i)).map(|e| e.path.clone()).collect()
    }

    /// Selection simple (clic, fleches).
    pub fn select(&mut self, ix: usize, cx: &mut Context<Self>) {
        if ix < self.entries.len() {
            self.commit_rename(cx);
            self.set_single(ix);
            self.rebuild_columns();
            if let Some(Column { selected, .. }) = self.columns.first_mut() {
                *selected = Some(ix);
            }
            self.expand_columns(0);
            cx.notify();
        }
    }

    /// Clic avec modificateurs : Ctrl bascule l'element, Maj etend depuis l'ancre.
    pub fn click_select(&mut self, ix: usize, mods: Modifiers, cx: &mut Context<Self>) {
        if ix >= self.entries.len() {
            return;
        }
        if mods.shift {
            self.commit_rename(cx);
            let a = self.anchor.unwrap_or(ix);
            self.marked = (a.min(ix)..=a.max(ix)).collect();
            self.selected = Some(ix);
            cx.notify();
        } else if mods.control {
            self.commit_rename(cx);
            if !self.marked.remove(&ix) {
                self.marked.insert(ix);
                self.selected = Some(ix);
            } else {
                self.selected = self.marked.iter().next_back().copied();
            }
            self.anchor = Some(ix);
            cx.notify();
        } else {
            self.select(ix, cx);
        }
    }

    pub fn select_all(&mut self, cx: &mut Context<Self>) {
        if self.view == ViewMode::Columns || self.entries.is_empty() {
            return;
        }
        self.marked = (0..self.entries.len()).collect();
        self.selected.get_or_insert(0);
        cx.notify();
    }

    pub fn select_in_column(&mut self, col: usize, ix: usize, cx: &mut Context<Self>) {
        if let Some(c) = self.columns.get_mut(col) {
            if ix < c.entries.len() {
                c.selected = Some(ix);
                self.columns.truncate(col + 1);
                if col == 0 {
                    self.set_single(ix);
                }
                self.expand_columns(col);
                cx.notify();
            }
        }
    }

    /// Si l'entree selectionnee de la colonne `col` est un dossier, ajoute sa colonne.
    pub fn expand_columns(&mut self, col: usize) {
        self.columns.truncate(col + 1);
        let Some(c) = self.columns.get(col) else { return };
        let Some(entry) = c.selected.and_then(|i| c.entries.get(i)) else { return };
        if entry.is_dir {
            let entries = fs::read_dir(&entry.path, self.show_hidden);
            self.columns.push(Column { entries, selected: None });
        }
    }

    pub fn move_vertical(&mut self, delta: isize, cx: &mut Context<Self>) {
        if self.view == ViewMode::Columns {
            let c = self.active_column();
            let Some(col) = self.columns.get(c) else { return };
            if col.entries.is_empty() {
                return;
            }
            let cur = col.selected.map(|i| i as isize).unwrap_or(-delta.signum().min(0) - 1 + 1);
            let next = (cur + delta).clamp(0, col.entries.len() as isize - 1) as usize;
            self.select_in_column(c, next, cx);
            return;
        }
        if self.entries.is_empty() {
            return;
        }
        let cur = self.selected.map(|i| i as isize).unwrap_or(if delta > 0 { -1 } else { 0 });
        let next = (cur + delta).clamp(0, self.entries.len() as isize - 1) as usize;
        self.select(next, cx);
        self.reveal(next);
    }

    /// Defilement minimal : la liste ne bouge que si la ligne sort de la zone visible (jamais au simple clic).
    pub fn reveal(&self, ix: usize) {
        if self.view != ViewMode::List {
            return;
        }
        let state = self.list_scroll.0.borrow();
        let handle = &state.base_handle;
        let view_h = f32::from(handle.bounds().size.height);
        if view_h <= 0.0 {
            return;
        }
        let ri = self.display_rows().iter().position(|r| matches!(r, views::Row::Item(j) if *j == ix)).unwrap_or(ix);
        let top = ri as f32 * theme::LIST_ROW_HEIGHT;
        let bottom = top + theme::LIST_ROW_HEIGHT;
        let off = -f32::from(handle.offset().y);
        let target = if top < off {
            top
        } else if bottom > off + view_h {
            bottom - view_h
        } else {
            return;
        };
        handle.set_offset(gpui::point(handle.offset().x, px(-target)));
    }

    /// Maj+fleche : etend la selection depuis l'ancre.
    fn extend_selection(&mut self, delta: isize, cx: &mut Context<Self>) {
        if self.view == ViewMode::Columns || self.entries.is_empty() {
            return;
        }
        let cur = self.selected.unwrap_or(0) as isize;
        let next = (cur + delta).clamp(0, self.entries.len() as isize - 1) as usize;
        let a = self.anchor.unwrap_or(cur as usize);
        self.marked = (a.min(next)..=a.max(next)).collect();
        self.selected = Some(next);
        self.reveal(next);
        cx.notify();
    }

    pub fn move_horizontal(&mut self, dx: isize, cx: &mut Context<Self>) {
        match self.view {
            ViewMode::Icons | ViewMode::Gallery => self.move_vertical(dx, cx),
            ViewMode::Columns => {
                let c = self.active_column();
                if dx > 0 {
                    if let Some(next) = self.columns.get(c + 1) {
                        if !next.entries.is_empty() {
                            self.select_in_column(c + 1, 0, cx);
                        }
                    }
                } else if c > 0 {
                    self.columns.truncate(c + 1);
                    self.columns[c].selected = None;
                    cx.notify();
                }
            }
            ViewMode::List => self.list_horizontal(dx, cx),
        }
    }

    pub fn open_selection(&mut self, cx: &mut Context<Self>) {
        let Some(e) = self.selected_entry().cloned() else { return };
        if e.is_dir {
            if let Err(err) = std::fs::read_dir(&e.path) {
                self.show_notice(format!("Impossible d'ouvrir \u{ab} {} \u{bb} \u{2014} {err}", e.name), cx);
                return;
            }
            self.navigate(e.path, true, cx);
        } else {
            let _ = std::process::Command::new("cmd").args(["/C", "start", "", &e.path.to_string_lossy()]).spawn();
        }
    }

    /// Trie par colonne ; un second clic sur la meme colonne inverse l'ordre. La selection est conservee.
    pub fn sort_by(&mut self, key: fs::SortKey, cx: &mut Context<Self>) {
        let (cur, asc) = fs::sort();
        fs::set_sort(key, if cur == key { !asc } else { true });
        self.remember_sort();
        self.resort_in_place();
        cx.notify();
    }

    /// Retrie les elements affiches selon le tri courant en gardant la selection.
    pub fn resort_in_place(&mut self) {
        let keep: Vec<PathBuf> = self.marked.iter().filter_map(|&i| self.entries.get(i)).map(|e| e.path.clone()).collect();
        if self.search_query().is_some() {
            fs::resort(&mut self.entries);
        } else {
            self.entries = self.load_entries();
        }
        self.marked = self.entries.iter().enumerate().filter(|(_, e)| keep.contains(&e.path)).map(|(i, _)| i).collect();
        self.selected = self.marked.iter().next_back().copied();
        self.anchor = self.marked.iter().next().copied();
        self.rebuild_columns();
    }

    pub fn set_view(&mut self, view: ViewMode, cx: &mut Context<Self>) {
        self.view = view;
        self.remember_view(view);
        self.resort_in_place();
        if view == ViewMode::Columns {
            self.rebuild_columns();
            if let Some(ix) = self.selected {
                self.columns[0].selected = Some(ix);
                self.expand_columns(0);
            }
        }
        cx.notify();
    }

    /// Releve le dossier (ou relance la recherche) en gardant la selection.
    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        let keep = self.selected_paths();
        if self.search_query().is_some() {
            self.run_search(cx);
        } else {
            self.reload_select(&keep);
            if self.marked.is_empty() && !self.entries.is_empty() {
                self.set_single(0);
                self.rebuild_columns();
            }
        }
        cx.notify();
    }
}

impl Focusable for Finder {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for Finder {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.needs_focus {
            window.focus(&self.focus);
            self.needs_focus = false;
        }
        let content = match self.view {
            ViewMode::List => self.render_list(cx).into_any_element(),
            ViewMode::Icons => self.render_icons(window, cx).into_any_element(),
            ViewMode::Columns => self.render_columns(cx).into_any_element(),
            ViewMode::Gallery => self.render_gallery(cx).into_any_element(),
        };
        // Pendant une saisie (renommage, "Aller au dossier", recherche), les raccourcis du Finder se taisent.
        let typing = self.text_input_active(window, cx);

        div()
            .key_context(if typing { "Finder TextInput" } else { "Finder" })
            .track_focus(&self.focus)
            .on_action(cx.listener(|this, _: &MoveUp, _, cx| this.move_vertical(-1, cx)))
            .on_action(cx.listener(|this, _: &MoveDown, _, cx| this.move_vertical(1, cx)))
            .on_action(cx.listener(|this, _: &MoveLeft, _, cx| this.move_horizontal(-1, cx)))
            .on_action(cx.listener(|this, _: &MoveRight, _, cx| this.move_horizontal(1, cx)))
            .on_action(cx.listener(|this, _: &ExtendUp, _, cx| this.extend_selection(-1, cx)))
            .on_action(cx.listener(|this, _: &ExtendDown, _, cx| this.extend_selection(1, cx)))
            .on_action(cx.listener(|this, _: &OpenSelection, _, cx| this.open_selection(cx)))
            .on_action(cx.listener(|this, _: &GoParent, _, cx| this.go_parent(cx)))
            .on_action(cx.listener(|this, _: &GoBack, _, cx| this.go_back(cx)))
            .on_action(cx.listener(|this, _: &GoForward, _, cx| this.go_forward(cx)))
            .on_action(cx.listener(|this, _: &ToggleQuickLook, _, cx| this.toggle_quick_look(cx)))
            .on_action(cx.listener(|this, _: &CloseQuickLook, _, cx| {
                this.close_quick_look(cx);
            }))
            .on_action(cx.listener(|this, _: &Dismiss, window, cx| this.dismiss(window, cx)))
            .on_action(cx.listener(|this, _: &GetInfo, window, cx| this.show_info(window, cx)))
            .on_action(cx.listener(|this, _: &ShowAllTabs, _, cx| this.toggle_all_tabs(cx)))
            .on_action(cx.listener(|this, _: &ConnectServer, window, cx| this.open_goto_with(window, cx, Some("\\\\"))))
            .on_action(cx.listener(|this, _: &NewWindow, _, cx| {
                let here = (!fs::is_virtual(&this.cwd)).then(|| this.cwd.clone());
                cx.defer(move |app| {
                    open_finder_window(app, here);
                });
            }))
            .on_action(cx.listener(|this, _: &UndoOp, _, cx| this.undo(cx)))
            .on_action(cx.listener(|this, _: &RedoOp, _, cx| this.redo(cx)))
            .on_action(cx.listener(|this, _: &OpenSettings, _, cx| this.toggle_settings(cx)))
            .on_action(cx.listener(|this, _: &ToggleSidebar, _, cx| {
                this.show_sidebar = !this.show_sidebar;
                layout::set_setting("sidebar", if this.show_sidebar { "1" } else { "0" });
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &TogglePathBar, _, cx| {
                this.show_path_bar = !this.show_path_bar;
                layout::set_setting("pathbar", if this.show_path_bar { "1" } else { "0" });
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &ToggleStatusBar, _, cx| {
                this.show_status_bar = !this.show_status_bar;
                layout::set_setting("statusbar", if this.show_status_bar { "1" } else { "0" });
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &ViewOptions, _, cx| this.toggle_view_options(cx)))
            .on_action(cx.listener(|this, _: &NewTab, _, cx| this.new_tab(None, cx)))
            .on_action(cx.listener(|this, _: &CloseTab, window, cx| {
                let i = this.active_tab;
                this.close_tab(i, window, cx)
            }))
            .on_action(cx.listener(|this, _: &NextTab, _, cx| this.cycle_tab(1, cx)))
            .on_action(cx.listener(|this, _: &PrevTab, _, cx| this.cycle_tab(-1, cx)))
            .on_action(cx.listener(|this, _: &ViewIcons, _, cx| this.set_view(ViewMode::Icons, cx)))
            .on_action(cx.listener(|this, _: &ViewList, _, cx| this.set_view(ViewMode::List, cx)))
            .on_action(cx.listener(|this, _: &ViewColumns, _, cx| this.set_view(ViewMode::Columns, cx)))
            .on_action(cx.listener(|this, _: &ViewGallery, _, cx| this.set_view(ViewMode::Gallery, cx)))
            .on_action(cx.listener(|this, _: &ToggleHidden, _, cx| {
                this.show_hidden = !this.show_hidden;
                this.refresh(cx);
            }))
            .on_action(cx.listener(|this, _: &NewFolder, window, cx| this.new_folder(window, cx)))
            .on_action(cx.listener(|this, _: &RenameSelection, window, cx| this.begin_rename(window, cx)))
            .on_action(cx.listener(|this, _: &TrashSelection, _, cx| this.trash_selection(cx)))
            .on_action(cx.listener(|this, _: &CopySelection, _, cx| this.copy_selection(false, cx)))
            .on_action(cx.listener(|this, _: &CutSelection, _, cx| this.copy_selection(true, cx)))
            .on_action(cx.listener(|this, _: &PasteSelection, _, cx| this.paste(cx)))
            .on_action(cx.listener(|this, _: &DuplicateSelection, _, cx| this.duplicate(cx)))
            .on_action(cx.listener(|this, _: &SelectAll, _, cx| this.select_all(cx)))
            .on_action(cx.listener(|this, _: &GoToFolder, window, cx| this.open_goto(window, cx)))
            .on_action(cx.listener(|this, _: &FocusSearch, window, cx| this.open_search(window, cx)))
            .on_action(cx.listener(|this, _: &Refresh, _, cx| this.refresh(cx)))
            .on_mouse_down(gpui::MouseButton::Navigate(gpui::NavigationDirection::Back), cx.listener(|this, _, _, cx| this.go_back(cx)))
            .on_mouse_down(
                gpui::MouseButton::Navigate(gpui::NavigationDirection::Forward),
                cx.listener(|this, _, _, cx| this.go_forward(cx)),
            )
            .relative()
            .size_full()
            .on_mouse_move(cx.listener(|this, ev: &gpui::MouseMoveEvent, _, cx| this.update_zoom_hover(ev.position, cx)))
            // Redimensionnement de la barre laterale (135 a 400 pt), memorise a la fin du glisser.
            .on_drag_move::<sidebar::SidebarResize>(cx.listener(|this, ev: &gpui::DragMoveEvent<sidebar::SidebarResize>, _, cx| {
                this.sidebar_w = f32::from(ev.event.position.x).clamp(135.0, 400.0);
                cx.notify();
            }))
            .on_drop(cx.listener(|this, _: &sidebar::SidebarResize, _, _| {
                layout::set_setting("sidebar_w", &this.sidebar_w.to_string());
            }))
            // Glisser des elements hors de la fenetre : on passe la main au glisser-deposer natif (Explorateur, bureau...).
            .on_drag_move::<dragdrop::DragPaths>(cx.listener(|_, ev: &gpui::DragMoveEvent<dragdrop::DragPaths>, window, cx| {
                let p = ev.event.position;
                let vp = window.viewport_size();
                if p.x < px(0.) || p.y < px(0.) || p.x > vp.width || p.y > vp.height {
                    let paths = ev.drag(cx).paths.clone();
                    cx.stop_active_drag(window);
                    let this = cx.entity();
                    cx.defer(move |app| {
                        let effect = dragout::start(&paths);
                        this.update(app, |f, cx| f.after_drag_out(effect, cx));
                    });
                }
            }))
            .on_key_down(cx.listener(|this, ev: &gpui::KeyDownEvent, window, cx| {
                let m = ev.keystroke.modifiers;
                if m.control || m.alt || m.platform || this.text_input_active(window, cx) || this.confirm.is_some() {
                    return;
                }
                if let Some(ch) = ev.keystroke.key_char.as_deref().filter(|c| c.chars().count() == 1 && *c != " ") {
                    this.type_select(ch, cx);
                }
            }))
            // Fenetre sans cadre Windows : coins arrondis a la macOS (fond de fenetre transparent).
            .rounded(px(15.))
            .border_1()
            .border_color(crate::theme::rgba(0x00000026))
            .overflow_hidden()
            .flex()
            .bg(rgb(theme::BG))
            .text_color(rgb(theme::TEXT))
            .text_size(px(13.))
            .font_family("Segoe UI Variable Text")
            // Barre laterale sur toute la hauteur (avec les boutons de fenetre), contenu a droite.
            .children(if self.show_sidebar { Some(self.render_sidebar(cx)) } else { None })
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .flex()
                    .flex_col()
                    .child(self.render_toolbar(cx))
                    .children(self.render_tab_bar(cx))
                    .children(self.render_search_bar(cx))
                    .child(div().flex_1().min_h_0().flex().flex_col().child(content))
                    .child(self.render_bottom_bar(cx)),
            )
            .children(self.render_zoom_menu(cx))
            .children(self.render_menu(window, cx))
            .children(self.render_goto(cx))
            .children(self.render_info(cx))
            .children(self.render_confirm(cx))
            .children(self.render_all_tabs(cx))
            .children(self.render_tab_menu(cx))
            .children(self.render_settings(cx))
            .children(self.render_bulk(cx))
            .children(self.render_tag_pop(cx))
            .children(self.render_view_options(cx))
            .children(self.render_progress(cx))
            .children(self.render_conflict(cx))
            .children(self.render_notice())
    }
}

fn main() {
    // Mode sombre : --dark / --light (tests), sinon celui de Windows, surveille en continu.
    let forced = if std::env::args().any(|a| a == "--dark") {
        Some(true)
    } else if std::env::args().any(|a| a == "--light") {
        Some(false)
    } else {
        None
    };
    theme::set_dark(forced.unwrap_or_else(theme::system_is_dark));
    Application::new().run(move |cx: &mut App| {
        // Hors saisie seulement : pendant un renommage / une recherche, les touches vont au champ de texte.
        const FIN: &str = "Finder && !TextInput";
        cx.bind_keys([
            KeyBinding::new("up", MoveUp, Some(FIN)),
            KeyBinding::new("down", MoveDown, Some(FIN)),
            KeyBinding::new("left", MoveLeft, Some(FIN)),
            KeyBinding::new("right", MoveRight, Some(FIN)),
            KeyBinding::new("shift-up", ExtendUp, Some(FIN)),
            KeyBinding::new("shift-down", ExtendDown, Some(FIN)),
            // Comme le Finder : Entree renomme ; on ouvre avec Ctrl+O, Ctrl+Bas ou double-clic.
            KeyBinding::new("enter", RenameSelection, Some(FIN)),
            KeyBinding::new("f2", RenameSelection, Some(FIN)),
            KeyBinding::new("ctrl-o", OpenSelection, Some(FIN)),
            KeyBinding::new("ctrl-down", OpenSelection, Some(FIN)),
            KeyBinding::new("alt-up", GoParent, Some(FIN)),
            KeyBinding::new("ctrl-up", GoParent, Some(FIN)),
            KeyBinding::new("alt-left", GoBack, Some(FIN)),
            KeyBinding::new("ctrl-[", GoBack, Some(FIN)),
            KeyBinding::new("alt-right", GoForward, Some(FIN)),
            KeyBinding::new("ctrl-]", GoForward, Some(FIN)),
            KeyBinding::new("space", ToggleQuickLook, Some(FIN)),
            KeyBinding::new("ctrl-y", ToggleQuickLook, Some(FIN)),
            KeyBinding::new("delete", TrashSelection, Some(FIN)),
            KeyBinding::new("ctrl-backspace", TrashSelection, Some(FIN)),
            KeyBinding::new("ctrl-c", CopySelection, Some(FIN)),
            KeyBinding::new("ctrl-x", CutSelection, Some(FIN)),
            KeyBinding::new("ctrl-v", PasteSelection, Some(FIN)),
            KeyBinding::new("ctrl-d", DuplicateSelection, Some(FIN)),
            KeyBinding::new("ctrl-a", SelectAll, Some(FIN)),
            KeyBinding::new("ctrl-i", GetInfo, Some(FIN)),
            KeyBinding::new("ctrl-shift-a", ShowAllTabs, Some("Finder")),
            KeyBinding::new("ctrl-k", ConnectServer, Some(FIN)),
            KeyBinding::new("ctrl-n", NewWindow, Some("Finder")),
            KeyBinding::new("ctrl-z", UndoOp, Some(FIN)),
            KeyBinding::new("ctrl-shift-z", RedoOp, Some(FIN)),
            KeyBinding::new("ctrl-,", OpenSettings, Some("Finder")),
            KeyBinding::new("ctrl-alt-s", ToggleSidebar, Some("Finder")),
            KeyBinding::new("ctrl-alt-p", TogglePathBar, Some("Finder")),
            KeyBinding::new("ctrl-alt-b", ToggleStatusBar, Some("Finder")),
            KeyBinding::new("ctrl-j", ViewOptions, Some(FIN)),
            KeyBinding::new("ctrl-t", NewTab, Some("Finder")),
            KeyBinding::new("ctrl-w", CloseTab, Some("Finder")),
            KeyBinding::new("ctrl-tab", NextTab, Some("Finder")),
            KeyBinding::new("ctrl-shift-tab", PrevTab, Some("Finder")),
            KeyBinding::new("ctrl-shift-n", NewFolder, Some(FIN)),
            KeyBinding::new("f5", Refresh, Some(FIN)),
            KeyBinding::new("ctrl-r", Refresh, Some(FIN)),
            // Toujours actifs : fermer la couche en cours, ouvrir recherche / "Aller au dossier".
            KeyBinding::new("escape", Dismiss, Some("Finder")),
            KeyBinding::new("ctrl-f", FocusSearch, Some("Finder")),
            KeyBinding::new("ctrl-shift-g", GoToFolder, Some("Finder")),
            // Fenetre Quick Look : Espace/Echap ferment, les fleches pilotent la selection du Finder.
            KeyBinding::new("space", CloseQuickLook, Some("QuickLook")),
            KeyBinding::new("escape", CloseQuickLook, Some("QuickLook")),
            KeyBinding::new("ctrl-y", CloseQuickLook, Some("QuickLook")),
            KeyBinding::new("up", MoveUp, Some("QuickLook")),
            KeyBinding::new("down", MoveDown, Some("QuickLook")),
            KeyBinding::new("left", MoveLeft, Some("QuickLook")),
            KeyBinding::new("right", MoveRight, Some("QuickLook")),
            KeyBinding::new("ctrl-1", ViewIcons, Some(FIN)),
            KeyBinding::new("ctrl-2", ViewList, Some(FIN)),
            KeyBinding::new("ctrl-3", ViewColumns, Some(FIN)),
            KeyBinding::new("ctrl-4", ViewGallery, Some(FIN)),
            // Equivalents AZERTY : la rangee des chiffres produit & \u{e9} " ' sans Maj.
            KeyBinding::new("ctrl-&", ViewIcons, Some(FIN)),
            KeyBinding::new("ctrl-\u{e9}", ViewList, Some(FIN)),
            KeyBinding::new("ctrl-\"", ViewColumns, Some(FIN)),
            KeyBinding::new("ctrl-'", ViewGallery, Some(FIN)),
            KeyBinding::new("ctrl-shift-.", ToggleHidden, Some(FIN)),
        ]);

        let main_window = open_finder_window(cx, None);
        // Fermer la fenetre principale quitte l'application (l'apercu seul ne la garde pas ouverte).
        cx.on_window_closed(|cx| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
        square_dwm_frame();
        if forced.is_none() {
            cx.spawn(async move |cx| loop {
                cx.background_executor().timer(std::time::Duration::from_secs(3)).await;
                let dark = cx.background_executor().spawn(async { theme::system_is_dark() }).await;
                if dark != theme::is_dark() {
                    theme::set_dark(dark);
                    cx.update(|app| {
                        for w in app.windows() {
                            let _ = w.update(app, |_, window, _| window.refresh());
                        }
                    })
                    .ok();
                }
            })
            .detach();
        }
        if std::env::args().any(|a| a == "--info") {
            main_window.update(cx, |finder, window, cx| finder.show_info(window, cx)).ok();
        }
        // `--ql` ouvre l'apercu au demarrage (tests automatiques sans simuler de clavier).
        if std::env::args().any(|a| a == "--ql") {
            main_window.update(cx, |finder, _, cx| finder.toggle_quick_look(cx)).ok();
        }
        cx.activate(true);
    });
}

/// Retire le cadre arrondi et la bordure que Windows 11 ajoute a la fenetre : seuls nos propres coins arrondis
/// (18 pt, identiques partout) restent visibles.
fn square_dwm_frame() {
    use windows::Win32::Foundation::{HWND, LPARAM};
    use windows::Win32::Graphics::Dwm::{DWMWA_BORDER_COLOR, DWMWA_NCRENDERING_POLICY, DWMWA_WINDOW_CORNER_PREFERENCE, DwmSetWindowAttribute};
    use windows::Win32::UI::WindowsAndMessaging::{EnumWindows, GetWindowTextW, GetWindowThreadProcessId};
    use windows::core::BOOL;

    extern "system" fn apply(hwnd: HWND, _: LPARAM) -> BOOL {
        unsafe {
            let mut pid = 0u32;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            let mut title = [0u16; 16];
            let n = GetWindowTextW(hwnd, &mut title).max(0) as usize;
            if pid == std::process::id() && String::from_utf16_lossy(&title[..n]) == "Finder" {
                let donotround: i32 = 1; // DWMWCP_DONOTROUND
                let none: u32 = 0xFFFF_FFFE; // DWMWA_COLOR_NONE
                let disabled: i32 = 1; // DWMNCRP_DISABLED : plus d'ombre ni de liseré DWM
                let _ = DwmSetWindowAttribute(hwnd, DWMWA_WINDOW_CORNER_PREFERENCE, &donotround as *const _ as *const _, 4);
                let _ = DwmSetWindowAttribute(hwnd, DWMWA_NCRENDERING_POLICY, &disabled as *const _ as *const _, 4);
                let _ = DwmSetWindowAttribute(hwnd, DWMWA_BORDER_COLOR, &none as *const _ as *const _, 4);
            }
        }
        BOOL(1)
    }
    unsafe {
        let _ = EnumWindows(Some(apply), LPARAM(0));
    }
}

/// Ouvre une fenetre du Finder (sur `start` ou le dossier d'ouverture) et retire le cadre DWM de Windows.
fn open_finder_window(cx: &mut App, start: Option<PathBuf>) -> gpui::WindowHandle<Finder> {
    // Decale legerement les fenetres suivantes pour qu'elles ne se superposent pas exactement.
    let n = cx.windows().len() as f32;
    let mut bounds = Bounds::centered(None, size(px(1000.), px(640.)), cx);
    bounds.origin.x += px(26.0 * n.min(8.0));
    bounds.origin.y += px(26.0 * n.min(8.0));
    let handle = cx
        .open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(gpui::TitlebarOptions {
                    title: Some("Finder".into()),
                    appears_transparent: true,
                    ..Default::default()
                }),
                window_min_size: Some(size(px(640.), px(420.))),
                // Fond transparent : seuls les coins arrondis de notre contenu sont visibles.
                window_background: gpui::WindowBackgroundAppearance::Transparent,
                ..Default::default()
            },
            move |window, cx| {
                let view = cx.new(|cx| Finder::new_at(cx, start));
                window.focus(&view.focus_handle(cx));
                view
            },
        )
        .unwrap();
    square_dwm_frame();
    handle
}
