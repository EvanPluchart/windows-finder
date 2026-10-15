# Contributing to Windows Finder

Thanks for helping! Windows Finder is a macOS Finder-style file manager for Windows, written in Rust with [GPUI](https://crates.io/crates/gpui). Contributions of every size are welcome: bug reports, translations, docs, code.

## Quick start

```powershell
# Windows 10/11, Rust (stable) and the MSVC C++ build tools
git clone <repo-url>
cd windows-finder
cargo run                       # opens your home folder
cargo run -- C:\Windows --dark --view=list
```

Useful test flags: first argument = start folder (`::apps`, `::pc`, `::trash`, `::net`, `::tag:Name`), `--view=icons|list|columns|gallery`, `--select=<name>`, `--ql`, `--info`, `--sort=name|date|size|kind[:desc]`, `--dark`, `--light`.

## Ground rules

1. **Match the macOS Finder.** If a Windows feature is useful, give it the Finder's look and vocabulary (see `docs/` and `docs/research/`). No Fluent / Mica styling.
2. **Be fast.** Never block the UI thread on disk, shell or network; use `cx.background_executor()`.
3. **Be safe with files.** Deletion goes to the Recycle Bin. Anything irreversible needs an in-app confirmation. Registry writes stay under `HKEY_CURRENT_USER` and are reversible.
4. **Every visible string is translatable.** Write the French text in code (`crate::i18n::t("…")`, or `tf` for `{name}` templates) and add one line to `assets/i18n.txt`: `français ::: English ::: Español ::: Deutsch`. Constants cannot call `t`; translate where they are displayed.
5. **Keep it small.** One feature per pull request, matching the surrounding style and comment density (comments explain why, in plain words).

## Translations

Add a language: extend `Lang` and `LANGS` in `src/i18n.rs`, add a column to `assets/i18n.txt`, and add the language to the picker in `src/settings.rs`. Fixing a wording only needs a change in `assets/i18n.txt`.

## Pull requests

* `cargo build` must pass with no new warnings; run the app and try your change in light **and** dark mode.
* Describe what you tested; screenshots are great for UI changes.
* By contributing you agree your work is released under the MIT license.

## Reporting bugs

Open an issue with: Windows version, what you did, what you expected, what happened, and a screenshot if it is visual.

## Assets

Do **not** add Apple artwork. See `assets/NOTICE.md`.
