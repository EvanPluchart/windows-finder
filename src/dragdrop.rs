//! Glisser-deposer interne : deplacer des elements vers un dossier (liste, icones, colonnes, barre laterale,
//! chemin) et placer librement les icones dans la vue Icones.

use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use gpui::{
    Context, IntoElement, ParentElement, Pixels, Point, Render, RenderImage, SharedString, Styled, Window, div, img,
    prelude::*, px, rgb,
};

use crate::fs::Entry;
use crate::layout::{self, CELL_H, CELL_W, PAD};
use crate::{ops, theme, Finder};

/// Valeur portee par le glisser : les elements deplaces et celui saisi.
pub struct DragPaths {
    pub paths: Vec<PathBuf>,
    pub primary: PathBuf,
    label: SharedString,
    icon: Option<Arc<RenderImage>>,
    /// Position du curseur dans l'element saisi (remplie au debut du glisser).
    pub grab: Cell<(f32, f32)>,
}

/// Apercu qui suit le curseur pendant le glisser.
pub struct DragView {
    label: SharedString,
    count: usize,
    icon: Option<Arc<RenderImage>>,
}

impl Render for DragView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(CELL_W))
            .flex()
            .flex_col()
            .items_center()
            .gap(px(4.))
            .opacity(0.85)
            .child(match &self.icon {
                Some(i) => img(i.clone()).size(px(64.)).into_any_element(),
                None => div().size(px(64.)).into_any_element(),
            })
            .child(
                div()
                    .px(px(8.))
                    .py(px(1.))
                    .rounded(px(5.))
                    .bg(rgb(theme::SELECTION))
                    .text_color(rgb(0xffffff))
                    .text_size(px(13.))
                    .child(if self.count > 1 {
                        format!("{} \u{e9}l\u{e9}ments", self.count)
                    } else {
                        self.label.to_string()
                    }),
            )
    }
}

impl DragPaths {
    /// Ce que l'on emporte en saisissant `e` : toute la selection si `e` en fait partie, sinon `e` seul.
    pub fn new(finder: &Finder, e: &Entry, in_selection: bool) -> Self {
        let paths = if in_selection && finder.marked.len() > 1 { finder.selected_paths() } else { vec![e.path.clone()] };
        Self {
            paths,
            primary: e.path.clone(),
            label: e.name.clone(),
            icon: finder.icons.get(&e.path, 128, true),
            grab: Cell::new((CELL_W / 2.0, 46.0)),
        }
    }

    pub fn view(&self, offset: Point<Pixels>) -> DragView {
        self.grab.set((f32::from(offset.x), f32::from(offset.y)));
        DragView { label: self.label.clone(), count: self.paths.len(), icon: self.icon.clone() }
    }
}

fn same_drive(a: &Path, b: &Path) -> bool {
    let root = |p: &Path| p.components().next().map(|c| c.as_os_str().to_ascii_lowercase());
    root(a) == root(b)
}

impl Finder {
    /// Deplace (ou copie, avec Ctrl ou entre deux disques) les elements dans `target`.
    pub fn drop_into(&mut self, d: &DragPaths, target: PathBuf, copy: bool, cx: &mut Context<Self>) {
        let mut done = Vec::new();
        let mut errors = Vec::new();
        for src in &d.paths {
            let Some(name) = src.file_name().map(|n| n.to_string_lossy().into_owned()) else { continue };
            // Pas dans lui-meme, ni dans un de ses sous-dossiers, ni dans son dossier actuel.
            if *src == target || target.starts_with(src) || src.parent() == Some(target.as_path()) {
                continue;
            }
            let dest = ops::unique_path(&target, &name);
            let result = if copy || !same_drive(src, &target) {
                ops::copy_recursive(src, &dest)
            } else {
                ops::move_path(src, &dest)
            };
            match result {
                Ok(()) => done.push(dest),
                Err(e) => errors.push(format!("{name} : {e}")),
            }
        }
        self.reload_select(&[]);
        let dir =
            target.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| target.display().to_string());
        if let Some(e) = errors.first() {
            self.show_notice(format!("D\u{e9}placement incomplet \u{2014} {e}"), cx);
        } else if !done.is_empty() {
            let verb = if copy { "copi\u{e9}" } else { "d\u{e9}plac\u{e9}" };
            let s = if done.len() > 1 { "s" } else { "" };
            self.show_notice(format!("{} \u{e9}l\u{e9}ment{s} {verb}{s} vers \u{ab} {dir} \u{bb}", done.len()), cx);
        }
        cx.notify();
    }

    /// Nombre de colonnes de la grille automatique de la vue Icones.
    pub fn icon_cols(&self, width: f32) -> usize {
        (((width - theme::SIDEBAR_WIDTH - 2.0 * PAD) / CELL_W).floor() as usize).max(1)
    }

    /// Position (x, y) de chaque element : celle enregistree, sinon la premiere case libre de la grille.
    pub fn icon_positions(&self, cols: usize) -> Vec<(f32, f32)> {
        let saved = self.layouts.get(&self.cwd);
        let mut out: Vec<Option<(f32, f32)>> =
            self.entries.iter().map(|e| saved.and_then(|l| l.get(e.name.as_ref()).copied())).collect();
        let mut used: Vec<(f32, f32)> = out.iter().flatten().copied().collect();
        let mut next = 0usize;
        for (i, slot) in out.iter_mut().enumerate() {
            if slot.is_some() {
                continue;
            }
            let pos = if saved.is_none() {
                ((i % cols) as f32 * CELL_W, (i / cols) as f32 * CELL_H)
            } else {
                loop {
                    let p = ((next % cols) as f32 * CELL_W, (next / cols) as f32 * CELL_H);
                    next += 1;
                    if !used.iter().any(|u| (u.0 - p.0).abs() < CELL_W / 2.0 && (u.1 - p.1).abs() < CELL_H / 2.0) {
                        break p;
                    }
                }
            };
            used.push(pos);
            *slot = Some(pos);
        }
        out.into_iter().map(|p| p.unwrap_or((0.0, 0.0))).collect()
    }

    /// Depose des icones a un endroit libre de la vue Icones : le dossier passe en placement libre.
    pub fn drop_free(&mut self, d: &DragPaths, mouse: Point<Pixels>, width: f32, cx: &mut Context<Self>) {
        if d.primary.parent() != Some(self.cwd.as_path()) {
            return;
        }
        let origin = self.icon_scroll.bounds().origin;
        let scroll = self.icon_scroll.offset();
        let (gx, gy) = d.grab.get();
        let nx = (f32::from(mouse.x - origin.x - scroll.x) - PAD - gx).max(0.0);
        let ny = (f32::from(mouse.y - origin.y - scroll.y) - PAD - gy).max(0.0);

        let positions = self.icon_positions(self.icon_cols(width));
        let mut lay: layout::Layout =
            self.entries.iter().zip(positions).map(|(e, p)| (e.name.to_string(), p)).collect();
        let name_of = |p: &Path| p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let Some(&(ox, oy)) = lay.get(&name_of(&d.primary)) else { return };
        let (dx, dy) = (nx - ox, ny - oy);
        for p in d.paths.iter().filter(|p| p.parent() == Some(self.cwd.as_path())) {
            if let Some(pos) = lay.get_mut(&name_of(p)) {
                *pos = ((pos.0 + dx).max(0.0), (pos.1 + dy).max(0.0));
            }
        }
        self.layouts.insert(self.cwd.clone(), lay);
        layout::save(&self.layouts);
        self.marked =
            self.entries.iter().enumerate().filter(|(_, e)| d.paths.contains(&e.path)).map(|(i, _)| i).collect();
        self.selected = self.marked.iter().next_back().copied();
        cx.notify();
    }

    /// Retour a la grille automatique du dossier courant ("Ranger").
    pub fn clean_up(&mut self, cx: &mut Context<Self>) {
        if self.layouts.remove(&self.cwd).is_some() {
            layout::save(&self.layouts);
        }
        cx.notify();
    }
}
