//! Placement libre des icones (vue Icones) : position de chaque element par dossier, conservee entre les lancements.
//! Fichier texte `%APPDATA%\winfinder\layouts.tsv` : `dossier <TAB> nom <TAB> x <TAB> y`.
//! Aussi : reglages simples `cle=valeur` (settings.txt), gardes en memoire.
//!
//! Plusieurs fenetres (et plusieurs processus, un par dossier ouvert depuis l'Explorateur) partagent ces fichiers :
//! chaque ecriture relit le fichier, ne change que sa propre entree, puis l'ecrit d'un coup (ops::write_atomic).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime};

pub type Layout = HashMap<String, (f32, f32)>;
pub type Layouts = HashMap<PathBuf, Layout>;

pub const CELL_W: f32 = 128.0;
pub const PAD: f32 = 12.0;

fn dir() -> Option<PathBuf> {
    Some(PathBuf::from(std::env::var_os("APPDATA")?).join("winfinder"))
}

fn file() -> Option<PathBuf> {
    Some(dir()?.join("layouts.tsv"))
}

/// Ecrit un fichier de reglages (dossier cree au besoin). Une erreur est sans consequence : rien n'est tronque.
pub fn write_settings_file(f: &Path, text: &str) {
    if let Some(parent) = f.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = crate::ops::write_atomic(f, text.as_bytes());
}

/// Position plausible : un fichier modifie a la main ne doit pas envoyer NaN ou l'infini a la mise en page.
fn coord(v: &str) -> Option<f32> {
    v.parse::<f32>().ok().filter(|v| v.is_finite()).map(|v| v.clamp(0.0, 1_000_000.0))
}

pub fn load() -> Layouts {
    let mut all = Layouts::new();
    let Some(text) = file().and_then(|f| std::fs::read_to_string(f).ok()) else { return all };
    for line in text.trim_start_matches('\u{feff}').lines() {
        let mut p = line.split('\t');
        let (Some(dir), Some(name), Some(x), Some(y)) = (p.next(), p.next(), p.next(), p.next()) else { continue };
        let (Some(x), Some(y)) = (coord(x), coord(y)) else { continue };
        all.entry(PathBuf::from(dir)).or_default().insert(name.to_string(), (x, y));
    }
    all
}

fn encode(all: &Layouts) -> String {
    let mut out = String::new();
    for (dir, layout) in all {
        for (name, (x, y)) in layout {
            out.push_str(&format!("{}\t{}\t{}\t{}\n", dir.display(), name, x, y));
        }
    }
    out
}

/// Enregistre (ou retire avec `None`) les positions d'un seul dossier, sans toucher aux autres.
pub fn save_dir(dir: &Path, layout: Option<&Layout>) {
    let Some(f) = file() else { return };
    let mut all = load();
    match layout {
        Some(l) => {
            all.insert(dir.to_path_buf(), l.clone());
        }
        None => {
            all.remove(dir);
        }
    }
    write_settings_file(&f, &encode(&all));
}

/// Oublie toutes les positions (reinitialisation des options d'affichage).
pub fn clear_all() {
    if let Some(f) = file() {
        write_settings_file(&f, "");
    }
}

// ---- Reglages : settings.txt ----

fn settings_file() -> Option<PathBuf> {
    Some(dir()?.join("settings.txt"))
}

/// Copie en memoire de settings.txt. La barre d'outils lit une dizaine de reglages a chaque image affichee :
/// on ne relit le fichier que s'il a change (date verifiee au plus deux fois par seconde).
struct Cache {
    lines: Vec<(String, String)>,
    stamp: Option<SystemTime>,
    checked: Option<Instant>,
}

static SETTINGS: Mutex<Cache> = Mutex::new(Cache { lines: Vec::new(), stamp: None, checked: None });

fn read_settings(f: &Path) -> Vec<(String, String)> {
    std::fs::read_to_string(f)
        .unwrap_or_default()
        .trim_start_matches('\u{feff}')
        .lines()
        .filter_map(|l| l.split_once('=').map(|(k, v)| (k.to_string(), v.trim().to_string())))
        .collect()
}

fn mtime(f: &Path) -> Option<SystemTime> {
    std::fs::metadata(f).and_then(|m| m.modified()).ok()
}

pub fn load_view() -> Option<String> {
    get_setting("view")
}

/// Reglages simples `cle=valeur` dans settings.txt (conserves entre les lancements).
pub fn get_setting(key: &str) -> Option<String> {
    let f = settings_file()?;
    let mut c = SETTINGS.lock().unwrap_or_else(|e| e.into_inner());
    if c.checked.is_none_or(|t| t.elapsed() > Duration::from_millis(500)) {
        let stamp = mtime(&f);
        if c.checked.is_none() || stamp != c.stamp {
            c.lines = read_settings(&f);
            c.stamp = stamp;
        }
        c.checked = Some(Instant::now());
    }
    c.lines.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone())
}

pub fn set_setting(key: &str, value: &str) {
    let Some(f) = settings_file() else { return };
    let mut c = SETTINGS.lock().unwrap_or_else(|e| e.into_inner());
    // Relu du disque : un autre processus a pu changer d'autres reglages entre-temps.
    let mut lines = read_settings(&f);
    if lines.iter().any(|(k, v)| k == key && v == value) {
        return;
    }
    lines.retain(|(k, _)| k != key);
    lines.push((key.to_string(), value.to_string()));
    let text: String = lines.iter().map(|(k, v)| format!("{k}={v}\n")).collect();
    write_settings_file(&f, &text);
    c.lines = lines;
    c.stamp = mtime(&f);
    c.checked = Some(Instant::now());
}
