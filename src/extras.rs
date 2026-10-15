//! Fonctions ajoutees apres l'analyse de l'Explorateur Windows et de Files (docs/research) :
//! terminal, fond d'ecran, rotation d'images, execution en administrateur, elements epingles, onglets fermes.

use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use gpui::Context;

use crate::{layout, ops, Finder, ViewMode};

const NO_WINDOW: u32 = 0x0800_0000;

/// Onglets fermes recemment (pile), pour « Rouvrir l'onglet ferme ».
static CLOSED: Mutex<Vec<(PathBuf, ViewMode)>> = Mutex::new(Vec::new());

pub fn remember_closed(path: PathBuf, view: ViewMode) {
    if let Ok(mut v) = CLOSED.lock() {
        v.push((path, view));
        let n = v.len();
        if n > 20 {
            v.drain(..n - 20);
        }
    }
}

/// Dossiers epingles dans la barre laterale (reglage `pins`, separes par « | »).
pub fn pins() -> Vec<PathBuf> {
    layout::get_setting("pins")
        .map(|s| s.split('|').filter(|p| !p.is_empty()).map(PathBuf::from).filter(|p| p.is_dir()).collect())
        .unwrap_or_default()
}

fn save_pins(list: &[PathBuf]) {
    let joined = list.iter().map(|p| p.display().to_string()).collect::<Vec<_>>().join("|");
    layout::set_setting("pins", &joined);
}

fn is_image(p: &Path) -> bool {
    matches!(crate::fs::extension(p).as_str(), "png" | "jpg" | "jpeg" | "bmp" | "gif" | "tif" | "tiff")
}

impl Finder {
    pub fn reopen_closed_tab(&mut self, cx: &mut Context<Self>) {
        let last = CLOSED.lock().ok().and_then(|mut v| v.pop());
        match last {
            Some((path, view)) => {
                self.new_tab(Some(path), cx);
                if view != self.view {
                    self.set_view(view, cx);
                }
            }
            None => self.show_notice(crate::i18n::t("Aucun onglet ferm\u{e9} \u{e0} rouvrir."), cx),
        }
    }

    /// Chemin(s) de la selection (ou du dossier courant) dans le presse-papiers.
    pub fn copy_path(&mut self, cx: &mut Context<Self>) {
        let mut paths = self.selected_paths();
        if paths.is_empty() && !crate::fs::is_virtual(&self.cwd) {
            paths.push(self.cwd.clone());
        }
        let text = paths.iter().map(|p| p.display().to_string()).collect::<Vec<_>>().join("\n");
        if !text.is_empty() {
            cx.write_to_clipboard(gpui::ClipboardItem::new_string(text));
            self.show_notice(crate::i18n::t("Chemin copi\u{e9}"), cx);
        }
    }

    pub fn new_text_file(&mut self, cx: &mut Context<Self>) {
        if self.read_only_place(cx) {
            return;
        }
        let dest = ops::unique_path(&self.cwd, &format!("{}.txt", crate::i18n::t("Nouveau fichier texte")));
        match std::fs::write(&dest, b"") {
            Ok(()) => self.reload_select(&[dest]),
            Err(e) => self.show_notice(crate::i18n::tf(crate::i18n::t("Cr\u{e9}ation impossible : {e}"), &[("e", e.to_string())]), cx),
        }
        cx.notify();
    }

    /// Ouvre le Terminal Windows dans le dossier selectionne (ou le dossier courant).
    pub fn open_terminal(&mut self) {
        let dir = match self.selected_entry() {
            Some(e) if e.is_dir => e.path.clone(),
            _ if !crate::fs::is_virtual(&self.cwd) => self.cwd.clone(),
            _ => crate::fs::home_dir(),
        };
        let wt = std::process::Command::new("wt.exe").arg("-d").arg(&dir).creation_flags(NO_WINDOW).spawn();
        if wt.is_err() {
            let _ = std::process::Command::new("powershell.exe")
                .arg("-NoExit")
                .arg("-Command")
                .arg(format!("Set-Location -LiteralPath '{}'", dir.display().to_string().replace('\'', "''")))
                .spawn();
        }
    }

    pub fn run_as_admin(&mut self) {
        let Some(e) = self.selected_entry().filter(|e| !e.is_dir) else { return };
        let script = format!("Start-Process -FilePath '{}' -Verb RunAs", e.path.display().to_string().replace('\'', "''"));
        let _ = std::process::Command::new("powershell.exe").args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", &script]).creation_flags(NO_WINDOW).spawn();
    }

    /// Image selectionnee comme fond d'ecran.
    pub fn set_wallpaper(&mut self, cx: &mut Context<Self>) {
        use windows::core::PCWSTR;
        use windows::Win32::UI::WindowsAndMessaging::{SystemParametersInfoW, SPI_SETDESKWALLPAPER, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS};
        let Some(e) = self.selected_entry().filter(|e| !e.is_dir && is_image(&e.path)).map(|e| e.path.clone()) else {
            self.show_notice(crate::i18n::t("S\u{e9}lectionnez d'abord une image."), cx);
            return;
        };
        let wide: Vec<u16> = e.as_os_str().encode_wide_null();
        let ok = unsafe {
            SystemParametersInfoW(SPI_SETDESKWALLPAPER, 0, Some(wide.as_ptr() as *mut _), SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(3)).is_ok()
        };
        let _ = PCWSTR::null();
        self.show_notice(
            if ok { crate::i18n::t("Fond d'\u{e9}cran modifi\u{e9}") } else { crate::i18n::t("Impossible de changer le fond d'\u{e9}cran") },
            cx,
        );
    }

    /// Pivote les images selectionnees de 90 degres (PowerShell / System.Drawing, hors thread d'interface).
    pub fn rotate_images(&mut self, clockwise: bool, cx: &mut Context<Self>) {
        let files: Vec<PathBuf> = self.selected_paths().into_iter().filter(|p| is_image(p)).collect();
        if files.is_empty() {
            self.show_notice(crate::i18n::t("S\u{e9}lectionnez d'abord une image."), cx);
            return;
        }
        let mode = if clockwise { "Rotate90FlipNone" } else { "Rotate270FlipNone" };
        let list = files.iter().map(|p| format!("'{}'", p.display().to_string().replace('\'', "''"))).collect::<Vec<_>>().join(",");
        let script = format!(
            "Add-Type -AssemblyName System.Drawing; foreach($p in @({list})){{ $b=[System.IO.File]::ReadAllBytes($p); $ms=New-Object System.IO.MemoryStream(,$b); $i=[System.Drawing.Image]::FromStream($ms); $f=$i.RawFormat; $i.RotateFlip([System.Drawing.RotateFlipType]::{mode}); $i.Save($p,$f); $i.Dispose(); $ms.Dispose() }}"
        );
        let keep = self.selected_paths();
        cx.spawn(async move |this, cx| {
            let result = cx.background_executor().spawn(async move {
                std::process::Command::new("powershell.exe")
                    .args(["-NoProfile", "-NonInteractive", "-Command", &script])
                    .creation_flags(NO_WINDOW)
                    .output()
                    .map(|o| o.status.success())
                    .unwrap_or(false)
            }).await;
            this.update(cx, |f, cx| {
                f.icons.clear_all();
                f.reload_select(&keep);
                if !result {
                    f.show_notice(crate::i18n::t("Rotation impossible."), cx);
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// Epingle le dossier selectionne (ou courant) dans la barre laterale.
    pub fn pin_selection(&mut self, cx: &mut Context<Self>) {
        let target = match self.selected_entry() {
            Some(e) if e.is_dir && !crate::fs::is_virtual(&e.path) => e.path.clone(),
            None if !crate::fs::is_virtual(&self.cwd) => self.cwd.clone(),
            _ => return,
        };
        let mut list = pins();
        if !list.contains(&target) {
            list.push(target);
            save_pins(&list);
            self.places = crate::sidebar::default_places();
        }
        cx.notify();
    }

    pub fn unpin(&mut self, path: &Path, cx: &mut Context<Self>) {
        let list: Vec<PathBuf> = pins().into_iter().filter(|p| p != path).collect();
        save_pins(&list);
        self.places = crate::sidebar::default_places();
        cx.notify();
    }

    pub fn set_theme(&mut self, value: &str, cx: &mut Context<Self>) {
        layout::set_setting("theme", value);
        crate::settings::apply_theme();
        cx.defer(|app| {
            for w in app.windows() {
                let _ = w.update(app, |_, window, _| window.refresh());
            }
        });
        cx.notify();
    }
}

trait WideNull {
    fn encode_wide_null(&self) -> Vec<u16>;
}

impl WideNull for std::ffi::OsStr {
    fn encode_wide_null(&self) -> Vec<u16> {
        use std::os::windows::ffi::OsStrExt;
        self.encode_wide().chain(std::iter::once(0)).collect()
    }
}
