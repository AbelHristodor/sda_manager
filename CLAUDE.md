# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

Hymn Finder: a cross-platform desktop app to search a hymnal of PowerPoint
`.pptx` files by number, title, or lyric line, then open the deck in
PowerPoint. Search is fuzzy and diacritic-insensitive (typing `plecati` finds
*Plecaţi-vă*). Rust + the [Slint](https://slint.dev) GUI toolkit.

## Commands

```sh
cargo build --release -p hymnal-gui      # build the app -> target/release/hymnal-gui
cargo run -p hymnal-gui                  # run the app
cargo test -p hymnal-core                # run all core tests
cargo test -p hymnal-core search         # run one test file (matches test fn / file names)
cargo test -p hymnal-core -- matches_by_number   # run a single test by name
```

Cross-compile macOS → Windows (Windows → macOS is unsupported):
```sh
rustup target add x86_64-pc-windows-gnu  # needs mingw-w64: brew install mingw-w64
cargo build --release -p hymnal-gui --target x86_64-pc-windows-gnu
```
For a self-contained cross build, enable `git2`'s `vendored-libgit2` feature in
`crates/hymnal-core/Cargo.toml` (links system libgit2 by default otherwise).

## Architecture

Two-crate workspace with a deliberate logic/UI split:

- **`crates/hymnal-core`** — all logic, no UI, fully unit-tested. Pipeline:
  `pptx::extract` (unzip + parse slide XML) → `model::HymnEntry` →
  `index` (crawl + cache) → `search::Searcher` (fuzzy rank). Plus `sync` (git)
  and `library` (config + OS paths).
- **`crates/hymnal-gui`** — thin Slint shell. `build.rs` compiles
  `ui/app.slint` via `slint_build`; `slint::include_modules!()` generates the
  `AppWindow` type and its property/callback setters used in `main.rs`.

### Data flow & threading (the key thing to understand)

Slint UI handles are not `Send`, so indexing cannot block or touch the UI
directly. `main.rs` therefore:
1. Spawns a `std::thread` that loads config, git-syncs the default library,
   loads/refreshes the index cache, and sends `Vec<HymnEntry>` over an
   `mpsc::channel`.
2. Runs a Slint `Timer` (200ms `Repeated`) on the event loop that polls the
   channel with `try_recv`; on receipt it builds a `Searcher` and triggers an
   initial query. **The timer must be kept alive** (`let _timer = timer;`) or
   polling stops.
3. Wires callbacks (`query-changed`, `selection-changed`, `open-selected`,
   `reveal-selected`) that read from the shared `searcher` / `last_hits`
   (`Rc<RefCell<...>>`). Results are capped at 200 rows.

### Conventions / non-obvious decisions

- **Hymn numbers come from the filename stem, not slide text** (`pptx.rs`): the
  in-slide "Imnul N" marker is unreliably split across XML runs. Title = first
  non-marker line of the first slide; "Imnul …" and `N/M` counter lines are
  skipped (`is_marker`). Body = all slide text joined.
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
  `.pptx` is logged to stderr and skipped, never aborting the crawl. Lock files
  (`~$…`) and non-`.pptx` files are ignored.
- **Libraries & config** (`library.rs`): a library is a folder of `.pptx`
  files. The default library is git-managed (clone on first run, fast-forward
  pull). `DEFAULT_REPO_HYMNS_SUBDIR = "assets/920"` — the default repo (this
  one) holds app code alongside hymns, so the indexer points at the hymns
  subdir to avoid double-indexing fixtures. Config is TOML in the OS config dir
  (`directories` crate); a missing file yields `Config::default()`.
- **git sync** (`sync.rs`) is intentionally minimal: clone-if-absent, else
  fast-forward only (hardcoded `refs/heads/main`). No merge/rebase handling.

### Tests

Core tests live in `crates/hymnal-core/tests/` and run against **real `.pptx`
fixtures** in `tests/fixtures/` (`001.pptx`, `150.pptx`). When changing parsing,
folding, ranking, or cache logic, assert against these fixtures. `sync` and
network paths are only tested at the predicate level (no network in tests).
