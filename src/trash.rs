//! Corbeille dans l'application : liste des elements ($Recycle.Bin), "Remettre a sa place", "Vider la corbeille".
//! On lit directement les fichiers $I (chemin d'origine, date, taille) et $R (contenu) du format Windows 10/11.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use gpui::{Context, SharedString};

use crate::fs::Entry;
use crate::{ops, Finder};

pub const TRASH: &str = "::trash";

/// Dossiers de corbeille accessibles a l'utilisateur (un par disque et par compte).
fn bins() -> Vec<PathBuf> {
    let mut out = Vec::new();
    for letter in b'C'..=b'Z' {
        let root = PathBuf::from(format!("{}:\\$Recycle.Bin", letter as char));
        let Ok(read) = std::fs::read_dir(&root) else { continue };
        for e in read.flatten() {
            let p = e.path();
            // Lisible seulement pour le compte courant.
            if p.is_dir() && std::fs::read_dir(&p).is_ok() {
                out.push(p);
            }
        }
    }
    out
}

struct Info {
    original: PathBuf,
    size: u64,
    deleted: Option<SystemTime>,
}

/// Lit un fichier $I (version 1 = Windows Vista/7/8, version 2 = Windows 10/11).
fn read_info(path: &Path) -> Option<Info> {
    let b = std::fs::read(path).ok()?;
    if b.len() < 28 {
        return None;
    }
    let version = u64::from_le_bytes(b[0..8].try_into().ok()?);
    let size = u64::from_le_bytes(b[8..16].try_into().ok()?);
    let ft = u64::from_le_bytes(b[16..24].try_into().ok()?);
    let text_start = if version >= 2 { 28 } else { 24 };
    let units: Vec<u16> = b[text_start..].chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).take_while(|&u| u != 0).collect();
    let original = PathBuf::from(String::from_utf16_lossy(&units));
    // FILETIME (100 ns depuis 1601) -> SystemTime.
    let secs_1601_to_1970 = 11_644_473_600u64;
    let deleted = (ft / 10_000_000).checked_sub(secs_1601_to_1970).map(|s| UNIX_EPOCH + Duration::from_secs(s));
    Some(Info { original, size, deleted })
}

fn info_path(content: &Path) -> Option<PathBuf> {
    let name = content.file_name()?.to_string_lossy().into_owned();
    let suffix = name.strip_prefix("$R")?;
    Some(content.with_file_name(format!("$I{suffix}")))
}

/// Elements de la corbeille comme entrees de dossier (chemin = contenu $R, nom = nom d'origine).
pub fn entries() -> Vec<Entry> {
    let mut out = Vec::new();
    for bin in bins() {
        let Ok(read) = std::fs::read_dir(&bin) else { continue };
        for e in read.flatten() {
            let content = e.path();
            let fname = e.file_name().to_string_lossy().into_owned();
            if !fname.starts_with("$R") {
                continue;
            }
            let Some(info) = info_path(&content).and_then(|i| read_info(&i)) else { continue };
            let is_dir = content.is_dir();
            let name = info.original.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| fname.clone());
            out.push(Entry {
                kind: if is_dir { "Dossier".into() } else { crate::fs::kind_label(&info.original) },
                name: SharedString::from(name),
                path: content,
                is_dir,
                size: if is_dir { 0 } else { info.size },
                modified: info.deleted,
                created: info.deleted,
                depth: 0,
                hint: info.original.parent().map(|p| SharedString::from(p.display().to_string())),
            });
        }
    }
    out.sort_by_key(|e| e.name.to_lowercase());
    out
}

/// Remet un element a son emplacement d'origine (nom libre : "x 2" si l'ancien nom est pris).
fn restore_one(content: &Path) -> Result<PathBuf, String> {
    let info_file = info_path(content).ok_or("element invalide")?;
    let info = read_info(&info_file).ok_or("informations illisibles")?;
    let dir = info.original.parent().ok_or("emplacement inconnu")?.to_path_buf();
    if !dir.exists() {
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    }
    let name = info.original.file_name().map(|n| n.to_string_lossy().into_owned()).ok_or("nom inconnu")?;
    let target = ops::unique_path(&dir, &name);
    ops::move_path(content, &target).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(info_file);
    Ok(target)
}

impl Finder {
    pub fn in_trash(&self) -> bool {
        self.cwd.to_str() == Some(TRASH)
    }

    /// "Remettre a sa place" pour les elements choisis.
    pub fn put_back(&mut self, cx: &mut Context<Self>) {
        if !self.in_trash() {
            return;
        }
        let items = self.selected_paths();
        let (mut ok, mut errors) = (0, Vec::new());
        for p in &items {
            match restore_one(p) {
                Ok(_) => ok += 1,
                Err(e) => errors.push(e),
            }
        }
        self.reload_select(&[]);
        if let Some(e) = errors.first() {
            self.show_notice(format!("Restauration incompl\u{e8}te \u{2014} {e}"), cx);
        } else if ok > 0 {
            self.show_notice(format!("{ok} \u{e9}l\u{e9}ment(s) remis \u{e0} leur place"), cx);
        }
        cx.notify();
    }

    /// Supprime definitivement tout le contenu de la corbeille (apres confirmation dans l'interface).
    pub fn empty_trash(&mut self, cx: &mut Context<Self>) {
        let mut removed = 0;
        for bin in bins() {
            let Ok(read) = std::fs::read_dir(&bin) else { continue };
            for e in read.flatten() {
                let p = e.path();
                let fname = e.file_name().to_string_lossy().into_owned();
                if !(fname.starts_with("$R") || fname.starts_with("$I")) {
                    continue;
                }
                let done = if p.is_dir() { std::fs::remove_dir_all(&p) } else { std::fs::remove_file(&p) };
                if done.is_ok() && fname.starts_with("$R") {
                    removed += 1;
                }
            }
        }
        if self.in_trash() {
            self.reload_select(&[]);
        }
        self.show_notice(format!("Corbeille vid\u{e9}e ({removed} \u{e9}l\u{e9}ment(s) supprim\u{e9}(s))"), cx);
        cx.notify();
    }

    pub fn ask_empty_trash(&mut self, cx: &mut Context<Self>) {
        self.confirm = Some(crate::uninstall::Confirm { name: String::new(), empty_trash: true });
        cx.notify();
    }
}
