//! Reseau et Linux (WSL) : distributions installees, raccourcis reseau de l'utilisateur.

use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::sync::OnceLock;

use gpui::SharedString;

use crate::fs::Entry;

pub const NET: &str = "::net";

/// Distributions WSL enregistrees (registre de l'utilisateur), sans les distributions internes de Docker.
pub fn wsl_distros() -> &'static [String] {
    static CACHE: OnceLock<Vec<String>> = OnceLock::new();
    CACHE.get_or_init(|| {
        let out = crate::ps::tool("reg.exe")
            .args(["query", r"HKCU\Software\Microsoft\Windows\CurrentVersion\Lxss", "/s", "/v", "DistributionName"])
            .creation_flags(0x0800_0000)
            .output();
        let Ok(out) = out else { return Vec::new() };
        let text = String::from_utf8_lossy(&out.stdout).into_owned();
        let mut names: Vec<String> = text
            .lines()
            .filter_map(|l| l.trim().strip_prefix("DistributionName"))
            .filter_map(|rest| rest.trim().strip_prefix("REG_SZ").map(|v| v.trim().to_string()))
            .filter(|n| !n.is_empty() && !n.starts_with("docker-desktop"))
            .collect();
        names.sort();
        names
    })
}

pub fn wsl_path(distro: &str) -> PathBuf {
    // Chemin UNC : deux barres obliques inverses (une seule designait un dossier a la racine du disque courant).
    PathBuf::from(format!(r"\\wsl.localhost\{distro}"))
}

/// Contenu de la page Reseau : distributions Linux et raccourcis reseau.
pub fn entries() -> Vec<Entry> {
    let mut out = Vec::new();
    let mk = |name: String, path: PathBuf, kind: &str| Entry {
        name: SharedString::from(name),
        path,
        is_dir: true,
        size: 0,
        modified: None,
        created: None,
        depth: 0,
        kind: kind.to_string().into(),
        hint: None,
    };
    for d in wsl_distros() {
        out.push(mk(d.clone(), wsl_path(d), "Linux (WSL)"));
    }
    if let Some(dir) = std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join(r"Microsoft\Windows\Network Shortcuts")) {
        for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            let lnk = e.path().join("target.lnk");
            let target = crate::apps::link_target(&lnk).unwrap_or_else(|| e.path());
            out.push(mk(e.file_name().to_string_lossy().into_owned(), target, crate::i18n::t("Raccourci r\u{e9}seau")));
        }
    }
    out
}
