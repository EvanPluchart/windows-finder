//! Copie / deplacement avec resolution des conflits (Arreter, Garder les deux, Remplacer, Ignorer,
//! "Appliquer a tous") et progression, executes hors du thread d'interface.

use std::path::{Path, PathBuf};

use gpui::{
    AnyElement, Context, InteractiveElement, IntoElement, MouseButton, ParentElement, StatefulInteractiveElement, Styled,
    div, prelude::*, px,
};

use crate::theme::{rgb, rgba};
use crate::{ops, theme, Finder};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Copy,
    Move,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Resolution {
    KeepBoth,
    Replace,
    Skip,
    Stop,
}

pub struct Job {
    items: Vec<PathBuf>,
    dest: PathBuf,
    mode: Mode,
    idx: usize,
    created: Vec<PathBuf>,
    moved: Vec<(PathBuf, PathBuf)>,
    errors: Vec<String>,
    /// Choix "appliquer a tous" memorise pour la suite du lot.
    apply_all: Option<Resolution>,
    /// Choix de l'utilisateur pour l'element courant (apres une boite de conflit).
    current: Option<Resolution>,
    cancelled: bool,
}

pub struct Conflict {
    pub name: String,
    pub apply_all: bool,
}

impl Finder {
    /// Lance une copie ou un deplacement de `items` vers le dossier `dest`.
    pub fn start_transfer(&mut self, items: Vec<PathBuf>, dest: PathBuf, mode: Mode, cx: &mut Context<Self>) {
        if self.job.is_some() {
            self.show_notice(crate::i18n::t("Un transfert est d\u{e9}j\u{e0} en cours."), cx);
            return;
        }
        let items: Vec<PathBuf> = items
            .into_iter()
            // Pas dans lui-meme ni dans un de ses sous-dossiers ; un deplacement vers son dossier actuel est sans effet.
            .filter(|s| *s != dest && !dest.starts_with(s) && !(mode == Mode::Move && s.parent() == Some(dest.as_path())))
            .collect();
        if items.is_empty() {
            return;
        }
        self.job = Some(Job {
            items,
            dest,
            mode,
            idx: 0,
            created: Vec::new(),
            moved: Vec::new(),
            errors: Vec::new(),
            apply_all: None,
            current: None,
            cancelled: false,
        });
        self.step_job(cx);
    }

    /// Traite l'element suivant ; s'arrete sur une boite de conflit ou quand tout est fait.
    fn step_job(&mut self, cx: &mut Context<Self>) {
        loop {
            let Some(job) = self.job.as_mut() else { return };
            if job.cancelled || job.idx >= job.items.len() {
                self.finish_job(cx);
                return;
            }
            let src = job.items[job.idx].clone();
            let name = src.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            let is_dir = src.is_dir();
            let same_dir = src.parent() == Some(job.dest.as_path());
            let mut target = job.dest.join(&name);
            let mut replace = false;

            if same_dir && job.mode == Mode::Copy {
                // Copie dans le meme dossier : "x copie".
                target = ops::unique_path(&job.dest, &ops::copy_name(&name, is_dir));
            } else if target.exists() {
                let res = job.current.take().or(job.apply_all);
                match res {
                    None => {
                        let multiple = job.items.len() - job.idx > 1;
                        self.conflict = Some(Conflict { name, apply_all: false });
                        let _ = multiple;
                        cx.notify();
                        return;
                    }
                    Some(Resolution::Stop) => {
                        job.cancelled = true;
                        continue;
                    }
                    Some(Resolution::Skip) => {
                        job.idx += 1;
                        continue;
                    }
                    Some(Resolution::KeepBoth) => target = ops::unique_path(&job.dest, &name),
                    Some(Resolution::Replace) => replace = true,
                }
            }

            let mode = job.mode;
            let src_for_undo = src.clone();
            cx.notify();
            cx.spawn(async move |this, cx| {
                let t = target.clone();
                let result = cx
                    .background_executor()
                    .spawn(async move { run_one(&src, &t, mode, replace) })
                    .await;
                this.update(cx, |f, cx| {
                    if let Some(job) = f.job.as_mut() {
                        match result {
                            Ok(()) => {
                                if mode == Mode::Move {
                                    job.moved.push((src_for_undo.clone(), target.clone()));
                                }
                                job.created.push(target)
                            }
                            Err(e) => job.errors.push(format!("{name} : {e}")),
                        }
                        job.idx += 1;
                    }
                    f.step_job(cx);
                })
                .ok();
            })
            .detach();
            return;
        }
    }

    fn finish_job(&mut self, cx: &mut Context<Self>) {
        let Some(job) = self.job.take() else { return };
        self.conflict = None;
        if !job.moved.is_empty() {
            self.record(crate::undo::Op::Moves(job.moved.clone()));
        } else if job.mode == Mode::Copy && !job.created.is_empty() {
            self.record(crate::undo::Op::Created(job.created.clone()));
        }
        self.reload_select(&job.created);
        let verb = if job.mode == Mode::Copy { crate::i18n::t("copi\u{e9}") } else { crate::i18n::t("d\u{e9}plac\u{e9}") };
        let dir = crate::fs::display_name(&job.dest);
        if let Some(e) = job.errors.first() {
            self.show_notice(crate::i18n::tf(crate::i18n::t("Transfert incomplet \u{2014} {e}"), &[("e", (e).to_string())]), cx);
        } else if !job.created.is_empty() {
            let s = if job.created.len() > 1 { "s" } else { "" };
            self.show_notice(crate::i18n::tf(crate::i18n::t("{0} \u{e9}l\u{e9}ment{s} {verb}{s} vers \u{ab} {dir} \u{bb}"), &[("0", (job.created.len()).to_string()), ("s", (s).to_string()), ("verb", (verb).to_string()), ("dir", (dir).to_string())]), cx);
        }
        cx.notify();
    }

    pub fn resolve_conflict(&mut self, res: Resolution, cx: &mut Context<Self>) {
        let apply_all = self.conflict.take().map(|c| c.apply_all).unwrap_or(false);
        if let Some(job) = self.job.as_mut() {
            job.current = Some(res);
            if apply_all {
                job.apply_all = Some(res);
            }
        }
        self.step_job(cx);
    }

    pub fn cancel_job(&mut self, cx: &mut Context<Self>) {
        let waiting = self.conflict.take().is_some();
        if let Some(job) = self.job.as_mut() {
            job.cancelled = true;
        }
        // En attente d'une reponse : on termine tout de suite ; sinon l'element en cours finit d'abord.
        if waiting {
            self.step_job(cx);
        }
    }

    /// Pastille de progression en bas de la fenetre.
    pub fn render_progress(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let job = self.job.as_ref()?;
        if self.conflict.is_some() {
            return None;
        }
        let total = job.items.len();
        let current = job.items.get(job.idx).and_then(|p| p.file_name()).map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let verb = if job.mode == Mode::Copy { crate::i18n::t("Copie") } else { crate::i18n::t("D\u{e9}placement") };
        let frac = job.idx as f32 / total as f32;
        Some(
            div()
                .absolute()
                .bottom(px(70.))
                .left_0()
                .right_0()
                .flex()
                .justify_center()
                .child(
                    div()
                        .w(px(340.))
                        .p(px(12.))
                        .flex()
                        .flex_col()
                        .gap(px(8.))
                        .rounded(px(12.))
                        .bg(rgba(0xf6f6f6fc))
                        .border_1()
                        .border_color(rgb(0xd9d9de))
                        .shadow_lg()
                        .occlude()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .gap(px(8.))
                                .text_size(px(13.))
                                .text_color(rgb(theme::TEXT))
                                .child(crate::views::label(crate::i18n::tf(crate::i18n::t("{verb} de \u{ab} {current} \u{bb} ({0} sur {total})"), &[("verb", (verb).to_string()), ("current", (current).to_string()), ("0", (job.idx + 1).to_string()), ("total", (total).to_string())])).flex_1().min_w_0())
                                .child(
                                    div()
                                        .id("job-cancel")
                                        .size(px(18.))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .rounded_full()
                                        .text_size(px(10.))
                                        .text_color(rgb(theme::TEXT_SECONDARY))
                                        .hover(|s| s.bg(rgb(0xe3e3e6)))
                                        .child("\u{2715}")
                                        .cursor_pointer().on_click(cx.listener(|this, _, _, cx| this.cancel_job(cx))),
                                ),
                        )
                        .child(
                            div()
                                .h(px(5.))
                                .w_full()
                                .rounded_full()
                                .bg(rgb(0xe3e3e6))
                                .child(div().h_full().w(gpui::relative(frac.max(0.04))).rounded_full().bg(rgb(theme::SELECTION))),
                        ),
                )
                .into_any_element(),
        )
    }

    /// Boite "Un element de ce nom existe deja".
    pub fn render_conflict(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let c = self.conflict.as_ref()?;
        let job = self.job.as_ref()?;
        let batch = job.items.len() - job.idx > 1;
        let verb = if job.mode == Mode::Copy { crate::i18n::t("copiez") } else { crate::i18n::t("d\u{e9}placez") };
        let button = |id: &'static str, text: &'static str, primary: bool| {
            div()
                .id(id)
                .px(px(14.))
                .h(px(28.))
                .flex()
                .items_center()
                .rounded(px(7.))
                .text_size(px(13.))
                .when(primary, |d| d.bg(rgb(theme::SELECTION)).text_color(rgb(theme::SELECTION_TEXT)))
                .when(!primary, |d| d.bg(rgb(0xe9e9ec)).text_color(rgb(theme::TEXT)))
                .child(text)
        };
        let checked = c.apply_all;
        Some(
            div()
                .id("conflict-backdrop")
                .absolute()
                .inset_0()
                .flex()
                .justify_center()
                .items_start()
                .pt(px(110.))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(
                    div()
                        .w(px(400.))
                        .p(px(20.))
                        .flex()
                        .flex_col()
                        .gap(px(12.))
                        .rounded(px(12.))
                        .bg(rgba(0xf6f6f6fc))
                        .border_1()
                        .border_color(rgb(0xd9d9de))
                        .shadow_lg()
                        .occlude()
                        .child(
                            div()
                                .text_size(px(15.))
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .text_color(rgb(theme::TEXT))
                                .child(crate::i18n::tf(crate::i18n::t("Un \u{e9}l\u{e9}ment nomm\u{e9} \u{ab} {0} \u{bb} existe d\u{e9}j\u{e0} \u{e0} cet emplacement."), &[("0", (c.name).to_string())])),
                        )
                        .child(
                            div().text_size(px(13.)).text_color(rgb(theme::TEXT_SECONDARY)).child(crate::i18n::tf(crate::i18n::t("Voulez-vous le remplacer par celui que vous {verb} ? Les deux peuvent aussi \u{ea}tre conserv\u{e9}s."), &[("verb", (verb).to_string())])),
                        )
                        .when(batch, |d| {
                            d.child(
                                div()
                                    .id("conflict-all")
                                    .flex()
                                    .items_center()
                                    .gap(px(8.))
                                    .text_size(px(13.))
                                    .text_color(rgb(theme::TEXT))
                                    .child(
                                        div()
                                            .size(px(16.))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .rounded(px(4.))
                                            .border_1()
                                            .border_color(rgb(0xb0b0b6))
                                            .when(checked, |d| d.bg(rgb(theme::SELECTION)).text_color(rgb(theme::SELECTION_TEXT)).child("\u{2713}")),
                                    )
                                    .child(crate::i18n::t("Appliquer \u{e0} tous"))
                                    .cursor_pointer().on_click(cx.listener(|this, _, _, cx| {
                                        if let Some(c) = this.conflict.as_mut() {
                                            c.apply_all = !c.apply_all;
                                        }
                                        cx.notify();
                                    })),
                            )
                        })
                        .child(
                            div()
                                .flex()
                                .justify_end()
                                .gap(px(8.))
                                .child(button("cf-stop", crate::i18n::t("Arr\u{ea}ter"), false).cursor_pointer().on_click(cx.listener(|this, _, _, cx| this.resolve_conflict(Resolution::Stop, cx))))
                                .when(batch, |d| {
                                    d.child(button("cf-skip", crate::i18n::t("Ignorer"), false).cursor_pointer().on_click(cx.listener(|this, _, _, cx| this.resolve_conflict(Resolution::Skip, cx))))
                                })
                                .child(button("cf-both", crate::i18n::t("Garder les deux"), false).cursor_pointer().on_click(cx.listener(|this, _, _, cx| this.resolve_conflict(Resolution::KeepBoth, cx))))
                                .child(button("cf-replace", crate::i18n::t("Remplacer"), true).cursor_pointer().on_click(cx.listener(|this, _, _, cx| this.resolve_conflict(Resolution::Replace, cx)))),
                        ),
                )
                .into_any_element(),
        )
    }
}

/// Une operation sur un element, dans un thread de fond. `replace` : l'ancien element va d'abord a la corbeille.
fn run_one(src: &Path, target: &Path, mode: Mode, replace: bool) -> Result<(), String> {
    if replace && target.exists() {
        ops::trash(&[target.to_path_buf()])?;
    }
    match mode {
        Mode::Copy => ops::copy_recursive(src, target).map_err(|e| e.to_string()),
        Mode::Move => {
            let r = ops::move_path(src, target).map_err(|e| e.to_string());
            if r.is_ok() {
                crate::tags::rename_path(src, target);
            }
            r
        }
    }
}
