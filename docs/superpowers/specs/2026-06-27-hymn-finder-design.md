# Hymn Finder — Design

**Date:** 2026-06-27
**Status:** Approved

## Summary

A small cross-platform (macOS / Windows) desktop GUI that lets a user search a
hymnal by typing in a search bar and open the matching hymn. Hymns are stored as
PowerPoint `.pptx` files. The app indexes both filenames and the text *inside*
the slides so users can search by hymn number, title, or any line of the lyrics.

The bundled library is the Romanian SDA hymnal *Imnuri Creștine 2013* (hymns
1–920, ~924 `.pptx` files, ~125 MB), organized into range folders
(`1-99`, `100-199`, …). The app is expandable: users can add their own library
folders, which are crawled and indexed the same way.

Built in **Rust** (cross-compilable) with the **Slint** GUI toolkit.

## Key decisions

- **Open behavior:** hybrid. Show extracted verse text in an in-app preview pane
  for quick reading, plus a button to launch the real `.pptx` in PowerPoint /
  the OS default handler for projection. No in-app slide *rendering*.
- **Asset distribution:** git-based. A default hymns repo URL is baked into the
  binary but overridable in settings. On first run the app clones it into the
  app-data dir; later runs can `git pull` to update. The current `assets/`
  folder is the seed that gets published to that repo separately.
- **Search:** fuzzy + diacritic-insensitive. Typing `plecati` matches `Plecaţi`;
  typos are tolerated; numbers match hymn numbers. Ranks filename and content
  hits. Built on an in-memory index + a fuzzy matcher.

## Architecture

Single app, structured as a thin GUI over a reusable core library:

```
sda_manager/
  crates/
    hymnal-core/     # library: indexing, search, pptx parsing, library mgmt (no UI)
    hymnal-gui/      # binary: Slint UI, wires events to core
  ui/                # .slint files
  assets/            # seed hymnal (published to the git library repo)
  docs/
```

**Rationale for core/GUI split:** the PPTX-parsing, indexing, and search logic is
independent of Slint. Isolating it in `hymnal-core` makes the tricky parts
(zip/XML extraction, fuzzy ranking, diacritic folding, cache invalidation)
unit-testable headlessly, and keeps the GUI a small, mostly-declarative layer.
Leaves room for a CLI later.

## Components (`hymnal-core`)

- **`library`** — manages library folders. A `Library` = root path + metadata.
  The default library is git-managed (`git2` for clone/pull); user-added
  libraries are plain folders. Config persisted as TOML in the OS app-data dir
  (`directories` crate).
- **`pptx`** — extracts text from one `.pptx`: open the zip (`zip` crate), read
  `ppt/slides/slideN.xml` in slide order, pull `<a:t>` text runs (`quick-xml`),
  and split into:
  - `number` — parsed from the "Imnul N" line on slide 1, falling back to the
    filename stem (e.g. `001.pptx` → 1).
  - `title` — first meaningful line of slide 1.
  - `body` — concatenated verse text from slides 2+.
  Skips `~$*` lock files and non-`.pptx` files (e.g. the bundled `.zip`).
- **`index`** — crawls all enabled libraries for `.pptx`, parses each via
  `pptx`, builds an in-memory `Vec<HymnEntry>`. Cached to disk
  (`serde` + `bincode`) keyed by file path + mtime; subsequent launches skip
  re-parsing unchanged files.
- **`search`** — diacritic-folds query and entries, runs a fuzzy matcher
  (`nucleo-matcher`) across number, title, filename, and body; returns ranked
  `SearchHit`s recording which field matched.

## Data flow

1. GUI startup → load config.
2. Load cached index on a worker thread (UI stays responsive).
3. If default library missing → clone from git in the background, show a banner.
4. User types → debounced query → `search` returns ranked hits → results list
   updates.
5. Select a hit → preview pane shows extracted verses.
6. "Open in PowerPoint" → `open` crate launches the `.pptx`.

**Threading:** indexing and git operations run off the Slint event loop on a
worker thread; results are posted back via a channel so the window never freezes.

## UI (Slint)

Single main window:

- **Top:** auto-focused search bar + library filter dropdown ("All libraries").
- **Left:** ranked results list — each row shows hymn number, title, and a
  library/source badge. Arrow keys + Enter navigate.
- **Right:** preview pane — title, number, scrollable read-only verse text.
- **Below preview:** "Open in PowerPoint" + "Reveal in folder" buttons.
- **Settings (gear):** manage libraries (add/remove folder), set/override the
  default git repo URL, "Update default library" (git pull), "Re-index".
- **First run:** if default library absent, show a "Downloading hymn library…"
  banner while it clones in the background.

## Error handling

- **Corrupt/unparseable PPTX:** skip, log, keep indexing; surface a skipped-file
  count in settings rather than failing.
- **Git unavailable/offline:** app runs with whatever is cached locally; show a
  non-blocking notice that the update failed.
- **Missing library folder:** mark unavailable in settings; don't crash.
- **No PowerPoint installed:** fall back to OS default handler; if that fails,
  show the file path for manual opening.

## Testing

- **`hymnal-core` unit tests** with committed sample `.pptx` fixtures:
  - title / number / body extraction.
  - diacritic-insensitive + fuzzy ranking (`plecati` → hymn 1 as top hit).
  - index cache invalidates on mtime change.
  - `~$*` lock files and `.zip` files are ignored during crawl.
- **GUI** kept thin and manually verified; testable logic lives in core.

## Out of scope (YAGNI)

In-app PPTX slide rendering, editing hymns, playlists/sets, multi-window
projection / second-screen output. Can be added later.
