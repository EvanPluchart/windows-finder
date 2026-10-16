//! Couleurs mesurees sur le Finder macOS en mode clair (voir docs/colors.md).

pub const BG: u32 = 0xffffff;
pub const ALT_ROW: u32 = 0xf4f5f5;
pub const SEPARATOR: u32 = 0xd3d4d8;
pub const TEXT: u32 = 0x262626;
pub const TEXT_SECONDARY: u32 = 0x7f7f7f;
pub const SELECTION: u32 = 0x0064e1;
pub const SELECTION_TEXT: u32 = 0xfffffe;
pub const HOVER: u32 = 0xe6e6e6;

// Geometrie mesuree sur le Finder (points), voir docs/geometry.md.
pub const SIDEBAR_WIDTH: f32 = 203.0;
pub const TOOLBAR_HEIGHT: f32 = 52.0;
pub const LIST_ROW_HEIGHT: f32 = 20.0;
pub const LIST_HEADER_HEIGHT: f32 = 28.0;
pub const COLUMN_WIDTH: f32 = 245.0;
pub const COLUMN_ROW_HEIGHT: f32 = 23.0;
pub const PREVIEW_PANE_WIDTH: f32 = 312.0;

// ---- Mode sombre : toutes les couleurs ecrites en clair passent par `rgb` / `rgba` ci-dessous, qui les
// ---- remplacent par leur equivalent sombre (valeurs mesurees dans le dossier Finder) quand le mode sombre est actif.

use std::sync::atomic::{AtomicBool, Ordering};

static DARK: AtomicBool = AtomicBool::new(false);

pub fn set_dark(on: bool) {
    DARK.store(on, Ordering::Relaxed);
}

pub fn is_dark() -> bool {
    DARK.load(Ordering::Relaxed)
}

/// Equivalent sombre d'une couleur claire (0xRRGGBB) ; les couleurs inconnues restent inchangees.
pub fn map(c: u32) -> u32 {
    // Couleur d'accentuation choisie dans les reglages (la premiere est le bleu d'origine).
    let a = ACCENT.load(Ordering::Relaxed) as usize;
    if a != 0 {
        let t = ACCENTS[a];
        let dark = is_dark();
        match c {
            0x0064e1 => return if dark { t.3 } else { t.2 },
            0x2f7df4 => return if dark { t.5 } else { t.4 },
            _ => {}
        }
    }
    if !is_dark() {
        return c;
    }
    match c {
        0xffffff => 0x202021,
        // Fonds de panneaux, barre laterale, lignes zebrees.
        0xededed => 0x262627,
        0xf2f2f2 | 0xf6f6f6 | 0xfafafa | 0xf8f8fa => 0x2c2c2e,
        0xf4f5f5 | 0xf4f4f6 => 0x2a2a2b,
        0xf1f1f4 | 0xf0f6ff | 0xe9e9eb | 0xe9e9ec | 0xe9e9ee | 0xececf0 => 0x333335,
        // Filets et contours.
        0xd3d4d8 => 0x434143,
        0xd9d9de | 0xdadade | 0xd0d0d6 => 0x48484a,
        0xe4e5e6 => 0x454545,
        // Survols et pastilles.
        0xe3e3e6 | 0xe3e3e7 | 0xe4e4e8 | 0xe2e2e6 | 0xf0f0f0 => 0x3a3a3d,
        0xe6e6e6 | 0xdcdcdf | 0xdcdce0 | 0xdedee4 => 0x48484a,
        0xfdfdfd => 0x3a3a3c,
        0xc6c6cb => 0x5a5a5e,
        // Texte.
        0x262626 => 0xdddddd,
        0x7f7f7f => 0x9a9a9b,
        0x878787 | 0x6e6e73 => 0x98989d,
        0xa0a0a6 | 0x9a9aa0 => 0x8a8a8f,
        0xb0b0b6 | 0xb4b4b8 => 0x636366,
        // Accent et selections.
        0x0064e1 => 0x0458d0,
        0x2f7df4 => 0x4d94ff,
        0xc4daf9 | 0xcfe2fb => 0x1d3f6e,
        _ => c,
    }
}

pub fn rgb(c: u32) -> gpui::Rgba {
    gpui::rgb(map(c))
}

/// `c` = 0xRRGGBBAA : seule la partie couleur est remplacee.
pub fn rgba(c: u32) -> gpui::Rgba {
    gpui::rgba((map(c >> 8) << 8) | (c & 0xff))
}

#[link(name = "advapi32")]
unsafe extern "system" {
    fn RegGetValueW(hkey: isize, subkey: *const u16, value: *const u16, flags: u32, kind: *mut u32, data: *mut u32, len: *mut u32) -> i32;
}

/// Mode d'applications de Windows (sombre = `AppsUseLightTheme` a 0). Lecture directe du registre (quelques
/// microsecondes) : plus de `reg.exe` lance toutes les 3 secondes.
pub fn system_is_dark() -> bool {
    const HKEY_CURRENT_USER: isize = 0x8000_0001u32 as i32 as isize;
    const RRF_RT_REG_DWORD: u32 = 0x10;
    let wide = |s: &str| s.encode_utf16().chain(std::iter::once(0)).collect::<Vec<u16>>();
    let (key, name) = (wide(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize"), wide("AppsUseLightTheme"));
    let (mut value, mut len) = (1u32, 4u32);
    let status = unsafe { RegGetValueW(HKEY_CURRENT_USER, key.as_ptr(), name.as_ptr(), RRF_RT_REG_DWORD, std::ptr::null_mut(), &mut value, &mut len) };
    status == 0 && value == 0
}

/// Vrai si le theme a ete impose au lancement (--dark / --light) : il ne suit plus Windows.
static FORCED: AtomicBool = AtomicBool::new(false);

pub fn set_forced(on: bool) {
    FORCED.store(on, Ordering::Relaxed);
}

pub fn is_forced() -> bool {
    FORCED.load(Ordering::Relaxed)
}

/// Couleurs d'accentuation : (code, libelle, selection clair, selection sombre, icones clair, icones sombre).
pub const ACCENTS: [(&str, &str, u32, u32, u32, u32); 8] = [
    ("blue", "Bleu", 0x0064e1, 0x0458d0, 0x2f7df4, 0x4d94ff),
    ("purple", "Violet", 0x8944ab, 0x7d3f9e, 0xa550a7, 0xbf5af2),
    ("pink", "Rose", 0xd92b72, 0xc42a67, 0xf74f9e, 0xff6bb3),
    ("red", "Rouge", 0xd3302f, 0xbf2c2b, 0xf5524f, 0xff6b66),
    ("orange", "Orange", 0xe0640a, 0xc95a09, 0xf7822b, 0xff9a47),
    ("yellow", "Jaune", 0xd9a400, 0xc29200, 0xf0b82a, 0xffd23d),
    ("green", "Vert", 0x2ea043, 0x278a39, 0x3fbf5a, 0x4fd36d),
    ("graphite", "Graphite", 0x6b6b70, 0x5a5a5e, 0x8e8e93, 0xa0a0a5),
];

static ACCENT: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

pub fn set_accent(code: &str) {
    let i = ACCENTS.iter().position(|a| a.0 == code).unwrap_or(0);
    ACCENT.store(i as u8, Ordering::Relaxed);
}

/// Theme choisi dans les reglages : "system" (suit Windows), "light" ou "dark". Retourne vrai pour le mode sombre.
pub fn resolve_dark(setting: Option<&str>) -> bool {
    match setting {
        Some("dark") => true,
        Some("light") => false,
        _ => system_is_dark(),
    }
}
