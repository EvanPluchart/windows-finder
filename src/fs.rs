use std::path::{Path, PathBuf};
use std::time::SystemTime;

use gpui::SharedString;

#[derive(Clone)]
pub struct Entry {
    pub name: SharedString,
    pub path: PathBuf,
    pub is_dir: bool,
    pub size: u64,
    pub modified: Option<SystemTime>,
    pub kind: SharedString,
    /// Dossier d'origine (resultats de recherche seulement).
    pub hint: Option<SharedString>,
}

impl Entry {
    pub fn glyph(&self) -> &'static str {
        if self.is_dir {
            return "\u{1F4C1}";
        }
        match extension(&self.path).as_str() {
            "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" => "\u{1F5BC}",
            "pdf" => "\u{1F4D5}",
            "mp3" | "wav" | "flac" | "m4a" => "\u{1F3B5}",
            "mp4" | "mov" | "mkv" | "avi" => "\u{1F39E}",
            "zip" | "7z" | "rar" | "tar" | "gz" => "\u{1F5DC}",
            _ => "\u{1F4C4}",
        }
    }

    pub fn is_image(&self) -> bool {
        matches!(extension(&self.path).as_str(), "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp")
    }

    pub fn is_text(&self) -> bool {
        matches!(
            extension(&self.path).as_str(),
            "txt" | "md" | "rs" | "toml" | "json" | "yaml" | "yml" | "js" | "ts" | "tsx" | "css" | "html" | "xml"
                | "py" | "c" | "h" | "cpp" | "log" | "csv" | "ini" | "sh" | "ps1" | "lock"
        )
    }
}

pub fn extension(path: &Path) -> String {
    path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default()
}

fn kind_of(path: &Path, is_dir: bool) -> SharedString {
    if is_dir {
        return "Dossier".into();
    }
    match extension(path).as_str() {
        "" => "Document".into(),
        ext => format!("Fichier {}", ext.to_uppercase()).into(),
    }
}

/// Emplacements virtuels (pas de vrai dossier derriere) : programmes installes et disques.
pub const APPS: &str = "::apps";
pub const PC: &str = "::pc";

pub fn is_virtual(p: &Path) -> bool {
    p.to_str().map(|s| s.starts_with("::")).unwrap_or(false)
}

/// Nom affiche pour un emplacement (titre de fenetre, chemin).
pub fn display_name(p: &Path) -> String {
    match p.to_str() {
        Some(APPS) => "Applications".into(),
        Some(PC) => "Ce PC".into(),
        _ => p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| p.display().to_string()),
    }
}

/// Lit un dossier : dossiers d'abord, puis tri alphabetique insensible a la casse.
pub fn read_dir(path: &Path, show_hidden: bool) -> Vec<Entry> {
    if path == Path::new(APPS) {
        return crate::apps::entries();
    }
    if path == Path::new(PC) {
        return crate::apps::drives();
    }
    let mut entries: Vec<Entry> = std::fs::read_dir(path)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            if !show_hidden && name.starts_with('.') {
                return None;
            }
            let mut meta = e.metadata().ok()?;
            // Comme l'Explorateur : les elements caches ou systeme (jonctions "Mes videos", NTUSER.DAT,
            // desktop.ini...) ne s'affichent qu'avec "afficher les fichiers caches".
            use std::os::windows::fs::MetadataExt;
            if !show_hidden && meta.file_attributes() & 0x6 != 0 {
                return None;
            }
            // Liens et jonctions : on decrit leur cible (dossier ou fichier), si elle est accessible.
            if meta.file_type().is_symlink() || meta.file_attributes() & 0x400 != 0 {
                meta = std::fs::metadata(e.path()).ok()?;
            }
            let is_dir = meta.is_dir();
            Some(Entry {
                kind: kind_of(&e.path(), is_dir),
                path: e.path(),
                name: name.into(),
                is_dir,
                size: if is_dir { 0 } else { meta.len() },
                modified: meta.modified().ok(),
                hint: None,
            })
        })
        .collect();
    resort(&mut entries);
    entries
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SortKey {
    Name,
    Modified,
    Size,
    Kind,
}

/// Tri courant (cle * 2 + descendant), partage par toutes les lectures de dossier.
static SORT: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

pub fn sort() -> (SortKey, bool) {
    let v = SORT.load(std::sync::atomic::Ordering::Relaxed);
    let key = match v / 2 {
        1 => SortKey::Modified,
        2 => SortKey::Size,
        3 => SortKey::Kind,
        _ => SortKey::Name,
    };
    (key, v % 2 == 0)
}

pub fn set_sort(key: SortKey, ascending: bool) {
    let k = match key {
        SortKey::Name => 0,
        SortKey::Modified => 1,
        SortKey::Size => 2,
        SortKey::Kind => 3,
    };
    SORT.store(k * 2 + u8::from(!ascending), std::sync::atomic::Ordering::Relaxed);
}

/// Trie selon le tri courant : dossiers d'abord, puis la cle choisie, puis le nom.
pub fn resort(entries: &mut [Entry]) {
    let (key, asc) = sort();
    entries.sort_by(|a, b| {
        let by_name = || a.name.to_lowercase().cmp(&b.name.to_lowercase());
        let ord = match key {
            SortKey::Name => by_name(),
            SortKey::Modified => a.modified.cmp(&b.modified).then_with(by_name),
            SortKey::Size => a.size.cmp(&b.size).then_with(by_name),
            SortKey::Kind => a.kind.to_lowercase().cmp(&b.kind.to_lowercase()).then_with(by_name),
        };
        b.is_dir.cmp(&a.is_dir).then(if asc { ord } else { ord.reverse() })
    });
}

/// Format decimal comme le Finder (1 Ko = 1000 octets).
pub fn format_size(entry: &Entry) -> String {
    if entry.is_dir || entry.kind.as_ref() == "Application" {
        return "--".into();
    }
    let n = entry.size as f64;
    if n < 1000.0 {
        format!("{} octets", entry.size)
    } else if n < 1e6 {
        format!("{:.1} Ko", n / 1e3)
    } else if n < 1e9 {
        format!("{:.1} Mo", n / 1e6)
    } else {
        format!("{:.2} Go", n / 1e9)
    }
}

pub fn format_date(t: Option<SystemTime>) -> String {
    match t {
        Some(t) => chrono::DateTime::<chrono::Local>::from(t).format("%d/%m/%Y %H:%M").to_string(),
        None => "--".into(),
    }
}

pub fn read_text_preview(path: &Path) -> String {
    use std::io::Read;
    let mut buf = vec![0u8; 8192];
    let n = std::fs::File::open(path).and_then(|mut f| f.read(&mut buf)).unwrap_or(0);
    buf.truncate(n);
    String::from_utf8_lossy(&buf).into_owned()
}

pub fn home_dir() -> PathBuf {
    std::env::var_os("USERPROFILE").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."))
}

/// Recherche recursive par nom (sous-chaine, insensible a la casse) dans `root`.
/// Ignore les liens symboliques (pas de boucle), plafonne le nombre de resultats et de dossiers visites.
pub fn search(root: &Path, query: &str, show_hidden: bool, max_results: usize) -> Vec<Entry> {
    let needle = query.to_lowercase();
    let mut results: Vec<Entry> = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    let mut visited = 0usize;
    while let Some(dir) = stack.pop() {
        visited += 1;
        if visited > 100_000 || results.len() >= max_results {
            break;
        }
        let Ok(read) = std::fs::read_dir(&dir) else { continue };
        for e in read.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if !show_hidden && name.starts_with('.') {
                continue;
            }
            let Ok(ft) = e.file_type() else { continue };
            if ft.is_symlink() {
                continue;
            }
            let path = e.path();
            if ft.is_dir() {
                stack.push(path.clone());
            }
            if name.to_lowercase().contains(&needle) {
                let Ok(meta) = e.metadata() else { continue };
                let rel = dir.strip_prefix(root).ok().map(|p| p.display().to_string()).filter(|s| !s.is_empty());
                results.push(Entry {
                    kind: kind_of(&path, ft.is_dir()),
                    path,
                    name: name.into(),
                    is_dir: ft.is_dir(),
                    size: if ft.is_dir() { 0 } else { meta.len() },
                    modified: meta.modified().ok(),
                    hint: rel.map(Into::into),
                });
                if results.len() >= max_results {
                    break;
                }
            }
        }
    }
    results.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    results
}
