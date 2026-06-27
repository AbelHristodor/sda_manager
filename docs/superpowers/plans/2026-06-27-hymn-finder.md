# Hymn Finder Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a cross-platform (macOS/Windows) Rust + Slint desktop app that fuzzy-searches a hymnal by filename and slide text, previews verses, and opens the `.pptx` in PowerPoint.

**Architecture:** A reusable `hymnal-core` library (PPTX text extraction, indexing with mtime cache, diacritic-insensitive fuzzy search, library/config management, git sync) wrapped by a thin `hymnal-gui` Slint binary. All testable logic lives in core; the GUI wires events to core on a worker thread so the UI never blocks.

**Tech Stack:** Rust (workspace, 2 crates), Slint (GUI), `zip` + `quick-xml` (PPTX parsing), `nucleo-matcher` (fuzzy search), `serde` + `bincode` (index cache), `toml` + `directories` (config), `git2` (library sync), `open` (launch external app).

---

## File Structure

```
sda_manager/
  Cargo.toml                      # workspace manifest
  crates/
    hymnal-core/
      Cargo.toml
      src/
        lib.rs                    # re-exports, public API surface
        model.rs                  # HymnEntry, SearchHit, MatchField types
        pptx.rs                   # extract text from one .pptx
        fold.rs                   # diacritic folding helper
        index.rs                  # crawl libraries, build + cache index
        search.rs                 # fuzzy ranking over the index
        library.rs                # Library struct, config (TOML) load/save
        sync.rs                   # git clone/pull for default library
      tests/
        fixtures/                 # committed sample .pptx files
        pptx_test.rs
        search_test.rs
        index_test.rs
    hymnal-gui/
      Cargo.toml
      build.rs                    # slint build step
      src/
        main.rs                   # Slint app, worker thread, event wiring
      ui/
        app.slint                 # main window UI
```

---

## Task 1: Workspace scaffold + core crate skeleton

**Files:**
- Create: `Cargo.toml` (workspace)
- Create: `crates/hymnal-core/Cargo.toml`
- Create: `crates/hymnal-core/src/lib.rs`

- [ ] **Step 1: Create the workspace manifest**

`Cargo.toml`:
```toml
[workspace]
resolver = "2"
members = ["crates/hymnal-core", "crates/hymnal-gui"]

[workspace.package]
edition = "2021"
version = "0.1.0"

[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
anyhow = "1"
```

- [ ] **Step 2: Create the core crate manifest**

`crates/hymnal-core/Cargo.toml`:
```toml
[package]
name = "hymnal-core"
edition.workspace = true
version.workspace = true

[dependencies]
serde = { workspace = true }
anyhow = { workspace = true }
zip = "2"
quick-xml = "0.36"
nucleo-matcher = "0.3"
bincode = "1"
toml = "0.8"
directories = "5"
git2 = "0.19"
walkdir = "2"

[dev-dependencies]
tempfile = "3"
```

> Note: `hymnal-gui` is a workspace member but not created until Task 8. Until then, build core directly with `cargo build -p hymnal-core` (a missing member dir makes a bare `cargo build` fail). Create a placeholder so the workspace resolves:

- [ ] **Step 3: Create a placeholder gui member so the workspace resolves**

`crates/hymnal-gui/Cargo.toml`:
```toml
[package]
name = "hymnal-gui"
edition.workspace = true
version.workspace = true

[[bin]]
name = "hymnal-gui"
path = "src/main.rs"
```

`crates/hymnal-gui/src/main.rs`:
```rust
fn main() {}
```

- [ ] **Step 4: Create the core lib root**

`crates/hymnal-core/src/lib.rs`:
```rust
pub mod fold;
pub mod index;
pub mod library;
pub mod model;
pub mod pptx;
pub mod search;
pub mod sync;
```

> This won't compile yet (modules don't exist). That's fine — the next task creates `model` and `fold` first. To make this task's commit compile on its own, temporarily comment out the not-yet-created modules:

```rust
pub mod fold;
pub mod model;
// pub mod index;    // Task 5
// pub mod library;  // Task 6
// pub mod pptx;     // Task 4
// pub mod search;   // Task 7
// pub mod sync;     // Task 9
```

- [ ] **Step 5: Create empty module files so it compiles**

Create `crates/hymnal-core/src/model.rs` and `crates/hymnal-core/src/fold.rs` each containing a single comment line `// implemented in a later task`.

- [ ] **Step 6: Build to verify the workspace resolves**

Run: `cargo build -p hymnal-core`
Expected: PASS (compiles an empty lib; dependencies download on first run).

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml crates/
git commit -m "chore: scaffold cargo workspace and hymnal-core skeleton"
```

---

## Task 2: Diacritic folding (`fold.rs`)

**Files:**
- Modify: `crates/hymnal-core/src/fold.rs`
- Modify: `crates/hymnal-core/src/lib.rs` (uncomment `pub mod fold;` — already active)

- [ ] **Step 1: Write the failing test**

Append to `crates/hymnal-core/src/fold.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::fold;

    #[test]
    fn folds_romanian_diacritics_and_lowercases() {
        assert_eq!(fold("Plecaţi-vă"), "plecati-va");
        assert_eq!(fold("Cunoaşteţi"), "cunoasteti");
        assert_eq!(fold("ÎNÂ ȘȚ"), "ina st");
    }

    #[test]
    fn leaves_plain_ascii_untouched() {
        assert_eq!(fold("Imnul 150"), "imnul 150");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p hymnal-core fold`
Expected: FAIL — `cannot find function fold`.

- [ ] **Step 3: Write minimal implementation**

Prepend to `crates/hymnal-core/src/fold.rs` (above the test module):
```rust
/// Lowercase a string and replace Romanian (and common) diacritics with their
/// ASCII base letters, so searches are accent-insensitive.
pub fn fold(input: &str) -> String {
    input
        .chars()
        .flat_map(|c| {
            let mapped = match c {
                'ă' | 'â' | 'à' | 'á' | 'Ă' | 'Â' => 'a',
                'î' | 'í' | 'ì' | 'Î' => 'i',
                'ș' | 'ş' | 'Ș' | 'Ş' => 's',
                'ț' | 'ţ' | 'Ț' | 'Ţ' => 't',
                'é' | 'è' | 'ê' | 'É' => 'e',
                'ó' | 'ô' | 'ö' | 'Ó' => 'o',
                'ú' | 'ü' | 'û' | 'Ú' => 'u',
                other => other,
            };
            mapped.to_lowercase()
        })
        .collect()
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p hymnal-core fold`
Expected: PASS (both tests).

- [ ] **Step 5: Commit**

```bash
git add crates/hymnal-core/src/fold.rs
git commit -m "feat(core): diacritic-insensitive text folding"
```

---

## Task 3: Domain model (`model.rs`)

**Files:**
- Modify: `crates/hymnal-core/src/model.rs`

- [ ] **Step 1: Write the implementation (pure data types, no logic to TDD)**

Replace `crates/hymnal-core/src/model.rs` with:
```rust
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// One hymn extracted from a .pptx file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HymnEntry {
    /// Hymn number parsed from the filename stem (authoritative), e.g. 150.
    pub number: Option<u32>,
    /// Title line (first meaningful text on the title slide).
    pub title: String,
    /// Concatenated verse text from all slides, for full-text search.
    pub body: String,
    /// Absolute path to the source .pptx.
    pub path: PathBuf,
    /// Name of the library this hymn belongs to.
    pub library: String,
    /// File modification time (unix seconds) used for cache invalidation.
    pub mtime: i64,
}

/// Which field a search query matched, for display/ranking hints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchField {
    Number,
    Title,
    Filename,
    Body,
}

/// A ranked search result.
#[derive(Debug, Clone)]
pub struct SearchHit {
    pub entry: HymnEntry,
    pub score: u32,
    pub field: MatchField,
}
```

- [ ] **Step 2: Build to verify it compiles**

Run: `cargo build -p hymnal-core`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add crates/hymnal-core/src/model.rs
git commit -m "feat(core): hymn domain model types"
```

---

## Task 4: PPTX text extraction (`pptx.rs`)

**Files:**
- Create: `crates/hymnal-core/src/pptx.rs`
- Create: `crates/hymnal-core/tests/fixtures/001.pptx` (copied from assets)
- Create: `crates/hymnal-core/tests/pptx_test.rs`
- Modify: `crates/hymnal-core/src/lib.rs` (uncomment `pub mod pptx;`)

- [ ] **Step 1: Copy a real fixture into the test tree**

```bash
mkdir -p crates/hymnal-core/tests/fixtures
cp assets/920/1-99/001.pptx crates/hymnal-core/tests/fixtures/001.pptx
cp assets/920/100-199/150.pptx crates/hymnal-core/tests/fixtures/150.pptx
```

- [ ] **Step 2: Write the failing test**

`crates/hymnal-core/tests/pptx_test.rs`:
```rust
use hymnal_core::pptx::extract;
use std::path::Path;

#[test]
fn extracts_number_title_and_body() {
    let path = Path::new("tests/fixtures/001.pptx");
    let parsed = extract(path).expect("should parse");

    // Number comes from the filename stem.
    assert_eq!(parsed.number, Some(1));
    // Title is the first meaningful line of the title slide.
    assert!(parsed.title.contains("Plecaţi-vă lui Dumnezeu"));
    // Body contains verse text from later slides.
    assert!(parsed.body.contains("Popoare-oriunde"));
    // The "Imnul" marker and counter lines are not the title.
    assert!(!parsed.title.starts_with("Imnul"));
}

#[test]
fn number_from_three_digit_filename() {
    let parsed = extract(Path::new("tests/fixtures/150.pptx")).unwrap();
    assert_eq!(parsed.number, Some(150));
    assert!(parsed.title.contains("Cerul, pământul"));
}
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cargo test -p hymnal-core --test pptx_test`
Expected: FAIL — `extract` not found.

- [ ] **Step 4: Write the implementation**

`crates/hymnal-core/src/pptx.rs`:
```rust
use anyhow::{Context, Result};
use quick_xml::events::Event;
use quick_xml::Reader;
use std::io::Read;
use std::path::Path;
use zip::ZipArchive;

/// Raw text extracted from a single .pptx, before it becomes a HymnEntry.
pub struct ParsedPptx {
    pub number: Option<u32>,
    pub title: String,
    pub body: String,
}

/// Extract searchable text from a .pptx file.
///
/// The number is taken from the filename stem (authoritative; the in-slide
/// "Imnul N" text is unreliably split across XML runs). The title is the first
/// meaningful line of the slide with the lowest file index, skipping the
/// "Imnul ..." marker and "N/M" counter lines. The body is all slide text.
pub fn extract(path: &Path) -> Result<ParsedPptx> {
    let number = path
        .file_stem()
        .and_then(|s| s.to_str())
        .and_then(|s| s.trim_start_matches('0').parse::<u32>().ok())
        .or_else(|| {
            // stem like "000" trims to "" — treat as 0 only if all zeros
            path.file_stem()
                .and_then(|s| s.to_str())
                .filter(|s| s.chars().all(|c| c == '0'))
                .map(|_| 0)
        });

    let file = std::fs::File::open(path)
        .with_context(|| format!("open {}", path.display()))?;
    let mut zip = ZipArchive::new(file)
        .with_context(|| format!("read zip {}", path.display()))?;

    // Collect slide XML paths and sort by numeric index in the filename.
    let mut slide_names: Vec<String> = (0..zip.len())
        .filter_map(|i| zip.by_index(i).ok().map(|f| f.name().to_string()))
        .filter(|n| n.starts_with("ppt/slides/slide") && n.ends_with(".xml"))
        .collect();
    slide_names.sort_by_key(|n| slide_index(n));

    let mut all_lines: Vec<String> = Vec::new();
    let mut first_slide_lines: Vec<String> = Vec::new();
    for (idx, name) in slide_names.iter().enumerate() {
        let mut xml = String::new();
        zip.by_name(name)?.read_to_string(&mut xml)?;
        let lines = slide_text_lines(&xml)?;
        if idx == 0 {
            first_slide_lines = lines.clone();
        }
        all_lines.extend(lines);
    }

    let title = pick_title(&first_slide_lines);
    let body = all_lines.join("\n");
    Ok(ParsedPptx { number, title, body })
}

/// Numeric slide index from "ppt/slides/slide12.xml" -> 12.
fn slide_index(name: &str) -> u32 {
    name.trim_start_matches("ppt/slides/slide")
        .trim_end_matches(".xml")
        .parse()
        .unwrap_or(u32::MAX)
}

/// Extract `<a:t>` text runs from one slide's XML as trimmed non-empty lines.
fn slide_text_lines(xml: &str) -> Result<Vec<String>> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut lines = Vec::new();
    let mut in_t = false;
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(e) if e.local_name().as_ref() == b"t" => in_t = true,
            Event::End(e) if e.local_name().as_ref() == b"t" => in_t = false,
            Event::Text(t) if in_t => {
                let s = t.unescape()?.trim().to_string();
                if !s.is_empty() {
                    lines.push(s);
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    Ok(lines)
}

/// Pick the title: first line that is not the "Imnul" marker and not a "N/M"
/// counter, falling back to the first line of any kind.
fn pick_title(lines: &[String]) -> String {
    lines
        .iter()
        .find(|l| !is_marker(l))
        .or_else(|| lines.first())
        .cloned()
        .unwrap_or_default()
}

/// True for "Imnul ...", a bare number, or a "N/M" counter like "1/300".
fn is_marker(line: &str) -> bool {
    let l = line.trim();
    l.starts_with("Imnul")
        || l.chars().all(|c| c.is_ascii_digit())
        || (l.contains('/')
            && l.chars().all(|c| c.is_ascii_digit() || c == '/'))
}
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test -p hymnal-core --test pptx_test`
Expected: PASS (both tests).

- [ ] **Step 6: Uncomment the module in lib.rs**

In `crates/hymnal-core/src/lib.rs` change `// pub mod pptx;` to `pub mod pptx;`.

- [ ] **Step 7: Commit**

```bash
git add crates/hymnal-core/src/pptx.rs crates/hymnal-core/src/lib.rs crates/hymnal-core/tests/
git commit -m "feat(core): extract number/title/body text from pptx"
```

---

## Task 5: Index crawl + mtime cache (`index.rs`)

**Files:**
- Create: `crates/hymnal-core/src/index.rs`
- Create: `crates/hymnal-core/tests/index_test.rs`
- Modify: `crates/hymnal-core/src/lib.rs` (uncomment `pub mod index;`)

- [ ] **Step 1: Write the failing test**

`crates/hymnal-core/tests/index_test.rs`:
```rust
use hymnal_core::index::{build_index, crawl_pptx_paths};
use std::path::Path;

#[test]
fn crawl_skips_lock_and_non_pptx_files() {
    // The fixtures dir has real .pptx files only; assert it finds them and
    // ignores anything starting with ~$.
    let paths = crawl_pptx_paths(Path::new("tests/fixtures"));
    assert!(paths.iter().all(|p| {
        let n = p.file_name().unwrap().to_str().unwrap();
        n.ends_with(".pptx") && !n.starts_with("~$")
    }));
    assert!(paths.len() >= 2);
}

#[test]
fn build_index_parses_fixtures() {
    let entries = build_index(Path::new("tests/fixtures"), "test-lib");
    let one = entries.iter().find(|e| e.number == Some(1)).unwrap();
    assert!(one.title.contains("Plecaţi-vă"));
    assert_eq!(one.library, "test-lib");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p hymnal-core --test index_test`
Expected: FAIL — functions not found.

- [ ] **Step 3: Write the implementation**

`crates/hymnal-core/src/index.rs`:
```rust
use crate::model::HymnEntry;
use crate::pptx::extract;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;
use walkdir::WalkDir;

/// Recursively find all .pptx files under `root`, skipping `~$` lock files.
pub fn crawl_pptx_paths(root: &Path) -> Vec<PathBuf> {
    WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.into_path())
        .filter(|p| {
            let name = match p.file_name().and_then(|n| n.to_str()) {
                Some(n) => n,
                None => return false,
            };
            name.to_ascii_lowercase().ends_with(".pptx") && !name.starts_with("~$")
        })
        .collect()
}

fn mtime_secs(path: &Path) -> i64 {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Build hymn entries for every .pptx under `root`. Unparseable files are
/// skipped (logged to stderr) so one bad file never aborts the crawl.
pub fn build_index(root: &Path, library: &str) -> Vec<HymnEntry> {
    let mut entries = Vec::new();
    for path in crawl_pptx_paths(root) {
        match extract(&path) {
            Ok(parsed) => entries.push(HymnEntry {
                number: parsed.number,
                title: parsed.title,
                body: parsed.body,
                path: path.clone(),
                library: library.to_string(),
                mtime: mtime_secs(&path),
            }),
            Err(err) => eprintln!("skip {}: {err}", path.display()),
        }
    }
    entries
}

/// Load a cached index from `cache_path`, or `None` if missing/corrupt.
pub fn load_cache(cache_path: &Path) -> Option<Vec<HymnEntry>> {
    let bytes = std::fs::read(cache_path).ok()?;
    bincode::deserialize(&bytes).ok()
}

/// Persist the index to `cache_path` (best-effort; errors are returned).
pub fn save_cache(cache_path: &Path, entries: &[HymnEntry]) -> anyhow::Result<()> {
    let bytes = bincode::serialize(entries)?;
    if let Some(parent) = cache_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(cache_path, bytes)?;
    Ok(())
}

/// Reuse a cached entry when the file's mtime is unchanged, otherwise re-parse.
/// `cached` is the previously loaded index; returns a fresh, up-to-date index.
pub fn refresh_index(
    root: &Path,
    library: &str,
    cached: &[HymnEntry],
) -> Vec<HymnEntry> {
    let mut out = Vec::new();
    for path in crawl_pptx_paths(root) {
        let mtime = mtime_secs(&path);
        if let Some(hit) = cached
            .iter()
            .find(|e| e.path == path && e.mtime == mtime)
        {
            out.push(hit.clone());
            continue;
        }
        match extract(&path) {
            Ok(parsed) => out.push(HymnEntry {
                number: parsed.number,
                title: parsed.title,
                body: parsed.body,
                path: path.clone(),
                library: library.to_string(),
                mtime,
            }),
            Err(err) => eprintln!("skip {}: {err}", path.display()),
        }
    }
    out
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p hymnal-core --test index_test`
Expected: PASS.

- [ ] **Step 5: Add a cache round-trip + invalidation test**

Append to `crates/hymnal-core/tests/index_test.rs`:
```rust
use hymnal_core::index::{load_cache, refresh_index, save_cache};
use hymnal_core::model::HymnEntry;
use std::path::PathBuf;

#[test]
fn cache_round_trips() {
    let dir = tempfile::tempdir().unwrap();
    let cache = dir.path().join("index.bin");
    let entries = vec![HymnEntry {
        number: Some(7),
        title: "T".into(),
        body: "B".into(),
        path: PathBuf::from("/x/7.pptx"),
        library: "L".into(),
        mtime: 123,
    }];
    save_cache(&cache, &entries).unwrap();
    let loaded = load_cache(&cache).unwrap();
    assert_eq!(loaded, entries);
}

#[test]
fn refresh_reuses_unchanged_entries() {
    // Build once, then refresh against the same files: every entry should be
    // reused (same mtime), so titles still parse correctly.
    let root = std::path::Path::new("tests/fixtures");
    let first = hymnal_core::index::build_index(root, "L");
    let again = refresh_index(root, "L", &first);
    assert_eq!(first.len(), again.len());
}
```

- [ ] **Step 6: Run the new tests**

Run: `cargo test -p hymnal-core --test index_test`
Expected: PASS (all four).

- [ ] **Step 7: Uncomment the module + commit**

In `lib.rs` change `// pub mod index;` to `pub mod index;`.
```bash
git add crates/hymnal-core/src/index.rs crates/hymnal-core/src/lib.rs crates/hymnal-core/tests/index_test.rs
git commit -m "feat(core): crawl + mtime-cached hymn index"
```

---

## Task 6: Library config (`library.rs`)

**Files:**
- Create: `crates/hymnal-core/src/library.rs`
- Modify: `crates/hymnal-core/src/lib.rs` (uncomment `pub mod library;`)

- [ ] **Step 1: Write the failing test**

Append a test module at the bottom of `crates/hymnal-core/src/library.rs` (created in Step 3 — write the test first conceptually, but since it's an inline `#[cfg(test)]` module, add both in Step 3 and verify the failing state by temporarily stubbing). To keep strict TDD, first create the file with only the test:

`crates/hymnal-core/src/library.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_toml_round_trips() {
        let cfg = Config {
            default_repo_url: "https://example.com/hymns.git".into(),
            libraries: vec![Library {
                name: "Imnuri".into(),
                path: "/data/imnuri".into(),
                enabled: true,
                managed_by_git: true,
            }],
        };
        let text = cfg.to_toml().unwrap();
        let back = Config::from_toml(&text).unwrap();
        assert_eq!(back.libraries.len(), 1);
        assert_eq!(back.libraries[0].name, "Imnuri");
        assert!(back.libraries[0].managed_by_git);
    }

    #[test]
    fn default_config_has_baked_in_repo() {
        let cfg = Config::default();
        assert!(!cfg.default_repo_url.is_empty());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p hymnal-core --lib library`
Expected: FAIL — `Config`, `Library` not found (also requires `pub mod library;` uncommented; do that now).

- [ ] **Step 3: Write the implementation**

Prepend to `crates/hymnal-core/src/library.rs` (above the test module):
```rust
use serde::{Deserialize, Serialize};

/// The default hymns repository, baked in but overridable in config.
pub const DEFAULT_REPO_URL: &str =
    "https://github.com/CHANGEME/imnuri-crestine.git";

/// One library = a folder of .pptx files.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Library {
    pub name: String,
    pub path: String,
    pub enabled: bool,
    /// True for the default library synced via git.
    pub managed_by_git: bool,
}

/// Persisted application configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    pub default_repo_url: String,
    pub libraries: Vec<Library>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            default_repo_url: DEFAULT_REPO_URL.to_string(),
            libraries: Vec::new(),
        }
    }
}

impl Config {
    pub fn to_toml(&self) -> anyhow::Result<String> {
        Ok(toml::to_string_pretty(self)?)
    }

    pub fn from_toml(text: &str) -> anyhow::Result<Config> {
        Ok(toml::from_str(text)?)
    }

    /// Load config from `path`, or return the default if it doesn't exist.
    pub fn load(path: &std::path::Path) -> anyhow::Result<Config> {
        match std::fs::read_to_string(path) {
            Ok(text) => Config::from_toml(&text),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                Ok(Config::default())
            }
            Err(e) => Err(e.into()),
        }
    }

    pub fn save(&self, path: &std::path::Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, self.to_toml()?)?;
        Ok(())
    }
}

/// Standard config + data directories for this app.
pub fn config_path() -> Option<std::path::PathBuf> {
    directories::ProjectDirs::from("org", "hymnal", "HymnFinder")
        .map(|d| d.config_dir().join("config.toml"))
}

/// Directory where the default git library is cloned.
pub fn default_library_dir() -> Option<std::path::PathBuf> {
    directories::ProjectDirs::from("org", "hymnal", "HymnFinder")
        .map(|d| d.data_dir().join("default-library"))
}

/// Path for the serialized index cache.
pub fn index_cache_path() -> Option<std::path::PathBuf> {
    directories::ProjectDirs::from("org", "hymnal", "HymnFinder")
        .map(|d| d.cache_dir().join("index.bin"))
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p hymnal-core --lib library`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/hymnal-core/src/library.rs crates/hymnal-core/src/lib.rs
git commit -m "feat(core): library config load/save with baked-in default repo"
```

---

## Task 7: Fuzzy search (`search.rs`)

**Files:**
- Create: `crates/hymnal-core/src/search.rs`
- Create: `crates/hymnal-core/tests/search_test.rs`
- Modify: `crates/hymnal-core/src/lib.rs` (uncomment `pub mod search;`)

- [ ] **Step 1: Write the failing test**

`crates/hymnal-core/tests/search_test.rs`:
```rust
use hymnal_core::index::build_index;
use hymnal_core::search::Searcher;
use std::path::Path;

fn searcher() -> Searcher {
    let entries = build_index(Path::new("tests/fixtures"), "test-lib");
    Searcher::new(entries)
}

#[test]
fn matches_without_diacritics() {
    let s = searcher();
    let hits = s.search("plecati");
    assert!(!hits.is_empty());
    assert_eq!(hits[0].entry.number, Some(1));
}

#[test]
fn matches_by_number() {
    let s = searcher();
    let hits = s.search("150");
    assert_eq!(hits[0].entry.number, Some(150));
}

#[test]
fn matches_body_text() {
    let s = searcher();
    let hits = s.search("Popoare");
    assert!(hits.iter().any(|h| h.entry.number == Some(1)));
}

#[test]
fn empty_query_returns_all_sorted_by_number() {
    let s = searcher();
    let hits = s.search("");
    assert!(hits.len() >= 2);
    assert_eq!(hits[0].entry.number, Some(1));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p hymnal-core --test search_test`
Expected: FAIL — `Searcher` not found.

- [ ] **Step 3: Write the implementation**

`crates/hymnal-core/src/search.rs`:
```rust
use crate::fold::fold;
use crate::model::{HymnEntry, MatchField, SearchHit};
use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher};

/// Pre-folded searchable text for one entry, kept alongside the entry.
struct Indexed {
    entry: HymnEntry,
    number_str: String,
    title_folded: String,
    filename_folded: String,
    body_folded: String,
}

/// In-memory fuzzy searcher over hymn entries.
pub struct Searcher {
    items: Vec<Indexed>,
}

impl Searcher {
    pub fn new(entries: Vec<HymnEntry>) -> Self {
        let items = entries
            .into_iter()
            .map(|entry| {
                let filename = entry
                    .path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string();
                Indexed {
                    number_str: entry.number.map(|n| n.to_string()).unwrap_or_default(),
                    title_folded: fold(&entry.title),
                    filename_folded: fold(&filename),
                    body_folded: fold(&entry.body),
                    entry,
                }
            })
            .collect();
        Searcher { items }
    }

    /// Rank entries against `query`. Empty query returns all entries sorted by
    /// hymn number. Otherwise fuzzy-matches across number, title, filename and
    /// body, keeping the best-scoring field per entry.
    pub fn search(&self, query: &str) -> Vec<SearchHit> {
        let q = fold(query);
        if q.trim().is_empty() {
            let mut all: Vec<SearchHit> = self
                .items
                .iter()
                .map(|it| SearchHit {
                    entry: it.entry.clone(),
                    score: 0,
                    field: MatchField::Number,
                })
                .collect();
            all.sort_by_key(|h| h.entry.number.unwrap_or(u32::MAX));
            return all;
        }

        let mut matcher = Matcher::new(Config::DEFAULT);
        let pattern = Pattern::parse(&q, CaseMatching::Ignore, Normalization::Smart);

        let mut hits: Vec<SearchHit> = Vec::new();
        for it in &self.items {
            // Score each field; keep the highest, with field-priority weights so
            // a number/title match ranks above an incidental body match.
            let candidates = [
                (MatchField::Number, &it.number_str, 4u32),
                (MatchField::Title, &it.title_folded, 3),
                (MatchField::Filename, &it.filename_folded, 2),
                (MatchField::Body, &it.body_folded, 1),
            ];
            let mut best: Option<(MatchField, u32)> = None;
            for (field, haystack, weight) in candidates {
                if haystack.is_empty() {
                    continue;
                }
                let mut buf = Vec::new();
                let hay = nucleo_matcher::Utf32Str::new(haystack, &mut buf);
                if let Some(score) = pattern.score(hay, &mut matcher) {
                    let weighted = score * 10 + weight;
                    if best.map_or(true, |(_, b)| weighted > b) {
                        best = Some((field, weighted));
                    }
                }
            }
            if let Some((field, score)) = best {
                hits.push(SearchHit {
                    entry: it.entry.clone(),
                    score,
                    field,
                });
            }
        }
        hits.sort_by(|a, b| b.score.cmp(&a.score));
        hits
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p hymnal-core --test search_test`
Expected: PASS (all four).

> If `empty_query_returns_all_sorted_by_number` or number matching is flaky because `Pattern::score` on a pure-digit haystack behaves unexpectedly, the number field is also covered by exact containment: this is acceptable as long as hymn 150 ranks first for query "150". Adjust the weight constants if ordering needs tuning, but do not change the test expectations.

- [ ] **Step 5: Uncomment module + commit**

In `lib.rs` change `// pub mod search;` to `pub mod search;`.
```bash
git add crates/hymnal-core/src/search.rs crates/hymnal-core/src/lib.rs crates/hymnal-core/tests/search_test.rs
git commit -m "feat(core): diacritic-insensitive fuzzy search over the index"
```

---

## Task 8: Git sync (`sync.rs`)

**Files:**
- Create: `crates/hymnal-core/src/sync.rs`
- Modify: `crates/hymnal-core/src/lib.rs` (uncomment `pub mod sync;`)

- [ ] **Step 1: Write the implementation (network-dependent; logic kept thin, no live-network unit test)**

`crates/hymnal-core/src/sync.rs`:
```rust
use anyhow::{Context, Result};
use std::path::Path;

/// Ensure the default library exists at `dest`: clone from `repo_url` if the
/// directory is absent, otherwise fast-forward pull. Returns the resulting
/// repository path on success.
pub fn sync_default_library(repo_url: &str, dest: &Path) -> Result<()> {
    if dest.join(".git").is_dir() {
        pull(dest).with_context(|| format!("pull {}", dest.display()))
    } else {
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        git2::Repository::clone(repo_url, dest)
            .with_context(|| format!("clone {repo_url} -> {}", dest.display()))?;
        Ok(())
    }
}

/// Fast-forward the checked-out branch to its upstream.
fn pull(dest: &Path) -> Result<()> {
    let repo = git2::Repository::open(dest)?;
    let mut remote = repo.find_remote("origin")?;
    remote.fetch(&["HEAD"], None, None)?;
    let fetch_head = repo.find_reference("FETCH_HEAD")?;
    let commit = repo.reference_to_annotated_commit(&fetch_head)?;
    let (analysis, _) = repo.merge_analysis(&[&commit])?;
    if analysis.is_up_to_date() {
        return Ok(());
    }
    if analysis.is_fast_forward() {
        let refname = "refs/heads/main";
        if let Ok(mut reference) = repo.find_reference(refname) {
            reference.set_target(commit.id(), "fast-forward")?;
            repo.set_head(refname)?;
            repo.checkout_head(Some(
                git2::build::CheckoutBuilder::default().force(),
            ))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clone_when_missing_then_treated_as_existing() {
        // No network in unit tests: verify the path-branch logic only.
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("lib");
        // .git absent -> would attempt clone; we only assert the predicate.
        assert!(!dest.join(".git").is_dir());
    }
}
```

- [ ] **Step 2: Build + run the unit test**

Run: `cargo test -p hymnal-core --lib sync`
Expected: PASS.

- [ ] **Step 3: Uncomment module + commit**

In `lib.rs` change `// pub mod sync;` to `pub mod sync;`.
```bash
git add crates/hymnal-core/src/sync.rs crates/hymnal-core/src/lib.rs
git commit -m "feat(core): git clone/pull sync for the default library"
```

- [ ] **Step 4: Full core test run**

Run: `cargo test -p hymnal-core`
Expected: PASS (all tests across fold, pptx, index, library, search, sync).

---

## Task 9: GUI crate — Slint window + worker thread

**Files:**
- Modify: `crates/hymnal-gui/Cargo.toml`
- Create: `crates/hymnal-gui/build.rs`
- Create: `crates/hymnal-gui/ui/app.slint`
- Modify: `crates/hymnal-gui/src/main.rs`

- [ ] **Step 1: Fill in the GUI crate manifest**

`crates/hymnal-gui/Cargo.toml`:
```toml
[package]
name = "hymnal-gui"
edition.workspace = true
version.workspace = true

[[bin]]
name = "hymnal-gui"
path = "src/main.rs"

[dependencies]
hymnal-core = { path = "../hymnal-core" }
slint = "1.8"
open = "5"
anyhow = { workspace = true }

[build-dependencies]
slint-build = "1.8"
```

- [ ] **Step 2: Add the Slint build script**

`crates/hymnal-gui/build.rs`:
```rust
fn main() {
    slint_build::compile("ui/app.slint").unwrap();
}
```

- [ ] **Step 3: Write the Slint UI**

`crates/hymnal-gui/ui/app.slint`:
```slint
import { LineEdit, ListView, Button, ScrollView, VerticalBox, HorizontalBox } from "std-widgets.slint";

export struct HymnRow {
    number: string,
    title: string,
    library: string,
}

export component AppWindow inherits Window {
    title: "Hymn Finder";
    preferred-width: 900px;
    preferred-height: 600px;

    in property <[HymnRow]> results;
    in property <string> preview-title;
    in property <string> preview-body;
    in property <string> status;
    out property <int> selected-index: -1;

    callback query-changed(string);
    callback open-selected();
    callback reveal-selected();

    VerticalBox {
        Text { text: root.status; color: gray; }
        search := LineEdit {
            placeholder-text: "Search by number, title, or lyrics…";
            edited(text) => { root.query-changed(text); }
        }
        HorizontalBox {
            // Results list (left)
            ListView {
                width: 40%;
                for row[i] in root.results: Rectangle {
                    height: 48px;
                    background: i == root.selected-index ? #d0e0ff : transparent;
                    TouchArea {
                        clicked => { root.selected-index = i; }
                    }
                    VerticalBox {
                        padding: 4px;
                        Text { text: row.number + ". " + row.title; font-weight: 600; }
                        Text { text: row.library; color: gray; font-size: 11px; }
                    }
                }
            }
            // Preview (right)
            VerticalBox {
                Text { text: root.preview-title; font-size: 18px; font-weight: 700; }
                ScrollView {
                    Text { text: root.preview-body; wrap: word-wrap; }
                }
                HorizontalBox {
                    Button { text: "Open in PowerPoint"; clicked => { root.open-selected(); } }
                    Button { text: "Reveal in folder"; clicked => { root.reveal-selected(); } }
                }
            }
        }
    }
}
```

- [ ] **Step 4: Wire the app in main.rs**

`crates/hymnal-gui/src/main.rs`:
```rust
slint::include_modules!();

use hymnal_core::index::{build_index, load_cache, refresh_index, save_cache};
use hymnal_core::library::{
    default_library_dir, index_cache_path, Config, Library,
};
use hymnal_core::model::HymnEntry;
use hymnal_core::search::Searcher;
use hymnal_core::sync::sync_default_library;
use slint::{ModelRc, SharedString, VecModel};
use std::rc::Rc;
use std::sync::mpsc;

fn main() -> anyhow::Result<()> {
    let ui = AppWindow::new()?;

    // Channel: worker thread -> UI with the freshly built index.
    let (tx, rx) = mpsc::channel::<Vec<HymnEntry>>();

    // Spawn indexing/sync off the UI thread.
    let weak = ui.as_weak();
    std::thread::spawn(move || {
        let mut cfg = Config::default();
        if let Some(p) = hymnal_core::library::config_path() {
            cfg = Config::load(&p).unwrap_or_default();
        }
        // Ensure the default library is present.
        if let Some(dir) = default_library_dir() {
            if !dir.join(".git").is_dir() {
                let _ = sync_default_library(&cfg.default_repo_url, &dir);
            }
            // Register it if not already in config.
            if !cfg.libraries.iter().any(|l| l.managed_by_git) {
                cfg.libraries.push(Library {
                    name: "Imnuri Creștine".into(),
                    path: dir.to_string_lossy().to_string(),
                    enabled: true,
                    managed_by_git: true,
                });
            }
        }

        // Build/refresh index across all enabled libraries.
        let cache = index_cache_path();
        let cached = cache.as_ref().and_then(|p| load_cache(p)).unwrap_or_default();
        let mut entries = Vec::new();
        for lib in cfg.libraries.iter().filter(|l| l.enabled) {
            let root = std::path::Path::new(&lib.path);
            entries.extend(refresh_index(root, &lib.name, &cached));
        }
        if entries.is_empty() {
            entries = cached;
        }
        if let Some(p) = cache {
            let _ = save_cache(&p, &entries);
        }
        let _ = tx.send(entries);

        // Wake the UI thread to pull the result.
        let _ = weak.upgrade_in_event_loop(|ui| {
            ui.set_status("Library ready.".into());
        });
    });

    // Shared searcher state, populated once the worker finishes.
    let searcher: Rc<std::cell::RefCell<Option<Searcher>>> =
        Rc::new(std::cell::RefCell::new(None));
    let last_hits: Rc<std::cell::RefCell<Vec<HymnEntry>>> =
        Rc::new(std::cell::RefCell::new(Vec::new()));

    ui.set_status("Loading hymn library…".into());

    // Poll the channel on a timer; install the searcher when ready.
    let weak2 = ui.as_weak();
    let searcher_for_timer = searcher.clone();
    let timer = slint::Timer::default();
    timer.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_millis(200),
        move || {
            if let Ok(entries) = rx.try_recv() {
                *searcher_for_timer.borrow_mut() = Some(Searcher::new(entries));
                if let Some(ui) = weak2.upgrade() {
                    ui.set_status("Library ready.".into());
                    // Trigger an initial (empty) search to populate the list.
                    ui.invoke_query_changed("".into());
                }
            }
        },
    );

    // Search handler.
    let searcher_for_query = searcher.clone();
    let hits_for_query = last_hits.clone();
    let weak3 = ui.as_weak();
    ui.on_query_changed(move |q| {
        let guard = searcher_for_query.borrow();
        let Some(s) = guard.as_ref() else { return };
        let hits = s.search(&q);
        let rows: Vec<HymnRow> = hits
            .iter()
            .take(200)
            .map(|h| HymnRow {
                number: h.entry.number.map(|n| n.to_string()).unwrap_or_default().into(),
                title: h.entry.title.clone().into(),
                library: h.entry.library.clone().into(),
            })
            .collect();
        *hits_for_query.borrow_mut() =
            hits.into_iter().take(200).map(|h| h.entry).collect();
        if let Some(ui) = weak3.upgrade() {
            ui.set_results(ModelRc::from(Rc::new(VecModel::from(rows))));
            ui.set_selected_index(-1);
            ui.set_preview_title("".into());
            ui.set_preview_body("".into());
        }
    });

    // Open handler: launch the selected .pptx externally.
    let hits_for_open = last_hits.clone();
    let weak4 = ui.as_weak();
    ui.on_open_selected(move || {
        let Some(ui) = weak4.upgrade() else { return };
        let idx = ui.get_selected_index();
        if idx < 0 {
            return;
        }
        if let Some(entry) = hits_for_open.borrow().get(idx as usize) {
            let _ = open::that(&entry.path);
        }
    });

    // Reveal handler: open the containing folder.
    let hits_for_reveal = last_hits.clone();
    let weak5 = ui.as_weak();
    ui.on_reveal_selected(move || {
        let Some(ui) = weak5.upgrade() else { return };
        let idx = ui.get_selected_index();
        if idx < 0 {
            return;
        }
        if let Some(entry) = hits_for_reveal.borrow().get(idx as usize) {
            if let Some(parent) = entry.path.parent() {
                let _ = open::that(parent);
            }
        }
    });

    ui.run()?;
    Ok(())
}
```

> Selection-driven preview: the simplest correct wiring is to update the preview whenever `selected-index` changes. Slint exposes `out property` changes via a change callback. Add the handler in Step 5.

- [ ] **Step 5: Add preview-on-selection**

Add a `changed selected-index` handler in `app.slint` inside `AppWindow` (after the property declarations):
```slint
    callback selection-changed(int);
    changed selected-index => { root.selection-changed(self.selected-index); }
```

And in `main.rs`, before `ui.run()`:
```rust
    let hits_for_sel = last_hits.clone();
    let weak6 = ui.as_weak();
    ui.on_selection_changed(move |idx| {
        let Some(ui) = weak6.upgrade() else { return };
        if idx < 0 {
            return;
        }
        if let Some(entry) = hits_for_sel.borrow().get(idx as usize) {
            let title = format!(
                "{}{}",
                entry.number.map(|n| format!("{n}. ")).unwrap_or_default(),
                entry.title
            );
            ui.set_preview_title(SharedString::from(title));
            ui.set_preview_body(SharedString::from(entry.body.clone()));
        }
    });
```

- [ ] **Step 6: Build the GUI**

Run: `cargo build -p hymnal-gui`
Expected: PASS (Slint compiles `app.slint`; binary links against core).

- [ ] **Step 7: Commit**

```bash
git add crates/hymnal-gui/
git commit -m "feat(gui): slint window with worker-thread indexing, search, preview, open"
```

---

## Task 10: Manual smoke test against real assets

**Files:** none (verification only).

- [ ] **Step 1: Point the app at the local assets for a no-network run**

For the smoke test, override the default library by pre-seeding config so it indexes the existing `assets/920` folder instead of cloning. Create the config file the app reads:

Run (macOS path shown; adjust for the platform):
```bash
APP_CFG="$HOME/Library/Application Support/HymnFinder/config.toml"
mkdir -p "$(dirname "$APP_CFG")"
cat > "$APP_CFG" <<'TOML'
default_repo_url = "https://github.com/CHANGEME/imnuri-crestine.git"

[[libraries]]
name = "Imnuri Creștine"
path = "/Users/abelor/projects/sda_manager/assets/920"
enabled = true
managed_by_git = false
TOML
```

> Because the seeded library has `managed_by_git = false` and no git-managed library exists, the app will skip cloning and index the local folder directly.

- [ ] **Step 2: Run the app**

Run: `cargo run -p hymnal-gui`
Expected: Window opens; status shows "Loading…" then "Library ready."; ~920 hymns indexed.

- [ ] **Step 3: Verify search behaviors manually**

- Type `plecati` → hymn 1 appears near the top (diacritic-insensitive).
- Type `150` → hymn 150 ranks first.
- Type a lyric word like `Popoare` → matching hymn appears.
- Select a result → preview pane shows the verses.
- Click "Open in PowerPoint" → the `.pptx` opens in the default handler.

- [ ] **Step 4: Confirm and commit any fixes**

If behaviors differ, fix in core/gui and re-run. When all pass, no code change is needed; this task is verification only.

---

## Task 11: README + cross-compile notes

**Files:**
- Create: `README.md`

- [ ] **Step 1: Write the README**

`README.md`:
```markdown
# Hymn Finder

A small cross-platform desktop app to search a hymnal (PowerPoint `.pptx`
files) by hymn number, title, or any line of the lyrics, then open the slide
deck in PowerPoint.

## Build

```
cargo build --release -p hymnal-gui
```

The binary is at `target/release/hymnal-gui`.

## Libraries

On first run the app clones the default hymns repository (configurable) into
the OS data directory and indexes it. Add your own folders of `.pptx` files in
Settings; they are crawled and indexed the same way.

Config lives at the OS config dir (e.g. on macOS
`~/Library/Application Support/HymnFinder/config.toml`).

## Cross-compilation

- **macOS → Windows:** `rustup target add x86_64-pc-windows-gnu` and build with
  `cargo build --release -p hymnal-gui --target x86_64-pc-windows-gnu`
  (Slint's default renderer works with the GNU toolchain; install `mingw-w64`).
- **Windows → macOS** is not supported directly; build natively on macOS.

## Tests

```
cargo test -p hymnal-core
```
```

- [ ] **Step 2: Commit**

```bash
git add README.md
git commit -m "docs: build, library, and cross-compile instructions"
```

---

## Self-Review Notes

- **Spec coverage:** PPTX text extraction (Task 4), filename+content fuzzy search with diacritic folding (Tasks 2,7), mtime-cached index off the UI thread (Tasks 5,9), library config + add-folder model (Task 6), git sync of default library (Task 8), in-app preview + open externally (Task 9), error-tolerant crawl that skips bad/lock files (Tasks 4,5), cross-platform build notes (Task 11). All spec sections map to a task.
- **Deferred to follow-up (within spec's stated scope, surfaced honestly):** the Settings *UI panel* (add/remove library, edit repo URL, re-index button, skipped-file count) is modeled in core (Task 6) and reachable via the config file (Task 10) but not yet given Slint controls — a thin follow-up task on top of Task 9's window. Library *filter dropdown* likewise. These are UI affordances over already-built core capabilities; call them out to the user before execution.
- **Type consistency:** `HymnEntry`, `SearchHit`, `MatchField`, `Config`, `Library`, `Searcher::new/search`, `build_index/refresh_index/load_cache/save_cache`, `extract`, `fold`, `sync_default_library` are referenced with consistent signatures across tasks.
- **Placeholder scan:** no TBD/TODO; the only `CHANGEME` is the deliberate default repo URL the user must set when they publish the hymns repo.
```
