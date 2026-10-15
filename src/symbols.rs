//! Symboles du Finder (SF Symbols, PNG noirs sur fond transparent) recolores a la volee.
//! Usage personnel : a remplacer par un jeu libre avant toute distribution (voir docs/colors.md).

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use gpui::{AnyElement, IntoElement, RenderImage, Styled, img, px};
use image::Frame;

macro_rules! sf {
    ($($n:literal),* $(,)?) => {
        &[$(($n, include_bytes!(concat!("../assets/sf/", $n, ".png")) as &[u8])),*]
    };
}

static PNGS: &[(&str, &[u8])] = sf![
    "clock", "house", "menubar.dock.rectangle", "doc", "arrow.down.circle", "photo", "music.note", "video", "icloud",
    "internaldrive", "externaldrive", "eject.fill", "applications", "app.dashed", "globe", "trash", "laptopcomputer", "folder", "list.bullet", "rectangle.split.3x1", "square.grid.3x1.below.line.grid.1x2",
    "square.grid.3x2", "square.and.arrow.up", "tag", "ellipsis", "magnifyingglass",
    "chevron.left-medium", "chevron.right-medium", "chevron.down-medium", "square.grid.2x2-medium", "xmark-medium",
    "list.bullet-medium", "rectangle.split.3x1-medium", "square.grid.3x1.below.line.grid.1x2-medium",
    "square.grid.3x2-medium", "square.and.arrow.up-medium", "tag-medium", "ellipsis-medium", "magnifyingglass-medium",
];

type Entry = Option<(Arc<RenderImage>, f32, f32)>;

fn tinted(name: &str, color: u32, pt: f32) -> Entry {
    static CACHE: OnceLock<Mutex<HashMap<(String, u32, u32), Entry>>> = OnceLock::new();
    let mut cache = CACHE.get_or_init(Default::default).lock().unwrap();
    cache
        .entry((name.to_string(), color, (pt * 10.0) as u32))
        .or_insert_with(|| {
            let bytes = PNGS.iter().find(|(k, _)| *k == name)?.1;
            let rgba = image::load_from_memory_with_format(bytes, image::ImageFormat::Png).ok()?.to_rgba8();
            // Taille logique voulue, puis reduction de qualite (Lanczos) vers 2x cette taille : GPUI reduit mal les grandes
            // images (aliasing, flou), donc on fournit presque la taille finale.
            let (w0, h0) = rgba.dimensions();
            let k = pt / 96.0;
            let (lw, lh) = (w0 as f32 * k, h0 as f32 * k);
            let (pw, ph) = (((lw * 2.0).round() as u32).max(1), ((lh * 2.0).round() as u32).max(1));
            let mut rgba = image::imageops::resize(&rgba, pw, ph, image::imageops::FilterType::Lanczos3);
            // Le glyphe d'origine est noir : seule l'opacite compte. GPUI attend du BGRA.
            let (r, g, b) = ((color >> 16) as u8, (color >> 8) as u8, color as u8);
            for p in rgba.chunks_exact_mut(4) {
                p[0] = b;
                p[1] = g;
                p[2] = r;
            }
            Some((Arc::new(RenderImage::new(vec![Frame::new(rgba)])), lw, lh))
        })
        .clone()
}

/// Symbole `name` a la taille de police `pt` (le PNG fait 96 pt de corps), teinte `color` (0xRRGGBB).
pub fn sym(name: &str, pt: f32, color: u32) -> AnyElement {
    let color = crate::theme::map(color);
    match tinted(name, color, pt) {
        Some((image, w, h)) => {
            img(image).w(px(w)).h(px(h)).flex_none().into_any_element()
        }
        None => gpui::div().size(px(pt)).flex_none().into_any_element(),
    }
}
