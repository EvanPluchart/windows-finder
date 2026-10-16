//! Fonctions ajoutees apres l'analyse de l'Explorateur Windows et de Files (docs/research) :
//! terminal, fond d'ecran, rotation d'images, execution en administrateur, elements epingles, onglets fermes.

use std::os::windows::ffi::OsStrExt;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use gpui::Context;

use crate::{layout, ops, Finder, ViewMode};

const NO_WINDOW: u32 = 0x0800_0000;

/// Ouvre un fichier avec son application par defaut (comme un double-clic dans l'Explorateur).
/// ShellExecuteW et non `cmd /C start` : pas de console qui s'ouvre, et un nom de fichier contenant
/// `&`, `^` ou `%` n'est jamais interprete comme une commande. Verbe nul = action par defaut du type
/// (pas forcement « open »). Sans application associee, on propose « Ouvrir avec… » comme l'Explorateur.
pub fn shell_open(path: &Path) {
    let path = path.to_path_buf();
    std::thread::spawn(move || unsafe {
        use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE, CoInitializeEx};
        use windows::Win32::UI::Shell::ShellExecuteW;
        use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
        use windows::core::PCWSTR;
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE);
        let wide: Vec<u16> = path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
        let h = ShellExecuteW(None, PCWSTR::null(), PCWSTR(wide.as_ptr()), PCWSTR::null(), PCWSTR::null(), SW_SHOWNORMAL);
        // Valeur <= 32 : echec (SE_ERR_NOASSOC, ...).
        if h.0 as isize <= 32 {
            let _ = std::process::Command::new("rundll32.exe").arg("shell32.dll,OpenAs_RunDLL").arg(&path).spawn();
        }
    });
}

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

/// Taille de fenetre memorisee (reglage `win_size` = "largeur x hauteur"), 1000 x 640 par defaut.
pub fn saved_window_size() -> (f32, f32) {
    layout::get_setting("win_size")
        .and_then(|s| {
            let (w, h) = s.split_once('x')?;
            Some((w.parse::<f32>().ok()?, h.parse::<f32>().ok()?))
        })
        .filter(|(w, h)| (640.0..=5000.0).contains(w) && (420.0..=3000.0).contains(h))
        .unwrap_or((1000.0, 640.0))
}

impl Finder {
    /// Enregistre la taille de la fenetre un instant apres le dernier redimensionnement.
    pub fn remember_window_size(&mut self, window: &gpui::Window, cx: &mut Context<Self>) {
        let vs = window.viewport_size();
        let now = (f32::from(vs.width).round() as i32, f32::from(vs.height).round() as i32);
        if now == self.last_size || now.0 < 100 {
            return;
        }
        let first = self.last_size == (0, 0);
        self.last_size = now;
        if first {
            return;
        }
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(std::time::Duration::from_millis(600)).await;
            this.update(cx, |f, _| {
                layout::set_setting("win_size", &format!("{}x{}", f.last_size.0, f.last_size.1));
            })
            .ok();
        })
        .detach();
    }
}

// ---------------------------------------------------------------------------------------------
// Recherches enregistrees (equivalent des dossiers intelligents du Finder)
// ---------------------------------------------------------------------------------------------

#[derive(Clone)]
pub struct Saved {
    pub name: String,
    pub root: PathBuf,
    pub query: String,
    pub content: bool,
    pub kind: u8,
}

fn searches_file() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join("winfinder").join("searches.tsv"))
}

pub fn saved_searches() -> Vec<Saved> {
    let Some(text) = searches_file().and_then(|f| std::fs::read_to_string(f).ok()) else { return Vec::new() };
    text.lines()
        .filter_map(|l| {
            let p: Vec<&str> = l.split('\t').collect();
            (p.len() == 5).then(|| Saved {
                name: p[0].to_string(),
                root: PathBuf::from(p[1]),
                query: p[2].to_string(),
                content: p[3] == "1",
                kind: p[4].parse().unwrap_or(0),
            })
        })
        .collect()
}

fn write_saved(list: &[Saved]) {
    let Some(file) = searches_file() else { return };
    if let Some(dir) = file.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let text: String = list
        .iter()
        .map(|s| format!("{}\t{}\t{}\t{}\t{}\n", s.name.replace('\t', " "), s.root.display(), s.query.replace('\t', " "), u8::from(s.content), s.kind))
        .collect();
    let _ = std::fs::write(file, text);
}

/// Numero de la recherche enregistree designee par un emplacement `::search:N` de la barre laterale.
pub fn saved_index(path: &Path) -> Option<usize> {
    path.to_str()?.strip_prefix("::search:")?.parse().ok()
}

impl Finder {
    /// Enregistre la recherche en cours (dossier, texte, portee et type) dans la barre laterale.
    pub fn save_current_search(&mut self, cx: &mut Context<Self>) {
        let Some(query) = self.search_query().map(str::to_owned) else { return };
        let mut list = saved_searches();
        let name = query.clone();
        list.retain(|s| !(s.root == self.cwd && s.query == query));
        list.push(Saved { name, root: self.cwd.clone(), query, content: self.search_content, kind: self.search_kind });
        write_saved(&list);
        self.places = crate::sidebar::default_places();
        self.show_notice(crate::i18n::t("Recherche enregistr\u{e9}e dans la barre lat\u{e9}rale"), cx);
        cx.notify();
    }

    pub fn run_saved_search(&mut self, i: usize, window: &mut gpui::Window, cx: &mut Context<Self>) {
        let Some(s) = saved_searches().into_iter().nth(i) else { return };
        if !s.root.is_dir() {
            self.show_notice(crate::i18n::t("Le dossier de cette recherche n'existe plus."), cx);
            return;
        }
        self.close_search_silent();
        self.navigate(s.root.clone(), true, cx);
        self.search_content = s.content;
        self.search_kind = s.kind;
        self.open_search(window, cx);
        if let Some(search) = &self.search {
            let editor = search.editor.clone();
            editor.update(cx, |e, cx| e.set_text(&s.query, cx));
        }
        self.run_search(cx);
    }

    pub fn remove_saved_search(&mut self, i: usize, cx: &mut Context<Self>) {
        let mut list = saved_searches();
        if i < list.len() {
            list.remove(i);
            write_saved(&list);
            self.places = crate::sidebar::default_places();
            cx.notify();
        }
    }
}
