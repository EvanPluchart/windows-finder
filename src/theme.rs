//! Couleurs mesurees sur le Finder macOS en mode clair (voir docs/colors.md).

pub const BG: u32 = 0xffffff;
pub const SIDEBAR: u32 = 0xededed;
pub const TOOLBAR: u32 = 0xffffff;
pub const HEADER: u32 = 0xffffff;
pub const ALT_ROW: u32 = 0xf4f5f5;
pub const SEPARATOR: u32 = 0xd3d4d8;
pub const TEXT: u32 = 0x262626;
pub const TEXT_SECONDARY: u32 = 0x7f7f7f;
pub const SELECTION: u32 = 0x0064e1;
pub const SELECTION_TEXT: u32 = 0xfffffe;
pub const HOVER: u32 = 0xe6e6e6;
pub const PANEL: u32 = 0xf2f2f2;

// Geometrie mesuree sur le Finder (points), voir docs/geometry.md.
pub const SIDEBAR_WIDTH: f32 = 203.0;
pub const TOOLBAR_HEIGHT: f32 = 52.0;
pub const STATUS_HEIGHT: f32 = 28.0;
pub const LIST_ROW_HEIGHT: f32 = 20.0;
pub const LIST_HEADER_HEIGHT: f32 = 28.0;
pub const ICON_CELL: f32 = 112.0;
pub const ICON_SIZE: f32 = 64.0;
pub const COLUMN_WIDTH: f32 = 245.0;
pub const COLUMN_ROW_HEIGHT: f32 = 23.0;
pub const PREVIEW_PANE_WIDTH: f32 = 312.0;
pub const SIDEBAR_ROW_HEIGHT: f32 = 32.0;

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

/// Mode d'applications de Windows (sombre = `AppsUseLightTheme` a 0).
pub fn system_is_dark() -> bool {
    use std::os::windows::process::CommandExt;
    std::process::Command::new("reg")
        .args(["query", r"HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize", "/v", "AppsUseLightTheme"])
        .creation_flags(0x0800_0000)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains("0x0"))
        .unwrap_or(false)
}
