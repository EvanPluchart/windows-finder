//! Placement libre des icones (vue Icones) : position de chaque element par dossier, conservee entre les lancements.
//! Fichier texte `%APPDATA%\winfinder\layouts.tsv` : `dossier <TAB> nom <TAB> x <TAB> y`.

use std::collections::HashMap;
use std::path::PathBuf;

pub type Layout = HashMap<String, (f32, f32)>;
pub type Layouts = HashMap<PathBuf, Layout>;

pub const CELL_W: f32 = 128.0;
pub const CELL_H: f32 = 134.0;
pub const PAD: f32 = 12.0;

fn file() -> Option<PathBuf> {
    Some(PathBuf::from(std::env::var_os("APPDATA")?).join("winfinder").join("layouts.tsv"))
}

pub fn load() -> Layouts {
    let mut all = Layouts::new();
    let Some(text) = file().and_then(|f| std::fs::read_to_string(f).ok()) else { return all };
    for line in text.lines() {
        let mut p = line.split('\t');
        let (Some(dir), Some(name), Some(x), Some(y)) = (p.next(), p.next(), p.next(), p.next()) else { continue };
        let (Ok(x), Ok(y)) = (x.parse::<f32>(), y.parse::<f32>()) else { continue };
        all.entry(PathBuf::from(dir)).or_default().insert(name.to_string(), (x, y));
    }
    all
}

pub fn save(all: &Layouts) {
    let Some(f) = file() else { return };
    if let Some(parent) = f.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let mut out = String::new();
    for (dir, layout) in all {
        for (name, (x, y)) in layout {
            out.push_str(&format!("{}\t{}\t{}\t{}\n", dir.display(), name, x, y));
        }
    }
    let _ = std::fs::write(f, out);
}

// ---- Reglages : mode d'affichage choisi (conserve entre les lancements) ----

fn settings_file() -> Option<PathBuf> {
    Some(PathBuf::from(std::env::var_os("APPDATA")?).join("winfinder").join("settings.txt"))
}

pub fn load_view() -> Option<String> {
    let text = std::fs::read_to_string(settings_file()?).ok()?;
    text.lines().find_map(|l| l.strip_prefix("view=").map(|v| v.trim().to_string()))
}

pub fn save_view(name: &str) {
    let Some(f) = settings_file() else { return };
    if let Some(parent) = f.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(f, format!("view={name}\n"));
}

/// Reglages simples `cle=valeur` dans settings.txt (conserves entre les lancements).
pub fn get_setting(key: &str) -> Option<String> {
    let text = std::fs::read_to_string(settings_file()?).ok()?;
    let prefix = format!("{key}=");
    text.lines().find_map(|l| l.strip_prefix(prefix.as_str()).map(|v| v.trim().to_string()))
}

pub fn set_setting(key: &str, value: &str) {
    let Some(f) = settings_file() else { return };
    if let Some(parent) = f.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let prefix = format!("{key}=");
    let mut lines: Vec<String> =
        std::fs::read_to_string(&f).unwrap_or_default().lines().filter(|l| !l.starts_with(prefix.as_str())).map(str::to_owned).collect();
    lines.push(format!("{key}={value}"));
    let _ = std::fs::write(f, lines.join("\n") + "\n");
}
