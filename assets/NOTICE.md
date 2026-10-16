# Asset licensing notice

The code in this repository is MIT licensed (see `/LICENSE`). **Some bundled artwork is not.**

| Folder | Content | Status |
|---|---|---|
| `assets/open/` | Open icon set: symbols from [Lucide](https://lucide.dev) (ISC licence) rendered by `tools/gen-open-icons`, plus folder icons drawn for this project | MIT / ISC, free to redistribute |
| `assets/apple/macicons/` | folder icons extracted from macOS (CoreTypes) | **Apple property. Personal use only.** |
| `assets/apple/sf/` | SF Symbols rendered to PNG | **Apple property (SF Symbols licence). Personal use only.** |
| `assets/icon.ico`, `assets/logo.png` | Windows Finder logo | MIT, same as the code |
| `assets/i18n.txt` | UI translations | MIT |

## Choosing the icon set

The default build uses the open set. The Apple-derived set is opt-in through the `apple-assets` Cargo feature:

```
cargo build --release                                  # open icon set (default, redistributable)
cargo build --release --features apple-assets          # Apple-style icons (personal use only)
```

Both sets use the same file names, so they are interchangeable. To regenerate the open set: `cd tools/gen-open-icons && npm install && node gen.mjs`.

## Before this repository becomes public

1. (Done) the open set is the default.
2. Delete `assets/apple/` and rewrite the Git history if those files must not remain in it.

The app looks like the macOS Finder on purpose (layout, spacing, behaviour). Apple's trademarks (Finder, macOS) are used descriptively only; this project is not affiliated with or endorsed by Apple.

Lucide: ISC licence, Copyright (c) for portions of Lucide are held by Cole Bemis 2013-2022 as part of Feather (MIT). All other copyright (c) for Lucide are held by Lucide Contributors 2022.
