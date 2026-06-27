# Slide-by-Slide Preview + Status Bar Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Show hymn preview one slide at a time with ←/→ + Prev/Next navigation, and a bottom status bar reading `<title> — slide N/total`.

**Architecture:** Preserve per-slide text in the core parser (`slides: Vec<String>`), keeping `body = slides.join("\n")` so search is unchanged. The Slint preview pane renders the current slide's text, a Prev/Next row, and a pinned status bar; Rust tracks the current slide index and resolves the highlighted entry's slides via the existing row→entry-index map.

**Tech Stack:** Rust, Slint 1.x (StandardListView, FocusScope capture-key-pressed), existing hymnal-core/hymnal-gui crates.

---

## File Structure

- `crates/hymnal-core/src/pptx.rs` — add `slides: Vec<String>` to `ParsedPptx`; build it during extraction; derive `body` from it.
- `crates/hymnal-core/src/model.rs` — add `slides: Vec<String>` to `HymnEntry`.
- `crates/hymnal-core/src/index.rs` — populate `slides` in `build_index`/`refresh_index`; bump `CACHE_VERSION` to 3.
- `crates/hymnal-core/tests/pptx_test.rs` — assert slide count, slides[0] title, `body == slides.join("\n")`.
- `crates/hymnal-gui/ui/app.slint` — replace `preview-title`/`preview-body` with `slide-text` + `preview-status` + slide state; add Prev/Next buttons and ←/→ keys.
- `crates/hymnal-gui/src/main.rs` — slide state wiring + `show_slide` helper.

---

## Task 1: Core — preserve per-slide text

**Files:**
- Modify: `crates/hymnal-core/src/pptx.rs`
- Modify: `crates/hymnal-core/src/model.rs`
- Modify: `crates/hymnal-core/src/index.rs`
- Test: `crates/hymnal-core/tests/pptx_test.rs`

- [ ] **Step 1: Write the failing test**

Append to `crates/hymnal-core/tests/pptx_test.rs`:
```rust
#[test]
fn preserves_text_per_slide() {
    let parsed = extract(Path::new("tests/fixtures/001.pptx")).unwrap();
    // Hymn 1 has 5 slides.
    assert_eq!(parsed.slides.len(), 5);
    // The first slide holds the title.
    assert!(parsed.slides[0].contains("Plecaţi-vă lui Dumnezeu"));
    // body is exactly the slides joined by newlines (search input unchanged).
    assert_eq!(parsed.body, parsed.slides.join("\n"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p hymnal-core --test pptx_test preserves_text_per_slide`
Expected: FAIL — `ParsedPptx` has no field `slides`.

- [ ] **Step 3: Add `slides` to `ParsedPptx` and populate it**

In `crates/hymnal-core/src/pptx.rs`, change the struct:
```rust
/// Raw text extracted from a single .pptx, before it becomes a HymnEntry.
pub struct ParsedPptx {
    pub number: Option<u32>,
    pub title: String,
    pub body: String,
    /// Text of each slide (paragraphs joined by `\n`), in presentation order.
    pub slides: Vec<String>,
}
```

Replace the slide-collection loop and the final construction in `extract`
(the block that currently builds `all_lines`, `first_slide_lines`, `title`,
`body`) with:
```rust
    let mut slides: Vec<String> = Vec::with_capacity(slide_names.len());
    let mut first_slide_lines: Vec<String> = Vec::new();
    for (idx, name) in slide_names.iter().enumerate() {
        let mut xml = String::new();
        zip.by_name(name)?.read_to_string(&mut xml)?;
        let lines = slide_text_lines(&xml)?;
        if idx == 0 {
            first_slide_lines = lines.clone();
        }
        slides.push(lines.join("\n"));
    }

    let title = pick_title(&first_slide_lines);
    let body = slides.join("\n");
    Ok(ParsedPptx { number, title, body, slides })
```

- [ ] **Step 4: Add `slides` to `HymnEntry`**

In `crates/hymnal-core/src/model.rs`, add the field to `HymnEntry` (after `body`):
```rust
    /// Concatenated verse text from all slides, for full-text search.
    pub body: String,
    /// Text of each slide (paragraphs joined by `\n`), in presentation order.
    pub slides: Vec<String>,
```

- [ ] **Step 5: Populate `slides` when building entries + bump cache version**

In `crates/hymnal-core/src/index.rs`, both `build_index` and `refresh_index`
construct `HymnEntry { number, title, body, path, library, mtime }`. Add
`slides: parsed.slides,` to BOTH constructors (place it right after
`body: parsed.body,`). There are two such blocks — update both.

Then bump the cache version so old caches (lacking `slides`) are rejected:
```rust
pub const CACHE_VERSION: u32 = 3;
```

- [ ] **Step 6: Run the test to verify it passes**

Run: `cargo test -p hymnal-core --test pptx_test preserves_text_per_slide`
Expected: PASS.

- [ ] **Step 7: Run the full core suite (nothing else broke)**

Run: `cargo test -p hymnal-core`
Expected: PASS (all tests across fold, pptx, index, library, search, sync).
Note: `cache_round_trips` and `refresh_reuses_unchanged_entries` build
`HymnEntry` literals in the test file — if the compiler complains about a
missing `slides` field there, add `slides: vec![],` to those literals in
`crates/hymnal-core/tests/index_test.rs`.

- [ ] **Step 8: Commit**

```bash
git add crates/hymnal-core/src/pptx.rs crates/hymnal-core/src/model.rs crates/hymnal-core/src/index.rs crates/hymnal-core/tests/
git commit -m "feat(core): preserve per-slide text; bump cache version to 3"
```

---

## Task 2: GUI — slide-by-slide preview + status bar

**Files:**
- Modify: `crates/hymnal-gui/ui/app.slint`
- Modify: `crates/hymnal-gui/src/main.rs`

- [ ] **Step 1: Update the Slint UI**

Replace the entire contents of `crates/hymnal-gui/ui/app.slint` with:
```slint
import { LineEdit, StandardListView, Button, ScrollView, VerticalBox, HorizontalBox } from "std-widgets.slint";

export component AppWindow inherits Window {
    title: "Hymn Finder";
    preferred-width: 900px;
    preferred-height: 600px;

    // Rows shown in the results finder, one line of text each (fzf-style).
    in property <[StandardListViewItem]> results;
    in property <string> status;
    // Two-way bound to the list's highlighted row so Rust can read/set it.
    in-out property <int> current-index: -1;

    // Preview: one slide at a time.
    in property <string> slide-text;
    in property <int> slide-count;
    in-out property <int> slide-index: 0;
    in property <string> preview-status;

    callback query-changed(string);
    callback open-current();
    callback reveal-current();
    callback current-changed(int);
    callback prev-slide();
    callback next-slide();

    // Typing focus lives in the search field; the surrounding FocusScope
    // catches arrow keys to drive the list (Up/Down) and slides (Left/Right).
    forward-focus: search;

    key-handler := FocusScope {
        // capture-key-pressed fires top-down BEFORE the focused LineEdit's
        // TextInput sees the key, so we can claim the arrows.
        capture-key-pressed(event) => {
            if (event.text == Key.UpArrow) {
                if (root.current-index > 0) {
                    list.set-current-item(root.current-index - 1);
                }
                return accept;
            }
            if (event.text == Key.DownArrow) {
                if (root.current-index < root.results.length - 1) {
                    list.set-current-item(root.current-index + 1);
                }
                return accept;
            }
            if (event.text == Key.LeftArrow) {
                root.prev-slide();
                return accept;
            }
            if (event.text == Key.RightArrow) {
                root.next-slide();
                return accept;
            }
            // Enter handled by the LineEdit's `accepted`; let other keys type.
            return reject;
        }

        VerticalBox {
            Text {
                text: root.status;
                color: gray;
            }

            search := LineEdit {
                placeholder-text: "Search by number, title, or lyrics…";
                edited(text) => {
                    root.query-changed(text);
                }
                accepted(text) => {
                    root.open-current();
                }
            }

            HorizontalBox {
                list := StandardListView {
                    width: 40%;
                    model: root.results;
                    current-item <=> root.current-index;
                    current-item-changed(index) => {
                        root.current-changed(index);
                    }
                }

                VerticalBox {
                    // Current slide text.
                    ScrollView {
                        Text {
                            text: root.slide-text;
                            font-size: 16px;
                            wrap: word-wrap;
                        }
                    }

                    // Slide navigation.
                    HorizontalBox {
                        height: 36px;
                        Button {
                            text: "‹ Prev";
                            clicked => {
                                root.prev-slide();
                            }
                        }
                        Button {
                            text: "Next ›";
                            clicked => {
                                root.next-slide();
                            }
                        }
                    }

                    // Open / reveal actions.
                    HorizontalBox {
                        height: 40px;
                        Button {
                            text: "Open in PowerPoint";
                            clicked => {
                                root.open-current();
                            }
                        }
                        Button {
                            text: "Reveal in folder";
                            clicked => {
                                root.reveal-current();
                            }
                        }
                    }

                    // Status bar pinned at the bottom of the preview.
                    Rectangle {
                        height: 28px;
                        background: #eeeeee;
                        Text {
                            x: 6px;
                            vertical-alignment: center;
                            text: root.preview-status;
                            color: #333333;
                            font-size: 12px;
                        }
                    }
                }
            }
        }
    }
}
```

- [ ] **Step 2: Build to confirm the UI compiles (Rust glue still references old props → may warn/err next)**

Run: `cargo build -p hymnal-gui 2>&1 | tail -20`
Expected: compile ERRORS from `main.rs` referencing the removed
`set_preview_title`/`set_preview_body` and missing `on_prev_slide`/`on_next_slide`.
This is expected; the next step fixes `main.rs`. (If the `.slint` itself has a
syntax error, fix that first.)

- [ ] **Step 3: Add the `show_slide` helper to `main.rs`**

In `crates/hymnal-gui/src/main.rs`, add this free function just below the
existing `row_label` function:
```rust
/// Update the preview to show slide `idx` of `slides` for a hymn titled
/// `title` (numbered `number`). Clamps `idx` into range; sets slide text,
/// count, index, and the bottom status bar string.
fn show_slide(
    ui: &AppWindow,
    number: Option<u32>,
    title: &str,
    slides: &[String],
    idx: i32,
) {
    let count = slides.len() as i32;
    if count == 0 {
        ui.set_slide_text("".into());
        ui.set_slide_count(0);
        ui.set_slide_index(0);
        ui.set_preview_status(SharedString::from(format!("{title} — 0 slides")));
        return;
    }
    let idx = idx.clamp(0, count - 1);
    let number_prefix = number.map(|n| format!("{n}. ")).unwrap_or_default();
    ui.set_slide_text(SharedString::from(slides[idx as usize].clone()));
    ui.set_slide_count(count);
    ui.set_slide_index(idx);
    ui.set_preview_status(SharedString::from(format!(
        "{number_prefix}{title} — slide {}/{}",
        idx + 1,
        count
    )));
}
```

- [ ] **Step 4: Rewrite the `on_current_changed` handler to show slide 0**

Replace the existing `on_current_changed` closure (the block starting
`ui.on_current_changed(move |idx| {` and ending at its closing `});`) with:
```rust
    // ---- Highlight changed (keyboard arrows or click) -> show slide 0 ----
    let searcher_for_sel = searcher.clone();
    let rows_for_sel = row_to_entry.clone();
    let weak6 = ui.as_weak();
    ui.on_current_changed(move |idx| {
        let Some(ui) = weak6.upgrade() else { return };
        if idx < 0 {
            ui.set_slide_text("".into());
            ui.set_slide_count(0);
            ui.set_preview_status("".into());
            return;
        }
        let guard = searcher_for_sel.borrow();
        let Some(s) = guard.as_ref() else { return };
        let entry = rows_for_sel
            .borrow()
            .get(idx as usize)
            .and_then(|&ei| s.entry(ei));
        if let Some(entry) = entry {
            debug!("preview #{:?} {} ({} slides)", entry.number, entry.title, entry.slides.len());
            show_slide(&ui, entry.number, &entry.title, &entry.slides, 0);
        }
    });
```

- [ ] **Step 5: Add `on_prev_slide` / `on_next_slide` handlers**

Insert these two handlers immediately AFTER the `on_current_changed` block from
Step 4 (and before the `on_open_current` block):
```rust
    // ---- Slide navigation: step within the highlighted hymn's slides ----
    let searcher_for_prev = searcher.clone();
    let rows_for_prev = row_to_entry.clone();
    let weak_prev = ui.as_weak();
    ui.on_prev_slide(move || {
        let Some(ui) = weak_prev.upgrade() else { return };
        let row = ui.get_current_index();
        if row < 0 {
            return;
        }
        let guard = searcher_for_prev.borrow();
        let Some(s) = guard.as_ref() else { return };
        let entry = rows_for_prev.borrow().get(row as usize).and_then(|&ei| s.entry(ei));
        if let Some(entry) = entry {
            show_slide(&ui, entry.number, &entry.title, &entry.slides, ui.get_slide_index() - 1);
        }
    });

    let searcher_for_next = searcher.clone();
    let rows_for_next = row_to_entry.clone();
    let weak_next = ui.as_weak();
    ui.on_next_slide(move || {
        let Some(ui) = weak_next.upgrade() else { return };
        let row = ui.get_current_index();
        if row < 0 {
            return;
        }
        let guard = searcher_for_next.borrow();
        let Some(s) = guard.as_ref() else { return };
        let entry = rows_for_next.borrow().get(row as usize).and_then(|&ei| s.entry(ei));
        if let Some(entry) = entry {
            show_slide(&ui, entry.number, &entry.title, &entry.slides, ui.get_slide_index() + 1);
        }
    });
```

- [ ] **Step 6: Build to confirm everything compiles**

Run: `cargo build -p hymnal-gui 2>&1 | tail -20`
Expected: `Finished` with no errors. If `unused import` warnings appear for
`SharedString`, ignore them (it is used by `show_slide` and `row_label`).

- [ ] **Step 7: Smoke-test the data path live (logging)**

Run: `RUST_LOG=hymnal_gui=debug timeout 30 cargo run -q -p hymnal-gui 2>&1 | grep -iE "indexed|total|searcher ready|preview #|slides|panic|error" | head`
Expected: logs show `total 921 hymns indexed`, `searcher ready with 921 hymns`,
and a `preview #Some(1) ... (5 slides)` line for the initial selection.
(The window itself needs a display; the worker/preview logs print regardless.)

- [ ] **Step 8: Commit**

```bash
git add crates/hymnal-gui/ui/app.slint crates/hymnal-gui/src/main.rs
git commit -m "feat(gui): slide-by-slide preview with ←/→ nav and status bar"
```

---

## Task 3: Manual verification

**Files:** none (verification only).

- [ ] **Step 1: Run the release build and verify behavior**

Run: `cargo run --release -p hymnal-gui`
Verify:
- Selecting a hymn (↑/↓ or click) shows its **first slide** text and the status
  bar reads `<number>. <title> — slide 1/N`.
- **→** advances to the next slide (status `slide 2/N`), **←** goes back; both
  clamp at the ends (no wrap, no crash).
- **‹ Prev / Next ›** buttons do the same.
- **↑/↓** still move the results list (and reset the preview to slide 1 of the
  newly highlighted hymn).
- **Enter** / "Open in PowerPoint" opens the highlighted hymn's `.pptx`.
- A 1-slide hymn (e.g. search for a short one) shows `slide 1/1` and Prev/Next
  do nothing at the bounds.

- [ ] **Step 2: No code changes if all pass**

This task is verification only. If a behavior is wrong, fix in Task 1/2 code and
re-run.

---

## Self-Review Notes

- **Spec coverage:** per-slide parsing + `body = slides.join("\n")` (Task 1);
  `slides` on HymnEntry + cache bump to 3 (Task 1, Steps 4–5); one-slide-at-a-time
  preview, ←/→ + Prev/Next, ↑/↓ unaffected, reset-to-0 on hymn change, bottom
  status bar `title — slide N/total` (Task 2); test asserts slide count +
  slides[0] title + body equality (Task 1); manual GUI verification (Task 3).
  All spec sections map to a task.
- **Status bar format:** spec says `Title — slide N/total`; implementation
  prefixes the hymn number (`356. Ca un cerb… — slide 3/12`), matching the
  earlier preview-title format. This is a deliberate, minor enrichment; flag to
  the user if pure `Title — slide N/total` is preferred.
- **Type consistency:** `show_slide(ui, number, title, slides, idx)` is called
  identically in current-changed, prev, and next handlers; Slint property
  setters/getters used (`set_slide_text`, `set_slide_count`, `set_slide_index`,
  `set_preview_status`, `get_slide_index`, `get_current_index`) match the
  `in`/`in-out` properties declared in `app.slint`.
- **Placeholder scan:** no TBD/TODO; all steps contain concrete code/commands.
```
