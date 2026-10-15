# Feature gap analysis — Explorer, Files and Windows Finder

Sources: [`windows-explorer.md`](windows-explorer.md), [`files-app.md`](files-app.md). Rule: **features yes, Fluent look no** — everything keeps the macOS Finder design and vocabulary.

| Feature (source) | Status | Notes |
|---|---|---|
| Tabs, reorder, duplicate, close others (Explorer, Files) | done | |
| **Reopen closed tab** (Files) | **added** | `Ctrl+Shift+T` |
| Tab overview (Finder) | done | `Ctrl+Shift+A` |
| Icon / List / Columns / Gallery (Explorer, Files) | done | Cards/Adaptive of Files = Gallery |
| Group by / sort by, relative dates | done | |
| Per-folder view options (Files: SQLite) | done | `folders.tsv`, `Ctrl+J` |
| Quick Look / Preview popup (Files) | done | images, text, CSV, PDF, video |
| Details / Info pane (Explorer, Files) | done | Get Info + optional preview pane |
| **Command palette** (Files) | **added** | `Ctrl+Shift+P`, fuzzy search, shows shortcuts |
| **Native context menu / shell extensions** (Explorer) | **added** | « Plus d'options… » = real `IContextMenu` (Send to, Open with, Git, 7-Zip, Properties…) |
| **Copy path** (Explorer, Files) | **added** | `Ctrl+Shift+C`, toolbar share menu |
| **Open in Terminal** (Files, Explorer 11) | **added** | Windows Terminal, PowerShell fallback |
| **Pin folder to sidebar** (Explorer Quick Access, Files) | **added** | context menu; right-click a pinned entry to remove |
| **Set as desktop picture** (Explorer, Files) | **added** | PNG / JPG / BMP |
| **Rotate images** (Explorer, Files) | **added** | left / right, in place |
| **Run as administrator** (Explorer, Files) | **added** | exe, bat, cmd, msi, ps1 |
| **New text file** (Explorer « Nouveau ») | **added** | background menu |
| **Extract 7z / rar / tar / tgz / iso** (Explorer 23H2, Files) | **added** | via the `tar` (bsdtar) shipped with Windows 11 |
| Compress to ZIP (Explorer) | done | |
| Undo / redo (Explorer, Files) | done | `Ctrl+Z`, `Ctrl+Shift+Z` |
| Conflict dialog + progress (Explorer, Files) | done | keep both / replace / skip / stop |
| Tags (Files) | done | Finder colour tags |
| WSL distros in sidebar (Explorer, Files) | **added** | `\wsl.localhost\<distro>` |
| Network page (Explorer) | **added** | WSL + network shortcuts, in-app |
| Default file manager (Files) | done | per-user registry only |
| Multi-language (Files) | **added** | fr / en / es / de, follows Windows |
| Dual pane (Files) | skipped | not part of the Finder; tabs + drag between windows cover it |
| Shelf pane (Files) | skipped | no Finder equivalent |
| Filter header (Files) | skipped | Finder uses search tokens |
| Git integration (Files) | later | status columns would break the Finder column set |
| Compact overlay (Files) | skipped | |
| Format drive / Storage Sense (Files) | later | reachable through « Plus d'options… » on a drive |
| Alternate data streams, install font / cert / inf (Files) | later | reachable through « Plus d'options… » |
| Flatten folder, invert selection (Files) | later | |

## Method note

Every « added » row has an implementation in `src/` and a line in the in-app command palette or context menu. Evidence for the source features is in the two reports (REA evidence ids, hashes, limits).
