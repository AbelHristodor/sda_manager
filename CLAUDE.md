# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

SDA Manager: a cross-platform desktop app (Rust + the [Slint](https://slint.dev)
GUI toolkit) with three tabs:

- **Library** — search a hymnal of PowerPoint `.pptx` files by number, title, or
  lyric line and open the deck in PowerPoint. Search is fuzzy and
  diacritic-insensitive (typing `plecati` finds *Plecaţi-vă*).
- **Video Downloader** — download a YouTube URL via a bundled `yt-dlp`/`ffmpeg`.
- **Settings** — force re-sync of the hymn library and show update status.

The app self-updates from GitHub Releases and auto-syncs its hymn library via git.

## Commands

```sh
cargo build --release -p hymnal-gui      # build the app -> target/release/hymnal-gui
cargo run -p hymnal-gui                  # run the app (debug; keeps a console on Windows)
cargo test -p hymnal-core                # run all core tests
cargo test -p hymnal-core search         # run one test file (matches test fn / file names)
cargo test -p hymnal-core -- matches_by_number   # run a single test by name
RUST_LOG=hymnal_gui=debug,hymnal_core=debug cargo run -p hymnal-gui   # verbose logs
```

`hymnal-gui` has no tests; all logic is tested in `hymnal-core`. There is no
separate lint step beyond `cargo build` warnings — run `cargo clippy` if needed.

### Releases & distribution

- Releases are built by `.github/workflows/release.yml` — **manual trigger only**
  (`workflow_dispatch` with a `version` input like `v0.1.0`). It builds natively
  on `macos-14`, `windows-latest`, and `ubuntu-latest` (no cross-compilation),
  then publishes a GitHub Release with one archive per target.
- `install.sh` (macOS/Linux) and `install.ps1` (Windows) are one-liner installers
  that fetch the latest release asset. The README has the `curl … | sh` /
  `irm … | iex` commands.
- `git2` builds with `vendored-libgit2` always on (see `hymnal-core/Cargo.toml`)
  so released binaries don't depend on a system libgit2.

## Architecture

Two-crate workspace with a deliberate logic/UI split:

- **`crates/hymnal-core`** — all logic, no UI, unit-tested. Modules:
  - `pptx` (unzip + parse slide XML) → `model::HymnEntry` → `index` (crawl +
    bincode cache) → `search::Searcher` (fuzzy rank). `fold` does diacritic folding.
  - `library` (config + OS paths), `sync` (git clone/pull), `refresh` (the
    shared boot/force-sync pipeline that ties sync + index + cache together).
  - `downloader` (yt-dlp resolution, spawn, progress parsing) and `update`
    (GitHub-Releases binary self-update).
- **`crates/hymnal-gui`** — thin Slint shell. `build.rs` compiles `ui/app.slint`
  via `slint_build` (and on Windows embeds `assets/icon.ico` via `winresource`);
  `slint::include_modules!()` generates the `AppWindow` type and the
  property/callback setters used in `main.rs`. The crate-level
  `#![windows_subsystem = "windows"]` suppresses the console window in release
  builds (no-op in debug and on non-Windows).

### Data flow & threading (the key thing to understand)

Slint UI handles are not `Send`, so background work cannot touch the UI directly.
The pattern, all driven from `main.rs`:

1. **Worker threads** do the blocking work and hand results back over
   `mpsc::channel`s — never by touching the UI. There are three flows:
   - boot: `refresh::load_library` (git sync + index) → `Vec<HymnEntry>`, then a
     background `update::check_and_stage_update`;
   - force-sync (Settings tab): `refresh::force_clean` + `load_library(force=true)`;
   - downloads: `downloader` streams `DownloadEvent`s as yt-dlp runs.
2. A single Slint `Timer` (200ms `Repeated`) on the event loop polls all channels
   with `try_recv` and updates the UI. **Keep the timer binding alive** or polling
   stops.
3. UI-thread state (`searcher`, `row_to_entry`) lives in `Rc<RefCell<…>>`.
   `row_to_entry` maps a visible result row → the entry's index inside the
   `Searcher`, so hymn bodies are never cloned per keystroke. Callbacks
   (`query-changed`, `current-changed`, `open-current`, `reveal-current`,
   `prev-slide`/`next-slide`, `force-sync`, `choose-folder`, `start-download`)
   read through that map. Results are **not** capped — `StandardListView`
   virtualizes rendering.

### UI / Slint conventions (`ui/app.slint`)

- Tabs are an `if active-tab == N` switch, so each panel component is **destroyed
  and recreated on every tab switch**. State that must survive lives on
  `AppWindow` (the Rust side), not inside a panel.
- Arrow-key nav (Library): a `FocusScope` wraps the search box and uses
  `capture-key-pressed` to claim Up/Down (list) and Left/Right (slide) *before*
  the `LineEdit` sees them. This only fires when focus is inside that scope, so
  the panel re-focuses the search box via a one-shot `Timer` on mount (calling
  `focus()` directly in `init` runs before the window is mapped and is dropped).

### Conventions / non-obvious decisions

- **Hymn numbers come from the filename stem, not slide text** (`pptx.rs`): the
  in-slide "Imnul N" marker is unreliably split across XML runs. The number is
  `Option<String>` (not a `u32`) because some hymns carry a letter suffix
  (`664b`); `search::number_sort_key` orders these numerically with the suffix as
  tiebreaker (`664` < `664a` < `664b` < `665`; unparseable/`None` sort last).
  Title = first non-marker line of the first slide; "Imnul …" and `N/M` counter
  lines are skipped. Body = all slide text joined; `slides` keeps per-slide text.
- **Diacritic folding** (`fold.rs`) is applied to both indexed text and query
  before matching — that's what makes search accent-insensitive. Extend the
  match table there for new characters.
- **Search ranking** (`search.rs`): fuzzy score (`nucleo-matcher`) × 10, plus
  large bonuses for exact (20k) / prefix (10k) / substring (5k) matches, plus a
  small per-field weight (Number > Title > Filename > Body) to break ties. Best
  field per entry wins. Tune scoring here.
- **Index cache** (`index.rs`): bincode-serialized to the OS cache dir.
  `refresh_index` reuses a cached entry when path+mtime are unchanged, else
  re-parses. A corrupt/missing cache silently falls back. One unparseable
  `.pptx` is logged and skipped, never aborting the crawl. Lock files (`~$…`)
  and non-`.pptx` files are ignored.
- **Libraries & config** (`library.rs`): a library is a folder of `.pptx` files.
  The default library is git-managed (clone on first run, fast-forward pull).
  `DEFAULT_REPO_HYMNS_SUBDIR = "assets/920"` — the default repo (this one) holds
  app code alongside hymns, so the indexer points at the hymns subdir to avoid
  double-indexing fixtures. Config is TOML in the OS config dir (`directories`
  crate); a missing file yields `Config::default()`.
- **git sync** (`sync.rs`) is intentionally minimal: clone-if-absent, else
  fast-forward only (hardcoded `refs/heads/main`). No merge/rebase. Returns a
  `SyncOutcome` so callers can skip re-indexing when nothing changed.
- **Translations** (`i18n.rs`): `Language` (En/It/Ro) selects a `Strings` struct
  (one field per UI string). `main.rs::apply_language` maps it onto the Slint
  `export global I18n` and reuses it for dynamic status messages. The choice
  persists in `Config.language`; first run detects the OS locale (`sys-locale`),
  falling back to English. Adding a UI string is a compile error until all three
  languages supply it (`Strings` has named fields). Switching is live (no
  restart); the picker lives in the Settings tab.
- **Tool binaries** (`downloader.rs`): `yt-dlp`/`ffmpeg` are located on `PATH` or
  in the app's data `tools/` dir, else downloaded from upstream releases on
  demand. Downloaded binaries are `chmod +x`'d and de-quarantined on macOS.

### Tests

Core tests live in `crates/hymnal-core/tests/` and run against **real `.pptx`
fixtures** in `tests/fixtures/` (`001.pptx`, `150.pptx`, `356.pptx`, `664b.pptx`
— the last covers letter-suffixed numbers). When changing parsing, folding,
ranking, or cache logic, assert against these fixtures. `sync` is tested with
local git repos (`tempfile`); `downloader` parsing/path logic is tested without
network. Network and self-update paths are not exercised in tests.
