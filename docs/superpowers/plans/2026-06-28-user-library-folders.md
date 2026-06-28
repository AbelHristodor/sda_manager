# User Library Folders Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let users add their own folders of `.pptx` hymns (managed in the Settings tab: add / enable-disable / remove), indexed alongside the default git-managed library, taking effect immediately via a local-only re-index.

**Architecture:** Reuse `Config.libraries: Vec<Library>`. A user folder is a `Library { managed_by_git: false, enabled: true }`. Core gains pure helpers (`add_user_library`, `remove_user_library`, `set_library_enabled`, `library_available`) in `library.rs`; `refresh::index_enabled` already indexes all enabled libraries and `refresh::load_local` already re-indexes locally with no network. The GUI adds a "Libraries" section to `SettingsPanel` and wires add/remove/toggle through the existing worker-thread + 200ms-timer pattern, re-indexing via `load_local`.

**Tech Stack:** Rust, Slint 1.8, `rfd` (folder picker, already a dep), serde/toml (config), `directories`/`dirs` (paths).

---

## File Structure

- `crates/hymnal-core/src/library.rs` — **MODIFY**: add `add_user_library`, `remove_user_library`, `set_library_enabled`, `library_available` + unit tests.
- `crates/hymnal-gui/ui/app.slint` — **MODIFY**: add `LibraryRow` struct, `libraries`/`library-status` properties and `add-library`/`remove-library`/`set-library-enabled` callbacks on `AppWindow`; add a "Libraries" section to `SettingsPanel`.
- `crates/hymnal-gui/src/main.rs` — **MODIFY**: build the `libraries` model from the shared config, wire the three callbacks, and drain a new local-reindex channel in the existing timer.

---

## Task 1: Core — `library_available` + `set_library_enabled`

**Files:**
- Modify: `crates/hymnal-core/src/library.rs`

- [ ] **Step 1: Write the failing tests**

Add to the `tests` module at the bottom of `crates/hymnal-core/src/library.rs`:

```rust
    #[test]
    fn library_available_true_for_existing_dir() {
        let dir = tempfile::tempdir().unwrap();
        assert!(library_available(&dir.path().to_string_lossy()));
    }

    #[test]
    fn library_available_false_for_missing_dir() {
        assert!(!library_available("/no/such/path/hopefully/12345"));
    }

    #[test]
    fn set_library_enabled_flips_flag() {
        let mut cfg = Config {
            default_repo_url: "x".into(),
            libraries: vec![Library {
                name: "U".into(),
                path: "/tmp/u".into(),
                enabled: true,
                managed_by_git: false,
            }],
            download_dir: None,
        };
        set_library_enabled(&mut cfg, "/tmp/u", false);
        assert!(!cfg.libraries[0].enabled);
        set_library_enabled(&mut cfg, "/tmp/u", true);
        assert!(cfg.libraries[0].enabled);
    }

    #[test]
    fn set_library_enabled_can_disable_default() {
        let mut cfg = Config {
            default_repo_url: "x".into(),
            libraries: vec![Library {
                name: "Default".into(),
                path: "/data/default".into(),
                enabled: true,
                managed_by_git: true,
            }],
            download_dir: None,
        };
        set_library_enabled(&mut cfg, "/data/default", false);
        assert!(!cfg.libraries[0].enabled);
    }
```

`tempfile` is already a dev-dependency; confirm `use super::*;` is at the top of the `tests` module (it is in the existing tests).

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p hymnal-core library_available_true_for_existing_dir`
Expected: FAIL — `library_available` / `set_library_enabled` not defined (compile error).

- [ ] **Step 3: Implement**

Add to `crates/hymnal-core/src/library.rs` (module level, near the other helpers, e.g. after `downloads_dir`):

```rust
/// Whether a library folder is currently reachable on disk. Used for the
/// Settings "unavailable" marker; an unreachable folder is simply skipped at
/// index time (the crawl yields nothing) rather than being an error.
pub fn library_available(path: &str) -> bool {
    std::path::Path::new(path).is_dir()
}

/// Set the `enabled` flag of the library whose `path` matches. Works on any
/// library, including the default — the default may be disabled (just not
/// removed). No-op if no library has that path.
pub fn set_library_enabled(cfg: &mut Config, path: &str, enabled: bool) {
    for lib in cfg.libraries.iter_mut() {
        if lib.path == path {
            lib.enabled = enabled;
        }
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p hymnal-core library`
Expected: PASS — all `library` module tests green.

- [ ] **Step 5: Commit**

```bash
git add crates/hymnal-core/src/library.rs
git commit -m "feat(core): add library_available and set_library_enabled helpers"
```

---

## Task 2: Core — `add_user_library` + `remove_user_library`

**Files:**
- Modify: `crates/hymnal-core/src/library.rs`

- [ ] **Step 1: Write the failing tests**

Add to the `tests` module:

```rust
    #[test]
    fn add_user_library_uses_folder_name_and_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("MyHymns");
        std::fs::create_dir(&sub).unwrap();
        let mut cfg = Config::default();
        add_user_library(&mut cfg, &sub).unwrap();
        assert_eq!(cfg.libraries.len(), 1);
        let lib = &cfg.libraries[0];
        assert_eq!(lib.name, "MyHymns");
        assert!(lib.enabled);
        assert!(!lib.managed_by_git);
    }

    #[test]
    fn add_user_library_rejects_duplicate_path() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("Dup");
        std::fs::create_dir(&sub).unwrap();
        let mut cfg = Config::default();
        add_user_library(&mut cfg, &sub).unwrap();
        // Adding the same folder again (even via a non-canonical spelling) fails.
        let messy = sub.join(".").join("..").join("Dup");
        assert!(add_user_library(&mut cfg, &messy).is_err());
        assert_eq!(cfg.libraries.len(), 1);
    }

    #[test]
    fn remove_user_library_removes_user_but_not_default() {
        let mut cfg = Config {
            default_repo_url: "x".into(),
            libraries: vec![
                Library { name: "Default".into(), path: "/data/default".into(), enabled: true, managed_by_git: true },
                Library { name: "Mine".into(), path: "/tmp/mine".into(), enabled: true, managed_by_git: false },
            ],
            download_dir: None,
        };
        // Removing a user library works.
        remove_user_library(&mut cfg, "/tmp/mine");
        assert_eq!(cfg.libraries.len(), 1);
        // Attempting to remove the default is refused.
        remove_user_library(&mut cfg, "/data/default");
        assert_eq!(cfg.libraries.len(), 1);
        assert!(cfg.libraries[0].managed_by_git);
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p hymnal-core add_user_library_uses_folder_name_and_defaults`
Expected: FAIL — `add_user_library` / `remove_user_library` not defined.

- [ ] **Step 3: Implement**

Add to `crates/hymnal-core/src/library.rs` (module level):

```rust
/// Canonicalize `path` to an absolute, symlink-resolved form for stable
/// comparison; falls back to the input as-is if canonicalization fails (e.g.
/// the folder is on an unmounted drive).
fn canonical_string(path: &std::path::Path) -> String {
    std::fs::canonicalize(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .to_string()
}

/// Add a user folder as a `Library`. The display name is the folder's last
/// path component (falling back to the full path). `managed_by_git = false`,
/// `enabled = true`. The stored path is the canonicalized form. Returns `Err`
/// if the folder is already present (compared canonically) or does not exist.
pub fn add_user_library(cfg: &mut Config, path: &std::path::Path) -> anyhow::Result<()> {
    if !path.is_dir() {
        anyhow::bail!("not a folder: {}", path.display());
    }
    let canon = canonical_string(path);
    let exists = cfg
        .libraries
        .iter()
        .any(|l| canonical_string(std::path::Path::new(&l.path)) == canon);
    if exists {
        anyhow::bail!("folder already added");
    }
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| canon.clone());
    cfg.libraries.push(Library {
        name,
        path: canon,
        enabled: true,
        managed_by_git: false,
    });
    Ok(())
}

/// Remove the library whose `path` matches. Refuses to remove a
/// `managed_by_git` entry (the default library is locked against removal).
/// No-op if no matching removable library is found.
pub fn remove_user_library(cfg: &mut Config, path: &str) {
    cfg.libraries
        .retain(|l| l.managed_by_git || l.path != path);
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p hymnal-core library`
Expected: PASS.

- [ ] **Step 5: Extend the config round-trip test**

Find the existing `config_toml_round_trips` test. Add a user library to its `libraries` vec (alongside the existing entry) and assert it survives the round-trip with `managed_by_git == false`. If the existing test's struct literal would not otherwise exercise a mixed default+user config, this guarantees TOML persistence of user folders. Example addition inside that test's `libraries` vec:

```rust
            Library {
                name: "MyHymns".into(),
                path: "/tmp/myhymns".into(),
                enabled: true,
                managed_by_git: false,
            },
```

Then after the round-trip, assert e.g. `assert!(back.libraries.iter().any(|l| !l.managed_by_git && l.name == "MyHymns"));`.

- [ ] **Step 6: Run tests + clippy**

Run: `cargo test -p hymnal-core` then `cargo clippy -p hymnal-core --all-targets`
Expected: all tests PASS; clippy reports no warnings from the new code.

- [ ] **Step 7: Commit**

```bash
git add crates/hymnal-core/src/library.rs
git commit -m "feat(core): add/remove user library folders with dedup and default lock"
```

---

## Task 3: GUI — Slint surface (LibraryRow + Settings section)

**Files:**
- Modify: `crates/hymnal-gui/ui/app.slint`

- [ ] **Step 1: Add the `LibraryRow` struct and `SettingsPanel` inputs**

Near the top of `crates/hymnal-gui/ui/app.slint` (with the other `struct` declarations such as `DownloadState`), add:

```slint
struct LibraryRow {
    name: string,
    path: string,
    enabled: bool,
    removable: bool,   // false for the managed_by_git default
    available: bool,
}
```

In `component SettingsPanel`, add these members alongside the existing
`app-version` / `update-status` / `sync-status` / `syncing` / `force-sync`:

```slint
    in property <[LibraryRow]> libraries;
    in property <string> library-status;
    callback add-library();
    callback remove-library(string);
    callback set-library-enabled(string, bool);
```

- [ ] **Step 2: Add the "Libraries" section UI**

Inside `SettingsPanel`'s `VerticalBox`, after the existing "Library" (force-sync)
`Rectangle` block, add a new card. Use the existing `Theme` tokens for styling:

```slint
        Rectangle {
            background: Theme.panel;
            border-radius: Theme.radius;
            border-width: 1px;
            border-color: Theme.panel-border;
            VerticalBox {
                padding: 14px;
                spacing: 8px;
                HorizontalBox {
                    alignment: space-between;
                    Text {
                        text: "Your library folders";
                        color: Theme.text;
                        font-weight: 600;
                        vertical-alignment: center;
                    }
                    Rectangle {
                        height: 30px;
                        width: 120px;
                        border-radius: Theme.radius;
                        background: Theme.accent;
                        Text {
                            text: "Add folder…";
                            color: white;
                            vertical-alignment: center;
                            horizontal-alignment: center;
                        }
                        TouchArea { clicked => { root.add-library(); } }
                    }
                }
                Text {
                    text: "Add your own folders of .pptx hymns. They're searched alongside the built-in library.";
                    color: Theme.text-dim;
                    font-size: 12px;
                    wrap: word-wrap;
                }
                for row[i] in root.libraries: Rectangle {
                    height: 44px;
                    background: Theme.field;
                    border-radius: 6px;
                    HorizontalBox {
                        padding-left: 10px;
                        padding-right: 10px;
                        spacing: 10px;
                        alignment: start;
                        // Enable/disable toggle.
                        TouchArea {
                            width: 22px;
                            clicked => { root.set-library-enabled(row.path, !row.enabled); }
                            Rectangle {
                                width: 18px; height: 18px;
                                y: (parent.height - self.height) / 2;
                                border-radius: 4px;
                                border-width: 1px;
                                border-color: Theme.text-dim;
                                background: row.enabled ? Theme.accent : transparent;
                                Text {
                                    text: row.enabled ? "✓" : "";
                                    color: white;
                                    font-size: 12px;
                                    horizontal-alignment: center;
                                    vertical-alignment: center;
                                }
                            }
                        }
                        VerticalBox {
                            spacing: 0;
                            horizontal-stretch: 1;
                            Text {
                                text: row.available ? row.name : row.name + "  (unavailable)";
                                color: row.available ? Theme.text : Theme.text-dim;
                                font-weight: 600;
                                overflow: elide;
                            }
                            Text {
                                text: row.path;
                                color: Theme.text-dim;
                                font-size: 11px;
                                overflow: elide;
                            }
                        }
                        // Remove button — only for removable (non-default) rows.
                        Rectangle {
                            width: 28px; height: 28px;
                            y: (parent.height - self.height) / 2;
                            visible: row.removable;
                            border-radius: 6px;
                            background: Theme.danger;
                            Text {
                                text: "✕";
                                color: white;
                                horizontal-alignment: center;
                                vertical-alignment: center;
                            }
                            TouchArea {
                                clicked => { root.remove-library(row.path); }
                            }
                        }
                    }
                }
                Text {
                    text: root.library-status;
                    color: Theme.text-dim;
                    font-size: 12px;
                    wrap: word-wrap;
                }
            }
        }
```

- [ ] **Step 3: Expose the members on `AppWindow` and forward to `SettingsPanel`**

On `export component AppWindow`, next to the existing Settings members
(`app-version`, `update-status`, `sync-status`, `syncing`, `force-sync`), add:

```slint
    in property <[LibraryRow]> libraries;
    in property <string> library-status;
    callback add-library();
    callback remove-library(string);
    callback set-library-enabled(string, bool);
```

In the `if root.active-tab == 2: SettingsPanel { ... }` instantiation, forward
them (alongside the existing forwarded Settings props/callbacks):

```slint
            libraries: root.libraries;
            library-status: root.library-status;
            add-library => { root.add-library(); }
            remove-library(p) => { root.remove-library(p); }
            set-library-enabled(p, e) => { root.set-library-enabled(p, e); }
```

- [ ] **Step 4: Verify the Slint compiles**

Run: `cargo build -p hymnal-gui 2>&1 | grep -iE "slint|parse error|unknown" || echo NO_SLINT_ERRORS`
Expected: `NO_SLINT_ERRORS` for the `.slint` layer. The build may still fail with
**Rust** errors in `main.rs` about the new callbacks/properties not being
handled — that is expected until Task 4. Confirm there are no `.slint` syntax
errors specifically.

- [ ] **Step 5: Commit**

```bash
git add crates/hymnal-gui/ui/app.slint
git commit -m "feat(gui): Libraries section in Settings (add/toggle/remove rows)"
```

---

## Task 4: GUI — wire library management in main.rs

**Files:**
- Modify: `crates/hymnal-gui/src/main.rs`

Context: `main.rs` already holds `dl_cfg: Rc<RefCell<Config>>` (the shared,
persisted config) and `cfg_path: Option<PathBuf>`. The 200ms `timer` already
drains `rx` (boot), `dl_rx` (downloads), and `fs_rx` (force-sync). We add a
fourth channel for local library re-index results and three callbacks.

- [ ] **Step 1: Add a helper to build the `LibraryRow` model + a channel**

Near the top of `main.rs` (after the `use` lines), add a helper that mirrors
`register_default_library` so the default row shows even on first run before it's
been persisted to disk:

```rust
/// Build the Slint `LibraryRow` model from the config, ensuring the default
/// git-managed library appears even if it isn't yet written to the config on
/// disk (mirrors refresh::register_default_library's "add if no managed entry").
fn library_rows(cfg: &Config) -> Vec<LibraryRow> {
    use hymnal_core::library::library_available;
    let mut rows: Vec<LibraryRow> = cfg
        .libraries
        .iter()
        .map(|l| LibraryRow {
            name: l.name.clone().into(),
            path: l.path.clone().into(),
            enabled: l.enabled,
            removable: !l.managed_by_git,
            available: library_available(&l.path),
        })
        .collect();
    // If no managed_by_git entry exists yet, show a synthetic default row so the
    // user always sees the built-in library (it gets persisted on first index).
    if !cfg.libraries.iter().any(|l| l.managed_by_git) {
        if let Some(dir) = hymnal_core::library::default_library_dir() {
            let path = dir
                .join(hymnal_core::library::DEFAULT_REPO_HYMNS_SUBDIR)
                .to_string_lossy()
                .to_string();
            let available = hymnal_core::library::library_available(&path);
            rows.insert(0, LibraryRow {
                name: "Imnuri Creștine".into(),
                path: path.into(),
                enabled: true,
                removable: false,
                available,
            });
        }
    }
    rows
}
```

Then, where the other channels are declared (near `let (fs_tx, fs_rx) = ...`),
add a channel for local re-index results:

```rust
    // Local re-index after a library-folder change (no network).
    let (lib_tx, lib_rx) = mpsc::channel::<Result<Vec<HymnEntry>, String>>();
```

- [ ] **Step 2: Initialize the `libraries` model at startup**

After the existing `ui.set_download_dir(...)` / `ui.set_app_version(...)` block,
add:

```rust
    ui.set_libraries(ModelRc::from(Rc::new(VecModel::from(library_rows(&dl_cfg.borrow())))));
    ui.set_library_status("".into());
```

- [ ] **Step 3: Drain the local re-index channel in the timer**

Inside the existing `timer.start(...)` closure, after the `fs_rx` drain block,
add a `lib_rx` drain that rebuilds the searcher and refreshes the query (same
shape as the `fs_rx` block but writing `library-status`):

```rust
            if let Ok(result) = lib_rx.try_recv() {
                if let Some(ui) = weak2.upgrade() {
                    match result {
                        Ok(entries) => {
                            let n = entries.len();
                            *searcher_for_timer.borrow_mut() = Some(Searcher::new(entries));
                            ui.set_library_status(format!("Indexed {n} hymns.").into());
                            ui.invoke_query_changed("".into());
                        }
                        Err(e) => {
                            // Keep the existing searcher on failure.
                            ui.set_library_status(format!("Indexing failed: {e}").into());
                        }
                    }
                }
            }
```

- [ ] **Step 4: Add a shared re-index closure and the three callbacks**

Before `let _timer = timer;`, add the three handlers. They mutate the shared
`dl_cfg`, persist it, refresh the `libraries` model, and spawn a **local-only**
re-index via `refresh::load_local`.

```rust
    // ---- Library folder management (Settings tab) ----
    // Re-index locally (no network) from the current config, off the UI thread.
    let reindex = {
        let lib_tx = lib_tx.clone();
        move |cfg: Config| {
            let lib_tx = lib_tx.clone();
            std::thread::spawn(move || {
                // load_local never errors; wrap for channel symmetry.
                let entries = hymnal_core::refresh::load_local(cfg);
                let _ = lib_tx.send(Ok(entries));
            });
        }
    };

    // Add folder: pick, add to config, persist, refresh model, re-index.
    let weak_addlib = ui.as_weak();
    let cfg_addlib = dl_cfg.clone();
    let cfg_path_addlib = cfg_path.clone();
    let reindex_add = reindex.clone();
    ui.on_add_library(move || {
        let Some(ui) = weak_addlib.upgrade() else { return };
        let Some(folder) = rfd::FileDialog::new().pick_folder() else { return };
        let mut cfg = cfg_addlib.borrow_mut();
        match hymnal_core::library::add_user_library(&mut cfg, &folder) {
            Ok(()) => {
                if let Some(p) = &cfg_path_addlib {
                    if let Err(e) = cfg.save(p) { warn!("config save failed: {e}"); }
                }
                ui.set_libraries(ModelRc::from(Rc::new(VecModel::from(library_rows(&cfg)))));
                ui.set_library_status("Indexing…".into());
                reindex_add(cfg.clone());
            }
            Err(e) => {
                ui.set_library_status(format!("{e}").into());
            }
        }
    });

    // Remove folder.
    let weak_rmlib = ui.as_weak();
    let cfg_rmlib = dl_cfg.clone();
    let cfg_path_rmlib = cfg_path.clone();
    let reindex_rm = reindex.clone();
    ui.on_remove_library(move |path| {
        let Some(ui) = weak_rmlib.upgrade() else { return };
        let mut cfg = cfg_rmlib.borrow_mut();
        hymnal_core::library::remove_user_library(&mut cfg, &path);
        if let Some(p) = &cfg_path_rmlib {
            if let Err(e) = cfg.save(p) { warn!("config save failed: {e}"); }
        }
        ui.set_libraries(ModelRc::from(Rc::new(VecModel::from(library_rows(&cfg)))));
        ui.set_library_status("Indexing…".into());
        reindex_rm(cfg.clone());
    });

    // Toggle enabled.
    let weak_togglib = ui.as_weak();
    let cfg_togglib = dl_cfg.clone();
    let cfg_path_togglib = cfg_path.clone();
    let reindex_tog = reindex.clone();
    ui.on_set_library_enabled(move |path, enabled| {
        let Some(ui) = weak_togglib.upgrade() else { return };
        let mut cfg = cfg_togglib.borrow_mut();
        hymnal_core::library::set_library_enabled(&mut cfg, &path, enabled);
        if let Some(p) = &cfg_path_togglib {
            if let Err(e) = cfg.save(p) { warn!("config save failed: {e}"); }
        }
        ui.set_libraries(ModelRc::from(Rc::new(VecModel::from(library_rows(&cfg)))));
        ui.set_library_status("Indexing…".into());
        reindex_tog(cfg.clone());
    });
```

Note on the borrow: each handler holds `cfg.borrow_mut()` and then calls
`reindex_*(cfg.clone())`. Because `cfg` is a `RefMut`, `cfg.clone()` clones the
`Config` (not the `RefCell`) — that is the intended deep copy handed to the
worker thread. The `RefMut` is dropped at the end of the closure. This is safe:
no nested borrow, and the worker gets an owned snapshot.

- [ ] **Step 5: Build + clippy**

Run: `cargo build` then `cargo clippy --all-targets`
Expected: clean build, no warnings from the new code. If clippy flags the
`reindex` closure capture or clones, address only genuine issues (the
`cfg.clone()` snapshot is intentional).

- [ ] **Step 6: Manual verification**

Run: `cargo run -p hymnal-gui`
1. Go to Settings → "Your library folders". Confirm the built-in library row
   appears and has **no** remove button.
2. Click "Add folder…", pick a folder containing `.pptx` files. Confirm a row
   appears, "Indexing…" → "Indexed N hymns", and the new hymns are searchable in
   the Library tab.
3. Toggle the new folder off → its hymns disappear from search; on → reappear.
4. Toggle the **default** off → only your folder's hymns remain; confirm the
   default row has no remove button but the toggle works.
5. Click ✕ on your folder → it's removed and its hymns disappear.
6. Re-add a folder, quit, relaunch → folder persists and is indexed on boot.
7. Add a folder, delete it on disk, relaunch → row shows "(unavailable)",
   other libraries still work.

- [ ] **Step 7: Commit**

```bash
git add crates/hymnal-gui/src/main.rs
git commit -m "feat(gui): wire add/remove/toggle of user library folders with local re-index"
```

---

## Task 5: Final verification

- [ ] **Step 1: Full test suite**

Run: `cargo test`
Expected: all tests pass (core library tests + existing suites).

- [ ] **Step 2: Lint**

Run: `cargo clippy --all-targets`
Expected: no errors (pre-existing warnings, if any, acceptable).

- [ ] **Step 3: Regression check**

Launch the app; confirm Library search, slide preview (←/→), Video Downloader,
and the existing Settings force-sync / version / update status all still work.

---

## Notes / decisions baked in

- **Local re-index only** (`load_local`, not `load_library`): adding a user
  folder must never trigger a network re-clone of the default library.
- **Default lockable, not removable**: `remove_user_library` refuses
  `managed_by_git`; the UI hides the remove button (defense in depth). The
  default can be **disabled** (entry stays, `enabled=false`), and
  `register_default_library` re-adds it only if no managed entry exists, so a
  disabled default survives restarts.
- **Dedup by canonical path**: prevents adding the same folder twice under
  different spellings; canonicalization falls back to the raw path for
  unreachable folders.
- **One config handle**: reuse the existing `dl_cfg: Rc<RefCell<Config>>` as the
  single in-memory source of truth so download-dir and library changes don't
  clobber each other; persist after every change.
