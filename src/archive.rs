//! Compresser / extraire des archives zip (PowerShell Compress-Archive / Expand-Archive, hors thread d'interface).

use std::path::{Path, PathBuf};

use crate::{ops, Finder};
use gpui::Context;

fn ps_quote(p: &Path) -> String {
    format!("'{}'", p.to_string_lossy().replace('\'', "''"))
}

fn run_ps(script: String) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    let out = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .creation_flags(0x0800_0000)
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).lines().next().unwrap_or("erreur").to_string())
    }
}

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
            [one] => one.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "Archive".into()),
            _ => "Archive".to_string(),
        };
        let dest = ops::unique_path(&self.cwd, &format!("{base}.zip"));
        let list = paths.iter().map(|p| ps_quote(p)).collect::<Vec<_>>().join(",");
        let script = format!("Compress-Archive -LiteralPath {list} -DestinationPath {} -Force", ps_quote(&dest));
        self.show_notice("Compression en cours\u{2026}", cx);
        cx.spawn(async move |this, cx| {
            let result = cx.background_executor().spawn(async move { run_ps(script) }).await;
            this.update(cx, |f, cx| match result {
                Ok(()) => {
                    f.reload_select(&[dest.clone()]);
                    f.show_notice(format!("\u{ab} {} \u{bb} cr\u{e9}\u{e9}", dest.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()), cx);
                }
                Err(e) => f.show_notice(format!("Compression impossible \u{2014} {e}"), cx),
            })
            .ok();
        })
        .detach();
    }

    pub fn extract_selection(&mut self, cx: &mut Context<Self>) {
        if self.read_only_place(cx) {
            return;
        }
        let Some(zip) = self.selected_entry().filter(|e| fs_ext(&e.path) == "zip").map(|e| e.path.clone()) else { return };
        let stem = zip.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "Extrait".into());
        let dest: PathBuf = ops::unique_path(&self.cwd, &stem);
        let script = format!("Expand-Archive -LiteralPath {} -DestinationPath {} -Force", ps_quote(&zip), ps_quote(&dest));
        self.show_notice("Extraction en cours\u{2026}", cx);
        cx.spawn(async move |this, cx| {
            let result = cx.background_executor().spawn(async move { run_ps(script) }).await;
            this.update(cx, |f, cx| match result {
                Ok(()) => {
                    f.reload_select(&[dest]);
                    f.show_notice("Archive extraite", cx);
                }
                Err(e) => f.show_notice(format!("Extraction impossible \u{2014} {e}"), cx),
            })
            .ok();
        })
        .detach();
    }
}

fn fs_ext(p: &Path) -> String {
    crate::fs::extension(p)
}
