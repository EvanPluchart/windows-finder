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
    ["uninstall", "d\u{e9}sinstall", "desinstall", "readme", "documentation", "license", "licence", "manual", "help", "website", "release notes"]
        .iter()
        .any(|w| n.contains(w))
}

/// Programmes du menu Demarrer (raccourcis .lnk), tries par nom, sans doublons.
pub fn entries() -> Vec<Entry> {
    let mut found: Vec<Entry> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for root in start_menu_roots() {
        let mut stack = vec![(root, 0)];
        while let Some((dir, depth)) = stack.pop() {
            let Ok(read) = std::fs::read_dir(&dir) else { continue };
            for e in read.flatten() {
                let path = PathBuf::from(e.path().to_string_lossy().replace('/', "\\"));
                let Ok(meta) = e.metadata() else { continue };
                if meta.is_dir() {
                    if depth < 3 {
                        stack.push((path, depth + 1));
                    }
                    continue;
                }
                if path.extension().map(|x| x.to_ascii_lowercase()) != Some("lnk".into()) {
                    continue;
                }
                let name = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
                if name.is_empty() || is_noise(&name) || !seen.insert(name.to_lowercase()) {
                    continue;
                }
                found.push(Entry {
                    name: SharedString::from(name),
                    path,
                    is_dir: false,
                    size: 0,
                    modified: meta.modified().ok(),
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

