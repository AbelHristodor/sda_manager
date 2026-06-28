<div align="center">

# SDA Manager

**Find a hymn and put it on the screen — in seconds.**

A free, cross-platform desktop app for projecting hymns during a service.
Search a hymnal by number, title, or a half-remembered line of the lyrics,
then project it full-screen on the sanctuary display with your own styling — no
PowerPoint, no internet, no fiddling mid-service.

Runs on **macOS**, **Windows**, and **Linux**. Available in **English**,
**Italian**, and **Romanian**.

[Download](https://github.com/AbelHristodor/sda_manager/releases) ·
[Install](#install) · [Features](#features) · [For developers](#for-developers)

</div>

---

## Why

Getting the right hymn on the projector usually means hunting through folders of
PowerPoint files, opening the right one, and starting the slideshow — while the
congregation waits. SDA Manager replaces that with a single search box and a
**Project** button. Type `plecati`, hit Enter, and *Plecaţi-vă* is on the wall.

It ships with a full hymnal and stays current automatically, and you can add
your own folders of slides on top.

## Features

- 🔎 **Instant fuzzy search** — search across hymn number, title, filename, and
  every line of the lyrics. Accent-insensitive, so `plecati` finds *Plecaţi-vă*.
  The best match is highlighted as you type.
- 📽️ **Built-in projector** — project hymns full-screen on a second display.
  The app draws the slides itself, so there's **no dependency on PowerPoint** and
  it works the same on every OS. Live slide + next-slide preview on your laptop,
  blank-screen toggle, and arrow-key control.
- 🎨 **Custom themes** — design how projected slides look: fonts, sizes, colors,
  backgrounds (solid, gradient, or image), alignment, and footer. Live preview,
  multiple saved themes, switch the active one in a click.
- 📚 **Your own libraries** — comes with a built-in hymnal that updates itself,
  and you can add any folder of `.pptx` files alongside it. Enable, disable, or
  remove your folders from Settings.
- ⬇️ **Video downloader** — paste a YouTube link and save the video to a folder
  you choose, with live progress. Handy for grabbing illustrations or special
  music.
- 🌍 **Multilingual UI** — English, Italian, and Romanian, auto-detected from
  your system and switchable any time.
- ⌨️ **Keyboard-first** — arrow keys to browse and step through slides, Enter to
  open the highlighted hymn, `B` to blank the projector, `Esc` to stop. Built for
  live use.
- 🔄 **Always up to date** — the bundled hymnal refreshes on launch and the app
  checks for new versions automatically.

## Install

Prebuilt apps are on the
[Releases](https://github.com/AbelHristodor/sda_manager/releases) page for
macOS (Apple Silicon), Windows (x86_64), and Linux (x86_64).

**macOS / Linux** — paste into a terminal:

```sh
curl -fsSL https://raw.githubusercontent.com/AbelHristodor/sda_manager/main/install.sh | sh
```

> On macOS the app is unsigned, so the first time you open it, right-click the
> app and choose **Open** to get past Gatekeeper. (Installs to `~/.local/bin`;
> override with `BIN_DIR=…`.)

**Windows** — paste into PowerShell:

```powershell
irm https://raw.githubusercontent.com/AbelHristodor/sda_manager/main/install.ps1 | iex
```

> Installs to `%LOCALAPPDATA%\Programs\hymnal-gui`, adds a Desktop shortcut, and
> puts the app on your `PATH`.

Prefer to build it yourself? See [For developers](#building-from-source).

## Using it

1. **Search** — start typing in the Library tab. Results rank across number,
   title, filename, and lyrics. ↑/↓ move the highlight.
2. **Preview** — the highlighted hymn's slides show on the right; ←/→ step
   through them.
3. **Project** — click **Project** on a hymn to load it into the Control tab,
   pick your output display, and **Start projecting**. ←/→ or Space advance
   slides on the projector; `B` blanks the screen; `Esc` stops. (Enter opens the
   hymn in PowerPoint instead, if you'd rather present there.)
4. **Style it** — in the Themes tab, design how slides look and set the active
   theme. The preview matches exactly what the projector shows.
5. **Add your own hymns** — Settings → *Your library folders* → **Add folder…**
   to index your own `.pptx` files alongside the built-in hymnal.

Still have PowerPoint decks you'd rather run there? **Open in PowerPoint** and
**Reveal in folder** are one click away from any search result.

---

## For developers

SDA Manager is written in **Rust** with the [Slint](https://slint.dev) GUI
toolkit. The slide projector is rendered natively by the app — it parses the
text out of `.pptx` files and draws the slides itself.

### Workspace layout

```
crates/
  hymnal-core/   # pure logic: pptx text extraction, indexing, fuzzy search,
                 # config, git sync, themes, presentation state — fully tested
  hymnal-gui/    # Slint UI wired to the core on a worker thread; the projector
                 # is a second Slint window
```

The core crate has no UI dependency, so its logic (extraction, diacritic
folding, fuzzy ranking, cache invalidation, theme load/save, slide transitions)
is unit-tested headlessly.

### Building from source

```sh
cargo build --release -p hymnal-gui      # binary at target/release/hymnal-gui
cargo run   --release -p hymnal-gui      # build and run
```

Use `--release` for daily use — search is noticeably snappier (~5–15 ms vs tens
of ms per keystroke over ~900 hymns). Dependencies are compiled optimized even
in debug builds (`profile.dev.package."*"` in the workspace `Cargo.toml`), so
`cargo run` is fine for development.

Set `RUST_LOG=hymnal_gui=debug,hymnal_core=debug` to log indexing, sync, query,
and projection activity to the console.

### How libraries work

A *library* is a folder of `.pptx` files; you can use several. Each is crawled
recursively (`~$` lock files and non-`.pptx` files are ignored). The default
library is managed via git — cloned on first run, fast-forward-pulled on every
launch so newly published hymns appear automatically. Additional libraries are
plain folders you add in Settings. A cached index means subsequent launches only
re-parse files whose modification time changed.

### Configuration

Config is a TOML file in the OS config directory:

- **macOS:** `~/Library/Application Support/org.hymnal.HymnFinder/config.toml`
- **Windows:** `%APPDATA%\hymnal\HymnFinder\config\config.toml`
- **Linux:** `~/.config/HymnFinder/config.toml`

The cloned default library lives under `…/org.hymnal.HymnFinder/default-library`,
themes under `…/themes/*.json`, and the index cache under the OS cache dir
(`…/org.hymnal.HymnFinder/index.bin`).

Point `default_repo_url` at your own published hymns repository, and/or add
libraries directly:

```toml
default_repo_url = "https://github.com/CHANGEME/imnuri-crestine.git"

[[libraries]]
name = "Imnuri Creștine"
path = "/path/to/hymns"
enabled = true
managed_by_git = false
```

### Tests

```sh
cargo test -p hymnal-core
```

Covers text extraction, diacritic folding, fuzzy ranking, index-cache
invalidation, theme JSON round-trips, and presentation-state transitions, using
real `.pptx` fixtures under `crates/hymnal-core/tests/fixtures/`. The GUI and
projector are thin and verified manually.

### Cross-compilation

- **macOS → Windows:** add the target and a mingw-w64 toolchain
  (`brew install mingw-w64`):
  ```sh
  rustup target add x86_64-pc-windows-gnu
  cargo build --release -p hymnal-gui --target x86_64-pc-windows-gnu
  ```
- **Windows → macOS** is not supported; build natively on macOS.

`git2` builds against the system libgit2 by default; the release builds enable
the `vendored-libgit2` feature for self-contained binaries.

### Releases

Releases are produced manually: run the **Release** workflow from the Actions
tab with a tag like `v0.1.0`. The install one-liners fetch the latest release.
