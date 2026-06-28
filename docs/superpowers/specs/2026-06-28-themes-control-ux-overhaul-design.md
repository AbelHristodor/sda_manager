# Themes + Control UI/UX Overhaul — Design

**Date:** 2026-06-28
**Status:** Approved

## Summary

Re-UX the **Themes** and **Control** tabs. Current problems: text overflows the
preview, buttons are oversized, font/weight are bare text inputs (should be
dropdowns), and there are no real color controls. This overhaul introduces a
small set of reusable styled controls and rebuilds both tabs around them, adds a
visual **thumbnail theme picker**, proper **dropdowns**, **sliders**, and
**color fields with presets**, and enforces **fixed-size, non-overflowing**
preview boxes.

Scope: Themes + Control only. Library / Downloader / Settings are unchanged.

## Decisions (from brainstorming)

- **Reusable styled controls** added to `app.slint`, used by both tabs.
- **Themes editor:** contained, centered preview box (not full width); 2-column
  form (left = Name + Text controls; right = Alignment + Colors); small footer
  Save.
- **Color pickers:** swatch + `#RRGGBB` hex field + preset swatch row. No OS
  color dialog.
- **Font family:** populated from enumerated **system fonts** (fallback: curated
  list if enumeration unavailable). Weight is a dropdown; size is a slider+number.
- **Control tab:** fixed-size centered Live (300×169) and Next (300×90) boxes;
  compact toolbar; full-width search; results list left.
- **Theme selection lives ONLY in the Themes tab** (separation of concerns).
  - Thumbnail grid there; clicking a thumbnail **loads it into the editor**
    (edit-selection).
  - A separate **"Set as active"** action marks which theme the Control tab
    projects with (active-selection). Edit-selection and active-selection are
    independent — you can edit one theme while another stays active.
- **Control tab has no theme selector** — toolbar is Output + Start/Stop + Blank;
  it always projects with the active theme.
- **Thumbnail picker:** Control tab does NOT get it; it's Themes-only.

## Section 1: Shared styled components (new, in app.slint)

Pure-Slint reusable components (no new Rust types):

- `PrimaryButton` / `SecondaryButton` — fixed ~32px height, padded, theme-colored
  (accent / field). Replace ad-hoc oversized `Button`s.
- `FieldLabel` + `LabeledRow` — consistent label + control rows for the 2-column
  grid.
- `ColorField` — a swatch + editable `#RRGGBB` hex `LineEdit` + a row of preset
  swatches; exposes `in-out property <color> value` and a `changed()` callback.
- `ThemeThumb` — a mini-slide (real background + sample text, using the same
  flattened-property contract as the projector, scaled down) with a caption and
  two visual states: `selected` (accent border) and `active` (a "● Active"
  badge). The reusable heart of the thumbnail picker.

Font / Weight use Slint's stock `ComboBox`; size uses `Slider` + a number label;
alignment uses three small toggle buttons (L/C/R).

## Section 2: Themes tab

- **Left column (~170–190px):** a **thumbnail grid** of `ThemeThumb`s — every
  theme as a mini-slide. Distinct **edit-selection** (accent border) and
  **active** ("● Active" badge) states. Below: compact **+ New** / **Delete**
  row and a **"Set as active"** button.
- **Right column (editor):**
  - Contained, centered **preview box** (~340×150 fixed) at top, live-updating.
  - **2-column form:** left = Name (`LineEdit`), Font (`ComboBox`),
    Weight (`ComboBox`), Size (`Slider` + number); right = Alignment (3-way
    toggle), Text color (`ColorField`), Background color (`ColorField`).
  - Footer bar with small right-aligned **Save** (`PrimaryButton`).

**State model (Rust):** `selected` theme (loaded in editor) and `active` theme
(projected) tracked separately.
- Thumbnail click → set `selected`, load editor.
- "Set as active" → set `active`, persist `config.active_theme`, refresh badges.
- Font ComboBox model = enumerated system font families (fallback curated list).

## Section 3: Control tab

- **Top toolbar (panel):** Output display `ComboBox`, Start/Stop
  (`PrimaryButton`), Blank (`SecondaryButton`). **No theme picker.**
- **Full-width search** (`LineEdit`).
- **Two panes:** results `StandardListView` (~40% width) left; right pane: hymn
  title, **LIVE** fixed 300×169 box (centered, text wraps/centers) + slide
  counter, **NEXT** fixed 300×90 box, centered **Prev/Next**, keyboard hint.

**Wiring impact:** remove the `ctl_theme_picked` handler and the
`ctl-theme-names` model/property. `push_to_projector` already reads the shared
`active_theme`. Start/Stop/nav/blank/search/Project wiring is otherwise
unchanged. The active theme is set in the Themes tab; Control reads it.

## Section 4: Error handling

- **Hex color field:** invalid/partial input ignored until a valid 6-digit
  `#RRGGBB`; swatch updates only on a parseable value. Presets apply instantly.
- **Font enumeration fails/empty:** fall back to a curated family list (dropdown
  never empty). A theme referencing a missing font still renders (Slint falls
  back to a default face).
- **No active theme / active theme deleted:** Control projects with
  `Theme::default()`; deleting the active theme resets active to Default and
  persists that.
- **Thumbnail rendering** reuses the projector flatten path → cannot diverge
  from what actually projects.

## Testing

- **Core:** add hex ↔ `Rgba` helpers to `theme` (`Rgba::from_hex(&str) ->
  Option<Rgba>`, `Rgba::to_hex(&self) -> String`); unit-test round-trip and
  rejection of invalid input. If the "delete active → Default" rule becomes a
  non-trivial standalone function, unit-test it too.
- **GUI:** thin; manual verification + per-step `cargo build`/`clippy` and a
  headless `cargo run` boot smoke (no panic).
- **Font enumeration crate** (`font-kit` or lighter) confirmed to build on macOS
  at plan time; curated-list fallback means it cannot block the overhaul.

## Out of scope (YAGNI)

- Library / Downloader / Settings restyling (separate follow-up).
- The deferred renderer features ("5b"): gradient/image backgrounds, overlay,
  text shadow/outline, footer, auto-fit font sizing. This overhaul styles the
  editor controls only for properties the renderer currently honors (solid
  background + core text); 5b's controls slot into the same grouped form later.
- OS-native color dialog; theme import/export; per-slide-type styling.
