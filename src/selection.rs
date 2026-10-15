//! Selection avancee : saisie pour atteindre un element (type-to-select, delai 1 s comme le Finder)
//! et rectangle de selection a la souris (vue Icones).

use std::time::{Duration, Instant};

use gpui::{Context, Pixels, Point};

use crate::layout::PAD;
use crate::{Finder, ViewMode};

/// Rectangle de selection en coordonnees du contenu de la vue Icones (defilement compris).
#[derive(Clone, Copy)]
pub struct Band {
    pub start: (f32, f32),
    pub cur: (f32, f32),
}

impl Band {
    pub fn rect(&self) -> (f32, f32, f32, f32) {
        let (x0, x1) = (self.start.0.min(self.cur.0), self.start.0.max(self.cur.0));
        let (y0, y1) = (self.start.1.min(self.cur.1), self.start.1.max(self.cur.1));
        (x0, y0, x1 - x0, y1 - y0)
    }
}

impl Finder {
    /// Saisie au clavier : selectionne le premier element dont le nom commence par les lettres tapees.
    pub fn type_select(&mut self, text: &str, cx: &mut Context<Self>) {
        let now = Instant::now();
        if self.type_at.map(|t| now.duration_since(t) > Duration::from_secs(1)).unwrap_or(true) {
            self.type_buf.clear();
        }
        self.type_at = Some(now);
        self.type_buf.push_str(&text.to_lowercase());
        let prefix = self.type_buf.clone();
        if self.view == ViewMode::Columns {
            let c = self.columns.iter().rposition(|c| c.selected.is_some()).unwrap_or(0);
            let hit = self.columns.get(c).and_then(|col| col.entries.iter().position(|e| e.name.to_lowercase().starts_with(&prefix)));
            if let Some(i) = hit {
                self.select_in_column(c, i, cx);
            }
            return;
        }
        if let Some(i) = self.entries.iter().position(|e| e.name.to_lowercase().starts_with(&prefix)) {
            self.select(i, cx);
            self.reveal(i);
        }
    }

    /// Position d'un point de la fenetre dans le repere du contenu de la vue Icones.
    fn to_content(&self, p: Point<Pixels>) -> (f32, f32) {
        let origin = self.icon_scroll.bounds().origin;
        let off = self.icon_scroll.offset();
        (f32::from(p.x - origin.x - off.x), f32::from(p.y - origin.y - off.y))
    }

    pub fn band_start(&mut self, p: Point<Pixels>) {
        let c = self.to_content(p);
        self.band = Some(Band { start: c, cur: c });
    }

    /// Met a jour le rectangle et selectionne les icones qu'il touche.
    pub fn band_move(&mut self, p: Point<Pixels>, width: f32, cx: &mut Context<Self>) {
        let Some(mut b) = self.band else { return };
        b.cur = self.to_content(p);
        self.band = Some(b);
        let (bx, by, bw, bh) = b.rect();
        // Un simple clic (rectangle minuscule) ne selectionne rien : on ne bouge pas la selection.
        if bw < 3.0 && bh < 3.0 {
            return;
        }
        let (cell_w, cell_h) = self.cell();
        let positions = self.icon_positions(self.icon_cols(width));
        self.marked = positions
            .iter()
            .enumerate()
            .filter(|(_, (x, y))| {
                let (cx0, cy0) = (x + PAD, y + PAD);
                cx0 < bx + bw && cx0 + cell_w > bx && cy0 < by + bh && cy0 + cell_h > by
            })
            .map(|(i, _)| i)
            .collect();
        self.selected = self.marked.iter().next_back().copied();
        self.anchor = self.marked.iter().next().copied();
        cx.notify();
    }

    pub fn band_end(&mut self, cx: &mut Context<Self>) {
        if self.band.take().is_some() {
            cx.notify();
        }
    }
}
