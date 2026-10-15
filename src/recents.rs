//! Place virtuelle « Recents » : les derniers fichiers et dossiers ouverts (dossier Recent de Windows),
//! montres directement (et non leurs raccourcis .lnk), du plus recent au plus ancien.

use std::path::PathBuf;

use gpui::SharedString;

use crate::fs::Entry;

pub const RECENTS: &str = "::recents";

pub fn entries() -> Vec<Entry> {
    let Some(dir) = std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join(r"Microsoft\Windows\Recent")) else { return Vec::new() };
    let mut found: Vec<(std::time::SystemTime, PathBuf)> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().extension().map(|x| x.eq_ignore_ascii_case("lnk")).unwrap_or(false))
        .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path())))
        .collect();
    found.sort_by(|a, b| b.0.cmp(&a.0));
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for (opened, lnk) in found {
        let Some(target) = crate::apps::link_target(&lnk) else { continue };
        let Ok(meta) = std::fs::metadata(&target) else { continue };
        if !seen.insert(target.clone()) {
            continue;
        }
        let is_dir = meta.is_dir();
        out.push(Entry {
            kind: crate::i18n::t("Dossier").into(),
            name: SharedString::from(target.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| target.display().to_string())),
            is_dir,
            size: if is_dir { 0 } else { meta.len() },
            modified: Some(opened),
            created: meta.created().ok(),
            depth: 0,
            hint: target.parent().map(|p| SharedString::from(p.display().to_string())),
            path: target,
        });
        if out.len() >= 200 {
            break;
        }
    }
    for e in &mut out {
        if !e.is_dir {
            e.kind = crate::fs::kind_label(&e.path);
        }
    }
    out
}
