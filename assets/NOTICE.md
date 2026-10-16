# Asset licensing notice

The code and every asset in this repository are free to redistribute: MIT (see `/LICENSE`), plus the ISC-licensed Lucide symbols.

| Folder | Content | Status |
|---|---|---|
| `assets/open/` | Open icon set: symbols from [Lucide](https://lucide.dev) (ISC licence) rendered by `tools/gen-open-icons`, plus folder icons drawn for this project | MIT / ISC, free to redistribute |
| `assets/icon.ico`, `assets/logo.png` | Windows Finder logo | MIT, same as the code |
| `assets/i18n.txt` | UI translations | MIT |

## Icon set

Releases use the open set. To regenerate it: `cd tools/gen-open-icons && npm install && node gen.mjs`.

The `apple-assets` Cargo feature exists for personal builds only: it reads a local `assets/apple/` folder (same file names as `assets/open/`) that you extract yourself from your own Mac. That folder is ignored by Git and is never part of this repository or of a release.

```
cargo build --release                          # open icon set (default)
cargo build --release --features apple-assets  # personal build with your own local assets/apple/
```

The app looks like the macOS Finder on purpose (layout, spacing, behaviour). Apple's trademarks (Finder, macOS) are used descriptively only; this project is not affiliated with or endorsed by Apple.

Lucide: ISC licence, Copyright (c) for portions of Lucide are held by Cole Bemis 2013-2022 as part of Feather (MIT). All other copyright (c) for Lucide are held by Lucide Contributors 2022.
