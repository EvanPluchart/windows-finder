//! Vue Liste hierarchique : les dossiers se deplient en place (triangle ou fleche droite), comme sur le Finder.

use std::path::PathBuf;

use gpui::Context;

use crate::fs::{self, Entry};
use crate::{Finder, ViewMode};

impl Finder {
    /// Elements du dossier courant ; en vue Liste, les dossiers depliés montrent leur contenu juste en dessous.
    pub fn load_entries(&self) -> Vec<Entry> {
        let mut entries = fs::read_dir(&self.cwd, self.show_hidden);
        if self.view != ViewMode::List || fs::is_virtual(&self.cwd) || self.expanded.is_empty() {
            return entries;
        }
        let mut i = 0;
        while i < entries.len() {
            if entries[i].is_dir && entries[i].depth < 8 && self.expanded.contains(&entries[i].path) {
                let depth = entries[i].depth + 1;
                let mut children = fs::read_dir(&entries[i].path, self.show_hidden);
                for c in &mut children {
                    c.depth = depth;
                }
                let n = children.len();
                entries.splice(i + 1..i + 1, children);
                let _ = n;
            }
            i += 1;
        }
        entries
    }

    /// Plie ou deplie le dossier `ix` (vue Liste).
    pub fn toggle_expand(&mut self, ix: usize, cx: &mut Context<Self>) {
        let Some(e) = self.entries.get(ix).cloned() else { return };
        if !e.is_dir || self.view != ViewMode::List || self.search_query().is_some() {
            return;
        }
        if !self.expanded.remove(&e.path) {
            self.expanded.insert(e.path.clone());
        } else {
            // Replier aussi ce qui etait deplie en dessous.
            let prefix: PathBuf = e.path.clone();
            self.expanded.retain(|p| !p.starts_with(&prefix) || *p == prefix);
        }
        self.resort_in_place();
        cx.notify();
    }

    /// Fleche droite : deplie le dossier selectionne ; fleche gauche : le replie, ou remonte au dossier parent de la ligne.
    pub fn list_horizontal(&mut self, dx: isize, cx: &mut Context<Self>) {
        let Some(ix) = self.selected else { return };
        let Some(e) = self.entries.get(ix).cloned() else { return };
        if dx > 0 {
            if e.is_dir && !self.expanded.contains(&e.path) {
                self.toggle_expand(ix, cx);
            }
        } else if e.is_dir && self.expanded.contains(&e.path) {
            self.toggle_expand(ix, cx);
        } else if e.depth > 0 {
            // Remonte a la ligne du dossier qui contient cet element.
            if let Some(parent) = e.path.parent() {
                if let Some(pi) = self.entries.iter().position(|x| x.path == parent) {
                    self.select(pi, cx);
                    self.reveal(pi);
                }
            }
        }
    }
}
