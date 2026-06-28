# Control Tab — Native Themed Projection — Design

**Date:** 2026-06-28
**Status:** Approved

## Summary

Add a **Control** tab that turns the app into its own presentation engine: it
renders the extracted hymn slide text into a **second full-screen window** on a
chosen display (the projector), styled by user-created **themes**. A separate
**Themes** area lets users create/edit/delete named themes (background, fonts,
sizes, weights, colours, alignment, layout, footer) with a live in-app preview.

This deliberately does NOT drive PowerPoint via OS automation — the app draws
the slides itself. Cross-platform, no Office dependency, themes are trivial
because we control the rendering. (The existing "Open in PowerPoint" button
remains as an unrelated escape hatch.)

## Decisions (from brainstorming)

- **Rendering:** the app renders slide text itself into its own projector
  window (not PowerPoint automation, not PPTX rewriting).
- **Display:** projector window opens fullscreen on a user-selected display
  (defaults to a non-primary display when present).
- **Presenter view:** the control/laptop side shows current slide + next slide
  peek + controls.
- **Theme scope:** full property set (text + background + layout + footer).
- **Theme storage:** multiple named themes, one JSON file each, under the app
  config dir; one marked active; a built-in default always exists.
- **One style for all slides** (no per-slide-type override).
- **Hymn → Control:** "Project" action from Library (button + Enter) loads one
  hymn; Control tab also has its own search. No playlist in v1.
- **Theme preview:** live in-app preview pane in a dedicated Themes area.
- **Fonts:** system fonts enumerated at runtime.

## Architecture & components

Core/GUI split as elsewhere.

- **`hymnal-core::theme`** (new module): the `Theme` data model, JSON
  load/save of named themes in `<config>/themes/<name>.json`, list/create/
  delete, built-in `Theme::default()` (non-deletable). Pure, unit-testable.
- **`hymnal-core`** (new small module, e.g. `present.rs`): a `PresentationState`
  struct holding the loaded hymn's slides, current index, blank flag — with
  pure transition methods (`next`, `prev`, `blank_toggle`, `load_hymn` resets to
  slide 0, clamping at ends). Unit-testable headlessly.
- **`hymnal-gui`**:
  - `ProjectorWindow` — a second Slint window component rendering ONE slide,
    styled by flattened theme properties; shown fullscreen on the chosen
    display. Inputs only: resolved theme props + current slide text + blank.
  - **Control tab** (sidebar nav item): presenter view.
  - **Themes area** (sidebar nav item): theme editor + live preview.
  - Display enumeration via the `display-info` crate (see Display targeting).

## Theme model

`Theme` (serde; one JSON file per theme):

```
name: String
text:
  font_family: String         # from system fonts
  font_size: Pt(f32) | AutoFit
  font_weight: u16            # 100..900
  color: Rgba
  h_align: Left|Center|Right
  v_align: Top|Middle|Bottom
  line_spacing: f32           # multiplier
  shadow: { enabled, color, blur, dx, dy }
  outline: { enabled, color, width }
background:
  kind: Solid(Rgba) | Gradient(Rgba, Rgba, angle) | Image(PathBuf)
  image_fit: Cover|Contain
  overlay_color: Rgba
  overlay_opacity: f32        # 0..1
layout:
  margin: f32
  max_text_width: f32         # fraction of slide width
footer:
  show: bool
  content: HymnNumberTitle | SlideCounter | None
```

- `Theme::default()` is built in (white text on dark solid background, sensible
  sizes) and cannot be deleted.
- `config.active_theme: Option<String>` records the selected theme name.
- Listing themes skips corrupt files (logged) and always includes the default.

## Rendering

`ProjectorWindow.slint` takes flat properties (the GUI flattens `Theme` into
individual Slint props whenever theme or slide changes). The slide is a layered
stack:

1. Background: base `Rectangle` (solid/gradient) or `Image { image-fit }`.
2. Overlay `Rectangle` with `opacity` (dim image for legibility).
3. Centered `Text` with chosen font-family/size/weight/color, h/v alignment,
   `line-height` from line_spacing, constrained to `max_text_width`.
4. Optional footer `Text`.

- **Shadow/outline:** approximated with layered offset `Text` copies.
- **Auto-fit font size:** v1 may use a fixed large size with word-wrap; full
  auto-fit shrinks from a max until the longest line fits `max_text_width`
  (measured in Rust). Implementation picks the simplest that looks right;
  flagged in the plan.
- **Blank:** a property that hides text/footer and shows solid black.

The Themes editor's live preview uses the SAME render component at small size,
so the preview matches the projector exactly.

## Projector window & display targeting

- **Start projecting:** GUI creates a `ProjectorWindow` instance,
  `window().set_position(<display origin>)`, `show()`, `set_fullscreen(true)`.
  **Stop:** hide/drop it. (Slint exposes `set_fullscreen`, `set_position`,
  `set_maximized` on `Window`; multiple component windows are supported.)
- **Display selection:** Slint's public API does NOT enumerate monitors, so use
  the `display-info` crate to list displays (name + bounds) for the picker and
  to get the target display's origin for `set_position`. Fallbacks if it proves
  unreliable: (a) offer "Primary/Secondary" by bounds only; (b) open a normal
  window the user drags to the projector and fullscreens with a key. The picker
  defaults to a non-primary display when one is present.
- **Live updates:** slide/theme/blank changes update the existing projector
  window's properties in place — no reopen.

## Control tab workflow & input

- **Load a hymn:** Library tab gains a **Project** action (button + Enter on the
  highlighted hymn) → loads it into `PresentationState`, switches to Control.
  Control tab has its own search box (reuses `Searcher`) to swap hymns.
- **Presenter view:** current hymn title + slide list (active highlighted,
  click to jump), Live mirror + Next peek (both theme-styled, small), theme
  picker, output-display picker, Start/Stop projecting, Blank, Prev/Next.
- **Keyboard (only when Control tab active, via `capture-key-pressed`):**
  `→`/`Space` next, `←` prev, `B` blank toggle, `Esc` stop. Guarded by the
  active-tab index so it doesn't conflict with the Library tab's arrow nav.
  Advancing past the last slide is a no-op (single hymn, no playlist).
- **Persistence:** `active_theme` and last-used `output_display` in
  `config.toml`; themes in their own JSON files.

## Error handling

- No second display / saved display index gone → fall back to primary, log,
  never crash.
- Corrupt theme JSON → skip it (log), keep others + built-in default.
- Missing font family → fall back to a system default font, still render.
- Missing background image path → fall back to solid/overlay color, log.
- Start projecting with no hymn → projector shows blank/idle; controls no-op
  until a hymn loads.
- User closes the projector window via the OS → cleanly return to "not
  projecting" state.

## Testing

- **`theme`:** JSON round-trip; `default()` exists + non-deletable; listing
  skips corrupt files and always includes default; save/load to a temp dir.
- **`PresentationState`:** next/prev clamping, blank toggle, load-hymn-resets-to
  -slide-0 — unit-tested headlessly.
- **GUI / projector / display targeting:** manual verification (needs real
  monitors and a window); logic kept thin, real coverage in core.

## Out of scope (YAGNI)

Playlists/queues of hymns; per-slide-type theme overrides; slide transitions/
animations; phone/remote control; projecting non-hymn content; exporting themed
slides back to PPTX. All are possible future follow-ups.
