# Files (files.community) — feature map

> Reverse-engineering report. Subject: **Files 4.2.2.0** (Microsoft Store / MSIX, publisher Yair Aichenbaum), the most complete third-party file manager for Windows.
> Goal: list every feature it ships, so Windows Finder does not miss anything useful — keeping the **macOS Finder look**, not Files' Fluent look.

## 0. Method and evidence

| Step | Tool | Result |
|---|---|---|
| Package identity | `AppxManifest.xml` | `Files` 4.2.2.0 x64, entry point `Files.exe`, capabilities `internetClient`, `privateNetworkClientServer`, 144 DLLs |
| Managed identity | REA `inspect-managed-artifact` | `Files.dll` 13 123 584 B, sha256 `42be4f23…8f87`, evidence `ev_294c8001…be1f8`; **.NET 10 (`.NETCoreApp,Version=v10.0`)**, ReadyToRun, 3 885 types, 29 095 methods, 20 987 fields, 110 assembly references |
| Member inspection | REA `inspect-managed-members` | **Refused**: metadata exceeds REA's 64 MiB representation budget (`resource-limit`). Reported as a limit, not worked around inside REA. |
| Names and literals | Direct read of the CLI metadata heaps (`#Strings` 26 906 names, `#US` 6 281 string literals) located with the offsets REA reported | every class name `…Action`, settings keys, resource ids |

**Limits.** Static only; no code was executed; no pseudocode (no engine installed). Names are strong evidence of *existence* of a feature, not of its exact behaviour.

## 1. Technology

WinUI 3 + Windows App SDK, **CommunityToolkit.Mvvm** (MVVM source generators), CommunityToolkit.WinUI controls (Segmented, SettingsControls, Sizers, ColorPicker, MarkdownTextBlock, Behaviors, Animations), **Vanara** P/Invoke (Shell, Ole, ComCtl32, Gdi32), **Microsoft.Data.Sqlite** (history / state), **TagLibSharp** (audio tags), **DiscUtils.Udf** (ISO), 7-Zip native (`7z64.dll`), **LibGit2** (Git), Sentry (crash reports), `Microsoft.Windows.AppNotifications`, dependency injection (`Microsoft.Extensions.DependencyInjection`), 140+ `Files.App.*` namespaces. Single-instance with `OpenTabInExistingInstance`.

## 2. Command architecture

Everything is an **`IAction`** class (≈ 260 classes) bound to a **hotkey** and surfaced in the toolbar, context menus, sidebar menus and the **Command Palette**. Customisable toolbar (`CustomizeToolbar`, `ToggleToolbar`) and shortcuts (`ActionButtonCommand`). This is the model to copy: one registry of named commands → menu, toolbar, palette, shortcut.

## 3. Feature inventory (from the action classes)

### Tabs and panes
* Tabs: new, **reopen closed**, duplicate, next / previous, close others / left / right / all, drag to reorder, open in new window, `OpenInNewTabFromHome / Sidebar`.
* **Dual pane** (`ToggleDualPane`, `OpenInOtherPane`, `FocusOtherPane`, `CloseActivePane`, `SplitPaneVertically / Horizontally`, `ArrangePanes…`, `OpenCurrentFolderInOtherPane`).
* **Shelf** pane (`ToggleShelfPane`, cut / copy / delete from shelf): a scratch area to collect items and move them in one go.
* **Info pane / Details pane / Preview pane** (`ToggleInfoPane`, `ToggleDetailsPane`, `TogglePreviewPane`), **Preview popup** (`LaunchPreviewPopup`, like Quick Look).
* **Command palette** (`OpenCommandPalette`), **compact overlay** (`EnterCompactOverlay`), full screen.

### Layouts, sort, group
* Layouts: **Details, List, Cards, Grid, Columns, Adaptive**; increase / decrease size (`LayoutIncreaseSize / DecreaseSize`).
* Sort by name, type, size, date modified, date created, **tag, path, original folder, sync status**, ascending / descending; **files first / folders first / together**.
* Group by none, name, type, size, tag, folder path, sync status, **date (day, month, year; modified / created / deleted)**, group direction toggle, `ToggleGroupByDateUnit`.
* **Filter header** (`ToggleFilterHeader`): per-column filtering in details layout.
* Auto-fit columns, show hidden items, show file extensions, show dot-files setting.

### File operations
Create folder / file / shortcut / **folder with selection**, create from dialog, **create alternate data stream**, copy / cut / paste / **paste as shortcut** / paste into selection, copy path (with quotes), copy item path, rename, delete / **delete permanently**, undo / redo, select all / clear / **invert selection**, toggle select, **flatten folder**, open parent folder, open file location, edit path (address bar), open with **application picker**, run as admin / **as another user**, **install font / certificate / .inf driver**, Format drive, Storage Sense, restore / restore all / empty Recycle Bin, remove tags, open all tagged.

### Archives
Compress to Zip / 7z / custom (level, dictionary size, CPU threads, split size, password, encryption), **decompress here / smart here / to child folder**, archive preview.

### Images / media
Rotate left / right, **set as desktop wallpaper / lock-screen / slideshow / app background**, Play all (media), audio tag reading, thumbnails.

### Git (LibGit2)
Init, clone, fetch, pull, push, sync, branches (list / checkout / delete), per-row status & last-commit columns (status, message, author, sha, date), git actions panel, repo watcher; open repo / folder in IDE.

### Sidebar
Pinned folders (pin / unpin), Home widgets (quick access, drives, recent, network locations, file tags), library creation, cloud drives (OneDrive, Google Drive, iCloud, Box, Dropbox…), drives with Eject / Format / Storage Sense, network, WSL distros, **tags**, git repos, "open in terminal / IDE" from any item.

### Shell and system
Open terminal (Windows Terminal) / as admin, PowerShell script run, notepad edit, set as default file manager, release notes, help, log file, settings file.

### Settings pages (from `ViewModels.Settings`)
General (startup, open in new tab, language, date format, confirm delete, **default file manager**), Appearance (theme, accent, **background material**, density), Folders (layout defaults, group, sort, show hidden, file extensions, columns, **per-folder layout preferences stored in SQLite**), Actions (hotkeys, toolbar), Tags, Git, Advanced (folder size provider, shell integration, export / import settings), About.

## 4. UX details worth copying

* **Command palette** (Ctrl+Shift+P): fuzzy search across every action with its shortcut.
* Tabs/panes remembered across sessions; recently closed tabs restore.
* Toolbar items change with the context (`SelectedToolbarContextId`): different buttons when files vs folders vs Recycle Bin are selected.
* Progress centre for all operations with per-item status (`InProgress / Successful / Error / Canceled` brushes).
* Smart archive extract (single root folder detection).
* Folder size computed in the background, shown in details and status bar.

## 5. What Files does that does **not** fit the macOS look

Fluent/Mica translucency, NavigationView sidebar, heavy command-bar chrome. We keep our own macOS skin; features only.

## 6. Carry-over to Windows Finder

See `docs/research/feature-gap.md`.
