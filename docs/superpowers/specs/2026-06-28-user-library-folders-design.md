# User Library Folders — Design

**Date:** 2026-06-28
**Status:** Approved (pending spec review)

## Summary

Let users add their own folders of `.pptx` hymns to the library, on top of the
default git-managed library. Added folders are managed in the existing
**Settings** tab: add via a native folder picker, enable/disable, or remove.
Changes re-index locally and immediately so new hymns appear in search without
a restart. The default library is shown but cannot be removed (it can be
disabled).

## Goals

- Users can point the app at their own `.pptx` folders and search them alongside
  the provided content.
- Zero-friction add (pick a folder, done); manage (enable/disable/remove) from
  Settings.
- Changes take effect immediately via a fast, local-only re-index (no network).

## Non-Goals (YAGNI)

- No per-file add (folders only).
- No custom display-name prompt (auto-name from the folder).
- No network sync for user folders (they're local; only the default library is
  git-synced).
- No watching folders for live filesystem changes (re-index happens on explicit
  add/remove/toggle and on boot).

## Decisions (from brainstorming)

| Topic | Decision |
|---|---|
| Scanning | **Recursive**, like the default library (existing `WalkDir` crawl). |
| Management UI | List of rows: enable/disable toggle + Remove button per row. |
| Re-index timing | **Immediate, automatic, local-only** on each add/remove/toggle. |
| Naming | **Auto** from the folder's last path component; full path shown in Settings. |
| Missing folder | **Skip silently** during indexing, **keep** the config entry, show an "unavailable" marker on the row. |
| Default library | **Lockable**: can be disabled, cannot be removed. |

## Architecture

Reuse `Config.libraries: Vec<Library>` directly. A user folder is a
`Library { managed_by_git: false, enabled: true }`. No new data model and no new
indexing loop — `refresh::index_enabled()` already iterates
`cfg.libraries.iter().filter(|l| l.enabled)`, and the index cache keys on
absolute path + mtime, so cross-library collisions and re-parsing are already
handled. Rejected alternative: a separate `user_libraries` field (needless;
would duplicate the indexing loop and split logic).

## Section 1: Core (`hymnal-core/src/library.rs`)

New helpers, all pure/unit-testable without real app dirs or the network:

```rust
/// Add a user folder as a Library. Name defaults to the folder's last path
/// component (falling back to the full path). managed_by_git = false,
/// enabled = true. Returns Err if the path is already present (compared by
/// canonicalized path) or can't be canonicalized/read.
pub fn add_user_library(cfg: &mut Config, path: &Path) -> anyhow::Result<()>;

/// Remove a library by path. No-op if absent. MUST refuse to remove a
/// managed_by_git entry (the default library is locked against removal).
pub fn remove_user_library(cfg: &mut Config, path: &str);

/// Set the `enabled` flag of a library by path. Works on any library,
/// including the default (the default may be disabled, just not removed).
pub fn set_library_enabled(cfg: &mut Config, path: &str, enabled: bool);

/// Whether a library folder is currently reachable on disk
/// (`Path::new(path).is_dir()`), used for the Settings "unavailable" marker.
pub fn library_available(path: &str) -> bool;
```

Notes:
- **De-duplication:** `add_user_library` canonicalizes the candidate path and
  compares against existing library paths (canonicalized) so the same folder
  can't be added twice under different spellings.
- **Persistence:** callers persist with the existing `Config::save(&config_path())`.
- **Default survival:** `register_default_library` (in `refresh.rs`) keys off the
  *presence* of a `managed_by_git` entry, so a default with `enabled: false`
  persists across launches; removal is blocked, so it never vanishes.

## Section 2: GUI (Settings panel + wiring)

A "Libraries" section in the existing `SettingsPanel`:

- Header "Libraries" + an **"Add folder…"** button (native `rfd` folder picker,
  already a dependency).
- One row per `Library`:
  - enable/disable checkbox (bound to `enabled`),
  - name + dimmed/elided path,
  - an **"unavailable"** marker when `library_available(path)` is false,
  - a **Remove** button, hidden/disabled on the `managed_by_git` row.
- A status line: "Indexing…" / "Indexed N hymns".

New Slint surface on `AppWindow`:

```
struct LibraryRow {
    name: string,
    path: string,
    enabled: bool,
    removable: bool,   // false for the managed_by_git default
    available: bool,
}
in property <[LibraryRow]> libraries;
in property <string> library-status;
callback add-library();                       // open picker, add, re-index
callback remove-library(string);              // by path
callback set-library-enabled(string, bool);   // by path
```

### Data flow

Reuses the established worker-thread + 200 ms-timer pattern and, crucially, the
**local-only** re-index path (`refresh::load_local`, no clone/pull):

```
[Add / Remove / Toggle] (UI thread)
   mutate the shared Rc<RefCell<Config>>; Config::save(...)
   rebuild the `libraries` model from cfg; set library-status = "Indexing…"
   spawn worker → load_local(cfg.clone())          // local crawl only
        └─ sends Vec<HymnEntry> over the existing result channel
   existing 200 ms timer drains it → rebuild Searcher, re-run current query,
        set library-status = "Indexed N hymns"
```

The Add flow spawns this worker after the picker returns a folder. This mirrors
how force-sync already feeds the timer; reuse that channel/enum rather than add a
new mechanism. Using `load_local` (not `load_library`) is deliberate: changing a
user folder must never trigger a network re-clone of the default library.

## Section 3: Error handling

- **Add fails** (duplicate after canonicalization, or unreadable path) → show the
  reason in `library-status` (e.g. "Folder already added"); config unchanged.
- **Remove of the default** → guarded in core (`remove_user_library` ignores
  `managed_by_git`); the UI also hides the button (defense in depth).
- **Unavailable folder** (unmounted drive) → indexing skips it (WalkDir yields
  nothing), the entry stays, the row shows "unavailable"; it returns
  automatically on a later re-index when the drive is back.
- **Config save failure** → logged via `warn!` and surfaced in status; the
  in-memory change still applies for the session.
- **Re-index failure / empty result** → keep the existing in-memory `Searcher`
  (don't wipe results on a bad change), same principle as force-sync.

## Section 4: Testing

Core (no network, no GUI; temp dirs / path injection):

- `add_user_library`: adds with folder-name as name, `managed_by_git == false`,
  `enabled == true`; returns `Err` on a duplicate (canonicalized) path.
- `remove_user_library`: removes a user library by path; **refuses** to remove a
  `managed_by_git` entry (asserts it remains).
- `set_library_enabled`: flips `enabled` on both a user library and the default.
- `library_available`: true for an existing temp dir, false for a bogus path.
- Config round-trip: a `Config` with mixed default + user libraries survives TOML
  save/load (extends the existing config test).

GUI is thin → verified manually: add a folder of `.pptx` and confirm its hymns
appear in search; disable the default and confirm only user hymns remain; remove
a user folder and confirm its hymns are gone; relaunch with an unavailable folder
and confirm the row is marked unavailable and other libraries still work.

## Out of scope (YAGNI)

Per-file add, custom name prompt, filesystem watching/auto-refresh, network sync
for user folders, and reordering libraries.
