# Settings Tab + Force-Sync + Binary Auto-Update — Design

**Date:** 2026-06-27
**Status:** Approved

## Summary

Add a two-tab UI (Search | Settings). The Settings tab shows the app version
and an update status, and provides a **Force sync library** button that deletes
the cloned library + index cache, re-clones the library fresh, and fully
reindexes. Additionally, on every boot the app checks GitHub Releases for a
newer binary and, if found, downloads and stages it — applied on next restart
(never disrupts the running session).

## Section 1: Core — reusable indexing + force-clean

Today the sync→cache→index pipeline is inline in `hymnal-gui`'s boot worker
closure. Extract it into `hymnal-core` so both boot and force-sync reuse it.

New/extended core API (module `index` or a new `refresh` module):

- `fn load_library(cfg: &Config, force: bool) -> Vec<HymnEntry>` — ensure the
  default library exists (clone if missing, fast-forward pull if present via the
  existing `sync::sync_default_library`), load the cache (skipped when
  `force == true`), `refresh_index` each enabled library, and `save_cache`.
  Boot calls `force=false`; the button path calls `force=true`.
- `fn force_clean(cfg: &Config) -> anyhow::Result<()>` — delete the cloned
  git-managed library directory (`default_library_dir()`) and delete the index
  cache file (`index_cache_path()`). No-op/`Ok` when either is already absent.
  After this, `load_library` re-clones fresh (~53 MB) and reindexes from scratch
  — the true "force".

`sync::sync_default_library` already clones-if-missing, so once `force_clean`
removes the clone, the normal path re-clones it.

## Section 2: Core — binary self-update

New `hymnal-core` module `update.rs` wrapping the `self_update` crate
(GitHub Releases backend).

- `pub enum UpdateOutcome { UpToDate, Updated { version: String } }`
- `pub fn check_and_stage_update() -> anyhow::Result<UpdateOutcome>` — query
  releases for `AbelHristodor/sda_manager`, compare the latest `vX.Y.Z` tag
  against `env!("CARGO_PKG_VERSION")`; if newer, download the asset matching the
  running target triple (`hymnal-gui-<target>.tar.gz` on unix, `.zip` on
  Windows; binary nested in `hymnal-gui-<target>/`) and replace the current
  executable in place.
- **Notify, apply on restart:** `self_update` swaps the on-disk binary
  atomically; the running process keeps old code until the user restarts.
  `Updated { version }` means "new binary staged; restart to apply". No forced
  restart, no prompt-to-restart-now.
- **Failure handling:** all errors logged and swallowed — startup never blocked,
  no opt-out toggle.

Caveat: `self_update` replaces the binary at its current path. Works for the
install-script location (`~/.local/bin/hymnal-gui`, `%LOCALAPPDATA%\…`). On a
read-only mount it fails — handled as a logged error, no crash.

The `self_update` crate version is pinned at implementation time to the latest
stable (target `~0.41`); confirm at build.

## Section 3: GUI — tabs, settings, threading

Wrap the current UI in a Slint `TabWidget` with two tabs:

- **Search** — the entire existing UI (search bar, results list, slide preview,
  status bar) moves under this tab unchanged.
- **Settings** — new panel containing:
  - **Current version** (`env!("CARGO_PKG_VERSION")`).
  - **Update status** line: "Checking…" / "Up to date" / "Update vX.Y staged —
    restart to apply" / "Update check failed".
  - **Force sync library** button.
  - **Sync status** label: "Re-cloning…" / "Indexed N hymns" / error text.

New Slint properties/callbacks on `AppWindow`:

- `in property <string> app-version;`
- `in property <string> update-status;`
- `in property <string> sync-status;`
- `in property <bool> syncing;` (disables the button while a sync runs)
- `callback force-sync();`
- `in-out property <int> active-tab;` (or read TabWidget's current index) used to
  guard keyboard handling.

Threading (reuse the existing worker-thread + channel + 200 ms timer):

- A boot worker already sends the initial `Vec<HymnEntry>` via a channel; extend
  it to ALSO run `update::check_and_stage_update()` and post the update status
  back via `upgrade_in_event_loop`.
- `on_force_sync` spawns a thread that sets `syncing=true`, calls `force_clean`
  then `load_library(force=true)`, posting progress strings and the final
  `Vec<HymnEntry>` through a channel. The existing 200 ms timer drains this
  result, rebuilds the `Searcher`, refreshes the current query, and clears
  `syncing`. (Use one channel carrying an enum, or a second channel — impl
  detail.)
- The button is disabled while `syncing == true`.

Keyboard guard: the global `capture-key-pressed` (↑/↓/←/→) must only drive the
results list when the **Search** tab is active. Guard each arrow branch on the
active-tab index so arrows don't interfere on the Settings tab.

## Section 4: Error handling & testing

Error handling:

- Force-sync: if re-clone fails (offline/bad URL), show the error in
  `sync-status` and **keep the existing in-memory searcher** (don't wipe it on
  failure). Deletion uses `remove_dir_all`/`remove_file`; errors surfaced, not
  panicked.
- Self-update: every failure logged and swallowed; startup never blocked.
- Tab guard prevents arrow-key conflicts between tabs.

Testing:

- **Core:** `force_clean` — create a temp clone dir + cache file, call it, assert
  both are removed; assert it returns `Ok` when they are already absent. Use a
  `Config`/path-injection approach or test against temp paths so it doesn't touch
  the real app dirs.
- **Self-update:** no live-network unit test (hits GitHub); the wrapper stays
  thin (version compare + asset match handled by `self_update`). Verify it
  compiles and runs the check path manually.
- **GUI:** thin; verified manually and via boot/force-sync logging.

## Out of scope (YAGNI)

Auto-restart after update, update opt-out toggle, delta/partial updates,
progress bars for the download, signing/notarization of the binary, and
per-library force-sync (force-sync targets the git-managed default library).
