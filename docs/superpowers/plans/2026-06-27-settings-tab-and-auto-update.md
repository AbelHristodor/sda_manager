# Settings Tab + Force-Sync + Binary Auto-Update Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a Settings tab with a destructive "Force sync library" button (delete clone + cache → re-clone → reindex) and a version/update line, plus a boot-time binary self-update that stages a newer release for next restart.

**Architecture:** Extract the boot sync→cache→index pipeline into a reusable `hymnal-core::refresh::load_library`, add `force_clean` (deletes clone dir + cache file) and an `update` module wrapping the `self_update` crate. The GUI already uses a custom sidebar nav (tab 0 = Library, tab 1 = Video Downloader); add tab 2 = Settings following the same pattern, wiring force-sync through the existing worker-thread + 200 ms timer channel mechanism.

**Tech Stack:** Rust, Slint 1.x (custom Sidebar/NavItem nav, not TabWidget), `self_update` crate, existing hymnal-core/hymnal-gui crates.

---

## Current-state notes (read before starting)

- `crates/hymnal-gui/ui/app.slint` already has: a `Theme` global (colors incl.
  `danger`, `panel`, `field`, `accent`, `text`, `text-dim`, `radius`, `gap`); a
  `NavItem` component; a `Sidebar` component with two `NavItem`s (Library →
  `active-tab=0`, Video Downloader → `active-tab=1`); `LibraryPanel`;
  `DownloaderPanel`; and `AppWindow` with `in-out property <int> active-tab: 0`
  and `if root.active-tab == 0/1` panel blocks.
- `crates/hymnal-gui/src/main.rs` boot worker (≈ lines 87–169) does inline:
  load config → `sync_default_library` → register default library →
  `load_cache` → `refresh_index` per library → `save_cache` → `tx.send(entries)`.
  A 200 ms `slint::Timer` drains `rx`, builds `Searcher`, calls
  `invoke_query_changed("")`.
- `hymnal-core::library` exposes `Config`, `config_path()`,
  `default_library_dir()`, `index_cache_path()`, `DEFAULT_REPO_HYMNS_SUBDIR`,
  `DEFAULT_REPO_URL`. `hymnal-core::sync::sync_default_library(url, dest)`
  clones-if-missing / ff-pulls-if-present. `hymnal-core::index` has
  `load_cache`, `save_cache`, `refresh_index`, `CACHE_VERSION`.

---

## File Structure

- `crates/hymnal-core/src/refresh.rs` (new) — `load_library(cfg, force)` and
  `force_clean(cfg)`; the reusable indexing pipeline.
- `crates/hymnal-core/src/update.rs` (new) — `check_and_stage_update()` + `UpdateOutcome`.
- `crates/hymnal-core/src/lib.rs` — declare the two new modules.
- `crates/hymnal-core/Cargo.toml` — add `self_update`.
- `crates/hymnal-core/tests/refresh_test.rs` (new) — `force_clean` tests.
- `crates/hymnal-gui/ui/app.slint` — add Settings `NavItem`, `SettingsPanel`,
  AppWindow properties/callbacks + `if active-tab == 2` block.
- `crates/hymnal-gui/src/main.rs` — use `load_library`; add force-sync worker +
  timer drain; boot update check; wire settings properties.
- `assets/icon-settings.svg` (new) — sidebar icon.

---

## Task 1: Core — extract `load_library` + `force_clean`

**Files:**
- Create: `crates/hymnal-core/src/refresh.rs`
- Modify: `crates/hymnal-core/src/lib.rs`
- Create: `crates/hymnal-core/tests/refresh_test.rs`

- [ ] **Step 1: Declare the module**

In `crates/hymnal-core/src/lib.rs`, add (with the other `pub mod` lines):
```rust
pub mod refresh;
```

- [ ] **Step 2: Write the failing test for `force_clean`**

Create `crates/hymnal-core/tests/refresh_test.rs`:
```rust
use hymnal_core::refresh::force_clean_paths;
use std::fs;

#[test]
fn force_clean_removes_clone_dir_and_cache_file() {
    let tmp = tempfile::tempdir().unwrap();
    let clone = tmp.path().join("default-library");
    let cache = tmp.path().join("index.bin");
    fs::create_dir_all(clone.join(".git")).unwrap();
    fs::write(clone.join("a.pptx"), b"x").unwrap();
    fs::write(&cache, b"cached").unwrap();

    force_clean_paths(Some(&clone), Some(&cache)).unwrap();

    assert!(!clone.exists(), "clone dir should be deleted");
    assert!(!cache.exists(), "cache file should be deleted");
}

#[test]
fn force_clean_is_ok_when_absent() {
    let tmp = tempfile::tempdir().unwrap();
    let clone = tmp.path().join("nope-dir");
    let cache = tmp.path().join("nope.bin");
    // Neither exists — must not error.
    force_clean_paths(Some(&clone), Some(&cache)).unwrap();
    force_clean_paths(None, None).unwrap();
}
```

- [ ] **Step 3: Run the test to verify it fails**

Run: `cargo test -p hymnal-core --test refresh_test`
Expected: FAIL — `force_clean_paths` not found.

- [ ] **Step 4: Implement `refresh.rs`**

Create `crates/hymnal-core/src/refresh.rs`:
```rust
//! Reusable library-loading pipeline shared by app boot and "force sync":
//! ensure the default library is present (clone/pull), index all enabled
//! libraries, and maintain the on-disk cache.

use crate::index::{load_cache, refresh_index, save_cache};
use crate::library::{
    default_library_dir, index_cache_path, Config, Library, DEFAULT_REPO_HYMNS_SUBDIR,
};
use crate::model::HymnEntry;
use crate::sync::sync_default_library;
use log::{debug, info, warn};
use std::path::Path;

/// Delete the default git-managed library clone and the index cache, given
/// explicit paths. Missing paths are not an error. Split out from `force_clean`
/// so it is unit-testable without touching real app directories.
pub fn force_clean_paths(
    clone_dir: Option<&Path>,
    cache_file: Option<&Path>,
) -> anyhow::Result<()> {
    if let Some(dir) = clone_dir {
        if dir.exists() {
            info!("force_clean: removing clone dir {}", dir.display());
            std::fs::remove_dir_all(dir)?;
        }
    }
    if let Some(file) = cache_file {
        if file.exists() {
            info!("force_clean: removing cache file {}", file.display());
            std::fs::remove_file(file)?;
        }
    }
    Ok(())
}

/// Force-clean using the standard app directories.
pub fn force_clean(_cfg: &Config) -> anyhow::Result<()> {
    force_clean_paths(default_library_dir().as_deref(), index_cache_path().as_deref())
}

/// Ensure the default library is registered in `cfg` (clones/pulls it and adds
/// a git-managed Library entry if none exists). Mutates `cfg` in place.
fn ensure_default_library(cfg: &mut Config) {
    let Some(dir) = default_library_dir() else {
        warn!("could not determine default library dir");
        return;
    };
    let fresh = !dir.join(".git").is_dir();
    info!(
        "{} default library from {} -> {}",
        if fresh { "cloning" } else { "updating" },
        cfg.default_repo_url,
        dir.display()
    );
    match sync_default_library(&cfg.default_repo_url, &dir) {
        Ok(()) => info!("clone/sync ok"),
        Err(e) => warn!("clone/sync failed: {e}"),
    }
    if !cfg.libraries.iter().any(|l| l.managed_by_git) {
        let hymns = dir.join(DEFAULT_REPO_HYMNS_SUBDIR);
        debug!("registering default library at {}", hymns.display());
        cfg.libraries.push(Library {
            name: "Imnuri Creștine".into(),
            path: hymns.to_string_lossy().to_string(),
            enabled: true,
            managed_by_git: true,
        });
    }
}

/// Load (and cache) the hymn index for all enabled libraries in `cfg`. When
/// `force` is true the on-disk cache is ignored, forcing a full re-parse.
/// `cfg` is taken by value (the caller's config is cloned into the worker).
pub fn load_library(mut cfg: Config, force: bool) -> Vec<HymnEntry> {
    ensure_default_library(&mut cfg);

    let cache = index_cache_path();
    let cached = if force {
        Vec::new()
    } else {
        cache.as_ref().and_then(|p| load_cache(p)).unwrap_or_default()
    };
    debug!("loaded {} cached entries (force={force})", cached.len());

    let mut entries = Vec::new();
    for lib in cfg.libraries.iter().filter(|l| l.enabled) {
        let root = Path::new(&lib.path);
        let before = entries.len();
        entries.extend(refresh_index(root, &lib.name, &cached));
        info!(
            "indexed library '{}' at {} -> {} hymns",
            lib.name,
            lib.path,
            entries.len() - before
        );
    }
    if entries.is_empty() && !force {
        warn!("no entries indexed; falling back to {} cached", cached.len());
        entries = cached;
    }
    info!("total {} hymns indexed", entries.len());

    if let Some(p) = cache {
        match save_cache(&p, &entries) {
            Ok(()) => debug!("wrote index cache to {}", p.display()),
            Err(e) => warn!("failed to write index cache: {e}"),
        }
    }
    entries
}
```

- [ ] **Step 5: Run the test to verify it passes**

Run: `cargo test -p hymnal-core --test refresh_test`
Expected: PASS (both tests).

- [ ] **Step 6: Run the full core suite**

Run: `cargo test -p hymnal-core`
Expected: PASS (all existing tests plus the two new ones).

- [ ] **Step 7: Commit**

```bash
git add crates/hymnal-core/src/refresh.rs crates/hymnal-core/src/lib.rs crates/hymnal-core/tests/refresh_test.rs
git commit -m "feat(core): reusable load_library + force_clean for library refresh"
```

---

## Task 2: Core — binary self-update wrapper

**Files:**
- Modify: `crates/hymnal-core/Cargo.toml`
- Create: `crates/hymnal-core/src/update.rs`
- Modify: `crates/hymnal-core/src/lib.rs`

- [ ] **Step 1: Add the `self_update` dependency**

In `crates/hymnal-core/Cargo.toml`, under `[dependencies]`, add:
```toml
self_update = { version = "0.41", default-features = false, features = ["archive-tar", "archive-zip", "compression-flate2", "rustls"] }
```
(If `0.41` fails to resolve, run `cargo search self_update` and pin the latest
0.x; the feature names above are stable across recent 0.x. `rustls` avoids a
system OpenSSL dependency for the HTTPS download.)

- [ ] **Step 2: Declare the module**

In `crates/hymnal-core/src/lib.rs`, add:
```rust
pub mod update;
```

- [ ] **Step 3: Implement `update.rs`**

Create `crates/hymnal-core/src/update.rs`:
```rust
//! Binary self-update from GitHub Releases. Compares the running version
//! (`CARGO_PKG_VERSION`) against the latest release tag and, if newer,
//! downloads the asset for this target triple and replaces the executable in
//! place. The running process keeps old code until the next restart.

use anyhow::Result;

/// GitHub repo that publishes releases (owner, name).
const REPO_OWNER: &str = "AbelHristodor";
const REPO_NAME: &str = "sda_manager";
/// Asset name stem, e.g. "hymnal-gui-x86_64-unknown-linux-gnu.tar.gz".
const BIN_NAME: &str = "hymnal-gui";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateOutcome {
    UpToDate,
    Updated { version: String },
}

/// Check releases and, if a newer version exists, download + stage it.
/// Returns the outcome; errors are returned (callers on boot should log+ignore).
pub fn check_and_stage_update() -> Result<UpdateOutcome> {
    let current = env!("CARGO_PKG_VERSION");
    let status = self_update::backends::github::Update::configure()
        .repo_owner(REPO_OWNER)
        .repo_name(REPO_NAME)
        .bin_name(BIN_NAME)
        .current_version(current)
        // The published assets are named hymnal-gui-<target>.{tar.gz,zip};
        // self_update matches the running target triple automatically.
        .no_confirm(true)
        .show_download_progress(false)
        .show_output(false)
        .build()?
        .update()?;

    if status.updated() {
        Ok(UpdateOutcome::Updated {
            version: status.version().to_string(),
        })
    } else {
        Ok(UpdateOutcome::UpToDate)
    }
}
```

- [ ] **Step 4: Build to confirm it compiles**

Run: `cargo build -p hymnal-core 2>&1 | tail -20`
Expected: `Finished`. If `self_update`'s API differs for the pinned version
(e.g. a renamed builder method), adjust to the crate's actual API while keeping
the same behavior (configure repo/bin/current_version → `.update()` →
inspect `.updated()`/`.version()`). Do NOT run the update against the network
here.

- [ ] **Step 5: Commit**

```bash
git add crates/hymnal-core/Cargo.toml crates/hymnal-core/src/update.rs crates/hymnal-core/src/lib.rs
git commit -m "feat(core): binary self-update wrapper over self_update crate"
```

---

## Task 3: GUI — Settings tab UI (Slint)

**Files:**
- Create: `assets/icon-settings.svg`
- Modify: `crates/hymnal-gui/ui/app.slint`

- [ ] **Step 1: Add a settings icon**

Create `assets/icon-settings.svg` (a simple gear, monochrome so the existing
`colorize` tint applies):
```svg
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/></svg>
```

- [ ] **Step 2: Add the Settings nav item to the Sidebar**

In `crates/hymnal-gui/ui/app.slint`, inside the `Sidebar` component's
`VerticalBox`, after the "Video Downloader" `NavItem`, add:
```slint
        NavItem {
            label: "Settings";
            icon: @image-url("../../../assets/icon-settings.svg");
            selected: root.active-tab == 2;
            clicked => { root.active-tab = 2; }
        }
```

- [ ] **Step 3: Add the `SettingsPanel` component**

In `crates/hymnal-gui/ui/app.slint`, add this component (place it just before
`export component AppWindow`):
```slint
component SettingsPanel inherits Rectangle {
    in property <string> app-version;
    in property <string> update-status;
    in property <string> sync-status;
    in property <bool> syncing;
    callback force-sync();

    background: Theme.bg;

    VerticalBox {
        padding: 20px;
        spacing: 14px;
        alignment: start;

        Text {
            text: "Settings";
            color: Theme.text;
            font-size: 20px;
            font-weight: 700;
        }

        // Version + update status.
        Rectangle {
            background: Theme.panel;
            border-radius: Theme.radius;
            border-width: 1px;
            border-color: Theme.panel-border;
            VerticalBox {
                padding: 14px;
                spacing: 6px;
                Text {
                    text: "Version " + root.app-version;
                    color: Theme.text;
                    font-weight: 600;
                }
                Text {
                    text: root.update-status;
                    color: Theme.text-dim;
                    font-size: 12px;
                }
            }
        }

        // Library maintenance.
        Rectangle {
            background: Theme.panel;
            border-radius: Theme.radius;
            border-width: 1px;
            border-color: Theme.panel-border;
            VerticalBox {
                padding: 14px;
                spacing: 8px;
                Text {
                    text: "Library";
                    color: Theme.text;
                    font-weight: 600;
                }
                Text {
                    text: "Force sync deletes the local hymn library and cache, then re-downloads and reindexes everything.";
                    color: Theme.text-dim;
                    font-size: 12px;
                    wrap: word-wrap;
                }
                Rectangle {
                    height: 36px;
                    width: 180px;
                    border-radius: Theme.radius;
                    background: root.syncing ? Theme.field : Theme.danger;
                    Text {
                        text: root.syncing ? "Syncing…" : "Force sync library";
                        color: white;
                        vertical-alignment: center;
                        horizontal-alignment: center;
                    }
                    TouchArea {
                        enabled: !root.syncing;
                        clicked => { root.force-sync(); }
                    }
                }
                Text {
                    text: root.sync-status;
                    color: Theme.text-dim;
                    font-size: 12px;
                    wrap: word-wrap;
                }
            }
        }
    }
}
```

- [ ] **Step 4: Add AppWindow properties/callbacks and the panel block**

In `export component AppWindow`, add these members (after the Downloader block
of properties):
```slint
    // Settings
    in property <string> app-version;
    in property <string> update-status;
    in property <string> sync-status;
    in property <bool> syncing;
    callback force-sync();
```

Then inside the top-level `HorizontalBox`, after the `if root.active-tab == 1:`
block, add:
```slint
        if root.active-tab == 2: SettingsPanel {
            horizontal-stretch: 1;
            app-version: root.app-version;
            update-status: root.update-status;
            sync-status: root.sync-status;
            syncing: root.syncing;
            force-sync => { root.force-sync(); }
        }
```

- [ ] **Step 5: Build to confirm the UI compiles**

Run: `cargo build -p hymnal-gui 2>&1 | tail -20`
Expected: compile ERRORS from `main.rs` (it doesn't yet set the new properties /
handle `on_force_sync`) — that's fine, Task 4 wires it. If the `.slint` itself
has a syntax error, fix it. (You can confirm slint compiled by checking the
errors are about missing Rust setters like `set_app_version`, not slint syntax.)

- [ ] **Step 6: Commit**

```bash
git add assets/icon-settings.svg crates/hymnal-gui/ui/app.slint
git commit -m "feat(gui): Settings tab UI with version + force-sync button"
```

---

## Task 4: GUI — wire force-sync, boot update check, and version

**Files:**
- Modify: `crates/hymnal-gui/src/main.rs`

- [ ] **Step 1: Set the version + initial update status, and use `load_library` on boot**

At the top of `main()` after `let ui = AppWindow::new()?;`, set the version and
an initial status:
```rust
    ui.set_app_version(env!("CARGO_PKG_VERSION").into());
    ui.set_update_status("Checking for updates…".into());
    ui.set_sync_status("".into());
```

Replace the inline boot pipeline inside the existing
`std::thread::spawn(move || { ... })` (the block that loads config, calls
`sync_default_library`, registers the library, `load_cache`/`refresh_index`/
`save_cache`, and `tx.send(entries)`) with a call to the new core helper:
```rust
    let weak = ui.as_weak();
    std::thread::spawn(move || {
        let cfg = match hymnal_core::library::config_path() {
            Some(p) => Config::load(&p).unwrap_or_else(|e| {
                warn!("config load failed ({e}); using defaults");
                Config::default()
            }),
            None => Config::default(),
        };
        let entries = hymnal_core::refresh::load_library(cfg, false);
        if tx.send(entries).is_err() {
            warn!("UI gone before index delivered");
        }
        let _ = weak.upgrade_in_event_loop(|ui| {
            ui.set_status("Library ready.".into());
        });

        // Background binary self-update check (errors logged, never block).
        match hymnal_core::update::check_and_stage_update() {
            Ok(hymnal_core::update::UpdateOutcome::UpToDate) => {
                let _ = weak.upgrade_in_event_loop(|ui| ui.set_update_status("Up to date.".into()));
            }
            Ok(hymnal_core::update::UpdateOutcome::Updated { version }) => {
                let _ = weak.upgrade_in_event_loop(move |ui| {
                    ui.set_update_status(
                        format!("Update {version} staged — restart to apply.").into(),
                    );
                });
            }
            Err(e) => {
                warn!("update check failed: {e}");
                let _ = weak.upgrade_in_event_loop(|ui| ui.set_update_status("Update check failed.".into()));
            }
        }
    });
```
Note: `weak` is moved into the closure and used three times across
`upgrade_in_event_loop` calls; that's fine because `Weak` is `Clone` and the
closure captures it by move once — but each `upgrade_in_event_loop` consumes a
clone. If the borrow checker complains, add `let weak = ui.as_weak();` clones
(`let weak_u = weak.clone();`) before each use. Keep `tx` as the existing
channel sender.

Remove now-unused imports if the compiler warns (e.g. `sync_default_library`,
`load_cache`, `refresh_index`, `save_cache`, `Library`, `default_library_dir`,
`index_cache_path` may no longer be referenced in `main.rs`). Delete only those
that are genuinely unused after this change.

- [ ] **Step 2: Add a force-sync channel and the `on_force_sync` handler**

Near the other channel (`let (tx, rx) = mpsc::channel::<Vec<HymnEntry>>();`),
add a second channel for force-sync results:
```rust
    // Force-sync delivers a freshly rebuilt index (or an error message).
    let (fs_tx, fs_rx) = mpsc::channel::<Result<Vec<HymnEntry>, String>>();
```

After the existing handler registrations (e.g. near `on_open_current`), add:
```rust
    // ---- Force sync: wipe clone+cache, re-clone, reindex (off the UI thread) ----
    let weak_fs = ui.as_weak();
    ui.on_force_sync(move || {
        let Some(ui) = weak_fs.upgrade() else { return };
        if ui.get_syncing() {
            return; // already running
        }
        ui.set_syncing(true);
        ui.set_sync_status("Re-cloning and reindexing…".into());
        let fs_tx = fs_tx.clone();
        std::thread::spawn(move || {
            let cfg = match hymnal_core::library::config_path() {
                Some(p) => Config::load(&p).unwrap_or_default(),
                None => Config::default(),
            };
            let result = match hymnal_core::refresh::force_clean(&cfg) {
                Ok(()) => Ok(hymnal_core::refresh::load_library(cfg, true)),
                Err(e) => Err(format!("force clean failed: {e}")),
            };
            let _ = fs_tx.send(result);
        });
    });
```

- [ ] **Step 3: Drain the force-sync channel in the existing 200 ms timer**

Inside the existing `timer.start(... move || { ... })` closure (which already
drains `rx` and the download channel), add a drain for `fs_rx`. Capture clones
needed (`searcher` and a weak handle) — the timer closure already clones
`searcher` as `searcher_for_timer` and a weak `weak2`; reuse the same pattern.
Add:
```rust
            if let Ok(result) = fs_rx.try_recv() {
                if let Some(ui) = weak2.upgrade() {
                    match result {
                        Ok(entries) => {
                            let n = entries.len();
                            *searcher_for_timer.borrow_mut() = Some(Searcher::new(entries));
                            ui.set_sync_status(format!("Synced — indexed {n} hymns.").into());
                            ui.invoke_query_changed("".into());
                        }
                        Err(e) => {
                            ui.set_sync_status(format!("Sync failed: {e}").into());
                        }
                    }
                    ui.set_syncing(false);
                }
            }
```
IMPORTANT: `fs_rx` must be moved into the timer closure. The timer closure is
`move`, and `rx`/`dl_rx` are already moved in; move `fs_rx` the same way (it is
captured by being referenced inside the closure). Ensure `fs_rx` is declared
before the `timer.start(...)` call so it is in scope to be captured.

- [ ] **Step 4: Build**

Run: `cargo build -p hymnal-gui 2>&1 | tail -20`
Expected: `Finished` with no errors. Fix any borrow/move errors per the notes in
Step 1 (clone `weak` handles as needed). Resolve unused-import warnings by
removing genuinely unused imports.

- [ ] **Step 5: Smoke-test boot path (logging)**

Run: `RUST_LOG=hymnal_gui=info,hymnal_core=info timeout 30 cargo run -q -p hymnal-gui 2>&1 | grep -iE "total|searcher ready|update|sync|panic|error" | head`
Expected: `total 921 hymns indexed`, `searcher ready with 921 hymns`, and an
update-check log line (either an error/timeout if offline, or a result) — the
app must not crash regardless of network.

- [ ] **Step 6: Commit**

```bash
git add crates/hymnal-gui/src/main.rs
git commit -m "feat(gui): wire force-sync worker, boot update check, version display"
```

---

## Task 5: Manual verification

**Files:** none (verification only).

- [ ] **Step 1: Run the release build**

Run: `cargo run --release -p hymnal-gui`
Verify:
- A **Settings** item appears in the sidebar; clicking it shows the panel with
  "Version 0.1.0" and an update line ("Up to date." / "Update … staged …" /
  "Update check failed.").
- **Force sync library** button: clicking it shows "Re-cloning and reindexing…",
  the button disables ("Syncing…"), and on completion shows
  "Synced — indexed 921 hymns." The Library tab still searches correctly
  afterward.
- Switching to Settings and back to Library does not break arrow-key navigation
  on the Library tab.
- Offline behavior: with no network, the app still boots and indexes from the
  existing clone/cache; the update line shows "Update check failed." and
  force-sync surfaces a "Sync failed: …" message without crashing.

- [ ] **Step 2: No code changes if all pass** — verification only; fix in Task
  1–4 code and re-run if a behavior is wrong.

---

## Self-Review Notes

- **Spec coverage:** `load_library`/`force_clean` extraction (Task 1);
  `force_clean` = delete clone dir + cache then re-clone via load_library
  (Task 1 + Task 4 force-sync); binary self-update wrapper, boot check, swallow
  errors, notify-on-restart (Task 2 + Task 4 Step 1); Settings tab with version,
  update status, force-sync button + sync status, disabled while syncing
  (Tasks 3–4); off-UI-thread force-sync via worker + 200 ms timer drain, keep
  existing searcher on failure (Task 4); `force_clean` unit tests (Task 1);
  manual GUI + offline verification (Task 5). All spec sections map to a task.
- **Deviation from spec (intentional, matches real codebase):** the spec said
  "wrap the UI in a Slint TabWidget"; the actual app already uses a custom
  Sidebar/NavItem nav with `active-tab` (tabs 0 Library, 1 Video Downloader), so
  the plan adds a third nav item + `SettingsPanel` in that existing pattern
  instead of introducing a TabWidget. Same user-facing result. The spec's
  "guard arrow keys by active tab" is already satisfied: arrow handling lives in
  `LibraryPanel`'s own `FocusScope`, which only receives keys when that panel is
  shown (`if active-tab == 0`), so no extra guard is needed — Task 5 verifies.
- **Type/name consistency:** `load_library(cfg: Config, force: bool)`,
  `force_clean(&Config)`, `force_clean_paths(Option<&Path>, Option<&Path>)`,
  `check_and_stage_update() -> Result<UpdateOutcome>`, and Slint members
  `app-version`/`update-status`/`sync-status`/`syncing`/`force-sync` (Rust:
  `set_app_version`, `set_update_status`, `set_sync_status`, `set_syncing`,
  `get_syncing`, `on_force_sync`) are used consistently across tasks.
- **Placeholder scan:** no TBD/TODO; every code step has concrete code. The one
  conditional ("if self_update API differs, adjust") is a real compatibility
  note, not a placeholder — the canonical builder call is given.
```
