//! Icones et miniatures reelles via le shell Windows (IShellItemImageFactory).
//! Chargement en arriere-plan (4 threads), cache partage, le rendu ne bloque jamais.

use std::collections::{HashMap, HashSet};
use std::ffi::c_void;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use gpui::RenderImage;
use image::{Frame, RgbaImage};
use windows::Win32::Foundation::SIZE;
use windows::Win32::Graphics::Gdi::{
    BITMAP, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, DeleteObject, GetDC, GetDIBits, GetObjectW,
    HGDIOBJ, ReleaseDC,
};
use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx, IBindCtx};
use windows::Win32::UI::Shell::{
    IShellItemImageFactory, SHCreateItemFromParsingName, SIIGBF_BIGGERSIZEOK, SIIGBF_ICONONLY,
};
use windows::core::PCWSTR;

#[derive(Clone, PartialEq, Eq, Hash)]
struct Key {
    path: PathBuf,
    px: i32,
    thumbnail: bool,
}

struct Request {
    key: Key,
}

#[derive(Default)]
struct State {
    ready: HashMap<Key, Option<Arc<RenderImage>>>,
    pending: HashSet<Key>,
}

/// Cache d'icones partage entre l'UI et les threads de chargement.
#[derive(Clone)]
pub struct IconCache {
    state: Arc<Mutex<State>>,
    requests: flume::Sender<Request>,
}

impl IconCache {
    /// Lance les threads de chargement. `done` recoit un signal par image prete.
    pub fn new(done: flume::Sender<()>) -> Self {
        let state = Arc::new(Mutex::new(State::default()));
        let (tx, rx) = flume::unbounded::<Request>();
        for _ in 0..4 {
            let rx = rx.clone();
            let state = state.clone();
            let done = done.clone();
            std::thread::spawn(move || {
                unsafe {
                    let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
                }
                while let Ok(req) = rx.recv() {
                    let image = mac_folder_image(&req.key.path, req.key.px)
                        .or_else(|| shell_image(&req.key.path, req.key.px, req.key.thumbnail));
                    let mut s = state.lock().unwrap();
                    s.pending.remove(&req.key);
                    s.ready.insert(req.key, image);
                    drop(s);
                    let _ = done.send(());
                }
            });
        }
        Self { state, requests: tx }
    }

    /// Oublie toutes les images (apres modification de fichiers).
    pub fn clear_all(&self) {
        let mut s = self.state.lock().unwrap();
        s.ready.clear();
    }

    /// Retourne l'image si elle est prete ; sinon la demande en arriere-plan et retourne None.
    pub fn get(&self, path: &Path, px: i32, thumbnail: bool) -> Option<Arc<RenderImage>> {
        let key = Key { path: path.to_path_buf(), px, thumbnail };
        let mut s = self.state.lock().unwrap();
        if let Some(img) = s.ready.get(&key) {
            return img.clone();
        }
        if s.pending.insert(key.clone()) {
            let _ = self.requests.send(Request { key });
        }
        None
    }
}

fn shell_image(path: &Path, px: i32, thumbnail: bool) -> Option<Arc<RenderImage>> {
    // Raccourcis (.lnk) : pas de miniature, seulement l'icone de l'application visee.
    let thumbnail = thumbnail && path.extension().map(|e| !e.eq_ignore_ascii_case("lnk")).unwrap_or(true);
    unsafe {
        let wide: Vec<u16> = path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
        let factory: IShellItemImageFactory =
            SHCreateItemFromParsingName(PCWSTR(wide.as_ptr()), None::<&IBindCtx>).ok()?;
        let flags = if thumbnail { SIIGBF_BIGGERSIZEOK } else { SIIGBF_BIGGERSIZEOK | SIIGBF_ICONONLY };
        let hbmp = factory.GetImage(SIZE { cx: px, cy: px }, flags).ok()?;

        let mut bm = BITMAP::default();
        GetObjectW(HGDIOBJ(hbmp.0), size_of::<BITMAP>() as i32, Some(&mut bm as *mut _ as *mut c_void));
        let (w, h) = (bm.bmWidth, bm.bmHeight.abs());
        if w <= 0 || h <= 0 {
            let _ = DeleteObject(HGDIOBJ(hbmp.0));
            return None;
        }

        let mut info = BITMAPINFO::default();
        info.bmiHeader = BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w,
            biHeight: -h, // de haut en bas
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        };
        let mut buf = vec![0u8; (w * h * 4) as usize];
        let hdc = GetDC(None);
        let lines = GetDIBits(hdc, hbmp, 0, h as u32, Some(buf.as_mut_ptr() as *mut c_void), &mut info, DIB_RGB_COLORS);
        ReleaseDC(None, hdc);
        let _ = DeleteObject(HGDIOBJ(hbmp.0));
        if lines == 0 {
            return None;
        }

        // Le shell fournit du BGRA pre-multiplie ; GPUI attend du BGRA non pre-multiplie.
        for p in buf.chunks_exact_mut(4) {
            let a = p[3] as u32;
            if a > 0 && a < 255 {
                for c in &mut p[..3] {
                    *c = ((*c as u32 * 255 + a / 2) / a).min(255) as u8;
                }
            }
        }
        let image = RgbaImage::from_raw(w as u32, h as u32, buf)?;
        Some(Arc::new(RenderImage::new(vec![Frame::new(image)])))
    }
}

// ---- Dossiers a la macOS (PNG du Finder, usage personnel) ----

static FOLDER_PNGS: [(&str, &[u8]); 7] = [
    ("generic", include_bytes!("../assets/macicons/GenericFolderIcon.png")),
    ("Desktop", include_bytes!("../assets/macicons/DesktopFolderIcon.png")),
    ("Documents", include_bytes!("../assets/macicons/DocumentsFolderIcon.png")),
    ("Downloads", include_bytes!("../assets/macicons/DownloadsFolder.png")),
    ("Videos", include_bytes!("../assets/macicons/MovieFolderIcon.png")),
    ("Music", include_bytes!("../assets/macicons/MusicFolderIcon.png")),
    ("Pictures", include_bytes!("../assets/macicons/PicturesFolderIcon.png")),
];

/// Icone de dossier macOS pour `path` (dossier special du profil, sinon dossier generique) ; None si ce n'est pas un dossier.
fn mac_folder_image(path: &Path, px: i32) -> Option<Arc<RenderImage>> {
    use std::sync::OnceLock;
    static CACHE: OnceLock<Mutex<HashMap<(&'static str, i32), Option<Arc<RenderImage>>>>> = OnceLock::new();
    if path.parent().is_none() || !path.is_dir() {
        return None; // racine de disque ou fichier : icone du shell
    }
    let home = crate::fs::home_dir();
    let key = ["Desktop", "Documents", "Downloads", "Videos", "Music", "Pictures"]
        .into_iter()
        .find(|k| path == home.join(k))
        .unwrap_or("generic");
    let mut cache = CACHE.get_or_init(Default::default).lock().unwrap();
    cache
        .entry((key, px))
        .or_insert_with(|| {
            let bytes = FOLDER_PNGS.iter().find(|(k, _)| *k == key)?.1;
            let src = image::load_from_memory_with_format(bytes, image::ImageFormat::Png).ok()?.to_rgba8();
            // Reduction de qualite vers la taille demandee (evite le flou/aliasing d'une grande image reduite par le GPU).
            let target = (px.max(8) as u32).min(256);
            let mut rgba = if target < 256 {
                image::imageops::resize(&src, target, target, image::imageops::FilterType::Lanczos3)
            } else {
                src
            };
            // GPUI attend du BGRA (alpha non pre-multiplie).
            for p in rgba.chunks_exact_mut(4) {
                p.swap(0, 2);
            }
            Some(Arc::new(RenderImage::new(vec![Frame::new(rgba)])))
        })
        .clone()
}
