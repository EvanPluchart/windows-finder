//! Palette de commandes (Ctrl+Maj+P) : recherche floue dans toutes les commandes de l'application,
//! avec leur raccourci. Equivalent de la recherche du menu Aide (Cmd+Maj+/) du Finder.

use gpui::{
    AnyElement, Context, Entity, Focusable, InteractiveElement, IntoElement, MouseButton, ParentElement, StatefulInteractiveElement,
    Styled, Subscription, Window, div, prelude::*, px,
};

use crate::editor::{Editor, EditorEvent};
use crate::i18n::t;
use crate::theme::{rgb, rgba};
use crate::{layout, theme, Finder, ViewMode};

type Run = fn(&mut Finder, &mut Window, &mut Context<Finder>);

pub struct Cmd {
    pub label: &'static str,
    pub keys: &'static str,
    pub run: Run,
}

pub struct Palette {
    pub editor: Entity<Editor>,
    _sub: Subscription,
    pub results: Vec<usize>,
    pub sel: usize,
}

fn c(label: &'static str, keys: &'static str, run: Run) -> Cmd {
    Cmd { label, keys, run }
}

fn set_flag(key: &str, on: bool) {
    layout::set_setting(key, if on { "1" } else { "0" });
}

/// Toutes les commandes (libelles francais = cles du dictionnaire de traduction).
pub fn commands() -> Vec<Cmd> {
    vec![
        c(crate::i18n::t("Nouveau dossier"), "Ctrl+Maj+N", |f, w, cx| f.new_folder(w, cx)),
        c(crate::i18n::t("Nouveau fichier texte"), "", |f, _, cx| f.new_text_file(cx)),
        c(crate::i18n::t("Nouvel onglet"), "Ctrl+T", |f, _, cx| f.new_tab(None, cx)),
        c(crate::i18n::t("Fermer l'onglet"), "Ctrl+W", |f, w, cx| {
            let i = f.active_tab;
            f.close_tab(i, w, cx)
        }),
        c(crate::i18n::t("Rouvrir l'onglet ferm\u{e9}"), "Ctrl+Maj+T", |f, _, cx| f.reopen_closed_tab(cx)),
        c(crate::i18n::t("Vue d'ensemble des onglets"), "Ctrl+Maj+A", |f, _, cx| f.toggle_all_tabs(cx)),
        c(crate::i18n::t("Aller au dossier\u{2026}"), "Ctrl+Maj+G", |f, w, cx| f.open_goto(w, cx)),
        c(crate::i18n::t("Se connecter au serveur\u{2026}"), "Ctrl+K", |f, w, cx| f.open_goto_with(w, cx, Some("\\\\"))),
        c(crate::i18n::t("Actualiser"), "F5", |f, _, cx| f.refresh(cx)),
        c(crate::i18n::t("Copier"), "Ctrl+C", |f, _, cx| f.copy_selection(false, cx)),
        c(crate::i18n::t("Couper"), "Ctrl+X", |f, _, cx| f.copy_selection(true, cx)),
        c(crate::i18n::t("Coller"), "Ctrl+V", |f, _, cx| f.paste(cx)),
        c(crate::i18n::t("Copier le chemin"), "Ctrl+Maj+C", |f, _, cx| f.copy_path(cx)),
        c(crate::i18n::t("Dupliquer"), "Ctrl+D", |f, _, cx| f.duplicate(cx)),
        c(crate::i18n::t("Renommer"), "F2", |f, w, cx| f.begin_rename(w, cx)),
        c(crate::i18n::t("Mettre \u{e0} la corbeille"), "Suppr", |f, _, cx| f.trash_selection(cx)),
        c(crate::i18n::t("Annuler la dernière action"), "Ctrl+Z", |f, _, cx| f.undo(cx)),
        c(crate::i18n::t("R\u{e9}tablir"), "Ctrl+Maj+Z", |f, _, cx| f.redo(cx)),
        c(crate::i18n::t("Lire les informations"), "Ctrl+I", |f, w, cx| f.show_info(w, cx)),
        c(crate::i18n::t("Aper\u{e7}u rapide"), "Espace", |f, _, cx| f.toggle_quick_look(cx)),
        c(crate::i18n::t("Compresser"), "", |f, _, cx| f.compress_selection(cx)),
        c(crate::i18n::t("Extraire ici"), "", |f, _, cx| f.extract_selection(cx)),
        c(crate::i18n::t("Cr\u{e9}er un raccourci"), "", |f, _, cx| f.make_alias(cx)),
        c(crate::i18n::t("Afficher dans l'Explorateur"), "", |f, _, _| f.reveal_in_explorer()),
        c(crate::i18n::t("Ouvrir dans le Terminal"), "", |f, _, _| f.open_terminal()),
        c(crate::i18n::t("Ajouter \u{e0} la barre lat\u{e9}rale"), "", |f, _, cx| f.pin_selection(cx)),
        c(crate::i18n::t("D\u{e9}finir comme fond d'\u{e9}cran"), "", |f, _, cx| f.set_wallpaper(cx)),
        c(crate::i18n::t("Pivoter \u{e0} gauche"), "", |f, _, cx| f.rotate_images(false, cx)),
        c(crate::i18n::t("Pivoter \u{e0} droite"), "", |f, _, cx| f.rotate_images(true, cx)),
        c(crate::i18n::t("Ex\u{e9}cuter en tant qu'administrateur"), "", |f, _, _| f.run_as_admin()),
        c(crate::i18n::t("Plus d'options\u{2026}"), "Maj+F10", |f, w, cx| f.native_menu(w, cx)),
        c(crate::i18n::t("Ic\u{f4}nes"), "Ctrl+1", |f, _, cx| f.set_view(ViewMode::Icons, cx)),
        c(crate::i18n::t("Liste"), "Ctrl+2", |f, _, cx| f.set_view(ViewMode::List, cx)),
        c(crate::i18n::t("Colonnes"), "Ctrl+3", |f, _, cx| f.set_view(ViewMode::Columns, cx)),
        c(crate::i18n::t("Galerie"), "Ctrl+4", |f, _, cx| f.set_view(ViewMode::Gallery, cx)),
        c(crate::i18n::t("Afficher les options d'affichage"), "Ctrl+J", |f, _, cx| f.toggle_view_options(cx)),
        c(crate::i18n::t("Afficher les fichiers cach\u{e9}s et syst\u{e8}me"), "Ctrl+Maj+.", |f, _, cx| {
            f.show_hidden = !f.show_hidden;
            f.refresh(cx)
        }),
        c(crate::i18n::t("Barre lat\u{e9}rale"), "Ctrl+Alt+S", |f, _, cx| {
            f.show_sidebar = !f.show_sidebar;
            set_flag("sidebar", f.show_sidebar);
            cx.notify()
        }),
        c(crate::i18n::t("Afficher la barre de chemin"), "Ctrl+Alt+P", |f, _, cx| {
            f.show_path_bar = !f.show_path_bar;
            set_flag("pathbar", f.show_path_bar);
            cx.notify()
        }),
        c(crate::i18n::t("Afficher la barre d'\u{e9}tat"), "Ctrl+Alt+B", |f, _, cx| {
            f.show_status_bar = !f.show_status_bar;
            set_flag("statusbar", f.show_status_bar);
            cx.notify()
        }),
        c(crate::i18n::t("Tout s\u{e9}lectionner"), "Ctrl+A", |f, _, cx| f.select_all(cx)),
        c(crate::i18n::t("Vider la corbeille\u{2026}"), "", |f, _, cx| f.ask_empty_trash(cx)),
        c(crate::i18n::t("Th\u{e8}me clair"), "", |f, _, cx| f.set_theme("light", cx)),
        c(crate::i18n::t("Th\u{e8}me sombre"), "", |f, _, cx| f.set_theme("dark", cx)),
        c(crate::i18n::t("Th\u{e8}me du syst\u{e8}me"), "", |f, _, cx| f.set_theme("system", cx)),
        c(crate::i18n::t("R\u{e9}glages\u{2026}"), "Ctrl+,", |f, _, cx| f.toggle_settings(cx)),
    ]
}

/// Score de correspondance floue (None = pas de correspondance) : plus petit = meilleur.
fn fuzzy(hay: &str, needle: &str) -> Option<usize> {
    let lower = hay.to_lowercase();
    let q = needle.trim().to_lowercase();
    if q.is_empty() {
        return Some(0);
    }
    // Sous-chaine : au debut d'un mot d'abord, puis ailleurs ; la correspondance floue vient en dernier.
    if let Some(p) = lower.find(&q) {
        let word_start = p == 0 || lower[..p].ends_with(' ');
        return Some(if word_start { p } else { 100 + p });
    }
    let hay: Vec<char> = lower.chars().collect();
    let mut pos = 0usize;
    let mut score = 0usize;
    let mut last: Option<usize> = None;
    for n in needle.to_lowercase().chars().filter(|c| !c.is_whitespace()) {
        let found = hay[pos..].iter().position(|&h| h == n)? + pos;
        score += 1000 + match last {
            Some(l) => found - l - 1,
            None => found,
        };
        last = Some(found);
        pos = found + 1;
    }
    Some(score)
}

impl Finder {
    pub fn open_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.commit_rename(cx);
        self.menu = None;
        let editor = cx.new(|cx| Editor::new("", true, cx));
        let sub = cx.subscribe_in(&editor, window, |this, _, ev: &EditorEvent, window, cx| match ev {
            EditorEvent::Commit => this.palette_run(window, cx),
            EditorEvent::Escape => {
                this.palette = None;
                this.needs_focus = true;
                cx.notify();
            }
            EditorEvent::Changed => this.palette_refresh(cx),
            EditorEvent::Arrow(d) => {
                if let Some(p) = &mut this.palette {
                    let n = p.results.len().max(1) as i32;
                    p.sel = ((p.sel as i32 + d).rem_euclid(n)) as usize;
                }
                cx.notify();
            }
            _ => {}
        });
        window.focus(&editor.focus_handle(cx));
        let results = (0..commands().len()).collect();
        self.palette = Some(Palette { editor, _sub: sub, results, sel: 0 });
        cx.notify();
    }

    fn palette_refresh(&mut self, cx: &mut Context<Self>) {
        if let Some(p) = &mut self.palette {
            let query = p.editor.read(cx).text();
            let all = commands();
            let mut scored: Vec<(usize, usize)> = all
                .iter()
                .enumerate()
                .filter_map(|(i, cmd)| fuzzy(&crate::i18n::td(cmd.label), &query).map(|s| (s, i)))
                .collect();
            scored.sort();
            p.results = scored.into_iter().map(|(_, i)| i).collect();
            p.sel = 0;
        }
        cx.notify();
    }

    fn palette_run(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(p) = self.palette.take() else { return };
        self.needs_focus = true;
        if let Some(&i) = p.results.get(p.sel) {
            let run = commands()[i].run;
            run(self, window, cx);
        }
        cx.notify();
    }

    pub fn render_palette(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let p = self.palette.as_ref()?;
        let all = commands();
        let rows = p.results.iter().take(9).enumerate().map(|(row, &i)| {
            let cmd = &all[i];
            let selected = row == p.sel;
            div()
                .id(("palette-row", row))
                .h(px(28.))
                .px(px(10.))
                .flex()
                .items_center()
                .justify_between()
                .rounded(px(6.))
                .text_size(px(13.))
                .when(selected, |d| d.bg(rgb(theme::SELECTION)).text_color(rgb(0xfffffe)))
                .child(t(cmd.label))
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(if selected { rgba(0xfffffecc) } else { rgb(theme::TEXT_SECONDARY) })
                        .child(cmd.keys),
                )
                .cursor_pointer()
                .on_click(cx.listener(move |this, _, window, cx| {
                    if let Some(p) = &mut this.palette {
                        p.sel = row;
                    }
                    this.palette_run(window, cx);
                }))
        });
        Some(
            div()
                .id("palette-backdrop")
                .absolute()
                .inset_0()
                .flex()
                .justify_center()
                .items_start()
                .pt(px(80.))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _, _, cx| {
                        this.palette = None;
                        this.needs_focus = true;
                        cx.notify();
                    }),
                )
                .child(
                    div()
                        .w(px(520.))
                        .p(px(14.))
                        .flex()
                        .flex_col()
                        .gap(px(8.))
                        .rounded(px(14.))
                        .bg(rgba(0xf8f8faf6))
                        .border_1()
                        .border_color(rgb(0xd9d9de))
                        .shadow_lg()
                        .occlude()
                        .child(
                            div()
                                .h(px(32.))
                                .rounded(px(7.))
                                .bg(rgb(0xffffff))
                                .border_2()
                                .border_color(rgb(0x6aa6f8))
                                .child(p.editor.clone()),
                        )
                        .child(div().flex().flex_col().children(rows))
                        .children(p.results.is_empty().then(|| {
                            div().px(px(10.)).py(px(6.)).text_size(px(13.)).text_color(rgb(theme::TEXT_SECONDARY)).child(t("Aucune commande"))
                        })),
                )
                .into_any_element(),
        )
    }
}
