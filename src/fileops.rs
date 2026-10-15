//! Actions du Finder sur les fichiers : copier / couper / coller, dupliquer, corbeille, nouveau dossier,
//! renommage en ligne. Les erreurs s'affichent dans une pastille d'information en bas de la fenetre.

use std::path::{Path, PathBuf};
use std::time::Duration;

use gpui::{Context, Entity, Focusable, Subscription, Window, prelude::*};

use crate::editor::{Editor, EditorEvent};
use crate::ops;
use crate::Finder;

pub struct Clip {
    pub paths: Vec<PathBuf>,
    pub cut: bool,
}

pub struct Rename {
    pub path: PathBuf,
    pub editor: Entity<Editor>,
    _sub: Subscription,
}

impl Finder {
    /// Vrai si l'element est dans le presse-papiers en mode "couper" (affiche en attenue).
    pub fn is_cut(&self, path: &Path) -> bool {
        self.clipboard.as_ref().map(|c| c.cut && c.paths.iter().any(|p| p == path)).unwrap_or(false)
    }

    /// Pastille d'information (erreur ou confirmation) qui disparait apres 5 secondes.
    pub fn show_notice(&mut self, text: impl Into<String>, cx: &mut Context<Self>) {
        self.notice_id += 1;
        let id = self.notice_id;
        self.notice = Some((id, text.into()));
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(Duration::from_secs(5)).await;
            this.update(cx, |f, cx| {
                if f.notice.as_ref().map(|n| n.0) == Some(id) {
                    f.notice = None;
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    pub fn copy_selection(&mut self, cut: bool, cx: &mut Context<Self>) {
        if cut && self.read_only_place(cx) {
            return;
        }
        let paths = self.selected_paths();
        if paths.is_empty() {
            return;
        }
        let n = paths.len();
        crate::syscb::set_files(&paths, cut);
        self.clipboard = Some(Clip { paths, cut });
        self.show_notice(
            if cut {
                format!("{n} \u{e9}l\u{e9}ment(s) coup\u{e9}(s) \u{2014} collez-les avec Ctrl+V")
            } else {
                format!("{n} \u{e9}l\u{e9}ment(s) copi\u{e9}(s)")
            },
            cx,
        );
    }

    pub fn paste(&mut self, cx: &mut Context<Self>) {
        if self.read_only_place(cx) {
            return;
        }
        // Le presse-papiers Windows fait foi (fichiers copies depuis l'Explorateur compris).
        let system = crate::syscb::get_files();
        let internal = self.clipboard.take();
        let clip = match (system, internal) {
            (Some((paths, _)), Some(c)) if c.paths == paths => c,
            (Some((paths, cut)), _) => Clip { paths, cut },
            (None, Some(c)) => c,
            (None, None) => return,
        };
        let items = clip.paths.clone();
        let mode = if clip.cut { crate::transfer::Mode::Move } else { crate::transfer::Mode::Copy };
        // Apres un "couper", la source disparait : le presse-papiers est vide. Apres une copie, on le garde.
        if clip.cut {
            crate::syscb::clear();
        } else {
            self.clipboard = Some(clip);
        }
        let dest = self.cwd.clone();
        self.start_transfer(items, dest, mode, cx);
    }

    pub fn duplicate(&mut self, cx: &mut Context<Self>) {
        if self.read_only_place(cx) {
            return;
        }
        let mut created = Vec::new();
        let mut errors = Vec::new();
        for src in self.selected_paths() {
            let Some(name) = src.file_name().map(|n| n.to_string_lossy().into_owned()) else { continue };
            let dir = src.parent().map(Path::to_path_buf).unwrap_or_else(|| self.cwd.clone());
            let target = ops::unique_path(&dir, &ops::copy_name(&name, src.is_dir()));
            match ops::copy_recursive(&src, &target) {
                Ok(()) => created.push(target),
                Err(e) => errors.push(format!("{name} : {e}")),
            }
        }
        if created.is_empty() && errors.is_empty() {
            return;
        }
        self.reload_select(&created);
        if !created.is_empty() {
            self.record(crate::undo::Op::Created(created.clone()));
        }
        if let Some(e) = errors.first() {
            self.show_notice(format!("Duplication impossible \u{2014} {e}"), cx);
        }
        cx.notify();
    }

    pub fn trash_selection(&mut self, cx: &mut Context<Self>) {
        if self.read_only_place(cx) {
            return;
        }
        let paths = self.selected_paths();
        if paths.is_empty() {
            return;
        }
        // Position a reselectionner apres la suppression : le premier element supprime.
        let first = self.marked.iter().next().copied().unwrap_or(0);
        match ops::trash(&paths) {
            Ok(()) => {
                self.record(crate::undo::Op::Trashed(paths.clone()));
                self.cancel_rename_silent();
                self.refresh_after_delete(first, cx);
                self.show_notice(format!("{} \u{e9}l\u{e9}ment(s) d\u{e9}plac\u{e9}(s) dans la corbeille", paths.len()), cx);
            }
            Err(e) => self.show_notice(e, cx),
        }
    }

    fn refresh_after_delete(&mut self, near: usize, cx: &mut Context<Self>) {
        if self.search_query().is_some() {
            self.run_search(cx);
            return;
        }
        self.entries = self.load_entries();
        self.clear_selection();
        if !self.entries.is_empty() {
            self.set_single(near.min(self.entries.len() - 1));
        }
        self.rebuild_columns();
        cx.notify();
    }

    pub fn new_folder(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.read_only_place(cx) {
            return;
        }
        match ops::create_folder(&self.cwd) {
            Ok(path) => {
                self.record(crate::undo::Op::Created(vec![path.clone()]));
                self.close_search_silent();
                self.reload_select(&[path]);
                self.begin_rename(window, cx);
            }
            Err(e) => self.show_notice(format!("Cr\u{e9}ation impossible : {e}"), cx),
        }
    }

    /// Renommage en ligne de l'element selectionne (nom sans extension pre-selectionne).
    pub fn begin_rename(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.read_only_place(cx) {
            return;
        }
        if self.rename.is_some() {
            return;
        }
        let paths = self.selected_paths();
        if paths.len() > 1 {
            self.open_bulk_rename(window, cx);
            return;
        }
        let [path] = paths.as_slice() else { return };
        let Some(name) = path.file_name().map(|n| n.to_string_lossy().into_owned()) else { return };
        let (stem, _) = ops::split_name(&name, path.is_dir());
        let stem_len = stem.chars().count();
        let editor = cx.new(|cx| {
            let mut e = Editor::new(&name, true, cx);
            e.select_range(0, stem_len);
            e
        });
        let sub = cx.subscribe_in(&editor, window, |this, _, ev: &EditorEvent, _, cx| match ev {
            EditorEvent::Commit => this.commit_rename(cx),
            EditorEvent::Escape => this.cancel_rename_silent_notify(cx),
            _ => {}
        });
        window.focus(&editor.focus_handle(cx));
        self.rename = Some(Rename { path: path.clone(), editor, _sub: sub });
        cx.notify();
    }

    /// Valide le renommage en cours (Entree, ou clic ailleurs).
    pub fn commit_rename(&mut self, cx: &mut Context<Self>) {
        let Some(r) = self.rename.take() else { return };
        self.needs_focus = true;
        let new_name = r.editor.read(cx).text();
        match ops::rename(&r.path, &new_name) {
            Ok(new_path) => {
                if new_path != r.path {
                    self.record(crate::undo::Op::Rename { from: r.path.clone(), to: new_path.clone() });
                }
                self.reload_select(&[new_path]);
                if self.search_query().is_some() {
                    self.run_search(cx);
                }
            }
            Err(e) => {
                self.reload_select(&[r.path.clone()]);
                self.show_notice(e, cx);
            }
        }
        cx.notify();
    }

    pub fn cancel_rename_silent(&mut self) {
        if self.rename.take().is_some() {
            self.needs_focus = true;
        }
    }

    fn cancel_rename_silent_notify(&mut self, cx: &mut Context<Self>) {
        self.cancel_rename_silent();
        cx.notify();
    }
}
