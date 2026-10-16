# gen-open-icons

Generates the open icon set in `assets/open/` (same file names as the Apple-derived set in `assets/apple/`).

```
npm install
node gen.mjs
```

- Symbols: [Lucide](https://lucide.dev) glyphs (ISC), black on transparent, trimmed to their ink box. The app tints them at runtime.
- Folders: drawn in `gen.mjs` (MIT). Edit the gradients or emblems there and re-run.

Build the app with the open set: `cargo build --release`.
