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
    pub created: Option<SystemTime>,
    /// Niveau d'imbrication en vue Liste hierarchique (0 = dossier courant).
    pub depth: u16,
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

pub fn kind_label(path: &Path) -> SharedString {
    kind_of(path, false)
}

fn kind_of(path: &Path, is_dir: bool) -> SharedString {
    if is_dir {
        return crate::i18n::t("Dossier").into();
    }
    match extension(path).as_str() {
        "" => crate::i18n::t("Document").into(),
        ext => crate::i18n::tf(crate::i18n::t("Fichier {0}"), &[("0", (ext.to_uppercase()).to_string())]).into(),
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
        Some(APPS) => crate::i18n::t("Applications").into(),
        Some(PC) => "Ce PC".into(),
        Some("::recents") => crate::i18n::t("R\u{e9}cents").into(),
        Some("::trash") => crate::i18n::t("Corbeille").into(),
        Some("::net") => crate::i18n::t("R\u{e9}seau").into(),
        Some(s) if s.starts_with("::tag:") => s[6..].to_string(),
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
    if path == Path::new(crate::recents::RECENTS) {
        return crate::recents::entries();
    }
    if path == Path::new(crate::netplaces::NET) {
        return crate::netplaces::entries();
    }
    if path == Path::new(crate::trash::TRASH) {
        return crate::trash::entries();
    }
    if let Some(tag) = path.to_str().and_then(|s| s.strip_prefix("::tag:")) {
        return crate::tags::entries(tag);
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
                created: meta.created().ok(),
                depth: 0,
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
    Created,
}

/// Tri courant (cle * 2 + descendant), partage par toutes les lectures de dossier.
static SORT: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

pub fn sort() -> (SortKey, bool) {
    let v = SORT.load(std::sync::atomic::Ordering::Relaxed);
    let key = match v / 2 {
        1 => SortKey::Modified,
        2 => SortKey::Size,
        3 => SortKey::Kind,
        4 => SortKey::Created,
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
        SortKey::Created => 4,
    };
    SORT.store(k * 2 + u8::from(!ascending), std::sync::atomic::Ordering::Relaxed);
}

/// Regroupement courant (0 aucun, 1 nom, 2 type, 3 modification, 4 creation, 5 taille).
static GROUP: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

pub fn group() -> u8 {
    GROUP.load(std::sync::atomic::Ordering::Relaxed)
}

pub fn set_group(g: u8) {
    GROUP.store(g, std::sync::atomic::Ordering::Relaxed);
}

fn date_bucket(t: Option<SystemTime>) -> (i64, String) {
    let Some(t) = t else { return (9, crate::i18n::t("Inconnu").into()) };
    let d = chrono::DateTime::<chrono::Local>::from(t).date_naive();
    let today = chrono::Local::now().date_naive();
    let days = (today - d).num_days();
    match days {
        i64::MIN..=0 => (0, crate::i18n::t("Aujourd'hui").into()),
        1 => (1, crate::i18n::t("Hier").into()),
        2..=7 => (2, crate::i18n::t("7 derniers jours").into()),
        8..=30 => (3, crate::i18n::t("30 derniers jours").into()),
        _ => (4, crate::i18n::t("Plus ancien").into()),
    }
}

/// (ordre, titre) du groupe d'un element pour le regroupement `g`.
pub fn group_of(e: &Entry, g: u8) -> (i64, String) {
    match g {
        1 => {
            let c = e.name.chars().next().map(|c| c.to_uppercase().next().unwrap_or(c)).unwrap_or('#');
            if c.is_alphanumeric() { (c as i64, c.to_string()) } else { (0, "#".into()) }
        }
        2 => (0, e.kind.to_string()),
        3 => date_bucket(e.modified),
        4 => date_bucket(e.created),
        5 => {
            if e.is_dir {
                (0, crate::i18n::t("Dossiers").into())
            } else {
                match e.size {
                    0 => (1, crate::i18n::t("Vide").into()),
                    1..=99_999 => (2, crate::i18n::t("Moins de 100 Ko").into()),
                    100_000..=999_999 => (3, format!("100 {} \u{2013} 1 {}", crate::i18n::unit("Ko"), crate::i18n::unit("Mo")).into()),
                    1_000_000..=9_999_999 => (4, format!("1 \u{2013} 10 {}", crate::i18n::unit("Mo")).into()),
                    10_000_000..=99_999_999 => (5, format!("10 \u{2013} 100 {}", crate::i18n::unit("Mo")).into()),
                    100_000_000..=999_999_999 => (6, format!("100 {} \u{2013} 1 {}", crate::i18n::unit("Mo"), crate::i18n::unit("Go")).into()),
                    _ => (7, crate::i18n::t("Plus de 1 Go").into()),
                }
            }
        }
        _ => (0, String::new()),
    }
}

/// Trie selon le tri courant : dossiers d'abord, puis la cle choisie, puis le nom.
pub fn resort(entries: &mut [Entry]) {
    let (key, asc) = sort();
    let grp = group();
    entries.sort_by(|a, b| {
        if grp != 0 {
            let (ka, kb) = (group_of(a, grp), group_of(b, grp));
            if ka != kb {
                return ka.cmp(&kb);
            }
        }
        let by_name = || a.name.to_lowercase().cmp(&b.name.to_lowercase());
        let ord = match key {
            SortKey::Name => by_name(),
            SortKey::Modified => a.modified.cmp(&b.modified).then_with(by_name),
            SortKey::Created => a.created.cmp(&b.created).then_with(by_name),
            SortKey::Size => a.size.cmp(&b.size).then_with(by_name),
            SortKey::Kind => a.kind.to_lowercase().cmp(&b.kind.to_lowercase()).then_with(by_name),
        };
        b.is_dir.cmp(&a.is_dir).then(if asc { ord } else { ord.reverse() })
    });
}

/// Format decimal comme le Finder (1 Ko = 1000 octets).
pub fn format_size(entry: &Entry) -> String {
    if entry.is_dir || entry.kind.as_ref() == crate::i18n::t("Application") {
        return "--".into();
    }
    let n = entry.size as f64;
    if n < 1000.0 {
        crate::i18n::tf(crate::i18n::t("{0} octets"), &[("0", (entry.size).to_string())])
    } else if n < 1e6 {
        crate::i18n::size(n / 1e3, 0, 1)
    } else if n < 1e9 {
        crate::i18n::size(n / 1e6, 1, 1)
    } else {
        crate::i18n::size(n / 1e9, 2, 2)
    }
}

pub fn format_date(t: Option<SystemTime>) -> String {
    match t {
        Some(t) => chrono::DateTime::<chrono::Local>::from(t).format(crate::i18n::date_format()).to_string(),
        None => "--".into(),
    }
}

/// Date relative comme le Finder ("Aujourd'hui a 14:03", "Hier a ...") ou complete.
pub fn format_date_pref(t: Option<SystemTime>, relative: bool) -> String {
    let Some(t) = t else { return "--".into() };
    let d = chrono::DateTime::<chrono::Local>::from(t);
    if relative {
        let today = chrono::Local::now().date_naive();
        if d.date_naive() == today {
            return format!("{} {}", crate::i18n::t("Aujourd'hui"), d.format(crate::i18n::at_time_format()));
        }
        if d.date_naive() == today.pred_opt().unwrap_or(today) {
            return format!("{} {}", crate::i18n::t("Hier"), d.format(crate::i18n::at_time_format()));
        }
    }
    d.format(crate::i18n::date_format()).to_string()
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
/// Filtre de type de la recherche : 0 tous, 1 images, 2 documents, 3 dossiers, 4 audio / video.
fn kind_matches(kind: u8, path: &Path, is_dir: bool) -> bool {
    let ext = extension(path);
    match kind {
        1 => !is_dir && matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "svg" | "heic" | "tiff"),
        2 => !is_dir && matches!(ext.as_str(), "pdf" | "doc" | "docx" | "txt" | "md" | "rtf" | "odt" | "xls" | "xlsx" | "csv" | "ppt" | "pptx"),
        3 => is_dir,
        4 => !is_dir && matches!(ext.as_str(), "mp3" | "wav" | "flac" | "m4a" | "aac" | "ogg" | "mp4" | "mkv" | "mov" | "avi" | "webm" | "wmv" | "m4v"),
        _ => true,
    }
}

/// Recherche recursive par nom (et, si `content`, dans le texte des fichiers de moins de 2 Mo).
pub fn search(root: &Path, query: &str, show_hidden: bool, max_results: usize, content: bool, kind: u8) -> Vec<Entry> {
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
            let by_name = name.to_lowercase().contains(&needle);
            let by_content = !by_name
                && content
                && !ft.is_dir()
                && matches!(extension(&path).as_str(), "txt" | "md" | "rs" | "toml" | "json" | "yaml" | "yml" | "js" | "ts" | "tsx" | "css" | "html" | "xml" | "py" | "c" | "h" | "cpp" | "log" | "csv" | "ini" | "sh" | "ps1")
                && e.metadata().map(|m| m.len() <= 2_000_000).unwrap_or(false)
                && std::fs::read(&path).map(|b| String::from_utf8_lossy(&b).to_lowercase().contains(&needle)).unwrap_or(false);
            if (by_name || by_content) && kind_matches(kind, &path, ft.is_dir()) {
                let Ok(meta) = e.metadata() else { continue };
                let rel = dir.strip_prefix(root).ok().map(|p| p.display().to_string()).filter(|s| !s.is_empty());
                results.push(Entry {
                    kind: kind_of(&path, ft.is_dir()),
                    path,
                    name: name.into(),
                    is_dir: ft.is_dir(),
                    size: if ft.is_dir() { 0 } else { meta.len() },
                    modified: meta.modified().ok(),
                created: meta.created().ok(),
                depth: 0,
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

impl Entry {
    pub fn is_pdf(&self) -> bool {
        extension(&self.path) == "pdf"
    }

    pub fn is_video(&self) -> bool {
        matches!(extension(&self.path).as_str(), "mp4" | "mkv" | "mov" | "avi" | "webm" | "wmv" | "m4v")
    }
}
