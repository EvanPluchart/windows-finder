//! Compresser / extraire des archives (PowerShell et tar de Windows, hors thread d'interface).
//! Les chemins passent par des variables d'environnement (voir ps.rs), jamais dans le texte des scripts.

use std::path::{Path, PathBuf};

use crate::{ops, ps, Finder};
use gpui::Context;

fn run_ps(script: &str, vars: &[(&str, &std::ffi::OsStr)]) -> Result<(), String> {
    ps::run(script, vars).map(|_| ())
}

const COMPRESS: &str = r#"Compress-Archive -LiteralPath ($env:WF_LIST -split "`n") -DestinationPath $env:WF_DEST -Force"#;

/// ZIP : chaque entree est verifiee avant d'etre ecrite. Une entree qui sortirait du dossier de destination
/// (`..\`, chemin absolu, lecteur) arrete l'extraction (« zip slip »).
const EXTRACT_ZIP: &str = r#"
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.IO.Compression.FileSystem
$d = [IO.Path]::GetFullPath($env:WF_DEST)
$root = $d.TrimEnd('\') + '\'
[IO.Directory]::CreateDirectory($d) | Out-Null
$z = [IO.Compression.ZipFile]::OpenRead($env:WF_SRC)
try {
  foreach ($e in $z.Entries) {
    $t = [IO.Path]::GetFullPath([IO.Path]::Combine($d, $e.FullName))
    if (-not $t.StartsWith($root, [StringComparison]::OrdinalIgnoreCase)) { throw "Entree refusee : $($e.FullName)" }
    if ($e.FullName.EndsWith('/') -or $e.FullName.EndsWith('\')) { [IO.Directory]::CreateDirectory($t) | Out-Null; continue }
    [IO.Directory]::CreateDirectory([IO.Path]::GetDirectoryName($t)) | Out-Null
    [IO.Compression.ZipFileExtensions]::ExtractToFile($e, $t, $false)
  }
} finally { $z.Dispose() }
"#;

/// 7z, rar, tar, tgz... : bsdtar de Windows 11, qui refuse deja les chemins absolus et `..`.
const EXTRACT_TAR: &str = r#"
$ErrorActionPreference = 'Stop'
[IO.Directory]::CreateDirectory($env:WF_DEST) | Out-Null
& "$env:SystemRoot\System32\tar.exe" -xf $env:WF_SRC -C $env:WF_DEST
if ($LASTEXITCODE -ne 0) { throw 'tar' }
"#;

/// Les fichiers extraits d'une archive telechargee gardent la marque « provenant d'Internet » (flux
/// Zone.Identifier), comme avec l'Explorateur : SmartScreen et Office continuent de proteger l'utilisateur.
const KEEP_ZONE: &str = r#"
$zi = Get-Content -LiteralPath $env:WF_SRC -Stream Zone.Identifier -Raw -ErrorAction SilentlyContinue
if ($zi) { Get-ChildItem -LiteralPath $env:WF_DEST -Recurse -File -Force | ForEach-Object { Set-Content -LiteralPath $_.FullName -Stream Zone.Identifier -Value $zi } }
"#;

const ALIAS: &str = r#"$s = (New-Object -ComObject WScript.Shell).CreateShortcut($env:WF_DEST); $s.TargetPath = $env:WF_SRC; $s.Save()"#;

impl Finder {
    pub fn compress_selection(&mut self, cx: &mut Context<Self>) {
        if self.read_only_place(cx) {
            return;
        }
        let paths = self.selected_paths();
        if paths.is_empty() {
            return;
        }
        let base = match paths.as_slice() {
            [one] => one.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| crate::i18n::t("Archive").into()),
            _ => crate::i18n::t("Archive").to_string(),
        };
        let dest = ops::unique_path(&self.cwd, &format!("{base}.zip"));
        let list = ps::list(&paths);
        let out = dest.clone();
        self.show_notice(crate::i18n::t("Compression en cours\u{2026}"), cx);
        cx.spawn(async move |this, cx| {
            let result = cx.background_executor().spawn(async move { run_ps(COMPRESS, &[("WF_LIST", &list), ("WF_DEST", ps::os(&out))]) }).await;
            this.update(cx, |f, cx| match result {
                Ok(()) => {
                    f.reload_select(&[dest.clone()]);
                    f.show_notice(crate::i18n::tf(crate::i18n::t("\u{ab} {0} \u{bb} cr\u{e9}\u{e9}"), &[("0", (dest.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()).to_string())]), cx);
                }
                Err(e) => f.show_notice(crate::i18n::tf(crate::i18n::t("Compression impossible \u{2014} {e}"), &[("e", (e).to_string())]), cx),
            })
            .ok();
        })
        .detach();
    }

    pub fn extract_selection(&mut self, cx: &mut Context<Self>) {
        if self.read_only_place(cx) {
            return;
        }
        let Some(zip) = self.selected_entry().filter(|e| is_archive(&e.path)).map(|e| e.path.clone()) else { return };
        let stem = zip.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| crate::i18n::t("Extrait").into());
        let dest: PathBuf = ops::unique_path(&self.cwd, &stem);
        let script = format!("{}{KEEP_ZONE}", if fs_ext(&zip) == "zip" { EXTRACT_ZIP } else { EXTRACT_TAR });
        let out = dest.clone();
        self.show_notice(crate::i18n::t("Extraction en cours\u{2026}"), cx);
        cx.spawn(async move |this, cx| {
            let result = cx.background_executor().spawn(async move { run_ps(&script, &[("WF_SRC", ps::os(&zip)), ("WF_DEST", ps::os(&out))]) }).await;
            this.update(cx, |f, cx| match result {
                Ok(()) => {
                    f.reload_select(&[dest]);
                    f.show_notice(crate::i18n::t("Archive extraite"), cx);
                }
                Err(e) => f.show_notice(crate::i18n::tf(crate::i18n::t("Extraction impossible \u{2014} {e}"), &[("e", (e).to_string())]), cx),
            })
            .ok();
        })
        .detach();
    }
}

pub fn is_archive(p: &Path) -> bool {
    matches!(crate::fs::extension(p).as_str(), "zip" | "7z" | "rar" | "tar" | "gz" | "tgz" | "bz2" | "xz" | "iso")
}

fn fs_ext(p: &Path) -> String {
    crate::fs::extension(p)
}

impl Finder {
    /// "Ouvrir avec..." : la fenetre de choix d'application de Windows.
    pub fn open_with(&mut self) {
        if let Some(e) = self.selected_entry().filter(|e| !e.is_dir) {
            crate::extras::open_with_dialog(&e.path);
        }
    }

    /// Montre l'element dans une fenetre de l'Explorateur Windows.
    pub fn reveal_in_explorer(&mut self) {
        use std::os::windows::process::CommandExt;
        if let Some(e) = self.selected_entry() {
            let arg = format!("/select,\"{}\"", e.path.display());
            let explorer = std::env::var_os("SystemRoot").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(r"C:Windows")).join("explorer.exe");
            let _ = std::process::Command::new(explorer).raw_arg(arg).spawn();
        }
    }

    /// Cree un raccourci (.lnk) vers l'element selectionne, a cote de lui.
    pub fn make_alias(&mut self, cx: &mut Context<Self>) {
        if self.read_only_place(cx) {
            return;
        }
        let Some(src) = self.selected_entry().map(|e| e.path.clone()) else { return };
        let name = src.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "element".into());
        let dir = src.parent().map(Path::to_path_buf).unwrap_or_else(|| self.cwd.clone());
        let dest = ops::unique_path(&dir, &format!("{name} - Raccourci.lnk"));
        let out = dest.clone();
        cx.spawn(async move |this, cx| {
            let result = cx.background_executor().spawn(async move { run_ps(ALIAS, &[("WF_SRC", ps::os(&src)), ("WF_DEST", ps::os(&out))]) }).await;
            this.update(cx, |f, cx| match result {
                Ok(()) => {
                    f.reload_select(&[dest]);
                    f.show_notice(crate::i18n::t("Raccourci cr\u{e9}\u{e9}"), cx);
                }
                Err(e) => f.show_notice(crate::i18n::tf(crate::i18n::t("Raccourci impossible \u{2014} {e}"), &[("e", (e).to_string())]), cx),
            })
            .ok();
        })
        .detach();
    }

    /// "Afficher l'original" pour un raccourci : ouvre le dossier de sa cible et la selectionne.
    pub fn show_original(&mut self, cx: &mut Context<Self>) {
        let Some(lnk) = self.selected_entry().map(|e| e.path.clone()) else { return };
        match crate::apps::link_target(&lnk) {
            Some(target) if target.exists() => {
                if let Some(dir) = target.parent().map(Path::to_path_buf) {
                    self.navigate(dir, true, cx);
                    self.reload_select(&[target]);
                }
            }
            _ => self.show_notice(crate::i18n::t("L'original est introuvable."), cx),
        }
    }
}
