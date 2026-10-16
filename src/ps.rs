//! Lancement sur de PowerShell et des outils systeme.
//!
//! Les chemins et les noms ne sont jamais ecrits dans le texte d'un script : ils passent par des variables
//! d'environnement (`$env:WF_SRC`...). Doubler `'` ne suffit pas : PowerShell traite aussi U+2018 a U+201B
//! comme des apostrophes, et un nom comme `photo’;calc;’.zip` ferait executer `calc`.
//! Les programmes sont appeles par leur chemin complet dans System32, jamais cherches ailleurs.

use std::ffi::{OsStr, OsString};
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Pas de console pour les outils en ligne de commande lances par l'application.
pub const NO_WINDOW: u32 = 0x0800_0000;

/// Chemin complet d'un programme de `%SystemRoot%\System32`.
pub fn system32(exe: &str) -> PathBuf {
    std::env::var_os("SystemRoot").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(r"C:\Windows")).join("System32").join(exe)
}

/// Outil systeme sans fenetre (reg, tar...), par son chemin complet.
pub fn tool(exe: &str) -> Command {
    let mut c = Command::new(system32(exe));
    c.creation_flags(NO_WINDOW);
    c
}

/// Commande PowerShell : `script` est un texte fixe, les donnees sont dans `vars` (nom, valeur).
pub fn command(script: &str, vars: &[(&str, &OsStr)]) -> Command {
    let mut c = tool(r"WindowsPowerShell\v1.0\powershell.exe");
    c.args(["-NoProfile", "-NonInteractive", "-Command", script]);
    for (k, v) in vars {
        c.env(k, v);
    }
    c
}

/// Lance le script et attend la fin. Erreur : la premiere ligne non vide de la sortie d'erreur.
pub fn run(script: &str, vars: &[(&str, &OsStr)]) -> Result<String, String> {
    let out = command(script, vars).output().map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        let err = String::from_utf8_lossy(&out.stderr);
        Err(err.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("erreur").to_string())
    }
}

/// Plusieurs chemins dans une seule variable, un par ligne (un nom de fichier ne contient jamais de saut de
/// ligne). Cote script : `$env:WF_LIST -split "`n"`.
pub fn list(paths: &[PathBuf]) -> OsString {
    let mut s = OsString::new();
    for (i, p) in paths.iter().enumerate() {
        if i > 0 {
            s.push("\n");
        }
        s.push(p.as_os_str());
    }
    s
}

/// Raccourci pour passer un chemin en variable.
pub fn os(p: &Path) -> &OsStr {
    p.as_os_str()
}
