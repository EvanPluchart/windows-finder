# Architecture

Windows Finder is a single Rust binary (`WindowsFinder.exe`) built on [GPUI](https://crates.io/crates/gpui) for rendering and the `windows` crate for Win32 / WinRT. One `Finder` entity (in `src/main.rs`) owns the state of a window; every other module adds behaviour with an `impl Finder` block, which keeps files small and focused.

> Source comments are currently written in French (the project's first language). English comments are welcome in new code; please do not mass-translate existing ones in unrelated pull requests.

## Data flow

```
user input ──> actions / key bindings (main.rs) ──> Finder methods ──> state change ──> cx.notify() ──> render()
                                                        │
                                                        └─ slow work (disk, shell, PowerShell) runs on cx.background_executor()
                                                           and reports back with this.update(cx, ...)
```

## Module map

| Area | Files | What they do |
|---|---|---|
| Window and state | `main.rs`, `theme.rs`, `traffic.rs`, `toolbar.rs`, `sidebar.rs` | window creation (transparent, rounded), `Finder` struct, actions and shortcuts, colour tokens and dark-mode remapping, traffic lights and the green-button tiling popover, toolbar capsules, sidebar places |
| Views | `views.rs`, `tree.rs`, `selection.rs`, `layout.rs`, `icons.rs`, `symbols.rs` | icon / list / column / gallery rendering, expandable list rows, rubber-band and type-to-select, free icon placement, shell icons and thumbnails, SF-style symbols |
| File operations | `fileops.rs`, `ops.rs`, `transfer.rs`, `undo.rs`, `trash.rs`, `archive.rs`, `bulk.rs`, `tags.rs` | copy / cut / paste, unique names, conflicts and progress, undo / redo, in-app Recycle Bin, zip and tar extraction, bulk rename, colour tags |
| Drag and drop | `dragdrop.rs`, `dragout.rs`, `syscb.rs` | in-app drag, OLE drag to Explorer, Windows file clipboard (`CF_HDROP`) |
| Preview and info | `quicklook.rs`, `pdf.rs`, `video.rs`, `editor.rs`, `csvview.rs`, `info.rs` | Quick Look window (images, text, CSV, PDF, video), text editor widget, Get Info panel |
| Overlays | `overlays.rs`, `palette.rs`, `shellmenu.rs`, `tabs.rs`, `prefs.rs`, `settings.rs` | context menu, Go to Folder, search bar, command palette, native Explorer menu, tabs, per-folder view options, settings page |
| Places | `apps.rs`, `netplaces.rs`, `fs.rs`, `uninstall.rs` | virtual places (`::apps`, `::pc`, `::net`, `::trash`, `::tag:Name`), directory reading, sorting, grouping, WSL and network |
| Integration | `explorer.rs`, `extras.rs`, `i18n.rs` | default-file-manager registry keys (HKCU only), terminal / wallpaper / rotate / pins, translations |

## Conventions

* **Virtual places** are paths that start with `::`; `fs::is_virtual` tells them apart. They are read-only.
* **Settings** are `key=value` lines in `%APPDATA%\winfinder\settings.txt` (`layout::get_setting` / `set_setting`). Per-folder options live in `folders.tsv`, free icon positions in `layouts.tsv`, tags in `tags.tsv`.
* **Colours** are written in their light value and pass through `theme::rgb` / `rgba`, which remap them in dark mode. A colour that is not in the map in `theme.rs` stays unchanged in dark mode, so add new light colours there.
* **Text** is written in French and translated at render time with `crate::i18n::t` (or `tf` for templates with `{name}` placeholders). See `CONTRIBUTING.md`.
* **Safety**: deletes go to the Recycle Bin; irreversible actions ask first; registry writes are HKCU only.

## Test flags

`WindowsFinder.exe [folder] [--view=icons|list|columns|gallery] [--select=name] [--ql] [--info] [--sort=name|date|size|kind[:desc]] [--dark|--light]`. The folder can be a virtual place (`::apps`, `::pc`, `::trash`, `::net`).
