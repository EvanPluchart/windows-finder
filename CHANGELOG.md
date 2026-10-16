# Changelog

## 0.1.1 (2026-10-16)

Security
* PowerShell scripts no longer embed file names: a name containing a typographic apostrophe (`’`) could run commands (compress, extract, alias, rotate, undo, eject). Paths now travel in environment variables; system tools are called by their full System32 path.
* "Run as administrator" uses `ShellExecuteW` (`runas`); files open through `ShellExecuteW` instead of `cmd /C start`.
* ZIP extraction checks every entry (zip slip) and extracted files keep the "downloaded from the Internet" mark (Zone.Identifier).
* "Put Back" only restores inside the same drive, from fixed drives' recycle bins; reserved names (CON, NUL, COM1...) are refused.
* Moving to the Recycle Bin on a USB key or network share now warns before deleting permanently.

Fixes
* No console window at launch (GUI subsystem); crashes are logged to `%APPDATA%\winfinder\crash.log`.
* Uninstalling removes the optional Explorer integration (default folder handler, context-menu entry).
* Drives opened from Explorer open the drive instead of the home folder; PDF and video previews work on network shares; WSL places use a valid UNC path.
* Quick Look editors keep encoding, BOM, line endings and delimiters, write atomically, refuse to overwrite a file changed elsewhere, and report save errors. Non-UTF-8 or tab-indented files are read-only.
* Settings, tags, layouts and view options are written atomically and merged, so two windows no longer overwrite each other.

Performance
* No more `reg.exe` every 3 seconds (theme follows Windows through `WM_SETTINGCHANGE`).
* Folders are read in the background; icon view only builds visible cells; sorting, selection and dragging no longer re-read the disk or copy the selection per item.
* Search is debounced and cancels the previous walk; GPU textures of video frames, PDF pages and evicted icons are released; the icon cache is bounded.

## 0.1.0 (2026-10-16)

First public-ready state.

* Four views (icons, list, columns, gallery) with per-folder options, tabs, Quick Look, Get Info, tags, bulk rename, undo / redo, in-app Recycle Bin, conflicts and progress.
* Light / dark / system theme, eight accent colours, English / Français / Español / Deutsch (Windows language by default).
* Settings page, default file manager option (HKCU), Explorer context-menu entry.
* Command palette, native Explorer context menu, pinned folders, WSL and network places, Terminal, wallpaper, image rotation, run as administrator, 7z / rar / tar extraction.
* Research: reports on the native File Explorer and the Files app (`docs/research/`).
