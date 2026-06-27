# Slide-by-Slide Preview + Status Bar — Design

**Date:** 2026-06-27
**Status:** Approved

## Summary

Upgrade the preview pane so it shows hymn content **one slide at a time** with
navigation, plus a **status bar pinned at the bottom** of the preview reading
`<title> — slide N/total`. Today the preview shows a flat blob of all slide
text joined together, and slide boundaries are discarded during parsing.

## Data model change (core)

The parser currently joins every slide into one `body` string, losing slide
boundaries. Change it to preserve text per slide:

- `pptx::ParsedPptx` and `model::HymnEntry` gain `slides: Vec<String>` — one
  entry per slide, holding that slide's joined paragraph text (paragraphs
  separated by `\n`, same per-`<a:p>` joining as the title fix).
- **`body` stays** and is now derived as `slides.join("\n")`. Search continues
  to index `body`, so search behavior and the diacritic-folded index are
  unchanged.
- `slides[0]` is the title slide; the title is still extracted from it via the
  existing `pick_title` logic.

Bump `index::CACHE_VERSION` to `3` so existing on-disk caches (which lack the
`slides` field) are auto-rejected and rebuilt — the same self-healing
mechanism already in place. `HymnEntry` keeps deriving
`Serialize/Deserialize/Clone/PartialEq`.

## UI & interaction (GUI)

Preview pane (right side), top to bottom:

1. Scroll area showing **one slide's text at a time** (the current slide's
   lines) in a readable font.
2. A **Prev / Next** button row.
3. The existing **Open in PowerPoint / Reveal in folder** buttons.
4. A **status bar pinned at the bottom**: `Ca un cerb setos de ape — slide 3/12`.

New Slint state on `AppWindow`:

- `in property <string> slide-text;` — current slide's text.
- `in property <int> slide-count;` — total slides in the highlighted hymn.
- `in-out property <int> slide-index;` — current slide (0-based).
- `in property <string> preview-status;` — the bottom status-bar string.
- callbacks `prev-slide()` / `next-slide()` for the buttons.

Navigation:

- **← / →** keys step slides, clamped at `[0, slide-count-1]`. Added to the
  existing `capture-key-pressed` handler (`Key.LeftArrow` / `Key.RightArrow`),
  returning `accept`. **↑ / ↓ still move the results list** — no conflict.
- **Prev / Next** buttons call `prev-slide()` / `next-slide()`, same clamping.
- Selecting a different hymn (↑/↓ or click → `current-changed`) **resets
  `slide-index` to 0**, recomputes `slide-count`, slide text, and status.

Rust glue:

- A shared helper `show_slide(ui, entry, idx)` sets `slide-text`, `slide-count`,
  `slide-index`, and `preview-status` from the highlighted entry's `slides`.
- The highlighted entry is resolved via the existing row→entry-index map and
  `Searcher::entry(index)` (no extra cloning of bodies).
- `on_current_changed` resets to slide 0 and calls `show_slide`.
- `on_prev_slide` / `on_next_slide` and the ←/→ keys adjust `slide-index`
  (clamped) and call `show_slide`.

The old `preview-title` / `preview-body` properties are replaced by
`slide-text` + `preview-status` (title now lives in the status bar).

Edge cases:

- 1-slide hymn → "slide 1/1"; Prev/Next are no-ops at the bounds.
- Empty `slides` → empty preview, status shows "0 slides" (defensive; should
  not occur for real hymns).

## Testing

- **Core (`pptx_test`):** assert `slides.len()` for a known fixture
  (hymn 1 has 5 slides), `slides[0]` contains the title, and
  `body == slides.join("\n")` so search input is unchanged.
- **GUI:** kept thin; slide-stepping/clamping is simple and verified manually.
  The testable parsing-into-slides logic lives in core.

## Out of scope (YAGNI)

Rendering actual slide layout/images/fonts (still text-only), thumbnails, and
jumping to an arbitrary slide by number. Sequential ←/→ navigation only.
