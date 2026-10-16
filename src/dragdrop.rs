//! Glisser-deposer interne : deplacer des elements vers un dossier (liste, icones, colonnes, barre laterale,
//! chemin) et placer librement les icones dans la vue Icones.

use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use gpui::{
    Context, IntoElement, ParentElement, Pixels, Point, Render, RenderImage, SharedString, Styled, Window, div, img,
    px, };
use crate::theme::{rgb};

use crate::fs::Entry;
use crate::layout::{self, PAD};
use crate::{theme, Finder};

/// Valeur portee par le glisser : les elements deplaces et celui saisi.
/// Creee pour chaque element a chaque rendu : rien de couteux ici. La selection et l'icone ne sont lues qu'au
/// debut du glisser (avant, chaque element selectionne copiait toute la selection : quadratique apres Ctrl+A).
pub struct DragPaths {
    paths: std::cell::RefCell<Arc<Vec<PathBuf>>>,
    pub primary: PathBuf,
    label: SharedString,
    in_selection: bool,
    finder: gpui::WeakEntity<Finder>,
    icons: crate::icons::IconCache,
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
            .w(px(crate::layout::CELL_W))
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
                    .text_color(rgb(0xfffffe))
                    .text_size(px(13.))
                    .child(if self.count > 1 {
                        crate::i18n::tf(crate::i18n::t("{0} \u{e9}l\u{e9}ments"), &[("0", (self.count).to_string())])
                    } else {
                        self.label.to_string()
                    }),
            )
    }
}

impl DragPaths {
    /// Ce que l'on emporte en saisissant `e` : toute la selection si `e` en fait partie, sinon `e` seul.
    pub fn new(finder: &Finder, e: &Entry, in_selection: bool) -> Self {
        Self {
            paths: std::cell::RefCell::new(Arc::new(vec![e.path.clone()])),
            primary: e.path.clone(),
            label: e.name.clone(),
            in_selection: in_selection && finder.marked.len() > 1,
            finder: finder.this.clone(),
            icons: finder.icons.clone(),
            grab: Cell::new((finder.cell().0 / 2.0, 46.0)),
        }
    }

    /// Elements emportes (fixes au debut du glisser).
    pub fn paths(&self) -> Arc<Vec<PathBuf>> {
        self.paths.borrow().clone()
    }

    /// Debut du glisser : lit la selection et l'icone, puis construit l'apercu qui suit le curseur.
    pub fn view(&self, offset: Point<Pixels>, cx: &gpui::App) -> DragView {
        self.grab.set((f32::from(offset.x), f32::from(offset.y)));
        if self.in_selection {
            if let Some(f) = self.finder.upgrade() {
                *self.paths.borrow_mut() = Arc::new(f.read(cx).selected_paths());
            }
        }
        let count = self.paths.borrow().len();
        DragView { label: self.label.clone(), count, icon: self.icons.get(&self.primary, 128, true) }
    }
}

fn same_drive(a: &Path, b: &Path) -> bool {
    let root = |p: &Path| p.components().next().map(|c| c.as_os_str().to_ascii_lowercase());
    root(a) == root(b)
}

impl Finder {
    /// Deplace (ou copie, avec Ctrl ou entre deux disques) les elements dans `target`.
    pub fn drop_into(&mut self, d: &DragPaths, target: PathBuf, copy: bool, cx: &mut Context<Self>) {
        let any_other_drive = d.paths().iter().any(|s| !same_drive(s, &target));
        let mode = if copy || any_other_drive { crate::transfer::Mode::Copy } else { crate::transfer::Mode::Move };
        // Une copie vers le dossier d'origine n'a pas de sens ici (Ctrl + glisser dans le meme dossier).
        let items: Vec<PathBuf> = d.paths().iter().filter(|s| s.parent() != Some(target.as_path())).cloned().collect();
        self.start_transfer(items, target, mode, cx);
    }

    /// Nombre de colonnes de la grille automatique de la vue Icones.
    pub fn icon_cols(&self, width: f32) -> usize {
        (((width - self.sidebar_w - 2.0 * PAD) / self.cell().0).floor() as usize).max(1)
    }

    /// Titres de groupe (y, texte) de la vue Icones groupee ; vide sans regroupement ni en placement libre.
    pub fn icon_headers(&self, cols: usize) -> Vec<(f32, String)> {
        if self.layouts.contains_key(&self.cwd) || crate::fs::group() == 0 {
            return Vec::new();
        }
        self.grouped_layout(cols).1
    }

    /// Grille par groupes : chaque groupe commence sur une nouvelle ligne, sous son titre.
    fn grouped_layout(&self, cols: usize) -> (Vec<(f32, f32)>, Vec<(f32, String)>) {
        const HEADER_H: f32 = 30.0;
        let (cell_w, cell_h) = self.cell();
        let g = crate::fs::group();
        let mut positions = Vec::with_capacity(self.entries.len());
        let mut headers = Vec::new();
        let (mut y, mut k, mut last): (f32, usize, Option<String>) = (0.0, 0, None);
        for e in &self.entries {
            let title = crate::fs::group_of(e, g).1;
            if last.as_deref() != Some(title.as_str()) {
                if last.is_some() {
                    y += (k.div_ceil(cols)) as f32 * cell_h + 8.0;
                }
                headers.push((y, title.clone()));
                y += HEADER_H;
                k = 0;
                last = Some(title);
            }
            positions.push(((k % cols) as f32 * cell_w, y + (k / cols) as f32 * cell_h));
            k += 1;
        }
        (positions, headers)
    }

    /// Position (x, y) de chaque element : celle enregistree, sinon la premiere case libre de la grille.
    pub fn icon_positions(&self, cols: usize) -> Vec<(f32, f32)> {
        if !self.layouts.contains_key(&self.cwd) && crate::fs::group() != 0 {
            return self.grouped_layout(cols).0;
        }
        let (cell_w, cell_h) = self.cell();
        let saved = self.layouts.get(&self.cwd);
        let mut out: Vec<Option<(f32, f32)>> =
            self.entries.iter().map(|e| saved.and_then(|l| l.get(e.name.as_ref()).copied())).collect();
        // Cases de la grille deja prises (ensemble : un dossier de 50 000 elements reste lineaire).
        let cell_of = |p: (f32, f32)| ((p.0 / cell_w).round() as i64, (p.1 / cell_h).round() as i64);
        let mut used: std::collections::HashSet<(i64, i64)> = out.iter().flatten().map(|&p| cell_of(p)).collect();
        let mut next = 0usize;
        for (i, slot) in out.iter_mut().enumerate() {
            if slot.is_some() {
                continue;
            }
            let pos = if saved.is_none() {
                ((i % cols) as f32 * cell_w, (i / cols) as f32 * cell_h)
            } else {
                loop {
                    let p = ((next % cols) as f32 * cell_w, (next / cols) as f32 * cell_h);
                    next += 1;
                    if !used.contains(&cell_of(p)) {
                        break p;
                    }
                }
            };
            used.insert(cell_of(pos));
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
        for p in d.paths().iter().filter(|p| p.parent() == Some(self.cwd.as_path())) {
            if let Some(pos) = lay.get_mut(&name_of(p)) {
                *pos = ((pos.0 + dx).max(0.0), (pos.1 + dy).max(0.0));
            }
        }
        layout::save_dir(&self.cwd, Some(&lay));
        self.layouts.insert(self.cwd.clone(), lay);
        self.marked =
            self.entries.iter().enumerate().filter(|(_, e)| d.paths().contains(&e.path)).map(|(i, _)| i).collect();
        self.selected = self.marked.iter().next_back().copied();
        cx.notify();
    }

    /// Retour a la grille automatique du dossier courant ("Ranger").
    pub fn clean_up(&mut self, cx: &mut Context<Self>) {
        if self.layouts.remove(&self.cwd).is_some() {
            layout::save_dir(&self.cwd, None);
        }
        cx.notify();
    }
}

impl Finder {
    /// Dossiers a ouverture automatique : un element glisse reste 1 s au-dessus d'un dossier, qui s'ouvre alors.
    pub fn spring_tick(&mut self, folder: PathBuf, inside: bool, cx: &mut Context<Self>) {
        if !inside {
            if self.spring_target.as_ref() == Some(&folder) {
                self.spring_target = None;
            }
            return;
        }
        if self.spring_target.as_ref() == Some(&folder) {
            return;
        }
        self.spring_target = Some(folder.clone());
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(std::time::Duration::from_millis(1000)).await;
            this.update(cx, |f, cx| {
                if f.spring_target.as_ref() == Some(&folder) && cx.has_active_drag() {
                    f.spring_target = None;
                    f.navigate(folder.clone(), true, cx);
                }
            })
            .ok();
        })
        .detach();
    }
}
