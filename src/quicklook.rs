//! Apercu avec Espace, comme sur macOS (docs/quicklook.md) : une fenetre separee, translucide,
//! centree sur l'ecran, dont la taille s'adapte au contenu (plafonnee a 90 % de l'ecran).

use std::path::PathBuf;

use gpui::{
    AnyElement, App, Bounds, Context, Entity, FocusHandle, Focusable, InteractiveElement, IntoElement, ObjectFit,
    ParentElement, Pixels, Render, Size, StatefulInteractiveElement, Styled, Subscription, TitlebarOptions, Window,
    WindowBackgroundAppearance, WindowBounds, WindowOptions, div, img, prelude::*, px, size,
};
use crate::theme::{rgb, rgba};

use crate::csvview::{CsvEvent, CsvTable};
use crate::editor::{Editor, EditorEvent};
use crate::fs::{self, Entry};
use crate::theme;
use crate::{CloseQuickLook, Finder, MoveDown, MoveLeft, MoveRight, MoveUp};

const TITLE_BAR: f32 = 37.0;

/// Taille de la fenetre pour une entree : image a 1:1 (+ marge 4 pt et barre de titre), sinon gabarit par type.
fn quick_look_size(e: &Entry, cx: &App) -> Size<Pixels> {
    let screen = cx.primary_display().map(|d| d.bounds().size).unwrap_or(size(px(1440.), px(900.)));
    let (max_w, max_h) = (f32::from(screen.width) * 0.9, f32::from(screen.height) * 0.9);
    let (w, h) = if e.is_image() {
        match image::image_dimensions(&e.path) {
            Ok((w, h)) => (w as f32 + 8.0, h as f32 + TITLE_BAR + 4.0),
            Err(_) => (800.0, 600.0),
        }
    } else if e.is_text() {
        (720.0, 520.0)
    } else if e.is_pdf() {
        (640.0, 800.0)
    } else if e.is_video() {
        (960.0, 600.0)
    } else {
        (480.0, 360.0)
    };
    let k = (max_w / w).min(max_h / h).min(1.0);
    size(px((w * k).max(360.0)), px((h * k).max(240.0)))
}

impl Finder {
    /// Icone ou miniature reelle du shell ; un espace vide sert de repli pendant le chargement.
    pub fn icon(&self, path: &std::path::Path, size: f32, thumbnail: bool) -> AnyElement {
        let px_size = (size * 2.0).round() as i32; // x2 pour les ecrans HiDPI
        match self.icons.get(path, px_size, thumbnail) {
            Some(image) => img(image).size(px(size)).object_fit(ObjectFit::Contain).flex_none().into_any_element(),
            None => div().size(px(size)).flex_none().into_any_element(),
        }
    }

    /// Corps de l'apercu pour les volets (colonnes, galerie) : image, texte, ou icone.
    pub fn preview_body(&self, e: &Entry, max_width: f32) -> AnyElement {
        if e.is_image() {
            img(e.path.clone())
                .max_w(px(max_width))
                .max_h(px(max_width))
                .object_fit(ObjectFit::Contain)
                .into_any_element()
        } else if e.is_text() {
            div()
                .w(px(max_width))
                .max_h(px(max_width))
                .overflow_hidden()
                .p_3()
                .rounded_md()
                .bg(rgb(theme::BG))
                .text_size(px(12.))
                .child(fs::read_text_preview(&e.path))
                .into_any_element()
        } else {
            self.icon(&e.path, 128.0, true)
        }
    }

    pub fn toggle_quick_look(&mut self, cx: &mut Context<Self>) {
        if self.close_quick_look(cx) {
            return;
        }
        let Some(entry) = self.selected_entry().cloned() else { return };
        let win_size = quick_look_size(&entry, cx);
        let finder = cx.entity();
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(None, win_size, cx))),
            titlebar: Some(TitlebarOptions {
                title: Some(crate::i18n::t("Aper\u{e7}u rapide").into()),
                appears_transparent: true,
                ..Default::default()
            }),
            window_background: WindowBackgroundAppearance::Blurred,
            window_min_size: Some(size(px(320.), px(200.))),
            ..Default::default()
        };
        // Ouverture differee : la nouvelle fenetre lit le Finder des son premier rendu, ce qui
        // panique tant que le Finder est en cours de mise a jour (ici).
        cx.defer(move |cx| {
            let opener = finder.clone();
            let opened = cx.open_window(options, move |window, cx| {
                let view = cx.new(|cx| QuickLook::new(finder, cx));
                window.focus(&view.focus_handle(cx));
                view
            });
            if let Ok(handle) = opened {
                opener.update(cx, |f, _| f.ql = Some(handle));
            }
        });
    }

    /// Ferme la fenetre Quick Look ; retourne true si elle etait ouverte.
    pub fn close_quick_look(&mut self, cx: &mut Context<Self>) -> bool {
        match self.ql.take() {
            Some(handle) => handle.update(cx, |_, window, _| window.remove_window()).is_ok(),
            None => false,
        }
    }
}

/// Contenu interactif de l'apercu : texte modifiable ou tableau CSV (sinon apercu statique).
enum Body {
    None,
    Text(Entity<Editor>),
    Csv(Entity<CsvTable>),
    Pdf(PdfState),
    Video(VideoState),
}

pub struct VideoState {
    path: PathBuf,
    video: Option<crate::video::Video>,
    image: Option<std::sync::Arc<gpui::RenderImage>>,
}

pub struct PdfState {
    path: PathBuf,
    page: u32,
    count: u32,
    image: Option<std::sync::Arc<gpui::RenderImage>>,
    loading: bool,
}

pub struct QuickLook {
    finder: Entity<Finder>,
    focus: FocusHandle,
    shown: Option<PathBuf>,
    body: Body,
    body_path: Option<PathBuf>,
    video_gen: u64,
    volume: f64,
    muted: bool,
    last_move: std::time::Instant,
    subs: Vec<Subscription>,
    _observe: Subscription,
}

impl QuickLook {
    fn new(finder: Entity<Finder>, cx: &mut Context<Self>) -> Self {
        // La fenetre suit la selection du Finder.
        let observe = cx.observe(&finder, |_, _, cx| cx.notify());
        Self {
            finder,
            focus: cx.focus_handle(),
            shown: None,
            body: Body::None,
            video_gen: 0,
            volume: 0.8,
            muted: false,
            last_move: std::time::Instant::now(),
            body_path: None,
            subs: Vec::new(),
            _observe: observe,
        }
    }

    /// Enregistre le contenu modifie (texte ou CSV) avant de changer de fichier ou de fermer.
    fn save_body(&mut self, cx: &mut Context<Self>) {
        match (&self.body, &self.body_path) {
            (Body::Text(ed), Some(path)) => {
                let (text, dirty) = {
                    let e = ed.read(cx);
                    (e.text(), e.dirty)
                };
                if dirty && std::fs::write(path, text).is_ok() {
                    ed.update(cx, |e, _| e.dirty = false);
                }
            }
            (Body::Csv(table), _) => {
                if table.read(cx).dirty {
                    table.update(cx, |t, cx| t.save(cx));
                }
            }
            _ => {}
        }
    }

    fn is_dirty(&self, cx: &App) -> bool {
        match &self.body {
            Body::Text(ed) => ed.read(cx).dirty,
            Body::Csv(t) => t.read(cx).dirty,
            Body::None | Body::Pdf(_) | Body::Video(_) => false,
        }
    }

    fn is_editing(&self, window: &Window, cx: &App) -> bool {
        match &self.body {
            Body::Text(ed) => ed.read(cx).is_focused(window),
            Body::Csv(t) => t.read(cx).is_editing(),
            Body::None | Body::Pdf(_) | Body::Video(_) => false,
        }
    }

    /// Cree le corps interactif adapte au fichier (apres avoir enregistre le precedent).
    fn load_body(&mut self, entry: Option<&Entry>, window: &mut Window, cx: &mut Context<Self>) {
        self.save_body(cx);
        self.subs.clear();
        self.body = Body::None;
        self.body_path = entry.map(|e| e.path.clone());
        let Some(e) = entry else { return };
        if e.is_video() {
            self.video_gen += 1;
            let generation = self.video_gen;
            let video = crate::video::Video::open(&e.path, crate::layout::get_setting("qlauto").as_deref() != Some("0")).ok();
            if let Some(v) = &video {
                v.set_volume(self.volume);
                v.set_muted(self.muted);
            }
            self.last_move = std::time::Instant::now();
            self.body = Body::Video(VideoState { path: e.path.clone(), video, image: None });
            self.pump_video(generation, cx);
            return;
        }
        if e.is_pdf() {
            self.body = Body::Pdf(PdfState { path: e.path.clone(), page: 0, count: 0, image: None, loading: false });
            self.request_pdf(cx);
            return;
        }
        let is_csv = fs::extension(&e.path) == "csv";
        if is_csv && e.size <= 5_000_000 {
            let table = cx.new(|cx| CsvTable::new(e.path.clone(), cx));
            self.subs.push(cx.subscribe_in(&table, window, |this, _, ev: &CsvEvent, window, cx| {
                if matches!(ev, CsvEvent::StoppedEditing) {
                    window.focus(&this.focus);
                }
                cx.notify();
            }));
            self.subs.push(cx.observe(&table, |_, _, cx| cx.notify()));
            self.body = Body::Csv(table);
        } else if e.is_text() && e.size <= 2_000_000 {
            let text = String::from_utf8_lossy(&std::fs::read(&e.path).unwrap_or_default()).into_owned();
            let editor = cx.new(|cx| Editor::new(&text, false, cx));
            self.subs.push(cx.subscribe_in(&editor, window, |this, _, ev: &EditorEvent, window, cx| {
                match ev {
                    // Echap : on quitte la saisie (le fichier est enregistre), un second Echap ferme l'apercu.
                    EditorEvent::Escape => {
                        this.save_body(cx);
                        window.focus(&this.focus);
                    }
                    EditorEvent::Save => this.save_body(cx),
                    _ => {}
                }
                cx.notify();
            }));
            self.subs.push(cx.observe(&editor, |_, _, cx| cx.notify()));
            self.body = Body::Text(editor);
        }
    }

    /// Recupere les images decodees (~30 par seconde) tant que cette lecture est la courante.
    fn pump_video(&mut self, generation: u64, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| loop {
            cx.background_executor().timer(std::time::Duration::from_millis(33)).await;
            let alive = this
                .update(cx, |ql, cx| {
                    if ql.video_gen != generation {
                        return false;
                    }
                    let Body::Video(v) = &mut ql.body else { return false };
                    if let Some(frame) = v.video.as_ref().and_then(|x| x.take_frame()) {
                        let mut data = frame.data;
                        for px in data.chunks_exact_mut(4) {
                            px[3] = 255;
                        }
                        if let Some(buf) = image::RgbaImage::from_raw(frame.width, frame.height, data) {
                            v.image = Some(std::sync::Arc::new(gpui::RenderImage::new(vec![image::Frame::new(buf)])));
                        }
                    }
                    cx.notify();
                    true
                })
                .unwrap_or(false);
            if !alive {
                break;
            }
        })
        .detach();
    }

    /// Rend la page courante du PDF en arriere-plan.
    fn request_pdf(&mut self, cx: &mut Context<Self>) {
        let Body::Pdf(s) = &mut self.body else { return };
        s.loading = true;
        let (path, page) = (s.path.clone(), s.page);
        cx.notify();
        cx.spawn(async move |this, cx| {
            let p = path.clone();
            let result = cx.background_executor().spawn(async move { crate::pdf::render_page(&p, page, 1000) }).await;
            this.update(cx, |ql, cx| {
                if let Body::Pdf(s) = &mut ql.body {
                    if s.path == path && s.page == page {
                        if let Some((image, count)) = result {
                            s.image = Some(image);
                            s.count = count;
                        }
                        s.loading = false;
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn pdf_page(&mut self, delta: i32, cx: &mut Context<Self>) {
        let Body::Pdf(s) = &mut self.body else { return };
        let next = (s.page as i32 + delta).clamp(0, s.count.max(1) as i32 - 1) as u32;
        if next != s.page {
            s.page = next;
            self.request_pdf(cx);
        }
    }

    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.save_body(cx);
        window.remove_window();
        self.finder.update(cx, |f, _| f.ql = None);
    }
}

impl Focusable for QuickLook {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for QuickLook {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entry = self.finder.read(cx).selected_entry().cloned();
        let path = entry.as_ref().map(|e| e.path.clone());
        if path != self.shown {
            if let Some(e) = &entry {
                window.resize(quick_look_size(e, cx));
            }
            self.load_body(entry.as_ref(), window, cx);
            self.shown = path;
        }

        let editing = self.is_editing(window, cx);
        let dirty = self.is_dirty(cx);
        let title = entry.as_ref().map(|e| e.name.to_string()).unwrap_or_default();
        let hint = match &self.body {
            Body::Text(_) => Some(crate::i18n::t("Cliquez pour modifier \u{b7} Ctrl+S enregistre")),
            Body::Csv(_) => Some(crate::i18n::t("Cliquez sur une cellule pour modifier")),
            Body::Pdf(_) => Some(crate::i18n::t("Pr\u{e9}c. / Suiv. pour changer de page")),
            Body::Video(_) => None,
            Body::None => None,
        };

        let body = match &self.body {
            Body::Text(ed) => Some(div().size_full().child(ed.clone()).into_any_element()),
            Body::Csv(t) => Some(div().size_full().child(t.clone()).into_any_element()),
            Body::Video(v) => {
                let (pos, dur) = v.video.as_ref().map(|x| x.times()).unwrap_or((0.0, 0.0));
                let playing = v.video.as_ref().map(|x| x.is_playing()).unwrap_or(false);
                let frac = if dur > 0.0 { pos / dur } else { 0.0 };
                let image = v.image.clone().or_else(|| self.finder.read(cx).icons.get(&v.path, 1920, true));
                let clock = |s: f64| format!("{}:{:02}", (s as u64) / 60, (s as u64) % 60);
                // Les commandes se masquent apres 2,5 s sans mouvement de souris pendant la lecture (comme macOS).
                let show = !playing || self.last_move.elapsed() < std::time::Duration::from_millis(2500);
                let (volume, muted) = (self.volume, self.muted);
                let white = rgb(0xfffffe);
                let dim = gpui::rgba(0xffffff4d);
                let toggle = |this: &mut QuickLook| {
                    if let Body::Video(v) = &this.body {
                        if let Some(video) = &v.video {
                            video.toggle();
                        }
                    }
                };
                let pill = div()
                    .w(px(560.))
                    .max_w_full()
                    .h(px(44.))
                    .px(px(14.))
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .rounded(px(14.))
                    .bg(gpui::rgba(0x1c1c1ee6))
                    .border_1()
                    .border_color(gpui::rgba(0xffffff1f))
                    .text_color(white)
                    .text_size(px(12.))
                    .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    // Lecture / pause
                    .child(
                        div()
                            .id("vid-toggle")
                            .size(px(26.))
                            .flex_none()
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_full()
                            .hover(|s| s.bg(gpui::rgba(0xffffff26)))
                            .child(if playing {
                                div()
                                    .flex()
                                    .gap(px(3.))
                                    .child(div().w(px(3.5)).h(px(13.)).rounded(px(1.)).bg(white))
                                    .child(div().w(px(3.5)).h(px(13.)).rounded(px(1.)).bg(white))
                                    .into_any_element()
                            } else {
                                div().font_family("Segoe UI Symbol").text_size(px(13.)).child("\u{25B6}\u{FE0E}").into_any_element()
                            })
                            .cursor_pointer().on_click(cx.listener(move |this, _, _, cx| {
                                toggle(this);
                                cx.notify();
                                cx.stop_propagation();
                            })),
                    )
                    .child(div().w(px(34.)).flex_none().child(clock(pos)))
                    // Barre de progression : trait fin, repere rond a la position courante ; clic ou glisser = deplacement.
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .h(px(20.))
                            .relative()
                            .flex()
                            .items_center()
                            .child(div().w_full().h(px(4.)).rounded_full().bg(dim))
                            .child(div().absolute().left_0().h(px(4.)).w(gpui::relative(frac as f32)).rounded_full().bg(white))
                            .child(
                                div()
                                    .absolute()
                                    .left(gpui::relative(frac as f32))
                                    .ml(px(-5.))
                                    .size(px(10.))
                                    .rounded_full()
                                    .bg(white)
                                    .shadow_sm(),
                            )
                            .child(div().absolute().inset_0().flex().children((0..80usize).map(|i| {
                                let seek = move |this: &mut QuickLook| {
                                    if let Body::Video(v) = &this.body {
                                        if let Some(video) = &v.video {
                                            let (_, d) = video.times();
                                            video.seek(d * (i as f64 + 0.5) / 80.0);
                                        }
                                    }
                                };
                                div()
                                    .id(("vid-seg", i))
                                    .flex_1()
                                    .h_full()
                                    .cursor_pointer().on_click(cx.listener(move |this, _, _, cx| {
                                        seek(this);
                                        cx.notify();
                                        cx.stop_propagation();
                                    }))
                                    .on_mouse_move(cx.listener(move |this, ev: &gpui::MouseMoveEvent, _, cx| {
                                        if ev.pressed_button == Some(gpui::MouseButton::Left) {
                                            seek(this);
                                            cx.notify();
                                        }
                                    }))
                            }))),
                    )
                    .child(div().w(px(40.)).flex_none().flex().justify_end().child(format!("-{}", clock((dur - pos).max(0.0)))))
                    // Son : bouton muet + reglette de volume.
                    .child(
                        div()
                            .id("vid-mute")
                            .size(px(26.))
                            .flex_none()
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_full()
                            .hover(|s| s.bg(gpui::rgba(0xffffff26)))
                            .font_family("Segoe Fluent Icons")
                            .text_size(px(15.))
                            .child(if muted || volume <= 0.0 { "\u{E74F}" } else if volume < 0.4 { "\u{E993}" } else if volume < 0.7 { "\u{E994}" } else { "\u{E995}" })
                            .cursor_pointer().on_click(cx.listener(|this, _, _, cx| {
                                this.muted = !this.muted;
                                if let Body::Video(v) = &this.body {
                                    if let Some(video) = &v.video {
                                        video.set_muted(this.muted);
                                    }
                                }
                                cx.notify();
                                cx.stop_propagation();
                            })),
                    )
                    .child(div().w(px(72.)).flex_none().h(px(20.)).relative().flex().items_center().child(div().w_full().h(px(4.)).rounded_full().bg(dim)).child(
                        div().absolute().left_0().h(px(4.)).w(gpui::relative(if muted { 0.0 } else { volume as f32 })).rounded_full().bg(white),
                    ).child(div().absolute().inset_0().flex().children((0..10usize).map(|i| {
                        let set = move |this: &mut QuickLook| {
                            this.volume = (i as f64 + 1.0) / 10.0;
                            this.muted = false;
                            if let Body::Video(v) = &this.body {
                                if let Some(video) = &v.video {
                                    video.set_volume(this.volume);
                                    video.set_muted(false);
                                }
                            }
                        };
                        div()
                            .id(("vid-vol", i))
                            .flex_1()
                            .h_full()
                            .cursor_pointer().on_click(cx.listener(move |this, _, _, cx| {
                                set(this);
                                cx.notify();
                                cx.stop_propagation();
                            }))
                            .on_mouse_move(cx.listener(move |this, ev: &gpui::MouseMoveEvent, _, cx| {
                                if ev.pressed_button == Some(gpui::MouseButton::Left) {
                                    set(this);
                                    cx.notify();
                                }
                            }))
                    }))));
                Some(
                    div()
                        .id("ql-video")
                        .size_full()
                        .relative()
                        .bg(rgb(0x101012))
                        .child(match image {
                            Some(i) => img(i).size_full().object_fit(ObjectFit::Contain).into_any_element(),
                            None => div().into_any_element(),
                        })
                        .on_mouse_move(cx.listener(|this, _, _, cx| {
                            let was_hidden = this.last_move.elapsed() >= std::time::Duration::from_millis(2500);
                            this.last_move = std::time::Instant::now();
                            if was_hidden {
                                cx.notify();
                            }
                        }))
                        .cursor_pointer().on_click(cx.listener(move |this, _, _, cx| {
                            toggle(this);
                            cx.notify();
                        }))
                        .child(
                            div()
                                .absolute()
                                .left_0()
                                .right_0()
                                .bottom(px(16.))
                                .px(px(16.))
                                .flex()
                                .justify_center()
                                .when(!show, |d| d.invisible())
                                .child(pill),
                        )
                        .into_any_element(),
                )
            }
            Body::Pdf(s) => {
                let (page, count, loading) = (s.page, s.count, s.loading);
                let nav = |id: &'static str, label: &'static str, delta: i32, cx: &mut Context<Self>| {
                    div()
                        .id(id)
                        .size(px(24.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded_full()
                        .text_size(px(15.))
                        .hover(|st| st.bg(rgb(0xe3e3e6)))
                        .child(label)
                        .cursor_pointer().on_click(cx.listener(move |this, _, _, cx| this.pdf_page(delta, cx)))
                };
                Some(
                    div()
                        .size_full()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .id("ql-pdf")
                                .flex_1()
                                .min_h_0()
                                .flex()
                                .justify_center()
                                .p(px(6.))
                                .child(match &s.image {
                                    Some(image) => img(image.clone()).h_full().object_fit(ObjectFit::Contain).into_any_element(),
                                    None => div()
                                        .text_color(rgb(theme::TEXT_SECONDARY))
                                        .child(if loading { crate::i18n::t("Chargement du PDF\u{2026}") } else { crate::i18n::t("Aper\u{e7}u PDF indisponible") })
                                        .into_any_element(),
                                }),
                        )
                        .child(
                            div()
                                .h(px(30.))
                                .flex_none()
                                .flex()
                                .items_center()
                                .justify_center()
                                .gap(px(10.))
                                .text_size(px(12.))
                                .text_color(rgb(theme::TEXT_SECONDARY))
                                .child(nav("pdf-prev", "\u{2039}", -1, cx))
                                .child(crate::i18n::tf(crate::i18n::t("Page {0} sur {1}"), &[("0", (page + 1).to_string()), ("1", (count.max(1)).to_string())]))
                                .child(nav("pdf-next", "\u{203a}", 1, cx)),
                        )
                        .into_any_element(),
                )
            }
            Body::None => entry.as_ref().map(|e| {
                if e.is_image() {
                    div()
                        .size_full()
                        .child(img(e.path.clone()).size_full().object_fit(ObjectFit::Contain).rounded(px(6.)))
                        .into_any_element()
                } else if e.is_text() {
                    div()
                        .id("ql-text")
                        .size_full()
                        .overflow_y_scroll()
                        .p_4()
                        .text_size(px(12.))
                        .child(fs::read_text_preview(&e.path))
                        .into_any_element()
                } else if e.is_video() {
                    let f = self.finder.read(cx);
                    let p = e.path.clone();
                    div()
                        .id("ql-video")
                        .size_full()
                        .relative()
                        .flex()
                        .items_center()
                        .justify_center()
                        .bg(rgb(0x101012))
                        .child(f.icon(&e.path, 560.0, true))
                        .child(
                            div()
                                .absolute()
                                .size(px(64.))
                                .rounded_full()
                                .flex()
                                .items_center()
                                .justify_center()
                                .bg(gpui::rgba(0x000000a8))
                                .text_color(rgb(0xfffffe))
                                .text_size(px(28.))
                                .child("\u{25B6}"),
                        )
                        .cursor_pointer().on_click(move |_, _, _| {
                            let _ = std::process::Command::new("cmd").args(["/C", "start", "", &p.to_string_lossy()]).spawn();
                        })
                        .into_any_element()
                } else {
                    let f = self.finder.read(cx);
                    div()
                        .size_full()
                        .flex()
                        .flex_col()
                        .items_center()
                        .justify_center()
                        .gap_3()
                        .child(f.icon(&e.path, 128.0, true))
                        .child(div().text_size(px(15.)).font_weight(gpui::FontWeight::SEMIBOLD).child(e.name.clone()))
                        .child(
                            div()
                                .text_color(rgb(theme::TEXT_SECONDARY))
                                .child(format!("{} \u{2014} {}", e.kind, fs::format_size(e))),
                        )
                        .into_any_element()
                }
            }),
        };
        let open_path = entry.as_ref().map(|e| e.path.clone());
        // Pendant la saisie, les fleches/Espace/Echap appartiennent a l'editeur : on change de contexte
        // pour que les raccourcis de navigation de l'apercu ne s'appliquent plus.
        let context = if editing { "QuickLookEdit" } else { "QuickLook" };

        div()
            .key_context(context)
            .track_focus(&self.focus)
            .on_action(cx.listener(|this, _: &CloseQuickLook, window, cx| this.close(window, cx)))
            .on_key_down(cx.listener(|this, ev: &gpui::KeyDownEvent, _, cx| match ev.keystroke.key.as_str() {
                "pagedown" => this.pdf_page(1, cx),
                "pageup" => this.pdf_page(-1, cx),
                _ => {}
            }))
            .on_action(cx.listener(|this, _: &MoveUp, _, cx| this.finder.update(cx, |f, cx| f.move_vertical(-1, cx))))
            .on_action(cx.listener(|this, _: &MoveDown, _, cx| this.finder.update(cx, |f, cx| f.move_vertical(1, cx))))
            .on_action(cx.listener(|this, _: &MoveLeft, _, cx| this.finder.update(cx, |f, cx| f.move_horizontal(-1, cx))))
            .on_action(cx.listener(|this, _: &MoveRight, _, cx| this.finder.update(cx, |f, cx| f.move_horizontal(1, cx))))
            .size_full()
            .flex()
            .flex_col()
            .bg(rgba(0xf6f6f6b8))
            .text_color(rgb(theme::TEXT))
            .text_size(px(13.))
            .font_family("Segoe UI Variable Text")
            .child(
                div()
                    .h(px(TITLE_BAR))
                    .flex_none()
                    // Glisser la barre de titre deplace la fenetre d'apercu.
                    .on_mouse_down(gpui::MouseButton::Left, |ev, window, _| crate::sidebar::window_drag(ev, window))
                    .px(px(16.))
                    .flex()
                    .items_center()
                    .gap(px(12.))
                    .child(
                        div()
                            .id("ql-close")
                            .size(px(16.))
                            .rounded_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .bg(rgb(0x262625))
                            .text_color(rgb(0xfffffe))
                            .font_family("Segoe Fluent Icons")
                            .text_size(px(8.))
                            .child("\u{E711}")
                            .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
                            .cursor_pointer().on_click(cx.listener(|this, _, window, cx| this.close(window, cx))),
                    )
                    .child(div().font_weight(gpui::FontWeight::BOLD).child(title))
                    .when(dirty, |d| d.child(div().size(px(7.)).rounded_full().bg(rgb(0xf5a623))))
                    .child(div().flex_1().h_full())
                    .children(hint.map(|h| div().text_size(px(11.)).text_color(rgb(theme::TEXT_SECONDARY)).child(h)))
                    .child(
                        div()
                            .id("ql-open")
                            .px(px(12.))
                            .h(px(24.))
                            .flex()
                            .items_center()
                            .rounded_full()
                            .bg(rgba(0x00000012))
                            .hover(|s| s.bg(rgba(0x00000022)))
                            .text_size(px(12.))
                            .child(crate::i18n::t("Ouvrir"))
                            .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
                            .cursor_pointer().on_click(move |_, _, _| {
                                if let Some(p) = &open_path {
                                    let _ = std::process::Command::new("cmd")
                                        .args(["/C", "start", "", &p.to_string_lossy()])
                                        .spawn();
                                }
                            }),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .mx(px(4.))
                    .mb(px(4.))
                    .rounded(px(8.))
                    .overflow_hidden()
                    .when(matches!(self.body, Body::Text(_) | Body::Csv(_)), |d| d.bg(rgb(0xffffff)))
                    .children(body),
            )
    }
}
