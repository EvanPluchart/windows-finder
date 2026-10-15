//! Annuler / retablir les operations sur les fichiers (Ctrl+Z, Ctrl+Maj+Z) : renommage, deplacement, copie,
//! duplication, nouveau dossier, mise a la corbeille (restauration depuis la corbeille Windows).

use std::path::{Path, PathBuf};

use gpui::Context;

use crate::{ops, Finder};

#[derive(Clone)]
pub enum Op {
    /// Un element renomme.
    Rename { from: PathBuf, to: PathBuf },
    /// Elements deplaces ou renommes en lot : (origine, destination).
    Moves(Vec<(PathBuf, PathBuf)>),
    /// Elements crees (copie, duplication, nouveau dossier) : annuler = mettre a la corbeille.
    Created(Vec<PathBuf>),
    /// Elements mis a la corbeille : annuler = restaurer.
    Trashed(Vec<PathBuf>),
}

fn name(p: &Path) -> String {
    p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

/// Restaure des elements depuis la corbeille Windows (PowerShell, verbe "undelete").
fn restore_from_bin(paths: &[PathBuf]) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    let list = paths.iter().map(|p| format!("'{}'", p.to_string_lossy().replace('\'', "''"))).collect::<Vec<_>>().join(",");
    let script = format!(
        "$t=@({list}); $sh=New-Object -ComObject Shell.Application; $rb=$sh.NameSpace(10); \
         foreach($i in $rb.Items()){{ $n=$i.Name; $e=[IO.Path]::GetExtension($i.Path); if($e -and -not $n.EndsWith($e)){{$n+=$e}}; \
         $o=Join-Path ($rb.GetDetailsOf($i,1)) $n; if($t -contains $o){{ $i.InvokeVerb('undelete') }} }}"
    );
    let out = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .creation_flags(0x0800_0000)
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).lines().next().unwrap_or("erreur").to_string());
    }
    // Verification : les elements sont revenus.
    if paths.iter().all(|p| p.exists()) { Ok(()) } else { Err("introuvable dans la corbeille".into()) }
}

impl Finder {
    /// Memorise une operation (efface la pile "retablir").
    pub fn record(&mut self, op: Op) {
        self.undo_stack.push(op);
        if self.undo_stack.len() > 100 {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
    }

    pub fn undo(&mut self, cx: &mut Context<Self>) {
        if crate::fs::is_virtual(&self.cwd) {
            return;
        }
        let Some(op) = self.undo_stack.pop() else {
            self.show_notice(crate::i18n::t("Rien \u{e0} annuler."), cx);
            return;
        };
        self.commit_rename(cx);
        match self.apply_inverse(&op, cx) {
            Ok((label, select)) => {
                if !matches!(op, Op::Created(_)) {
                    self.redo_stack.push(op);
                }
                self.reload_select(&select);
                self.show_notice(crate::i18n::tf(crate::i18n::t("Annul\u{e9} : {label}"), &[("label", (label).to_string())]), cx);
            }
            Err(e) => self.show_notice(crate::i18n::tf(crate::i18n::t("Annulation impossible \u{2014} {e}"), &[("e", (e).to_string())]), cx),
        }
        cx.notify();
    }

    pub fn redo(&mut self, cx: &mut Context<Self>) {
        if crate::fs::is_virtual(&self.cwd) {
            return;
        }
        let Some(op) = self.redo_stack.pop() else {
            self.show_notice(crate::i18n::t("Rien \u{e0} r\u{e9}tablir."), cx);
            return;
        };
        let result: Result<(String, Vec<PathBuf>), String> = match &op {
            Op::Rename { from, to } => std::fs::rename(from, to).map(|_| (crate::i18n::tf(crate::i18n::t("renommage de \u{ab} {0} \u{bb}"), &[("0", (name(from)).to_string())]), vec![to.clone()])).map_err(|e| e.to_string()),
            Op::Moves(pairs) => (|| {
                let mut sel = Vec::new();
                for (from, to) in pairs {
                    ops::move_path(from, to).map_err(|e| e.to_string())?;
                    crate::tags::rename_path(from, to);
                    sel.push(to.clone());
                }
                Ok((crate::i18n::tf(crate::i18n::t("{0} d\u{e9}placement(s)"), &[("0", (pairs.len()).to_string())]), sel))
            })(),
            Op::Trashed(paths) => ops::trash(paths).map(|_| (crate::i18n::t("mise \u{e0} la corbeille").to_string(), Vec::new())),
            Op::Created(_) => Err("non r\u{e9}tablissable".into()),
        };
        match result {
            Ok((label, select)) => {
                self.undo_stack.push(op);
                self.reload_select(&select);
                self.show_notice(crate::i18n::tf(crate::i18n::t("R\u{e9}tabli : {label}"), &[("label", (label).to_string())]), cx);
            }
            Err(e) => self.show_notice(crate::i18n::tf(crate::i18n::t("R\u{e9}tablissement impossible \u{2014} {e}"), &[("e", (e).to_string())]), cx),
        }
        cx.notify();
    }

    fn apply_inverse(&mut self, op: &Op, _cx: &mut Context<Self>) -> Result<(String, Vec<PathBuf>), String> {
        match op {
            Op::Rename { from, to } => {
                if from.exists() && from.to_string_lossy().to_lowercase() != to.to_string_lossy().to_lowercase() {
                    return Err(crate::i18n::tf(crate::i18n::t("\u{ab} {0} \u{bb} existe d\u{e9}j\u{e0}"), &[("0", (name(from)).to_string())]));
                }
                std::fs::rename(to, from).map_err(|e| e.to_string())?;
                crate::tags::rename_path(to, from);
                Ok((crate::i18n::tf(crate::i18n::t("renommage de \u{ab} {0} \u{bb}"), &[("0", (name(from)).to_string())]), vec![from.clone()]))
            }
            Op::Moves(pairs) => {
                let mut sel = Vec::new();
                for (from, to) in pairs {
                    if from.exists() {
                        return Err(crate::i18n::tf(crate::i18n::t("\u{ab} {0} \u{bb} existe d\u{e9}j\u{e0}"), &[("0", (name(from)).to_string())]));
                    }
                    ops::move_path(to, from).map_err(|e| e.to_string())?;
                    crate::tags::rename_path(to, from);
                    sel.push(from.clone());
                }
                Ok((crate::i18n::tf(crate::i18n::t("{0} \u{e9}l\u{e9}ment(s) remis en place"), &[("0", (pairs.len()).to_string())]), sel))
            }
            Op::Created(paths) => {
                ops::trash(paths)?;
                Ok((crate::i18n::tf(crate::i18n::t("{0} \u{e9}l\u{e9}ment(s) cr\u{e9}\u{e9}(s)"), &[("0", (paths.len()).to_string())]), Vec::new()))
            }
            Op::Trashed(paths) => {
                restore_from_bin(paths)?;
                Ok((crate::i18n::tf(crate::i18n::t("{0} \u{e9}l\u{e9}ment(s) restaur\u{e9}(s)"), &[("0", (paths.len()).to_string())]), paths.clone()))
            }
        }
    }
}
