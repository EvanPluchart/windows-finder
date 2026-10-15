//! Emplacements virtuels : "Applications" (programmes installes, avec desinstallation) et "Ce PC" (disques).

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use gpui::SharedString;

use crate::fs::Entry;

fn start_menu_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(p) = std::env::var_os("ProgramData") {
        roots.push(PathBuf::from(p).join(r"Microsoft\Windows\Start Menu\Programs"));
    }
    if let Some(p) = std::env::var_os("APPDATA") {
        roots.push(PathBuf::from(p).join(r"Microsoft\Windows\Start Menu\Programs"));
    }
    roots
}

fn is_noise(name: &str) -> bool {
    let n = name.to_lowercase();
    [
        "uninstall", "d\u{e9}sinstall", "desinstall", "readme", "documentation", "license", "licence", "manual", "help",
        "website", "release notes", "aide de", "\u{e0} propos", "a propos", "configurer", "configure ", "visit", "tutorial",
        "changelog", "support", "faq",
    ]
    .iter()
    .any(|w| n.contains(w))
}

/// Dossiers du menu Demarrer qui regroupent des outils Windows (pas des applications installees par l'utilisateur).
fn is_system_folder(name: &str) -> bool {
    let n = name.to_lowercase();
    [
        "windows powershell", "windows tools", "windows system", "windows accessories", "windows administrative tools",
        "windows ease of access", "accessories", "accessoires windows", "accessoires", "administrative tools",
        "outils d'administration windows", "outils d\u{2019}administration windows", "syst\u{e8}me windows", "facilit\u{e9} d'acc\u{e8}s windows",
        "startup", "d\u{e9}marrage", "maintenance", "outils syst\u{e8}me", "system tools",
    ]
    .contains(&n.as_str())
        || n == "administrative tools"
}

/// Cible d'un raccourci .lnk (chemin du programme), si elle est lisible.
pub fn link_target(lnk: &Path) -> Option<PathBuf> {
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx, IPersistFile, STGM_READ};
    use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};
    use windows::core::{Interface, PCWSTR};
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).ok()?;
        let file: IPersistFile = link.cast().ok()?;
        let wide: Vec<u16> = lnk.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
        file.Load(PCWSTR(wide.as_ptr()), STGM_READ).ok()?;
        let mut buf = [0u16; 520];
        link.GetPath(&mut buf, std::ptr::null_mut(), 0).ok()?;
        let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        let s = String::from_utf16_lossy(&buf[..len]);
        (!s.is_empty()).then(|| PathBuf::from(s))
    }
}

/// Vrai si la cible est un composant de Windows (dans le dossier Windows).
fn target_is_system(target: &Path) -> bool {
    let win = std::env::var_os("SystemRoot").or_else(|| std::env::var_os("windir")).map(PathBuf::from);
    match win {
        Some(w) => target.to_string_lossy().to_lowercase().starts_with(&w.to_string_lossy().to_lowercase()),
        None => false,
    }
}

/// Programmes du menu Demarrer (raccourcis .lnk), tries par nom, sans doublons.
pub fn entries() -> Vec<Entry> {
    // Reglage "Afficher les applications systeme" : sans lui, seules les applications installees apparaissent.
    let all = crate::layout::get_setting("apps_all").as_deref() == Some("1");
    let mut found: Vec<Entry> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for root in start_menu_roots() {
        let mut stack: Vec<(PathBuf, u32, Option<String>)> = vec![(root, 0, None)];
        while let Some((dir, depth, top)) = stack.pop() {
            let Ok(read) = std::fs::read_dir(&dir) else { continue };
            for e in read.flatten() {
                let path = PathBuf::from(e.path().to_string_lossy().replace('/', "\\"));
                let Ok(meta) = e.metadata() else { continue };
                if meta.is_dir() {
                    if depth < 3 {
                        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                        if !all && is_system_folder(&name) {
                            continue;
                        }
                        let top = top.clone().or(Some(name));
                        stack.push((path, depth + 1, top));
                    }
                    continue;
                }
                if path.extension().map(|x| x.to_ascii_lowercase()) != Some("lnk".into()) {
                    continue;
                }
                let name = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
                if name.is_empty() || !seen.insert(name.to_lowercase()) {
                    continue;
                }
                if !all {
                    if is_noise(&name) || is_system_folder(&name) {
                        continue;
                    }
                    if link_target(&path).map(|t| target_is_system(&t)).unwrap_or(false) {
                        continue;
                    }
                }
                found.push(Entry {
                    name: SharedString::from(name),
                    path,
                    is_dir: false,
                    size: 0,
                    modified: meta.modified().ok(),
                    created: meta.created().ok(),
                depth: 0,
                    kind: "Application".into(),
                    hint: None,
                });
            }
        }
    }
    found.sort_by_key(|e| e.name.to_lowercase());
    found
}

/// Disques du PC, comme entrees de dossier (double-clic = ouvrir le disque).
pub fn drives() -> Vec<Entry> {
    (b'A'..=b'Z')
        .filter_map(|l| {
            let root = PathBuf::from(format!("{}:\\", l as char));
            root.exists().then(|| {
                let (label, _) = crate::sidebar::drive_info(&root);
                Entry {
                    name: SharedString::from(label),
                    path: root,
                    is_dir: true,
                    size: 0,
                    modified: None::<SystemTime>,
                    created: None,
                    depth: 0,
                    kind: "Disque".into(),
                    hint: None,
                }
            })
        })
        .collect()
}

/// Commande de desinstallation officielle d'un programme (registre Windows), trouvee par son nom.
/// Interroge PowerShell : a appeler hors du thread d'interface.
pub fn find_uninstall(app: &str) -> Option<String> {
    let script = r#"$k='HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*','HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*','HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*'; Get-ItemProperty $k -ErrorAction SilentlyContinue | Where-Object { $_.DisplayName -and $_.UninstallString } | ForEach-Object { "$($_.DisplayName)`t$($_.UninstallString)" }"#;
    let out = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    let wanted = app.to_lowercase();
    let mut best: Option<(usize, String)> = None;
    for line in text.lines() {
        let Some((name, cmd)) = line.split_once('\t') else { continue };
        let n = name.trim().to_lowercase();
        let score = if n == wanted {
            0
        } else if wanted.len() >= 4 && (n.contains(&wanted) || (n.len() >= 4 && wanted.contains(&n))) {
            1 + n.len().abs_diff(wanted.len())
        } else {
            continue;
        };
        if best.as_ref().map(|(s, _)| score < *s).unwrap_or(true) {
            best = Some((score, cmd.trim().to_string()));
        }
    }
    best.map(|(_, c)| c)
}

/// Lance la commande de desinstallation (l'assistant du programme demande lui-meme sa confirmation).
pub fn launch(cmd: &str) {
    use std::os::windows::process::CommandExt;
    let _ = std::process::Command::new("cmd").arg("/C").raw_arg(format!("\"{cmd}\"")).spawn();
}

