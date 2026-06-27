# Hymn Finder

A small cross-platform desktop app to search a hymnal (PowerPoint `.pptx`
files) by hymn number, title, or any line of the lyrics, then open the slide
deck in PowerPoint.

Built in Rust with the [Slint](https://slint.dev) GUI toolkit. Search is
fuzzy and diacritic-insensitive, so typing `plecati` finds *Plecaţi-vă*.

## Workspace layout

```
crates/
  hymnal-core/   # library: pptx text extraction, indexing, search, config, git sync
  hymnal-gui/    # binary: Slint UI wired to the core on a worker thread
```

## Build

```
cargo build --release -p hymnal-gui
```

The binary is at `target/release/hymnal-gui`.

## Run

```
cargo run -p hymnal-gui
```

On first run the app reads its config (see below). If no git-managed library
is configured it clones the default hymns repository into the OS data
directory and indexes it. Indexing runs on a background thread, so the window
stays responsive; a status line shows progress. Subsequent launches reuse a
cached index and only re-parse files whose modification time changed.

- **Search:** type in the search bar — results rank across hymn number, title,
  filename, and lyrics, accent-insensitively.
- **Preview:** select a result to read its extracted verses in the right pane.
- **Open in PowerPoint:** launches the `.pptx` in the OS default handler.
- **Reveal in folder:** opens the containing folder.

## Libraries

A *library* is a folder of `.pptx` files. You can use more than one. Each is
crawled recursively; `~$` lock files and non-`.pptx` files are ignored.

The default library is managed via git (clone on first run, fast-forward pull
to update). Additional libraries are plain folders you point the app at.

## Configuration

Config is a TOML file in the OS config directory:

- **macOS:** `~/Library/Application Support/org.hymnal.HymnFinder/config.toml`
- **Windows:** `%APPDATA%\hymnal\HymnFinder\config\config.toml`

The cloned default library lives next to it under `…/org.hymnal.HymnFinder/default-library`, and the index cache under `~/Library/Caches/org.hymnal.HymnFinder/index.bin`. The app fast-forward-pulls the default library on each launch, so newly published hymns appear automatically.

Example (index a local folder directly, no git):

```toml
default_repo_url = "https://github.com/CHANGEME/imnuri-crestine.git"

[[libraries]]
name = "Imnuri Creștine"
path = "/path/to/hymns"
enabled = true
managed_by_git = false
```

Set `default_repo_url` to your published hymns repository. The index cache and
the cloned default library live in the OS cache and data directories
respectively.

## Cross-compilation

- **macOS → Windows:** add the target and a mingw-w64 toolchain, then build:
  ```
  rustup target add x86_64-pc-windows-gnu
  cargo build --release -p hymnal-gui --target x86_64-pc-windows-gnu
  ```
  Slint's default (Femtovg/Skia software or GL) renderer works with the GNU
  toolchain; install `mingw-w64` (e.g. `brew install mingw-w64`).
- **Windows → macOS** is not supported directly; build natively on macOS.

`git2` builds against the system libgit2 by default. For a self-contained
cross build, enable the vendored library in `crates/hymnal-core/Cargo.toml`:
`git2 = { version = "0.19", features = ["vendored-libgit2"] }`.

## Tests

```
cargo test -p hymnal-core
```

Core logic (text extraction, diacritic folding, fuzzy ranking, index cache
invalidation) is covered by unit and integration tests using real `.pptx`
fixtures under `crates/hymnal-core/tests/fixtures/`.
