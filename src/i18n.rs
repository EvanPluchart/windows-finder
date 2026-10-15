//! Langues de l'interface : francais (langue des textes du code), anglais, espagnol, allemand.
//! La langue suit celle de Windows par defaut ; le dictionnaire (assets/i18n.txt) associe chaque texte francais
//! a ses traductions, et `t` / `tf` sont appeles a l'affichage, donc le changement de langue est immediat.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::OnceLock;

use crate::layout;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Fr = 0,
    En = 1,
    Es = 2,
    De = 3,
}

pub const LANGS: [(Lang, &str, &str); 4] =
    [(Lang::Fr, "fr", "Fran\u{e7}ais"), (Lang::En, "en", "English"), (Lang::Es, "es", "Espa\u{f1}ol"), (Lang::De, "de", "Deutsch")];

static CURRENT: AtomicU8 = AtomicU8::new(0);

fn from_u8(v: u8) -> Lang {
    match v {
        1 => Lang::En,
        2 => Lang::Es,
        3 => Lang::De,
        _ => Lang::Fr,
    }
}

pub fn current() -> Lang {
    from_u8(CURRENT.load(Ordering::Relaxed))
}

pub fn set(l: Lang) {
    CURRENT.store(l as u8, Ordering::Relaxed);
}

/// Langue d'affichage de Windows (anglais si elle n'est pas geree).
pub fn system() -> Lang {
    use windows::Win32::Globalization::GetUserDefaultLocaleName;
    let mut buf = [0u16; 85];
    let n = unsafe { GetUserDefaultLocaleName(&mut buf) };
    let name = String::from_utf16_lossy(&buf[..(n.max(1) as usize - 1).min(85)]).to_lowercase();
    match name.get(..2) {
        Some("fr") => Lang::Fr,
        Some("es") => Lang::Es,
        Some("de") => Lang::De,
        _ => Lang::En,
    }
}

/// Reglage `lang` : "auto" (ou absent) = langue de Windows.
pub fn resolve(setting: Option<&str>) -> Lang {
    LANGS.iter().find(|(_, code, _)| Some(*code) == setting).map(|(l, _, _)| *l).unwrap_or_else(system)
}

/// Applique le reglage enregistre.
pub fn apply() {
    set(resolve(layout::get_setting("lang").as_deref()));
}

type Table = HashMap<&'static str, [&'static str; 3]>;

fn table() -> &'static Table {
    static T: OnceLock<Table> = OnceLock::new();
    T.get_or_init(|| {
        let mut m = Table::new();
        for line in include_str!("../assets/i18n.txt").lines() {
            let parts: Vec<&'static str> = line.split(" ::: ").collect();
            if parts.len() == 4 {
                m.insert(parts[0], [parts[1], parts[2], parts[3]]);
            }
        }
        m
    })
}

/// Traduit un texte francais du code ; renvoie le texte tel quel s'il n'a pas de traduction.
pub fn t(s: &'static str) -> &'static str {
    match current() {
        Lang::Fr => s,
        l => table().get(s).map(|v| v[l as usize - 1]).unwrap_or(s),
    }
}

/// Comme `t` pour un texte calcule (nom de dossier, etiquette...).
pub fn td(s: &str) -> String {
    match current() {
        Lang::Fr => s.to_string(),
        l => table().get(s).map(|v| v[l as usize - 1].to_string()).unwrap_or_else(|| s.to_string()),
    }
}

/// Remplace les `{nom}` du modele par leurs valeurs.
pub fn tf(template: &str, vals: &[(&str, String)]) -> String {
    let mut out = template.to_string();
    for (k, v) in vals {
        out = out.replace(&format!("{{{k}}}"), v);
    }
    out
}

/// Format de date complet selon la langue.
pub fn date_format() -> &'static str {
    match current() {
        Lang::En => "%m/%d/%Y %I:%M %p",
        Lang::De => "%d.%m.%Y %H:%M",
        _ => "%d/%m/%Y %H:%M",
    }
}

/// Fin des dates relatives : "<Hier> a 14:03".
pub fn at_time_format() -> &'static str {
    match current() {
        Lang::En => "at %I:%M %p",
        Lang::Es => "a las %H:%M",
        Lang::De => "um %H:%M",
        _ => "\u{e0} %H:%M",
    }
}

/// "12,3 Mo" / "12.3 MB" : `v` deja divise, `unit` 0 = Ko, 1 = Mo, 2 = Go, `decimals` chiffres apres la virgule.
pub fn size(v: f64, unit: usize, decimals: usize) -> String {
    let units = if current() == Lang::Fr { ["Ko", "Mo", "Go"] } else { ["KB", "MB", "GB"] };
    let s = format!("{v:.decimals$}");
    let s = if current() == Lang::En { s } else { s.replace('.', ",") };
    format!("{s} {}", units[unit])
}

/// Nom de l'unite pour les libelles fixes ("Ko", "Mo", "Go").
pub fn unit(u: &'static str) -> &'static str {
    if current() == Lang::Fr {
        return u;
    }
    match u {
        "Ko" => "KB",
        "Mo" => "MB",
        _ => "GB",
    }
}
