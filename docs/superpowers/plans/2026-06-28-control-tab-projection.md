# Control Tab — Native Themed Projection Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a Control tab that projects hymn slide text into a themed full-screen window on a chosen display, plus a Themes editor for creating/editing named themes.

**Architecture:** Pure, testable core modules (`theme` = the Theme model + named-JSON storage; `present` = PresentationState slide/blank logic) drive a thin Slint GUI: a second `ProjectorWindow` component rendered from flattened theme props, a Control presenter tab, and a Themes editor with a live preview. Display targeting via the `display-info` crate.

**Tech Stack:** Rust, Slint 1.x (multi-window via `set_position`/`set_fullscreen`), `serde_json` (theme files), `display-info` (monitor enumeration), existing hymnal-core/hymnal-gui crates.

---

## Phasing (each phase is shippable on its own)

- **Phase 1 (Tasks 1–2):** core `theme` module — model + named-JSON storage. Fully unit-tested.
- **Phase 2 (Task 3):** core `present` module — PresentationState logic. Fully unit-tested.
- **Phase 3 (Tasks 4–5):** ProjectorWindow + display targeting (GUI; manual verify).
- **Phase 4 (Tasks 6–7):** Themes editor area with live preview (GUI; manual verify).
- **Phase 5 (Tasks 8–9):** Control tab + Library "Project" wiring + keyboard (GUI; manual verify).

Each phase ends green (builds, core tests pass) and is independently useful.

---

## Current-state notes (read before starting)

- `hymnal-core` modules: fold, model, index, library, pptx, search, sync,
  downloader, refresh, update, i18n. Add `theme` and `present`.
- `Config` (in `library.rs`) has `default_repo_url, libraries, download_dir,
  language`, all `#[serde(default)]` for the optional ones; persisted as TOML
  via `Config::to_toml`/`save`. Add `active_theme: Option<String>` and
  `output_display: Option<i32>` the same way.
- Dir helpers in `library.rs` use
  `directories::ProjectDirs::from("org","hymnal","HymnFinder")`. Add
  `themes_dir()` the same way.
- `serde_json` is NOT yet a core dependency — Task 1 adds it.
- `HymnEntry` has `number: Option<String>, title: String, slides: Vec<String>`.
- The GUI uses a custom sidebar nav (`Sidebar` with `NavItem`s, `active-tab`
  0=Library, 1=Video Downloader, 2=Settings) and `if root.active-tab == N:`
  panel blocks in `AppWindow`. Strings flow through the `I18n` global filled by
  `apply_language` in `main.rs`. Add nav items 3=Control, 4=Themes.
- Keyboard handling pattern: `FocusScope { capture-key-pressed(event) => {...} }`
  inside each panel, so keys only reach the panel that's shown.

---

## Phase 1 — Core theme model + storage

### Task 1: `Theme` data model + JSON round-trip

**Files:**
- Modify: `crates/hymnal-core/Cargo.toml`
- Create: `crates/hymnal-core/src/theme.rs`
- Modify: `crates/hymnal-core/src/lib.rs`
- Test: `crates/hymnal-core/tests/theme_test.rs`

- [ ] **Step 1: Add `serde_json` dependency**

In `crates/hymnal-core/Cargo.toml` under `[dependencies]`, add:
```toml
serde_json = "1"
```

- [ ] **Step 2: Declare the module**

In `crates/hymnal-core/src/lib.rs`, add with the other `pub mod` lines:
```rust
pub mod theme;
```

- [ ] **Step 3: Write the failing test**

Create `crates/hymnal-core/tests/theme_test.rs`:
```rust
use hymnal_core::theme::{Background, Theme};

#[test]
fn theme_json_round_trips() {
    let t = Theme::default();
    let json = t.to_json().unwrap();
    let back = Theme::from_json(&json).unwrap();
    assert_eq!(back, t);
}

#[test]
fn default_theme_has_name_and_dark_background() {
    let t = Theme::default();
    assert_eq!(t.name, "Default");
    assert!(matches!(t.background.kind, Background::Solid { .. }));
}

#[test]
fn custom_theme_round_trips_all_fields() {
    let mut t = Theme::default();
    t.name = "Christmas".into();
    t.text.font_family = "Georgia".into();
    t.text.font_size_pt = Some(48.0);
    t.text.font_weight = 700;
    t.background.kind = Background::Image { path: "/tmp/bg.jpg".into() };
    t.background.overlay_opacity = 0.4;
    let back = Theme::from_json(&t.to_json().unwrap()).unwrap();
    assert_eq!(back, t);
}
```

- [ ] **Step 4: Run test to verify it fails**

Run: `cargo test -p hymnal-core --test theme_test`
Expected: FAIL — `theme` module / `Theme` not found.

- [ ] **Step 5: Implement the model**

Create `crates/hymnal-core/src/theme.rs`:
```rust
//! Theme data model: a saved set of styling applied to projected slides.
//! Serialized one-theme-per-JSON-file; see `theme_store` for persistence.

use serde::{Deserialize, Serialize};

/// RGBA colour as 4 bytes; serialized as a "#rrggbbaa" string is overkill —
/// keep it as a struct for round-trip simplicity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba {
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Rgba { r, g, b, a }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HAlign {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VAlign {
    Top,
    Middle,
    Bottom,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Shadow {
    pub enabled: bool,
    pub color: Rgba,
    pub blur: f32,
    pub dx: f32,
    pub dy: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Outline {
    pub enabled: bool,
    pub color: Rgba,
    pub width: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextStyle {
    pub font_family: String,
    /// `None` = auto-fit; `Some(pt)` = fixed point size.
    pub font_size_pt: Option<f32>,
    pub font_weight: u16,
    pub color: Rgba,
    pub h_align: HAlign,
    pub v_align: VAlign,
    pub line_spacing: f32,
    pub shadow: Shadow,
    pub outline: Outline,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Background {
    Solid { color: Rgba },
    Gradient { from: Rgba, to: Rgba, angle: f32 },
    Image { path: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageFit {
    Cover,
    Contain,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BackgroundStyle {
    pub kind: Background,
    pub image_fit: ImageFit,
    pub overlay_color: Rgba,
    pub overlay_opacity: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayoutStyle {
    pub margin: f32,
    /// Fraction of slide width (0..1) the text may occupy.
    pub max_text_width: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FooterContent {
    HymnNumberTitle,
    SlideCounter,
    None,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FooterStyle {
    pub show: bool,
    pub content: FooterContent,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Theme {
    pub name: String,
    pub text: TextStyle,
    pub background: BackgroundStyle,
    pub layout: LayoutStyle,
    pub footer: FooterStyle,
}

impl Default for Theme {
    fn default() -> Self {
        Theme {
            name: "Default".into(),
            text: TextStyle {
                font_family: "sans-serif".into(),
                font_size_pt: Some(44.0),
                font_weight: 700,
                color: Rgba::new(255, 255, 255, 255),
                h_align: HAlign::Center,
                v_align: VAlign::Middle,
                line_spacing: 1.3,
                shadow: Shadow {
                    enabled: false,
                    color: Rgba::new(0, 0, 0, 180),
                    blur: 4.0,
                    dx: 2.0,
                    dy: 2.0,
                },
                outline: Outline {
                    enabled: false,
                    color: Rgba::new(0, 0, 0, 255),
                    width: 1.0,
                },
            },
            background: BackgroundStyle {
                kind: Background::Solid {
                    color: Rgba::new(11, 31, 58, 255),
                },
                image_fit: ImageFit::Cover,
                overlay_color: Rgba::new(0, 0, 0, 255),
                overlay_opacity: 0.0,
            },
            layout: LayoutStyle {
                margin: 48.0,
                max_text_width: 0.8,
            },
            footer: FooterStyle {
                show: false,
                content: FooterContent::None,
            },
        }
    }
}

impl Theme {
    pub fn to_json(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    pub fn from_json(text: &str) -> anyhow::Result<Theme> {
        Ok(serde_json::from_str(text)?)
    }

    /// The built-in default theme cannot be deleted or overwritten on disk.
    pub fn is_builtin_default(&self) -> bool {
        self.name == "Default"
    }
}
```

- [ ] **Step 6: Run test to verify it passes**

Run: `cargo test -p hymnal-core --test theme_test`
Expected: PASS (all three).

- [ ] **Step 7: Commit**

```bash
git add crates/hymnal-core/Cargo.toml crates/hymnal-core/src/theme.rs crates/hymnal-core/src/lib.rs crates/hymnal-core/tests/theme_test.rs
git commit -m "feat(core): Theme data model with JSON round-trip"
```

---

### Task 2: Theme storage (named files) + `themes_dir`

**Files:**
- Modify: `crates/hymnal-core/src/library.rs` (add `themes_dir`)
- Modify: `crates/hymnal-core/src/theme.rs` (add a `store` submodule)
- Test: `crates/hymnal-core/tests/theme_store_test.rs`

- [ ] **Step 1: Add `themes_dir` helper**

In `crates/hymnal-core/src/library.rs`, after `index_cache_path()`, add:
```rust
/// Directory holding user theme JSON files (one per theme).
pub fn themes_dir() -> Option<std::path::PathBuf> {
    directories::ProjectDirs::from("org", "hymnal", "HymnFinder")
        .map(|d| d.config_dir().join("themes"))
}
```

- [ ] **Step 2: Write the failing test**

Create `crates/hymnal-core/tests/theme_store_test.rs`:
```rust
use hymnal_core::theme::store::{delete_theme, list_themes, load_theme, save_theme};
use hymnal_core::theme::Theme;

#[test]
fn save_then_list_and_load_includes_default_and_custom() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    // Empty dir: listing still yields the built-in default.
    let names: Vec<String> = list_themes(root).iter().map(|t| t.name.clone()).collect();
    assert!(names.contains(&"Default".to_string()));

    // Save a custom theme, then it appears in the list and loads back.
    let mut t = Theme::default();
    t.name = "Christmas".into();
    save_theme(root, &t).unwrap();
    let names: Vec<String> = list_themes(root).iter().map(|t| t.name.clone()).collect();
    assert!(names.contains(&"Christmas".to_string()));
    assert!(names.contains(&"Default".to_string()));
    let loaded = load_theme(root, "Christmas").unwrap();
    assert_eq!(loaded.name, "Christmas");
}

#[test]
fn corrupt_theme_file_is_skipped_default_remains() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir_all(root).unwrap();
    std::fs::write(root.join("broken.json"), b"{ not valid json").unwrap();
    let names: Vec<String> = list_themes(root).iter().map(|t| t.name.clone()).collect();
    // Corrupt file skipped; default still present; no panic.
    assert!(names.contains(&"Default".to_string()));
    assert!(!names.contains(&"broken".to_string()));
}

#[test]
fn cannot_delete_builtin_default() {
    let dir = tempfile::tempdir().unwrap();
    assert!(delete_theme(dir.path(), "Default").is_err());
}

#[test]
fn delete_removes_custom_theme() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let mut t = Theme::default();
    t.name = "Temp".into();
    save_theme(root, &t).unwrap();
    delete_theme(root, "Temp").unwrap();
    let names: Vec<String> = list_themes(root).iter().map(|t| t.name.clone()).collect();
    assert!(!names.contains(&"Temp".to_string()));
}
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cargo test -p hymnal-core --test theme_store_test`
Expected: FAIL — `theme::store` not found.

- [ ] **Step 4: Implement the store submodule**

Append to `crates/hymnal-core/src/theme.rs`:
```rust
/// Named-file persistence for themes. One `<name>.json` per theme in a
/// directory; the built-in `Default` is always present and never written or
/// deleted on disk.
pub mod store {
    use super::Theme;
    use anyhow::{bail, Result};
    use std::path::Path;

    /// Sanitize a theme name into a safe file stem (alphanumerics, space, -, _).
    fn file_stem(name: &str) -> String {
        name.chars()
            .map(|c| if c.is_alphanumeric() || c == ' ' || c == '-' || c == '_' { c } else { '_' })
            .collect()
    }

    /// List all themes: the built-in default first, then every valid
    /// `<dir>/*.json`. Corrupt files are skipped (logged via eprintln).
    pub fn list_themes(dir: &Path) -> Vec<Theme> {
        let mut out = vec![Theme::default()];
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("json") {
                    continue;
                }
                match std::fs::read_to_string(&path).ok().and_then(|s| Theme::from_json(&s).ok()) {
                    Some(t) if !t.is_builtin_default() => out.push(t),
                    Some(_) => {} // a file literally named Default — dedup, default already present
                    None => eprintln!("skip corrupt theme {}", path.display()),
                }
            }
        }
        out
    }

    /// Load one theme by name. Returns the built-in default for "Default".
    pub fn load_theme(dir: &Path, name: &str) -> Result<Theme> {
        if name == "Default" {
            return Ok(Theme::default());
        }
        let path = dir.join(format!("{}.json", file_stem(name)));
        let text = std::fs::read_to_string(&path)?;
        Theme::from_json(&text)
    }

    /// Save a theme to `<dir>/<name>.json`. Refuses to overwrite the built-in
    /// default's reserved name.
    pub fn save_theme(dir: &Path, theme: &Theme) -> Result<()> {
        if theme.is_builtin_default() {
            bail!("the built-in Default theme cannot be saved over");
        }
        std::fs::create_dir_all(dir)?;
        let path = dir.join(format!("{}.json", file_stem(&theme.name)));
        std::fs::write(path, theme.to_json()?)?;
        Ok(())
    }

    /// Delete a theme file. Refuses to delete the built-in default.
    pub fn delete_theme(dir: &Path, name: &str) -> Result<()> {
        if name == "Default" {
            bail!("the built-in Default theme cannot be deleted");
        }
        let path = dir.join(format!("{}.json", file_stem(name)));
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        Ok(())
    }
}
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test -p hymnal-core --test theme_store_test`
Expected: PASS (all four).

- [ ] **Step 6: Run the full core suite + commit**

Run: `cargo test -p hymnal-core` (expect all pass).
```bash
git add crates/hymnal-core/src/theme.rs crates/hymnal-core/src/library.rs crates/hymnal-core/tests/theme_store_test.rs
git commit -m "feat(core): named-file theme storage (list/load/save/delete)"
```

---

## Phase 2 — Core presentation state

### Task 3: `PresentationState`

**Files:**
- Create: `crates/hymnal-core/src/present.rs`
- Modify: `crates/hymnal-core/src/lib.rs`
- Test: `crates/hymnal-core/tests/present_test.rs`

- [ ] **Step 1: Declare the module**

In `crates/hymnal-core/src/lib.rs`, add `pub mod present;`.

- [ ] **Step 2: Write the failing test**

Create `crates/hymnal-core/tests/present_test.rs`:
```rust
use hymnal_core::present::PresentationState;

fn slides() -> Vec<String> {
    vec!["title".into(), "verse 1".into(), "verse 2".into()]
}

#[test]
fn load_hymn_resets_to_first_slide_unblanked() {
    let mut p = PresentationState::default();
    p.blank = true;
    p.load_hymn(Some("150".into()), "T".into(), slides());
    assert_eq!(p.index, 0);
    assert!(!p.blank);
    assert_eq!(p.current_slide(), Some("title"));
}

#[test]
fn next_and_prev_clamp_at_bounds() {
    let mut p = PresentationState::default();
    p.load_hymn(None, "T".into(), slides());
    assert_eq!(p.index, 0);
    p.prev(); // already at start
    assert_eq!(p.index, 0);
    p.next();
    p.next();
    assert_eq!(p.index, 2);
    p.next(); // past end — clamp
    assert_eq!(p.index, 2);
    p.prev();
    assert_eq!(p.index, 1);
}

#[test]
fn blank_toggles() {
    let mut p = PresentationState::default();
    p.load_hymn(None, "T".into(), slides());
    assert!(!p.blank);
    p.toggle_blank();
    assert!(p.blank);
    p.toggle_blank();
    assert!(!p.blank);
}

#[test]
fn current_slide_none_when_empty() {
    let p = PresentationState::default();
    assert_eq!(p.current_slide(), None);
    assert_eq!(p.slide_count(), 0);
}

#[test]
fn next_slide_peek() {
    let mut p = PresentationState::default();
    p.load_hymn(None, "T".into(), slides());
    assert_eq!(p.next_slide(), Some("verse 1"));
    p.next();
    p.next();
    assert_eq!(p.next_slide(), None); // last slide, nothing after
}
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cargo test -p hymnal-core --test present_test`
Expected: FAIL — `present` module not found.

- [ ] **Step 4: Implement**

Create `crates/hymnal-core/src/present.rs`:
```rust
//! Pure presentation state for the Control tab: which hymn is loaded, which
//! slide is current, and whether output is blanked. No I/O, no UI — fully
//! unit-testable. The GUI mirrors this into the presenter view and projector.

#[derive(Debug, Clone, Default, PartialEq)]
pub struct PresentationState {
    pub number: Option<String>,
    pub title: String,
    pub slides: Vec<String>,
    pub index: usize,
    pub blank: bool,
}

impl PresentationState {
    /// Load a hymn for presentation: reset to the first slide, unblank.
    pub fn load_hymn(&mut self, number: Option<String>, title: String, slides: Vec<String>) {
        self.number = number;
        self.title = title;
        self.slides = slides;
        self.index = 0;
        self.blank = false;
    }

    pub fn slide_count(&self) -> usize {
        self.slides.len()
    }

    /// The current slide's text, or None if no hymn is loaded.
    pub fn current_slide(&self) -> Option<&str> {
        self.slides.get(self.index).map(|s| s.as_str())
    }

    /// A peek at the next slide, or None if on the last (or no) slide.
    pub fn next_slide(&self) -> Option<&str> {
        self.slides.get(self.index + 1).map(|s| s.as_str())
    }

    /// Advance one slide, clamped at the last slide (no playlist roll-over).
    pub fn next(&mut self) {
        if self.index + 1 < self.slides.len() {
            self.index += 1;
        }
    }

    /// Go back one slide, clamped at the first.
    pub fn prev(&mut self) {
        self.index = self.index.saturating_sub(1);
    }

    pub fn toggle_blank(&mut self) {
        self.blank = !self.blank;
    }
}
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test -p hymnal-core --test present_test`
Expected: PASS (all five).

- [ ] **Step 6: Commit**

```bash
git add crates/hymnal-core/src/present.rs crates/hymnal-core/src/lib.rs crates/hymnal-core/tests/present_test.rs
git commit -m "feat(core): PresentationState slide/blank logic"
```

---

## Phase 3 — Projector window + display targeting (GUI)

### Task 4: ProjectorWindow component + slide rendering

**Files:**
- Create: `crates/hymnal-gui/ui/projector.slint`
- Modify: `crates/hymnal-gui/ui/app.slint` (import projector, or keep separate include)
- Modify: `crates/hymnal-gui/src/main.rs` (flatten Theme → projector props; open/close)

**Note:** GUI tasks are verified by building + manual run (the project's established practice). Each ends with a build check and a logged smoke run.

- [ ] **Step 1: Define the projector window in Slint**

Create `crates/hymnal-gui/ui/projector.slint`:
```slint
// A standalone full-screen window that renders ONE slide styled by flattened
// theme properties. The Rust side flattens a hymnal_core::theme::Theme into
// these properties whenever the theme or current slide changes.
export component ProjectorWindow inherits Window {
    title: "Projector";
    background: root.bg-color;

    // Flattened theme + current slide (set from Rust).
    in property <string> slide-text;
    in property <bool> blank;
    in property <color> bg-color;
    in property <color> text-color;
    in property <string> font-family;
    in property <length> font-size;
    in property <int> font-weight;
    in property <length> margin: 48px;
    in property <string> h-align: "center"; // "left"|"center"|"right"

    Rectangle {
        background: root.bg-color;
        if !root.blank: Text {
            text: root.slide-text;
            color: root.text-color;
            font-family: root.font-family;
            font-size: root.font-size;
            font-weight: root.font-weight;
            wrap: word-wrap;
            horizontal-alignment: root.h-align == "left" ? TextHorizontalAlignment.left
                : root.h-align == "right" ? TextHorizontalAlignment.right
                : TextHorizontalAlignment.center;
            vertical-alignment: center;
            width: parent.width - root.margin * 2;
            x: root.margin;
        }
    }
}
```
> v1 scope: solid background + core text props (color/family/size/weight/align)
> + blank. Gradient/image background, overlay, shadow/outline, footer, and
> auto-fit are layered on in Task 5b/follow-up — see the deferral note at the
> end of this task. This keeps the first projector build small and verifiable.

- [ ] **Step 2: Make `slint::include_modules!()` see both files**

Slint's `build.rs` currently compiles `ui/app.slint`. Confirm how the build
script is set up:
Run: `cat crates/hymnal-gui/build.rs`
If it calls `slint_build::compile("ui/app.slint")`, add a second compile for the
projector OR (simpler) `import { ProjectorWindow } from "projector.slint";` at
the top of `app.slint` and re-export by referencing it — but the cleanest is to
compile both. Update `build.rs` to:
```rust
fn main() {
    slint_build::compile("ui/app.slint").unwrap();
    slint_build::compile("ui/projector.slint").unwrap();
}
```
If `slint_build::compile` called twice causes duplicate-symbol issues, instead
add `import { ProjectorWindow } from "projector.slint";` to the top of
`app.slint` (Slint dedups imported components) and revert build.rs. Verify by
building in Step 4.

- [ ] **Step 3: Add a Theme→projector flattening helper in main.rs**

In `crates/hymnal-gui/src/main.rs`, add near the other free functions:
```rust
use hymnal_core::theme::{Background, HAlign, Theme};

/// Convert a core Rgba to a Slint Color.
fn to_color(c: hymnal_core::theme::Rgba) -> slint::Color {
    slint::Color::from_argb_u8(c.a, c.r, c.g, c.b)
}

/// Push a theme + slide text onto a ProjectorWindow's flattened properties.
fn apply_theme_to_projector(p: &ProjectorWindow, theme: &Theme, slide: &str, blank: bool) {
    p.set_slide_text(slide.into());
    p.set_blank(blank);
    p.set_text_color(to_color(theme.text.color));
    p.set_font_family(theme.text.font_family.clone().into());
    p.set_font_size(slint::LogicalLength::new(theme.text.font_size_pt.unwrap_or(44.0)));
    p.set_font_weight(theme.text.font_weight as i32);
    p.set_margin(slint::LogicalLength::new(theme.layout.margin));
    p.set_h_align(match theme.text.h_align {
        HAlign::Left => "left",
        HAlign::Center => "center",
        HAlign::Right => "right",
    }.into());
    let bg = match &theme.background.kind {
        Background::Solid { color } => to_color(*color),
        // v1: gradient/image fall back to a solid colour until Task 5b adds them.
        Background::Gradient { from, .. } => to_color(*from),
        Background::Image { .. } => to_color(theme.text.color), // placeholder; overridden in 5b
    };
    p.set_bg_color(bg);
}
```
> The `Background::Image` placeholder is intentional for v1 (no image support
> yet); it is replaced in the deferred 5b work. It never panics.

- [ ] **Step 4: Build**

Run: `cargo build -p hymnal-gui 2>&1 | tail -20`
Expected: `Finished`. Resolve any Slint import/codegen errors per Step 2's
alternatives. `ProjectorWindow` is now a generated type usable from Rust.

- [ ] **Step 5: Commit**

```bash
git add crates/hymnal-gui/ui/projector.slint crates/hymnal-gui/build.rs crates/hymnal-gui/src/main.rs
git commit -m "feat(gui): ProjectorWindow component + theme flattening (solid bg, core text)"
```

**Deferred within this feature (tracked, not dropped):** Task "5b" — gradient/
image backgrounds, overlay opacity, text shadow/outline, footer, and auto-fit
font sizing in `projector.slint` + `apply_theme_to_projector`. Surfaced to the
user at plan handoff; implement after the Control tab works end-to-end so the
core path is verified first.

---

### Task 5: Display enumeration + open/close on chosen display

**Files:**
- Modify: `crates/hymnal-gui/Cargo.toml` (add `display-info`)
- Create: `crates/hymnal-gui/src/projector.rs` (open/close + display list)
- Modify: `crates/hymnal-gui/src/main.rs` (module decl)

- [ ] **Step 1: Add `display-info`**

In `crates/hymnal-gui/Cargo.toml` under `[dependencies]`:
```toml
display-info = "0.5"
```
(If 0.5 doesn't resolve, run `cargo add display-info --dry-run` and pin the
latest 0.x. The crate exposes `DisplayInfo::all() -> Result<Vec<DisplayInfo>>`
with fields `id, name, x, y, width, height, is_primary`.)

- [ ] **Step 2: Implement display listing + projector open/close**

Create `crates/hymnal-gui/src/projector.rs`:
```rust
//! Projector window lifecycle and display targeting.

use crate::ProjectorWindow;
use log::{info, warn};
use slint::ComponentHandle;

/// A connected display the user can project onto.
#[derive(Debug, Clone)]
pub struct Display {
    pub index: i32,
    pub label: String,
    pub x: i32,
    pub y: i32,
    pub is_primary: bool,
}

/// Enumerate connected displays. Returns at least one (index 0) even on error,
/// so the picker is never empty.
pub fn list_displays() -> Vec<Display> {
    match display_info::DisplayInfo::all() {
        Ok(list) if !list.is_empty() => list
            .into_iter()
            .enumerate()
            .map(|(i, d)| Display {
                index: i as i32,
                label: format!("{} ({}x{})", d.name, d.width, d.height),
                x: d.x,
                y: d.y,
                is_primary: d.is_primary,
            })
            .collect(),
        other => {
            if let Err(e) = other {
                warn!("display enumeration failed: {e}; assuming single display");
            }
            vec![Display {
                index: 0,
                label: "Primary".into(),
                x: 0,
                y: 0,
                is_primary: true,
            }]
        }
    }
}

/// Pick a sensible default output: first non-primary display if present,
/// else primary.
pub fn default_display_index(displays: &[Display]) -> i32 {
    displays
        .iter()
        .find(|d| !d.is_primary)
        .or_else(|| displays.first())
        .map(|d| d.index)
        .unwrap_or(0)
}

/// Create + show a ProjectorWindow positioned on the chosen display, fullscreen.
pub fn open_projector(displays: &[Display], target: i32) -> Option<ProjectorWindow> {
    let win = ProjectorWindow::new().ok()?;
    if let Some(d) = displays.iter().find(|d| d.index == target) {
        info!("opening projector on display {} at ({},{})", d.label, d.x, d.y);
        win.window()
            .set_position(slint::PhysicalPosition::new(d.x, d.y));
    }
    win.show().ok()?;
    win.window().set_fullscreen(true);
    Some(win)
}
```

- [ ] **Step 3: Declare the module in main.rs**

In `crates/hymnal-gui/src/main.rs`, add near the top (after `slint::include_modules!();`):
```rust
mod projector;
```

- [ ] **Step 4: Build**

Run: `cargo build -p hymnal-gui 2>&1 | tail -20`
Expected: `Finished`. Fix `display-info` API drift per Step 1's note if needed
(field names/`all()` signature).

- [ ] **Step 5: Commit**

```bash
git add crates/hymnal-gui/Cargo.toml crates/hymnal-gui/src/projector.rs crates/hymnal-gui/src/main.rs
git commit -m "feat(gui): display enumeration + projector open/close on chosen display"
```

---

## Phase 4 — Themes editor (GUI)

### Task 6: Themes nav item + editor panel with live preview

**Files:**
- Create: `assets/icon-themes.svg`
- Modify: `crates/hymnal-gui/ui/app.slint` (I18n strings, nav item, `ThemesPanel`, AppWindow props/callbacks + `active-tab == 4` block)

- [ ] **Step 1: Add a themes icon**

Create `assets/icon-themes.svg` (monochrome palette/brush; tinted by existing nav `colorize`):
```svg
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="13.5" cy="6.5" r="2.5"/><circle cx="17.5" cy="10.5" r="2.5"/><circle cx="8.5" cy="7.5" r="2.5"/><circle cx="6.5" cy="12.5" r="2.5"/><path d="M12 2a10 10 0 0 0 0 20 2.5 2.5 0 0 0 2-4 2.5 2.5 0 0 1 2-4h2a4 4 0 0 0 4-4 10 10 0 0 0-10-8z"/></svg>
```

- [ ] **Step 2: Add I18n strings for the Themes UI**

In `app.slint`'s `I18n` global, add:
```slint
    in property <string> nav-themes: "Themes";
    in property <string> themes-heading: "Themes";
    in property <string> theme-new: "New theme";
    in property <string> theme-save: "Save";
    in property <string> theme-delete: "Delete";
    in property <string> theme-preview-sample: "Plecaţi-vă lui Dumnezeu";
```
And in `main.rs`'s `apply_language`, add corresponding setters using
`Strings` fields (add these fields to `hymnal_core::i18n::Strings` with English/
Italian/Romanian values mirroring existing entries; follow the existing pattern
in `i18n.rs` exactly — every existing string has all three languages).

- [ ] **Step 3: Add the Themes nav item**

In the `Sidebar`'s `VerticalBox`, after the Settings `NavItem`:
```slint
        NavItem {
            label: I18n.nav-themes;
            icon: @image-url("../../../assets/icon-themes.svg");
            selected: root.active-tab == 4;
            clicked => { root.active-tab = 4; }
        }
```

- [ ] **Step 4: Add a `ThemesPanel` component**

In `app.slint`, before `export component AppWindow`, add a `ThemesPanel` that:
- lists theme names (`in property <[StandardListViewItem]> theme-names;`,
  `in-out property <int> theme-index;`),
- shows editor controls bound to flattened in-out properties
  (`in-out property <string> edit-font-family;`,
  `in-out property <float> edit-font-size;`,
  `in-out property <int> edit-font-weight;`,
  `in-out property <color> edit-text-color;`,
  `in-out property <color> edit-bg-color;`,
  `in-out property <string> edit-h-align;`),
- renders a **live preview** Rectangle using those same edit-* props (the same
  visual contract as `projector.slint`'s text block, at small size),
- exposes callbacks `theme-selected(int)`, `new-theme()`, `save-theme()`,
  `delete-theme()`, `edit-changed()`.

Concrete component (uses only Theme global colours + std-widgets):
```slint
component ThemesPanel inherits Rectangle {
    in property <[StandardListViewItem]> theme-names;
    in-out property <int> theme-index;
    in-out property <string> edit-font-family;
    in-out property <float> edit-font-size;
    in-out property <int> edit-font-weight;
    in-out property <color> edit-text-color;
    in-out property <color> edit-bg-color;
    in-out property <string> edit-h-align;
    callback theme-selected(int);
    callback new-theme();
    callback save-theme();
    callback delete-theme();
    callback edit-changed();
    background: Theme.bg;
    HorizontalBox {
        padding: 16px;
        spacing: 16px;
        // Left: theme list + actions.
        VerticalBox {
            width: 220px;
            Text { text: I18n.themes-heading; color: Theme.text; font-size: 18px; font-weight: 700; }
            StandardListView {
                model: root.theme-names;
                current-item <=> root.theme-index;
                current-item-changed(i) => { root.theme-selected(i); }
            }
            HorizontalBox {
                Button { text: I18n.theme-new; clicked => { root.new-theme(); } }
                Button { text: I18n.theme-delete; clicked => { root.delete-theme(); } }
            }
        }
        // Right: editor + live preview.
        VerticalBox {
            // Live preview.
            Rectangle {
                height: 200px;
                background: root.edit-bg-color;
                border-radius: Theme.radius;
                Text {
                    text: I18n.theme-preview-sample;
                    color: root.edit-text-color;
                    font-family: root.edit-font-family;
                    font-size: root.edit-font-size * 1px;
                    font-weight: root.edit-font-weight;
                    horizontal-alignment: root.edit-h-align == "left" ? TextHorizontalAlignment.left
                        : root.edit-h-align == "right" ? TextHorizontalAlignment.right
                        : TextHorizontalAlignment.center;
                    vertical-alignment: center;
                    width: parent.width - 32px;
                    x: 16px;
                    wrap: word-wrap;
                }
            }
            // Editor fields (font size + weight via LineEdit/SpinBox-like; colors
            // via simple hex LineEdits in v1; font-family via LineEdit).
            HorizontalBox {
                Text { text: "Font"; color: Theme.text-dim; vertical-alignment: center; }
                font-edit := LineEdit {
                    text <=> root.edit-font-family;
                    edited => { root.edit-changed(); }
                }
            }
            HorizontalBox {
                Text { text: "Size"; color: Theme.text-dim; vertical-alignment: center; }
                size-edit := LineEdit {
                    text: root.edit-font-size;
                    edited(t) => { root.edit-font-size = t.to-float(); root.edit-changed(); }
                }
            }
            Button { text: I18n.theme-save; clicked => { root.save-theme(); } }
        }
    }
}
```
> Color editing in v1 is hex-string `LineEdit`s parsed in Rust (full color
> pickers are a follow-up). Keep the preview as the source of truth for "what it
> looks like."

- [ ] **Step 5: Add AppWindow properties/callbacks + panel block**

In `AppWindow`, declare the same `theme-*`/`edit-*` properties and `theme-*`
callbacks as forwarded members, and add:
```slint
        if root.active-tab == 4: ThemesPanel {
            horizontal-stretch: 1;
            theme-names: root.theme-names;
            theme-index <=> root.theme-index;
            edit-font-family <=> root.edit-font-family;
            edit-font-size <=> root.edit-font-size;
            edit-font-weight <=> root.edit-font-weight;
            edit-text-color <=> root.edit-text-color;
            edit-bg-color <=> root.edit-bg-color;
            edit-h-align <=> root.edit-h-align;
            theme-selected(i) => { root.theme-selected(i); }
            new-theme => { root.new-theme(); }
            save-theme => { root.save-theme(); }
            delete-theme => { root.delete-theme(); }
            edit-changed => { root.edit-changed(); }
        }
```

- [ ] **Step 6: Build (Rust wiring lands in Task 7 — expect setter errors)**

Run: `cargo build -p hymnal-gui 2>&1 | tail -20`
Expected: slint compiles; Rust errors only about missing `set_*`/`on_*` for the
new members (wired in Task 7). If slint itself errors, fix the markup.

- [ ] **Step 7: Commit**

```bash
git add assets/icon-themes.svg crates/hymnal-gui/ui/app.slint crates/hymnal-core/src/i18n.rs crates/hymnal-gui/src/main.rs
git commit -m "feat(gui): Themes editor panel with live preview (UI)"
```

---

### Task 7: Wire the Themes editor to storage

**Files:**
- Modify: `crates/hymnal-gui/src/main.rs`

- [ ] **Step 1: Load themes on boot and populate the list + editor**

In `main.rs`, after the UI is created, add a UI-thread helper and state:
```rust
use hymnal_core::theme::store;
use std::cell::RefCell;

// Current set of themes + the one being edited.
let themes: Rc<RefCell<Vec<Theme>>> = Rc::new(RefCell::new(Vec::new()));

fn refresh_theme_list(ui: &AppWindow, themes: &Rc<RefCell<Vec<Theme>>>) {
    let dir = hymnal_core::library::themes_dir();
    let list = dir.as_deref().map(store::list_themes).unwrap_or_else(|| vec![Theme::default()]);
    let rows: Vec<slint::StandardListViewItem> = list
        .iter()
        .map(|t| slint::StandardListViewItem::from(slint::SharedString::from(t.name.clone())))
        .collect();
    ui.set_theme_names(slint::ModelRc::from(Rc::new(slint::VecModel::from(rows))));
    *themes.borrow_mut() = list;
}

fn load_theme_into_editor(ui: &AppWindow, t: &Theme) {
    ui.set_edit_font_family(t.text.font_family.clone().into());
    ui.set_edit_font_size(t.text.font_size_pt.unwrap_or(44.0));
    ui.set_edit_font_weight(t.text.font_weight as i32);
    ui.set_edit_text_color(to_color(t.text.color));
    let bg = match &t.background.kind {
        Background::Solid { color } => to_color(*color),
        Background::Gradient { from, .. } => to_color(*from),
        Background::Image { .. } => to_color(t.text.color),
    };
    ui.set_edit_bg_color(bg);
    ui.set_edit_h_align(match t.text.h_align {
        HAlign::Left => "left", HAlign::Center => "center", HAlign::Right => "right",
    }.into());
}
```
Call `refresh_theme_list(&ui, &themes);` then
`load_theme_into_editor(&ui, &Theme::default());` during setup.

- [ ] **Step 2: Wire the callbacks**

Add handlers (cloning `ui.as_weak()` + `themes` as needed):
```rust
// Select a theme -> load it into the editor.
{
    let themes = themes.clone();
    let weak = ui.as_weak();
    ui.on_theme_selected(move |i| {
        let Some(ui) = weak.upgrade() else { return };
        if let Some(t) = themes.borrow().get(i as usize) {
            load_theme_into_editor(&ui, t);
        }
    });
}
// Save the editor's values as a theme (reads back the edit-* props).
{
    let themes = themes.clone();
    let weak = ui.as_weak();
    ui.on_save_theme(move || {
        let Some(ui) = weak.upgrade() else { return };
        let idx = ui.get_theme_index().max(0) as usize;
        let mut t = themes.borrow().get(idx).cloned().unwrap_or_default();
        if t.is_builtin_default() {
            // Editing the default makes a copy named "Default copy".
            t.name = "Custom".into();
        }
        // Read editor fields back into the theme.
        t.text.font_family = ui.get_edit_font_family().to_string();
        t.text.font_size_pt = Some(ui.get_edit_font_size());
        t.text.font_weight = ui.get_edit_font_weight() as u16;
        // colors: the slint Color round-trips via to-argb; convert back.
        let c = ui.get_edit_text_color();
        t.text.color = hymnal_core::theme::Rgba::new(c.red(), c.green(), c.blue(), c.alpha());
        let b = ui.get_edit_bg_color();
        t.background.kind = Background::Solid {
            color: hymnal_core::theme::Rgba::new(b.red(), b.green(), b.blue(), b.alpha()),
        };
        if let Some(dir) = hymnal_core::library::themes_dir() {
            match store::save_theme(&dir, &t) {
                Ok(()) => info!("saved theme {}", t.name),
                Err(e) => warn!("save theme failed: {e}"),
            }
        }
        refresh_theme_list(&ui, &themes);
    });
}
// New theme: a fresh "Custom N" loaded into the editor (saved on Save).
{
    let weak = ui.as_weak();
    ui.on_new_theme(move || {
        let Some(ui) = weak.upgrade() else { return };
        let mut t = Theme::default();
        t.name = "Custom".into();
        load_theme_into_editor(&ui, &t);
    });
}
// Delete the selected theme.
{
    let themes = themes.clone();
    let weak = ui.as_weak();
    ui.on_delete_theme(move || {
        let Some(ui) = weak.upgrade() else { return };
        let idx = ui.get_theme_index().max(0) as usize;
        if let Some(t) = themes.borrow().get(idx).cloned() {
            if let Some(dir) = hymnal_core::library::themes_dir() {
                if let Err(e) = store::delete_theme(&dir, &t.name) {
                    warn!("delete theme failed: {e}");
                }
            }
        }
        refresh_theme_list(&ui, &themes);
    });
}
// edit-changed: live preview is already bound to edit-* props; nothing needed
// here beyond an optional log. (Kept as a no-op handler so the callback exists.)
ui.on_edit_changed(|| {});
```

- [ ] **Step 3: Build + smoke run**

Run: `cargo build -p hymnal-gui 2>&1 | tail -10` (expect `Finished`).
Run: `RUST_LOG=hymnal_gui=info timeout 20 cargo run -q -p hymnal-gui 2>&1 | grep -iE "theme|panic|error" | head`
Expected: no panic; (optionally) theme save logs when exercised.

- [ ] **Step 4: Commit**

```bash
git add crates/hymnal-gui/src/main.rs
git commit -m "feat(gui): wire Themes editor to named-file storage + live preview"
```

---

## Phase 5 — Control tab + wiring

### Task 8: Control nav item + presenter panel

**Files:**
- Create: `assets/icon-control.svg`
- Modify: `crates/hymnal-gui/ui/app.slint` (I18n, nav item, `ControlPanel`, AppWindow props/callbacks + `active-tab == 3` block)

- [ ] **Step 1: Add a control icon**

Create `assets/icon-control.svg` (monochrome projector/play glyph):
```svg
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="2" y="6" width="20" height="12" rx="2"/><polygon points="10 9 16 12 10 15 10 9"/></svg>
```

- [ ] **Step 2: I18n strings**

Add to `I18n` (and to `Strings` in `i18n.rs` with all three languages, plus
setters in `apply_language`):
```slint
    in property <string> nav-control: "Control";
    in property <string> control-heading: "Control";
    in property <string> control-start: "▶ Start projecting";
    in property <string> control-stop: "■ Stop";
    in property <string> control-blank: "Blank (B)";
    in property <string> control-prev: "◀ Prev";
    in property <string> control-next: "Next ▶";
    in property <string> control-live: "LIVE";
    in property <string> control-next-label: "NEXT";
    in property <string> control-output: "Output";
    in property <string> control-theme: "Theme";
```

- [ ] **Step 3: Add the Control nav item (between Downloader and Settings, index 3)**

Note: Settings is currently index 2. To keep Settings/Themes as the last items,
use these indices: 0 Library, 1 Downloader, 2 Settings, 3 Control, 4 Themes —
OR renumber. Simplest with least churn: **Control = 3, Themes = 4**, leave
Settings at 2. Add after the Settings `NavItem`:
```slint
        NavItem {
            label: I18n.nav-control;
            icon: @image-url("../../../assets/icon-control.svg");
            selected: root.active-tab == 3;
            clicked => { root.active-tab = 3; }
        }
```
(Themes nav item from Task 6 already uses index 4.)

- [ ] **Step 4: Add the `ControlPanel` component**

Add before `AppWindow`. It binds presenter state and emits control callbacks.
Properties: `current-title`, `live-text`, `next-text`, `slide-pos` (string like
"2/5"), `projecting` (bool), `blank` (bool), `theme-names`, `theme-index`,
`display-names`, `display-index`, plus its own search:
`search-results`, `search-current`. Callbacks: `start()`, `stop()`,
`blank-toggle()`, `prev()`, `next()`, `theme-picked(int)`, `display-picked(int)`,
`search-changed(string)`, `search-activated(int)` (load highlighted hymn).
```slint
component ControlPanel inherits Rectangle {
    in property <string> current-title;
    in property <string> live-text;
    in property <string> next-text;
    in property <string> slide-pos;
    in property <bool> projecting;
    in property <bool> blank;
    in property <[StandardListViewItem]> theme-names;
    in-out property <int> theme-index;
    in property <[StandardListViewItem]> display-names;
    in-out property <int> display-index;
    in property <[StandardListViewItem]> search-results;
    in-out property <int> search-current;
    callback start();
    callback stop();
    callback blank-toggle();
    callback prev();
    callback next();
    callback theme-picked(int);
    callback display-picked(int);
    callback search-changed(string);
    callback search-activated(int);

    forward-focus: ctl-keys;
    background: Theme.bg;

    ctl-keys := FocusScope {
        capture-key-pressed(event) => {
            if (event.text == Key.RightArrow || event.text == " ") { root.next(); return accept; }
            if (event.text == Key.LeftArrow) { root.prev(); return accept; }
            if (event.text == "b" || event.text == "B") { root.blank-toggle(); return accept; }
            if (event.text == Key.Escape) { root.stop(); return accept; }
            return reject;
        }
        VerticalBox {
            padding: 14px; spacing: 10px;
            // Top bar: theme + output + start/stop + blank.
            HorizontalBox {
                Text { text: I18n.control-theme + ":"; color: Theme.text-dim; vertical-alignment: center; }
                ComboBox { model: root.theme-names; current-index <=> root.theme-index;
                    selected => { root.theme-picked(self.current-index); } }
                Text { text: I18n.control-output + ":"; color: Theme.text-dim; vertical-alignment: center; }
                ComboBox { model: root.display-names; current-index <=> root.display-index;
                    selected => { root.display-picked(self.current-index); } }
                Button { text: root.projecting ? I18n.control-stop : I18n.control-start;
                    clicked => { if (root.projecting) { root.stop(); } else { root.start(); } } }
                Button { text: I18n.control-blank; clicked => { root.blank-toggle(); } }
            }
            // Search to load a hymn.
            LineEdit {
                placeholder-text: I18n.search-placeholder;
                edited(t) => { root.search-changed(t); }
                accepted(t) => { root.search-activated(root.search-current); }
            }
            HorizontalBox {
                // Left: results + slide list.
                StandardListView {
                    width: 38%;
                    model: root.search-results;
                    current-item <=> root.search-current;
                    current-item-changed(i) => { root.search-activated(i); }
                }
                // Right: live + next mirror.
                VerticalBox {
                    Text { text: root.current-title; color: Theme.accent-soft; font-weight: 700; }
                    Text { text: I18n.control-live + "  " + root.slide-pos; color: Theme.text-dim; font-size: 11px; }
                    Rectangle { height: 120px; background: #0b1f3a; border-radius: Theme.radius;
                        Text { text: root.blank ? "" : root.live-text; color: white; font-weight: 700;
                            horizontal-alignment: center; vertical-alignment: center;
                            width: parent.width - 24px; x: 12px; wrap: word-wrap; } }
                    Text { text: I18n.control-next-label; color: Theme.text-dim; font-size: 11px; }
                    Rectangle { height: 70px; background: #0b1f3a; border-radius: Theme.radius;
                        Text { text: root.next-text; color: #cbd5e1;
                            horizontal-alignment: center; vertical-alignment: center;
                            width: parent.width - 24px; x: 12px; wrap: word-wrap; } }
                    HorizontalBox { alignment: center;
                        Button { text: I18n.control-prev; clicked => { root.prev(); } }
                        Button { text: I18n.control-next; clicked => { root.next(); } } }
                }
            }
        }
    }
}
```

- [ ] **Step 5: AppWindow props/callbacks + panel block + Project from Library**

Declare the `ControlPanel` members on `AppWindow` as forwarded properties/
callbacks. Add the panel block:
```slint
        if root.active-tab == 3: ControlPanel {
            horizontal-stretch: 1;
            current-title: root.ctl-title;
            live-text: root.ctl-live;
            next-text: root.ctl-next;
            slide-pos: root.ctl-pos;
            projecting: root.ctl-projecting;
            blank: root.ctl-blank;
            theme-names: root.theme-names;
            theme-index <=> root.ctl-theme-index;
            display-names: root.display-names;
            display-index <=> root.display-index;
            search-results: root.ctl-search-results;
            search-current <=> root.ctl-search-current;
            start => { root.ctl-start(); }
            stop => { root.ctl-stop(); }
            blank-toggle => { root.ctl-blank-toggle(); }
            prev => { root.ctl-prev(); }
            next => { root.ctl-next(); }
            theme-picked(i) => { root.ctl-theme-picked(i); }
            display-picked(i) => { root.ctl-display-picked(i); }
            search-changed(t) => { root.ctl-search-changed(t); }
            search-activated(i) => { root.ctl-search-activated(i); }
        }
```
Also add a **Project** button to `LibraryPanel` (and a callback `project-current()`)
that the wiring in Task 9 hooks to load the highlighted hymn into Control and
switch `active-tab = 3`. Add to `LibraryPanel`:
```slint
    callback project-current();
```
and a button near Open/Reveal:
```slint
                    Button { text: "Project"; clicked => { root.project-current(); } }
```
forwarded through `AppWindow` like the other Library callbacks.

- [ ] **Step 6: Build (Rust wiring in Task 9 — expect setter errors)**

Run: `cargo build -p hymnal-gui 2>&1 | tail -20`
Expected: slint compiles; remaining errors are missing Rust `set_*`/`on_*` for
the new `ctl-*` members. If slint errors, fix markup.

- [ ] **Step 7: Commit**

```bash
git add assets/icon-control.svg crates/hymnal-gui/ui/app.slint crates/hymnal-core/src/i18n.rs
git commit -m "feat(gui): Control presenter panel + Library Project button (UI)"
```

---

### Task 9: Wire the Control tab (state, projector, search, persistence)

**Files:**
- Modify: `crates/hymnal-gui/src/main.rs`
- Modify: `crates/hymnal-core/src/library.rs` (Config: `active_theme`, `output_display`)

- [ ] **Step 1: Add Config fields**

In `library.rs` `Config`, add:
```rust
    /// Name of the active projection theme. `None` => built-in Default.
    #[serde(default)]
    pub active_theme: Option<String>,
    /// Index of the last-used output display. `None` => auto-pick.
    #[serde(default)]
    pub output_display: Option<i32>,
```
Add a round-trip assertion to an existing `library.rs` config test (set both,
to_toml/from_toml, assert equal).

- [ ] **Step 2: Control state on the UI thread**

In `main.rs`, add:
```rust
use hymnal_core::present::PresentationState;

let present = Rc::new(RefCell::new(PresentationState::default()));
let projector: Rc<RefCell<Option<ProjectorWindow>>> = Rc::new(RefCell::new(None));
let displays = Rc::new(projector::list_displays());
let active_theme = Rc::new(RefCell::new(Theme::default()));

// Populate display picker.
{
    let rows: Vec<slint::StandardListViewItem> = displays.iter()
        .map(|d| slint::StandardListViewItem::from(slint::SharedString::from(d.label.clone())))
        .collect();
    ui.set_display_names(slint::ModelRc::from(Rc::new(slint::VecModel::from(rows))));
    ui.set_display_index(projector::default_display_index(&displays));
}
```

- [ ] **Step 3: A `refresh_control_view` helper**

```rust
fn refresh_control_view(ui: &AppWindow, p: &PresentationState) {
    let number = p.number.as_deref().map(|n| format!("{n}. ")).unwrap_or_default();
    ui.set_ctl_title(format!("{number}{}", p.title).into());
    ui.set_ctl_live(p.current_slide().unwrap_or("").into());
    ui.set_ctl_next(p.next_slide().unwrap_or("").into());
    ui.set_ctl_pos(if p.slide_count() > 0 {
        format!("{}/{}", p.index + 1, p.slide_count())
    } else { "".into() }.into());
    ui.set_ctl_blank(p.blank);
}

fn push_to_projector(
    projector: &Rc<RefCell<Option<ProjectorWindow>>>,
    theme: &Theme,
    p: &PresentationState,
) {
    if let Some(win) = projector.borrow().as_ref() {
        apply_theme_to_projector(win, theme, p.current_slide().unwrap_or(""), p.blank);
    }
}
```

- [ ] **Step 4: Wire start/stop/prev/next/blank**

```rust
// Start projecting.
{
    let projector = projector.clone(); let displays = displays.clone();
    let present = present.clone(); let active_theme = active_theme.clone();
    let weak = ui.as_weak();
    ui.on_ctl_start(move || {
        let Some(ui) = weak.upgrade() else { return };
        if projector.borrow().is_some() { return; }
        let target = ui.get_display_index();
        if let Some(win) = projector::open_projector(&displays, target) {
            *projector.borrow_mut() = Some(win);
            ui.set_ctl_projecting(true);
            push_to_projector(&projector, &active_theme.borrow(), &present.borrow());
        }
    });
}
// Stop.
{
    let projector = projector.clone(); let weak = ui.as_weak();
    ui.on_ctl_stop(move || {
        let Some(ui) = weak.upgrade() else { return };
        if let Some(win) = projector.borrow_mut().take() { let _ = win.hide(); }
        ui.set_ctl_projecting(false);
    });
}
// Next / Prev / Blank: mutate state, refresh view + projector.
macro_rules! ctl_nav { ($setter:ident, $op:expr) => {{
    let present = present.clone(); let projector = projector.clone();
    let active_theme = active_theme.clone(); let weak = ui.as_weak();
    ui.$setter(move || {
        let Some(ui) = weak.upgrade() else { return };
        { let mut p = present.borrow_mut(); $op(&mut p); }
        refresh_control_view(&ui, &present.borrow());
        push_to_projector(&projector, &active_theme.borrow(), &present.borrow());
    });
}}; }
ctl_nav!(on_ctl_next, |p: &mut PresentationState| p.next());
ctl_nav!(on_ctl_prev, |p: &mut PresentationState| p.prev());
ctl_nav!(on_ctl_blank_toggle, |p: &mut PresentationState| p.toggle_blank());
```

- [ ] **Step 5: Theme + display pick, persistence**

```rust
// Theme picked in Control: load it as active, persist, push to projector.
{
    let themes = themes.clone(); let active_theme = active_theme.clone();
    let projector = projector.clone(); let present = present.clone();
    let weak = ui.as_weak();
    ui.on_ctl_theme_picked(move |i| {
        let Some(ui) = weak.upgrade() else { return };
        if let Some(t) = themes.borrow().get(i as usize).cloned() {
            *active_theme.borrow_mut() = t.clone();
            push_to_projector(&projector, &active_theme.borrow(), &present.borrow());
            // persist active_theme
            if let Some(p) = hymnal_core::library::config_path() {
                let mut cfg = Config::load(&p).unwrap_or_default();
                cfg.active_theme = Some(t.name.clone());
                let _ = cfg.save(&p);
            }
        }
    });
}
// Display picked: persist output_display.
{
    let weak = ui.as_weak();
    ui.on_ctl_display_picked(move |i| {
        let _ = weak.upgrade();
        if let Some(p) = hymnal_core::library::config_path() {
            let mut cfg = Config::load(&p).unwrap_or_default();
            cfg.output_display = Some(i);
            let _ = cfg.save(&p);
        }
    });
}
```

- [ ] **Step 6: Control's own search + Library "Project"**

```rust
// Control search reuses the same Searcher; maps rows -> searcher index.
let ctl_rows: Rc<RefCell<Vec<usize>>> = Rc::new(RefCell::new(Vec::new()));
{
    let searcher = searcher.clone(); let ctl_rows = ctl_rows.clone(); let weak = ui.as_weak();
    ui.on_ctl_search_changed(move |q| {
        let Some(ui) = weak.upgrade() else { return };
        let guard = searcher.borrow();
        let Some(s) = guard.as_ref() else { return };
        let hits = s.search(&q);
        let mut rows = Vec::new(); let mut map = Vec::new();
        for h in &hits { rows.push(slint::StandardListViewItem::from(
            slint::SharedString::from(row_label(h.entry)))); map.push(h.index); }
        *ctl_rows.borrow_mut() = map;
        ui.set_ctl_search_results(slint::ModelRc::from(Rc::new(slint::VecModel::from(rows))));
    });
}
// Load highlighted hymn into the presentation.
{
    let searcher = searcher.clone(); let ctl_rows = ctl_rows.clone();
    let present = present.clone(); let projector = projector.clone();
    let active_theme = active_theme.clone(); let weak = ui.as_weak();
    ui.on_ctl_search_activated(move |i| {
        let Some(ui) = weak.upgrade() else { return };
        if i < 0 { return; }
        let guard = searcher.borrow();
        let Some(s) = guard.as_ref() else { return };
        let entry = ctl_rows.borrow().get(i as usize).and_then(|&ei| s.entry(ei)).cloned();
        if let Some(e) = entry {
            present.borrow_mut().load_hymn(e.number.clone(), e.title.clone(), e.slides.clone());
            refresh_control_view(&ui, &present.borrow());
            push_to_projector(&projector, &active_theme.borrow(), &present.borrow());
        }
    });
}
// Library "Project" button: load highlighted Library hymn, switch to Control.
{
    let searcher = searcher.clone(); let row_to_entry = row_to_entry.clone();
    let present = present.clone(); let projector = projector.clone();
    let active_theme = active_theme.clone(); let weak = ui.as_weak();
    ui.on_project_current(move || {
        let Some(ui) = weak.upgrade() else { return };
        let idx = ui.get_current_index();
        if idx < 0 { return; }
        let guard = searcher.borrow();
        let Some(s) = guard.as_ref() else { return };
        let entry = row_to_entry.borrow().get(idx as usize).and_then(|&ei| s.entry(ei)).cloned();
        if let Some(e) = entry {
            present.borrow_mut().load_hymn(e.number.clone(), e.title.clone(), e.slides.clone());
            refresh_control_view(&ui, &present.borrow());
            push_to_projector(&projector, &active_theme.borrow(), &present.borrow());
            ui.set_active_tab(3);
        }
    });
}
```
> `searcher` and `row_to_entry` are the existing Library state Rcs; clone them.
> If they were defined after this code, move these handler registrations below
> their definitions.

- [ ] **Step 7: Load active theme on boot**

After `refresh_theme_list`, set the active theme from config:
```rust
if let Some(p) = hymnal_core::library::config_path() {
    let cfg = Config::load(&p).unwrap_or_default();
    if let (Some(name), Some(dir)) = (cfg.active_theme, hymnal_core::library::themes_dir()) {
        if let Ok(t) = store::load_theme(&dir, &name) { *active_theme.borrow_mut() = t; }
    }
    if let Some(d) = cfg.output_display { ui.set_display_index(d); }
}
```

- [ ] **Step 8: Build + smoke run + commit**

Run: `cargo build -p hymnal-gui 2>&1 | tail -20` (expect `Finished`; fix borrow/
move errors by cloning weak handles + Rcs as noted).
Run: `cargo test -p hymnal-core` (expect all pass — Config round-trip incl new fields).
Run: `RUST_LOG=hymnal_gui=info timeout 20 cargo run -q -p hymnal-gui 2>&1 | grep -iE "panic|error|projector|theme" | head` (no panic).
```bash
git add crates/hymnal-gui/src/main.rs crates/hymnal-core/src/library.rs
git commit -m "feat(gui): wire Control tab — projection, nav, search, persistence"
```

---

## Task 10: Manual end-to-end verification

**Files:** none (verification only).

- [ ] **Step 1: Run the release build with a second display attached**

Run: `cargo run --release -p hymnal-gui`
Verify:
- Sidebar shows **Control** and **Themes** items.
- **Themes:** create a new theme, change font/size/colors — the live preview
  updates; Save persists it (re-open app, it's listed); the built-in Default
  can't be deleted.
- **Control:** search/load a hymn (or use Library → Project); pick Output =
  external display; **Start projecting** opens a fullscreen themed slide on the
  projector; **Next/Prev** (and →/←/Space) change the projected slide; **B**
  blanks; **Esc**/Stop closes the projector.
- Switching tabs doesn't break Library arrow-key nav.
- Unplug the second display before starting → falls back to Primary, no crash.

- [ ] **Step 2: Verification only** — fix in earlier tasks and re-run if a
  behavior is wrong.

---

## Self-Review Notes

- **Spec coverage:** Theme model + all properties (Task 1), named-JSON storage
  with default-always-present / corrupt-skip / non-deletable-default (Task 2),
  PresentationState slide/blank/load logic (Task 3), ProjectorWindow rendering
  (Task 4) + display targeting/open-fullscreen (Task 5), Themes editor with live
  preview + system-font family field (Tasks 6–7), Control presenter tab with
  current+next mirror, theme/display pickers, start/stop/blank, keyboard, own
  search, Library Project action (Tasks 8–9), persistence of active_theme/
  output_display (Task 9), error fallbacks (display gone, corrupt theme, no
  hymn) across Tasks 2/5/9, manual E2E (Task 10).
- **Explicitly deferred within the feature (flagged to user, not dropped):**
  - Task "5b" — gradient/image backgrounds, overlay opacity, shadow/outline,
    footer, and auto-fit font sizing in the projector + flattening. v1 ships
    solid background + core text styling; the Theme *model* already holds all
    fields, so this is render-side only.
  - Full color pickers (v1 uses hex/Color-bound fields); vertical alignment and
    line-spacing controls in the editor UI (model supports them; editor exposes
    a core subset first).
  These keep Phase 3–5 shippable; the spec's full property set is preserved in
  the data model and applied as the projector renderer grows.
- **Type/name consistency:** `Theme`, `Theme::default/to_json/from_json/
  is_builtin_default`, `theme::store::{list_themes,load_theme,save_theme,
  delete_theme}`, `themes_dir()`, `PresentationState::{load_hymn,next,prev,
  toggle_blank,current_slide,next_slide,slide_count}`, `projector::{list_displays,
  default_display_index,open_projector,Display}`, `apply_theme_to_projector`,
  `to_color`, Config `active_theme`/`output_display`, and the Slint `ctl-*`/
  `edit-*`/`theme-*` members are used consistently across tasks.
- **Placeholder scan:** no TBD/TODO. The two "if the crate API differs, adjust"
  notes (display-info, slint multi-file compile) are real compatibility hedges
  with the canonical call given, not placeholders.
- **Nav indices:** 0 Library, 1 Downloader, 2 Settings, 3 Control, 4 Themes —
  consistent across Tasks 6 and 8.
```
