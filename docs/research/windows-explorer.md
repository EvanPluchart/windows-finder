# Windows File Explorer (native) — feature map

> Reverse-engineering report. Subject: `explorer.exe`, `ExplorerFrame.dll`, `shell32.dll` and their French MUI string tables on Windows 11 24H2 (build 26100).
> Goal: know what the native file manager does, so Windows Finder can match the useful parts **with the macOS look and feel**.

## 0. Method and evidence

| Step | Tool | Result |
|---|---|---|
| PE resource inventory | REA `inspect-pe-resources` (CLI, no engine needed) | `explorer.exe` 10.0.26100.8875, sha256 `87811a10…5d54`, 207 resources (icons, bitmaps, MUI link), evidence `ev_b2491e49…cabc` |
| | | `shell32.dll` 10.0.26100.9278, sha256 `57231c16…7afac`, evidence `ev_23cc2fc8…0f71a` |
| | | `ExplorerFrame.dll` 10.0.26100.8972, sha256 `d19f66b2…5778a`, evidence `ev_d3a1ac7f…2fc2f` |
| String tables | Win32 `LoadLibraryEx(DATAFILE)` + `LoadString` on `C:\Windows\System32\fr-FR\*.mui` | `shell32.dll.mui` 3 583 strings, `ExplorerFrame.dll.mui` 495, `propsys.dll.mui` 2 011, `explorer.exe.mui` 44 |
| Behaviour | Observation of the running shell (user's own session) | command bar, tabs, Quick Access, details pane |

**Limits.** No native decompiler (Hopper / Ghidra / IDA) is configured on this machine, so there is no pseudocode: everything below is **observed** (resources, strings, behaviour) or **inferred** from them. The code is language-neutral: all UI text lives in the `.mui` files, which is why the PE inventories only show icons and a `MUI` link record.

## 1. Architecture

* `explorer.exe` hosts the **shell**: taskbar, Start, desktop, notification area, and every File Explorer window (one process, one UI thread per window group).
* `ExplorerFrame.dll` is the **frame**: address bar, navigation buttons, search box, command bar (Windows 11) / ribbon (Windows 10), navigation pane, tabs, status bar. It owns the strings "Barre d’adresses", "Zone de recherche", "Boutons de navigation", "Volet de navigation".
* `shell32.dll` + the **namespace extensions** supply the folders: This PC, Network, Recycle Bin, Quick Access / Home, Libraries, Control Panel, OneDrive, Linux (WSL). Items are **IShellItem / PIDLs**, not paths, so "folders" can be virtual.
* `propsys.dll` (2 011 strings) is the **property system**: every column, group and filter is a *property key* (System.ItemNameDisplay, System.DateModified, System.Size, System.ItemTypeText, …).
* Views are `IShellView` implementations hosting a **list-view** with the five view modes below. The Windows 11 command bar and Home page are XAML (WinUI) islands (`Windows.UI.FileExplorer.dll`, 41 strings).

## 2. Window anatomy (Windows 11)

1. **Title bar + tab strip** (tabs since 22H2): "Ouvrir dans un nouvel onglet", "Ouvrir « %s » dans un groupe d’onglets", tab titles "%1 et %2 autres onglets".
2. **Navigation row**: Back (Alt+←), Forward (Alt+→), Up (Alt+↑), Refresh (F5), **breadcrumb address bar** (click a chevron to list sibling folders — "liste des frères" / "liste des parents"; click the empty part to edit the path), **search box** ("Rechercher dans : %1").
3. **Command bar**: *Nouveau* ▾, Cut, Copy, Paste, Rename, Share, Delete, *Trier* ▾, *Afficher* ▾, **…** overflow, **Détails** (pane toggle).
4. **Navigation pane**: Home (Accueil), Gallery, OneDrive, *pinned* Quick Access folders, This PC (drives), Network, Linux. Drag & drop onto items, pin/unpin.
5. **Content**: view mode, groups ("En-tête de groupe", expand/collapse), column header with sort arrows and filter drop-downs ("Filtrer liste déroulante").
6. **Status bar**: "%1!d! éléments", "%1!d! élément sélectionné", view switcher (Détails / Grandes icônes).
7. **Details / Preview pane** (Alt+Maj+P / Alt+P, since 11 also a *Détails* pane with properties and share actions).

## 3. Views and grouping

| Mode | Notes |
|---|---|
| Très grandes / Grandes / Moyennes / Petites icônes | thumbnails 256 / 96 / 48 / 16 px; Ctrl+wheel zooms continuously |
| Liste | single column flow |
| **Détails** | sortable columns (Name, Date modified, Type, Size by default), column chooser "Plus…", auto-fit (Ctrl+numpad +) |
| Mosaïques | icon + 3 lines of metadata |
| Contenu | icon + rich metadata, used by search |
| Compact toggle | reduces row spacing |
| Item check boxes, file name extensions, hidden items | View ▸ Show |

Sort by any property; **Group by** any property incl. date buckets (Aujourd’hui / Hier / Plus tôt cette semaine / Le mois dernier / Il y a longtemps…), size buckets (Vide, Minuscule, Petit, Moyen, Grand…), type, first letter. Group headers collapse.

## 4. Home / Quick Access / Gallery

* **Accueil**: *Quick access* (pinned folders + frequent), *Favorites* (pinned files), *Recent* (files + OneDrive activity).
* **Quick Access** is a pinned list stored under `%APPDATA%\Microsoft\Windows\Recent\AutomaticDestinations` and `f01b4d95cf55d32a.automaticDestinations-ms`; pin/unpin from the context menu or by drag.
* **Gallery**: photo-library style view over Pictures (+ OneDrive).

## 5. Context menu and command surface

Windows 11 shows a **compact** menu (icon row Cut/Copy/Rename/Share/Delete, then Open, Open in new tab / window, Pin to Quick access, Compress to ZIP, Copy as path, Properties) and a legacy **"Afficher plus d’options"** menu with every shell-extension verb (Send to, Open with, Restore previous versions, 7-Zip, Git, Terminal…). Relevant verbs found in `ExplorerFrame.dll.mui` / `shell32.dll.mui`:

* File: Ouvrir, Ouvrir avec, Ouvrir dans un nouvel onglet/une nouvelle fenêtre, Modifier, Imprimer, Exécuter en tant qu’administrateur, Ouvrir l’emplacement du fichier, Définir comme arrière-plan du Bureau, Rotation (gauche/droite), Extraire tout, Compresser, Synchroniser, Épingler à Démarrer / à l’Accès rapide, **Copier en tant que chemin d’accès** (Ctrl+Maj+C), Envoyer vers (Bureau, Destinataire, Dossier compressé, lecteur), **Propriétés** (Alt+Entrée).
* Background: Affichage, Trier par, Grouper par, Actualiser, Personnaliser ce dossier, Coller, Coller le raccourci, Annuler, Nouveau ▸ (Dossier, Raccourci, Document texte, …), Ouvrir dans Terminal, Propriétés.
* Drive: Formater, Éjecter, Vérifier, Nettoyage de disque, Épingler.
* Recycle Bin: Restaurer, Restaurer tous les éléments, Vider la corbeille, Propriétés (taille max).

## 6. Properties dialog

Tabs: Général (type, opens-with, location, size / size on disk, created / modified / accessed, attributes read-only / hidden), Partage, Sécurité (ACL), Versions précédentes, Personnaliser (folder type, picture), Détails (all property-system fields, editable), Compatibilité (exe). Multi-selection shows count + total size.

## 7. Search

Search box scopes to the current folder (recursive), shows "Résultats de la recherche dans %1", **Search tab filters**: Type (Documents, Images, Musique, Vidéo…), Taille (Vide … Gigantesque), Date de modification, Autres propriétés, *Options* (contents, subfolders, indexed / non-indexed locations: "Dans les emplacements non indexés"). Search history with "Effacer l’historique de recherche". Backed by Windows Search (index) — much faster than a crawl.

## 8. File operations

* Copy / move dialog with progress graph, pause, **conflict dialog** ("Remplacer le fichier", "Ignorer", "Comparer les informations des deux fichiers").
* Undo / redo of rename, move, copy, delete (Ctrl+Z / Ctrl+Y).
* Delete → Recycle Bin, Shift+Delete = permanent, confirmation optional.
* Drag & drop with modifier cue: Ctrl = copy, Shift = move, Alt = shortcut; tooltip "Déplacer vers …".
* Rename: F2, Tab to next item, multi-select rename with "(1)" numbering.
* **Create shortcut**, **Paste shortcut**, symbolic links via `mklink`.
* Compress to ZIP, extract all (built-in, now also 7z / RAR / TAR since 23H2 via libarchive).

## 9. Keyboard map (selection)

F2 rename · F3/Ctrl+E/Ctrl+F search · F4 address bar · F5 refresh · F6 cycle focus · Alt+↑/←/→ navigation · Alt+D address bar · Alt+Enter properties · Ctrl+N new window · Ctrl+T new tab · Ctrl+W close tab · Ctrl+Tab next tab · Ctrl+Shift+N new folder · Ctrl+Shift+E expand tree to current folder · Ctrl+1…8 view modes · Alt+P preview pane · Alt+Shift+P details pane · Ctrl+A / Ctrl+Shift+A… · Ctrl+Mouse wheel zoom · Backspace = back · type-ahead select.

## 10. Settings ("Options des dossiers")

Open File Explorer to (Home / This PC / Downloads), *Browse folders* in same / new window, Click items (single / double), Privacy (show recent / frequent), per-folder-type view **Apply to Folders** / **Reset**, Navigation pane (show all folders, expand to open folder, libraries), Advanced: hidden files, extensions, empty drives, protected OS files, "restore previous folder windows at logon", checkboxes, status bar, sync provider notifications, full path in title.

## 11. Integrations Windows Finder must respect

* Shell extensions (context-menu handlers) — currently only reachable through "Afficher dans l’Explorateur".
* OneDrive overlays and "Always keep on device / Free up space".
* WSL: `\\wsl.localhost\<distro>` in the navigation pane (done).
* Libraries, Quick Access, Recent items, Jump Lists.
* Default file manager association (`Directory\shell\open\command`) — done (HKCU only).

## 12. What this means for Windows Finder

Implemented already: tabs, 4 views with per-folder options, sort/group, Quick Look, Get Info, conflicts + progress, undo/redo, tags, bulk rename, trash, archives, search (+ content), Go to Folder, per-folder preferences, WSL, network page, settings.

Gaps surfaced by this report (carried into `docs/research/feature-gap.md`): Copy-as-path, Paste shortcut, new-file menu (Nouveau ▸ Document texte…), filtered header (Files), item check-boxes, Pin to Quick Access/Favorites, "Open in Terminal", Properties via shell, Rotate images, Set as wallpaper, size/date search filters, drag copy/move/shortcut cues.
