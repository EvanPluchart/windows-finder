//! Integration a Windows : gestionnaire de dossiers par defaut et entree du menu contextuel de l'Explorateur.
//! Tout est ecrit sous HKEY_CURRENT_USER (aucun droit administrateur), et reversible.

use std::os::windows::process::CommandExt;
use std::process::Command;

const DIR: &str = r"HKCU\Software\Classes\Directory\shell";
const DRIVE: &str = r"HKCU\Software\Classes\Drive\shell";
const BG: &str = r"HKCU\Software\Classes\Directory\Background\shell";

fn reg(args: &[&str]) -> Option<String> {
    let out = Command::new("reg").args(args).creation_flags(0x0800_0000).output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

fn exe() -> String {
    std::env::current_exe().map(|p| p.display().to_string()).unwrap_or_default()
}

fn command_line(arg: &str) -> String {
    format!("\"{}\" \"{}\"", exe(), arg)
}

/// Vrai si ouvrir un dossier (double-clic depuis un autre programme, « Afficher dans le dossier »...) lance Windows Finder.
pub fn is_default() -> bool {
    reg(&["query", &format!(r"{DIR}\open\command"), "/ve"])
        .map(|o| o.to_lowercase().contains("windowsfinder.exe"))
        .unwrap_or(false)
}

pub fn set_default(on: bool) -> bool {
    if on {
        let cmd = command_line("%1");
        let a = reg(&["add", &format!(r"{DIR}\open\command"), "/ve", "/d", &cmd, "/f"]).is_some();
        let b = reg(&["add", &format!(r"{DRIVE}\open\command"), "/ve", "/d", &cmd, "/f"]).is_some();
        let c = reg(&["add", DIR, "/ve", "/d", "open", "/f"]).is_some();
        a && b && c
    } else {
        // Supprimer nos cles remet les valeurs systeme (HKLM) en vigueur.
        let _ = reg(&["delete", &format!(r"{DIR}\open"), "/f"]);
        let _ = reg(&["delete", &format!(r"{DRIVE}\open"), "/f"]);
        let _ = reg(&["delete", DIR, "/ve", "/f"]);
        true
    }
}

/// Vrai si « Ouvrir dans Windows Finder » est dans le menu contextuel des dossiers.
pub fn context_entry() -> bool {
    reg(&["query", &format!(r"{DIR}\WindowsFinder")]).is_some()
}

pub fn set_context_entry(on: bool) -> bool {
    let key_dir = format!(r"{DIR}\WindowsFinder");
    let key_bg = format!(r"{BG}\WindowsFinder");
    if on {
        let label = crate::i18n::t("Ouvrir dans Windows Finder");
        let mut ok = true;
        for (key, arg) in [(&key_dir, "%1"), (&key_bg, "%V")] {
            ok &= reg(&["add", key, "/ve", "/d", label, "/f"]).is_some();
            ok &= reg(&["add", key, "/v", "Icon", "/d", &exe(), "/f"]).is_some();
            ok &= reg(&["add", &format!(r"{key}\command"), "/ve", "/d", &command_line(arg), "/f"]).is_some();
        }
        ok
    } else {
        let _ = reg(&["delete", &key_dir, "/f"]);
        let _ = reg(&["delete", &key_bg, "/f"]);
        true
    }
}

/// Vrai si la commande de la cle lance cet executable-ci (pas une autre copie, portable par exemple).
fn points_here(key: &str) -> bool {
    let me = exe().to_lowercase();
    !me.is_empty() && reg(&["query", key, "/ve"]).map(|o| o.to_lowercase().contains(&me)).unwrap_or(false)
}

/// Desinstallation : retire le gestionnaire par defaut et l'entree du menu contextuel s'ils pointent ici.
pub fn unregister() {
    if points_here(&format!(r"{DIR}\open\command")) {
        set_default(false);
    }
    if points_here(&format!(r"{DIR}\WindowsFinder\command")) {
        set_context_entry(false);
    }
}
