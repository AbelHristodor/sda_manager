# Themes + Control UI/UX Overhaul Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rebuild the Themes and Control tabs with reusable styled controls, a thumbnail theme picker (Themes-only), real dropdowns/sliders/color-fields, and fixed-size non-overflowing preview boxes.

**Architecture:** A testable core hex↔Rgba helper; reusable Slint components (buttons, color field, theme thumbnail); Themes tab rebuilt with a thumbnail grid + 2-column form + separate edit-selection/active-selection; Control tab rebuilt with fixed-size Live/Next boxes and its theme picker removed (it reads the shared active theme).

**Tech Stack:** Rust, Slint 1.17 (ComboBox/Slider/SpinBox, custom components), system font enumeration (font-kit with curated fallback), existing hymnal-core/hymnal-gui.

---

## Current-state facts (read before starting)

- `crates/hymnal-core/src/theme.rs`: `Rgba { r,g,b,a: u8 }` with `Rgba::new(...)`. `Theme` with `text.{font_family:String, font_size_pt:Option<f32>, font_weight:u16, color:Rgba, h_align:HAlign}`, `background.kind: Background::Solid{color}|Gradient{from,..}|Image{..}`.
- `crates/hymnal-gui/src/main.rs`: `to_color(Rgba)->slint::Color` (line ~97) via `slint::Color::from_argb_u8(a,r,g,b)`; slint `Color` has `.red()/.green()/.blue()/.alpha()->u8`. Theme editor handlers at ~874 (`on_theme_selected`), ~884 (`on_save_theme`), ~911 (`on_new_theme`), ~923 (`on_delete_theme`). `active_theme: Rc<RefCell<Theme>>` (line 302). `themes: Rc<RefCell<Vec<Theme>>>`. Helpers `refresh_theme_list`, `load_theme_into_editor`. Control wiring: `set_ctl_theme_names` (~320), `on_ctl_theme_picked` (~1018).
- `crates/hymnal-gui/ui/app.slint`: `ThemesPanel` (line 656), `ControlPanel` (line 726). Imports line 1: `import { LineEdit, StandardListView, Button, ScrollView, VerticalBox, HorizontalBox, ComboBox } from "std-widgets.slint";`. `Theme` global tokens: bg, rail, panel, panel-border, field, field-border, accent, accent-soft, text, text-dim, nav-sel, danger, radius(9px), gap(12px).
- Slint provides `ComboBox`, `Slider`, `SpinBox` in std-widgets.

---

## Phase 0 — Core hex↔Rgba helper

### Task 1: `Rgba::from_hex` / `to_hex`

**Files:**
- Modify: `crates/hymnal-core/src/theme.rs`
- Test: `crates/hymnal-core/tests/theme_test.rs` (append)

- [ ] **Step 1: Write the failing test**

Append to `crates/hymnal-core/tests/theme_test.rs`:
```rust
use hymnal_core::theme::Rgba;

#[test]
fn rgba_hex_round_trips() {
    let c = Rgba::new(11, 31, 58, 255);
    assert_eq!(c.to_hex(), "#0B1F3A");
    assert_eq!(Rgba::from_hex("#0B1F3A"), Some(Rgba::new(11, 31, 58, 255)));
}

#[test]
fn rgba_from_hex_accepts_lowercase_and_no_hash() {
    assert_eq!(Rgba::from_hex("ffffff"), Some(Rgba::new(255, 255, 255, 255)));
    assert_eq!(Rgba::from_hex("#abcdef"), Some(Rgba::new(0xab, 0xcd, 0xef, 255)));
}

#[test]
fn rgba_from_hex_rejects_invalid() {
    assert_eq!(Rgba::from_hex("#FF"), None);       // too short
    assert_eq!(Rgba::from_hex("#GGGGGG"), None);   // non-hex
    assert_eq!(Rgba::from_hex(""), None);
    assert_eq!(Rgba::from_hex("#1234567"), None);  // too long
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p hymnal-core --test theme_test rgba`
Expected: FAIL — `to_hex`/`from_hex` not found.

- [ ] **Step 3: Implement**

In `crates/hymnal-core/src/theme.rs`, extend `impl Rgba` (after `new`):
```rust
    /// Format as `#RRGGBB` (uppercase, alpha dropped — the editor only edits RGB).
    pub fn to_hex(&self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }

    /// Parse `#RRGGBB` / `RRGGBB` (case-insensitive). Alpha is set to 255.
    /// Returns `None` for any non-6-hex-digit input.
    pub fn from_hex(s: &str) -> Option<Rgba> {
        let h = s.strip_prefix('#').unwrap_or(s);
        if h.len() != 6 || !h.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        let r = u8::from_str_radix(&h[0..2], 16).ok()?;
        let g = u8::from_str_radix(&h[2..4], 16).ok()?;
        let b = u8::from_str_radix(&h[4..6], 16).ok()?;
        Some(Rgba::new(r, g, b, 255))
    }
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p hymnal-core --test theme_test rgba`
Expected: PASS (3 tests).

- [ ] **Step 5: Full suite + commit**

Run: `cargo test -p hymnal-core` (all pass).
```bash
git add crates/hymnal-core/src/theme.rs crates/hymnal-core/tests/theme_test.rs
git commit -m "feat(core): Rgba hex parse/format helpers"
```

---

## Phase 1 — Shared styled Slint components

### Task 2: Reusable controls (PrimaryButton, SecondaryButton, ColorField, ThemeThumb)

**Files:**
- Modify: `crates/hymnal-gui/ui/app.slint` (add components near the top, after the `Theme` global / before `NavItem`)

**Note:** GUI task — verified by building. These components are added but only *used* in Tasks 3–5; Slint won't warn about unused components, and the build should stay green.

- [ ] **Step 1: Add the components**

In `crates/hymnal-gui/ui/app.slint`, after the `global Theme { ... }` block and before `component NavItem`, add:
```slint
component PrimaryButton inherits Rectangle {
    in property <string> text;
    in property <bool> enabled: true;
    callback clicked();
    height: 32px;
    min-width: 80px;
    border-radius: 7px;
    background: ta.pressed ? Theme.accent.darker(20%) : Theme.accent;
    opacity: root.enabled ? 1.0 : 0.5;
    HorizontalBox {
        padding-left: 14px; padding-right: 14px; alignment: center;
        Text { text: root.text; color: white; font-size: 13px; font-weight: 600; vertical-alignment: center; }
    }
    ta := TouchArea { enabled: root.enabled; clicked => { root.clicked(); } }
}

component SecondaryButton inherits Rectangle {
    in property <string> text;
    in property <bool> enabled: true;
    in property <bool> danger: false;
    callback clicked();
    height: 32px;
    min-width: 72px;
    border-radius: 7px;
    border-width: 1px;
    border-color: Theme.field-border;
    background: ta.pressed ? Theme.field.darker(15%) : Theme.field;
    opacity: root.enabled ? 1.0 : 0.5;
    HorizontalBox {
        padding-left: 12px; padding-right: 12px; alignment: center;
        Text { text: root.text; color: root.danger ? Theme.danger : Theme.text; font-size: 13px; vertical-alignment: center; }
    }
    ta := TouchArea { enabled: root.enabled; clicked => { root.clicked(); } }
}

// Swatch + #RRGGBB hex field + preset row. `value` is the live color; `hex` is
// the text the user edits. Rust parses/sets both via from_hex/to_hex.
component ColorField inherits VerticalBox {
    in-out property <color> value;
    in-out property <string> hex;        // "#RRGGBB" string, kept in sync by Rust
    callback hex-edited(string);          // user typed; Rust validates -> sets value+hex
    callback preset-picked(color);        // a preset swatch was clicked
    spacing: 6px;
    HorizontalBox {
        spacing: 8px;
        Rectangle { width: 28px; height: 22px; border-radius: 5px; background: root.value;
            border-width: 1px; border-color: Theme.field-border; }
        hexedit := LineEdit { text: root.hex; edited(t) => { root.hex-edited(t); } }
    }
    HorizontalBox {
        spacing: 5px; alignment: start;
        for c in [#ffffff, #000000, #0b1f3a, #f8fafc, #fde68a, #3b0d0d, #1e3a2f]: Rectangle {
            width: 20px; height: 20px; border-radius: 4px; background: c;
            border-width: 1px; border-color: Theme.field-border;
            TouchArea { clicked => { root.preset-picked(c); } }
        }
    }
}

// A theme rendered as a mini-slide (background + sample text) with caption.
// `selected` = currently in the editor; `active` = used for projection.
component ThemeThumb inherits Rectangle {
    in property <string> name;
    in property <color> bg;
    in property <color> fg;
    in property <string> font-family;
    in property <bool> selected;
    in property <bool> active;
    callback clicked();
    height: 64px;
    border-radius: 7px;
    border-width: root.selected ? 2px : 1px;
    border-color: root.selected ? Theme.accent-soft : Theme.panel-border;
    clip: true;
    Rectangle {
        background: root.bg;
        Text { text: "Aa " + root.name; color: root.fg; font-family: root.font-family;
            font-size: 12px; font-weight: 700; horizontal-alignment: center; vertical-alignment: center; }
        if root.active: Rectangle {
            x: 4px; y: 4px; width: 52px; height: 16px; border-radius: 8px; background: #0008;
            Text { text: "● Active"; color: Theme.accent-soft; font-size: 9px;
                horizontal-alignment: center; vertical-alignment: center; }
        }
    }
    TouchArea { clicked => { root.clicked(); } }
}
```
> `color.darker(pct%)` is a Slint built-in; if the exact syntax differs in 1.17,
> use a plain darker literal for the pressed state. The `for c in [#...]:` inline
> color array is valid Slint. Verify in the Task 4 build.

- [ ] **Step 2: Build**

Run: `cargo build -p hymnal-gui 2>&1 | tail -15`
Expected: `Finished`. Fix any Slint syntax issues (e.g. `darker`, inline array). The components are unused so far — that's fine.

- [ ] **Step 3: Commit**

```bash
git add crates/hymnal-gui/ui/app.slint
git commit -m "feat(gui): reusable styled controls (buttons, color field, theme thumb)"
```

---

## Phase 2 — Themes tab

### Task 3: System font enumeration (core or gui helper)

**Files:**
- Modify: `crates/hymnal-gui/Cargo.toml`
- Create: `crates/hymnal-gui/src/fonts.rs`
- Modify: `crates/hymnal-gui/src/main.rs` (`mod fonts;`)

- [ ] **Step 1: Add font-kit (with fallback plan)**

In `crates/hymnal-gui/Cargo.toml` under `[dependencies]`, add:
```toml
font-kit = "0.14"
```
Verify it resolves/builds on macOS: `cargo build -p hymnal-gui 2>&1 | tail`. If `font-kit` fails to build (system deps) OR doesn't resolve, SKIP it — set a flag in your report and implement `fonts.rs` with ONLY the curated fallback list (still satisfies the spec's "fallback" path). Do not block on font-kit.

- [ ] **Step 2: Implement `crates/hymnal-gui/src/fonts.rs`**

```rust
//! System font family enumeration with a curated fallback.

/// Curated families that exist on virtually all systems — used as a fallback
/// when enumeration is unavailable, and merged ahead of enumerated families.
const CURATED: &[&str] = &[
    "sans-serif", "serif", "monospace",
    "Arial", "Helvetica", "Times New Roman", "Georgia", "Verdana",
];

/// Return a sorted, de-duplicated list of font family names. Always begins with
/// the curated families (so common picks are at the top), followed by any
/// additional system families discovered.
pub fn families() -> Vec<String> {
    let mut out: Vec<String> = CURATED.iter().map(|s| s.to_string()).collect();
    out.extend(enumerate_system());
    // De-dup case-insensitively, preserving first occurrence (curated first).
    let mut seen = std::collections::HashSet::new();
    out.retain(|f| seen.insert(f.to_lowercase()));
    out
}

#[cfg(feature = "system-fonts")]
fn enumerate_system() -> Vec<String> {
    use font_kit::source::SystemSource;
    match SystemSource::new().all_families() {
        Ok(mut v) => { v.sort(); v }
        Err(_) => Vec::new(),
    }
}

#[cfg(not(feature = "system-fonts"))]
fn enumerate_system() -> Vec<String> {
    Vec::new()
}
```
And gate font-kit behind a feature in `Cargo.toml` so the curated-only build always works:
```toml
[features]
default = ["system-fonts"]
system-fonts = ["dep:font-kit"]
```
and change the dep line to:
```toml
font-kit = { version = "0.14", optional = true }
```
> If font-kit built fine in Step 1, keep `default = ["system-fonts"]`. If it did
> NOT build, set `default = []` so the app compiles with the curated list only,
> and note it in the report.

- [ ] **Step 3: Declare module + build**

In `main.rs` add `mod fonts;` near `mod projector;`.
Run: `cargo build -p hymnal-gui 2>&1 | tail -10` → `Finished`.

- [ ] **Step 4: Commit**

```bash
git add crates/hymnal-gui/Cargo.toml crates/hymnal-gui/src/fonts.rs crates/hymnal-gui/src/main.rs
git commit -m "feat(gui): system font enumeration with curated fallback"
```

---

### Task 4: Rebuild `ThemesPanel` markup

**Files:**
- Modify: `crates/hymnal-gui/ui/app.slint` (replace `ThemesPanel`)
- Modify: `crates/hymnal-core/src/i18n.rs` + `main.rs apply_language` (new strings)

- [ ] **Step 1: Add I18n strings**

In `app.slint` `I18n` global add:
```slint
    in property <string> theme-weight-label: "Weight";
    in property <string> theme-align-label: "Align";
    in property <string> theme-text-color-label: "Text color";
    in property <string> theme-bg-color-label: "Background";
    in property <string> theme-set-active: "Set as active";
    in property <string> theme-name-label: "Name";
```
Add matching `pub` fields to `Strings` in `crates/hymnal-core/src/i18n.rs` (en/it/ro — English = the defaults above; IT: "Spessore","Allinea","Colore testo","Sfondo","Imposta come attivo","Nome"; RO: "Grosime","Aliniere","Culoare text","Fundal","Setează ca activ","Nume"). Add `g.set_theme_weight_label(...)` etc. in `apply_language`. Run `cargo build -p hymnal-core` to confirm i18n compiles.

- [ ] **Step 2: Replace the `ThemesPanel` component**

Replace the entire `component ThemesPanel inherits Rectangle { ... }` (lines ~656–724) with:
```slint
component ThemesPanel inherits Rectangle {
    // Thumbnail data (parallel arrays, set from Rust).
    in property <[string]> theme-names;
    in property <[color]> theme-bgs;
    in property <[color]> theme-fgs;
    in property <[string]> theme-fonts;
    in-out property <int> selected-index;
    in property <int> active-index;
    // Editor fields.
    in-out property <string> edit-name;
    in property <[string]> font-families;
    in-out property <int> edit-font-index;
    in-out property <int> edit-weight-index;   // index into weight-options
    in property <[string]> weight-options;
    in-out property <float> edit-font-size;
    in-out property <color> edit-text-color;
    in-out property <string> edit-text-hex;
    in-out property <color> edit-bg-color;
    in-out property <string> edit-bg-hex;
    in-out property <string> edit-h-align;       // "left"|"center"|"right"
    callback theme-selected(int);
    callback set-active();
    callback new-theme();
    callback save-theme();
    callback delete-theme();
    callback font-picked(int);
    callback weight-picked(int);
    callback size-changed(float);
    callback align-picked(string);
    callback text-hex-edited(string);
    callback text-preset(color);
    callback bg-hex-edited(string);
    callback bg-preset(color);
    background: Theme.bg;

    HorizontalBox {
        padding: 16px; spacing: 16px;
        // LEFT: thumbnail list + actions.
        VerticalBox {
            width: 190px; spacing: 8px;
            Text { text: I18n.themes-heading; color: Theme.text; font-size: 18px; font-weight: 700; }
            ScrollView {
                VerticalBox {
                    spacing: 6px;
                    for name[i] in root.theme-names: ThemeThumb {
                        name: name;
                        bg: root.theme-bgs[i];
                        fg: root.theme-fgs[i];
                        font-family: root.theme-fonts[i];
                        selected: i == root.selected-index;
                        active: i == root.active-index;
                        clicked => { root.theme-selected(i); }
                    }
                }
            }
            SecondaryButton { text: I18n.theme-set-active; clicked => { root.set-active(); } }
            HorizontalBox {
                spacing: 6px;
                SecondaryButton { text: I18n.theme-new; clicked => { root.new-theme(); } }
                SecondaryButton { text: I18n.theme-delete; danger: true; clicked => { root.delete-theme(); } }
            }
        }
        // RIGHT: preview box + 2-column form + footer save.
        VerticalBox {
            spacing: 14px;
            // Contained, centered preview box.
            HorizontalBox {
                alignment: center;
                Rectangle {
                    width: 340px; height: 150px; border-radius: Theme.radius;
                    border-width: 1px; border-color: Theme.panel-border; clip: true;
                    Rectangle {
                        background: root.edit-bg-color;
                        Text {
                            text: I18n.theme-preview-sample;
                            color: root.edit-text-color;
                            font-family: root.font-families[root.edit-font-index];
                            font-size: root.edit-font-size * 1px;
                            horizontal-alignment: root.edit-h-align == "left" ? TextHorizontalAlignment.left
                                : root.edit-h-align == "right" ? TextHorizontalAlignment.right
                                : TextHorizontalAlignment.center;
                            vertical-alignment: center;
                            width: parent.width - 24px; x: 12px; wrap: word-wrap;
                        }
                    }
                }
            }
            // 2-column form.
            HorizontalBox {
                spacing: 24px;
                // col 1
                VerticalBox {
                    spacing: 10px;
                    Text { text: I18n.theme-name-label; color: Theme.text-dim; font-size: 11px; }
                    LineEdit { text <=> root.edit-name; }
                    Text { text: I18n.theme-font-label; color: Theme.text-dim; font-size: 11px; }
                    ComboBox { model: root.font-families; current-index <=> root.edit-font-index;
                        selected => { root.font-picked(self.current-index); } }
                    Text { text: I18n.theme-weight-label; color: Theme.text-dim; font-size: 11px; }
                    ComboBox { model: root.weight-options; current-index <=> root.edit-weight-index;
                        selected => { root.weight-picked(self.current-index); } }
                    Text { text: I18n.theme-size-label; color: Theme.text-dim; font-size: 11px; }
                    HorizontalBox {
                        spacing: 8px;
                        Slider { minimum: 12; maximum: 120; value: root.edit-font-size;
                            changed(v) => { root.size-changed(v); } }
                        Text { text: Math.round(root.edit-font-size); color: Theme.text;
                            vertical-alignment: center; width: 30px; }
                    }
                }
                // col 2
                VerticalBox {
                    spacing: 10px;
                    Text { text: I18n.theme-align-label; color: Theme.text-dim; font-size: 11px; }
                    HorizontalBox {
                        spacing: 4px; alignment: start;
                        SecondaryButton { text: "L"; clicked => { root.align-picked("left"); } }
                        SecondaryButton { text: "C"; clicked => { root.align-picked("center"); } }
                        SecondaryButton { text: "R"; clicked => { root.align-picked("right"); } }
                    }
                    Text { text: I18n.theme-text-color-label; color: Theme.text-dim; font-size: 11px; }
                    ColorField {
                        value: root.edit-text-color; hex: root.edit-text-hex;
                        hex-edited(t) => { root.text-hex-edited(t); }
                        preset-picked(c) => { root.text-preset(c); }
                    }
                    Text { text: I18n.theme-bg-color-label; color: Theme.text-dim; font-size: 11px; }
                    ColorField {
                        value: root.edit-bg-color; hex: root.edit-bg-hex;
                        hex-edited(t) => { root.bg-hex-edited(t); }
                        preset-picked(c) => { root.bg-preset(c); }
                    }
                }
            }
            Rectangle { } // spacer
            // Footer save (right-aligned, small).
            HorizontalBox {
                alignment: end;
                PrimaryButton { text: I18n.theme-save; clicked => { root.save-theme(); } }
            }
        }
    }
}
```
> `Slider.changed(v)` callback name verify in 1.17 (may be `changed(float)`).
> `Math.round` is valid Slint. If `Slider` lacks a `changed` callback, use its
> `value` two-way bind + read in Rust on save instead; adapt and note it.

- [ ] **Step 3: Update AppWindow members + the `active-tab == 4` block**

In `AppWindow`, REPLACE the old themes-related members (the `theme-names: [StandardListViewItem]`, `theme-index`, `edit-font-family`, `edit-font-size`, `edit-font-weight`, `edit-text-color`, `edit-bg-color`, `edit-h-align`, and the `theme-selected/new-theme/save-theme/delete-theme/edit-changed` callbacks) with the new set used by `ThemesPanel` above:
```slint
    in property <[string]> theme-names;
    in property <[color]> theme-bgs;
    in property <[color]> theme-fgs;
    in property <[string]> theme-fonts;
    in-out property <int> selected-index;
    in property <int> active-index;
    in-out property <string> edit-name;
    in property <[string]> font-families;
    in-out property <int> edit-font-index;
    in-out property <int> edit-weight-index;
    in property <[string]> weight-options;
    in-out property <float> edit-font-size;
    in-out property <color> edit-text-color;
    in-out property <string> edit-text-hex;
    in-out property <color> edit-bg-color;
    in-out property <string> edit-bg-hex;
    in-out property <string> edit-h-align;
    callback theme-selected(int);
    callback set-active();
    callback new-theme();
    callback save-theme();
    callback delete-theme();
    callback font-picked(int);
    callback weight-picked(int);
    callback size-changed(float);
    callback align-picked(string);
    callback text-hex-edited(string);
    callback text-preset(color);
    callback bg-hex-edited(string);
    callback bg-preset(color);
```
Replace the `if root.active-tab == 4: ThemesPanel { ... }` block to forward ALL of the above (each `prop: root.prop;` / `in-out` `<=>` / `callback(a) => { root.callback(a); }`).

- [ ] **Step 4: Build (Rust wiring lands in Task 5 — expect setter errors)**

Run: `cargo build -p hymnal-gui 2>&1 | tail -30`.
Expected: slint markup compiles; Rust errors ONLY about removed/renamed setters in main.rs's existing theme handlers (e.g. `set_edit_font_family` gone, `set_theme_names` type changed). That's fine — Task 5 rewrites that wiring. If slint markup itself errors (Slider/ComboBox/array types), fix it now. Confirm `cargo build -p hymnal-core` passes (i18n).

- [ ] **Step 5: Commit**

```bash
git add crates/hymnal-gui/ui/app.slint crates/hymnal-core/src/i18n.rs crates/hymnal-gui/src/main.rs
git commit -m "feat(gui): rebuild Themes panel — thumbnails, 2-col form, color fields (UI)"
```

---

### Task 5: Rewire `ThemesPanel` in main.rs

**Files:**
- Modify: `crates/hymnal-gui/src/main.rs`

- [ ] **Step 1: Replace `refresh_theme_list` + `load_theme_into_editor` and add thumbnail/active state**

Rework the theme helpers and state. The weight ComboBox uses a fixed option list mapped to u16:
```rust
/// Weight dropdown options shown in the UI, paired with their numeric weight.
const WEIGHT_OPTIONS: &[(&str, u16)] = &[("Light", 300), ("Regular", 400), ("Medium", 500), ("Bold", 700), ("Black", 900)];

fn weight_index(w: u16) -> i32 {
    WEIGHT_OPTIONS.iter().position(|(_, v)| *v == w).unwrap_or(3) as i32
}

/// Push the theme list into the thumbnail-grid models (parallel arrays).
fn refresh_theme_list(ui: &AppWindow, themes: &Rc<std::cell::RefCell<Vec<Theme>>>, active_name: &str) {
    let dir = hymnal_core::library::themes_dir();
    let list = dir.as_deref().map(hymnal_core::theme::store::list_themes)
        .unwrap_or_else(|| vec![Theme::default()]);
    let names: Vec<slint::SharedString> = list.iter().map(|t| t.name.clone().into()).collect();
    let bgs: Vec<slint::Color> = list.iter().map(|t| match &t.background.kind {
        Background::Solid { color } => to_color(*color),
        Background::Gradient { from, .. } => to_color(*from),
        Background::Image { .. } => to_color(t.text.color),
    }).collect();
    let fgs: Vec<slint::Color> = list.iter().map(|t| to_color(t.text.color)).collect();
    let fonts: Vec<slint::SharedString> = list.iter().map(|t| t.text.font_family.clone().into()).collect();
    let active_idx = list.iter().position(|t| t.name == active_name).unwrap_or(0) as i32;
    ui.set_theme_names(slint::ModelRc::from(Rc::new(slint::VecModel::from(names))));
    ui.set_theme_bgs(slint::ModelRc::from(Rc::new(slint::VecModel::from(bgs))));
    ui.set_theme_fgs(slint::ModelRc::from(Rc::new(slint::VecModel::from(fgs))));
    ui.set_theme_fonts(slint::ModelRc::from(Rc::new(slint::VecModel::from(fonts))));
    ui.set_active_index(active_idx);
    *themes.borrow_mut() = list;
}

fn load_theme_into_editor(ui: &AppWindow, t: &Theme, families: &[String]) {
    // font index
    let fidx = families.iter().position(|f| f.eq_ignore_ascii_case(&t.text.font_family)).unwrap_or(0) as i32;
    ui.set_edit_font_index(fidx);
    ui.set_edit_weight_index(weight_index(t.text.font_weight));
    ui.set_edit_font_size(t.text.font_size_pt.unwrap_or(44.0));
    ui.set_edit_text_color(to_color(t.text.color));
    ui.set_edit_text_hex(t.text.color.to_hex().into());
    let (bg_color, bg_rgba) = match &t.background.kind {
        Background::Solid { color } => (to_color(*color), *color),
        Background::Gradient { from, .. } => (to_color(*from), *from),
        Background::Image { .. } => (to_color(t.text.color), t.text.color),
    };
    ui.set_edit_bg_color(bg_color);
    ui.set_edit_bg_hex(bg_rgba.to_hex().into());
    ui.set_edit_h_align(match t.text.h_align {
        HAlign::Left => "left", HAlign::Center => "center", HAlign::Right => "right",
    }.into());
    ui.set_edit_name(t.name.clone().into());
}
```

- [ ] **Step 2: Initialize editor state on boot**

Where the old `refresh_theme_list(&ui, &themes);` / `load_theme_into_editor(...)` calls were, set up the font + weight option models and the working "draft" theme:
```rust
    // Font families + weight options for the editor dropdowns.
    let families: Rc<Vec<String>> = Rc::new(fonts::families());
    {
        let rows: Vec<slint::SharedString> = families.iter().map(|f| f.clone().into()).collect();
        ui.set_font_families(slint::ModelRc::from(Rc::new(slint::VecModel::from(rows))));
        let weights: Vec<slint::SharedString> = WEIGHT_OPTIONS.iter().map(|(l, _)| (*l).into()).collect();
        ui.set_weight_options(slint::ModelRc::from(Rc::new(slint::VecModel::from(weights))));
    }
    // The theme currently being edited (a working copy).
    let draft = Rc::new(std::cell::RefCell::new(Theme::default()));
    refresh_theme_list(&ui, &themes, &active_theme.borrow().name);
    load_theme_into_editor(&ui, &draft.borrow(), &families);
    ui.set_selected_index(0);
```

- [ ] **Step 3: Rewrite the handlers**

Replace the old `on_theme_selected`/`on_save_theme`/`on_new_theme`/`on_delete_theme` (and remove `on_edit_changed`) with handlers driving `draft` + the new callbacks. Each edit callback updates `draft` and the live preview props:
```rust
    // Select a thumbnail -> load it into the editor draft.
    {
        let themes = themes.clone(); let draft = draft.clone();
        let families = families.clone(); let weak = ui.as_weak();
        ui.on_theme_selected(move |i| {
            let Some(ui) = weak.upgrade() else { return };
            if let Some(t) = themes.borrow().get(i.max(0) as usize) {
                *draft.borrow_mut() = t.clone();
                load_theme_into_editor(&ui, t, &families);
                ui.set_selected_index(i);
            }
        });
    }
    // Font picked.
    {
        let draft = draft.clone(); let families = families.clone(); let weak = ui.as_weak();
        ui.on_font_picked(move |i| {
            let Some(ui) = weak.upgrade() else { return };
            if let Some(f) = families.get(i.max(0) as usize) {
                draft.borrow_mut().text.font_family = f.clone();
                ui.set_edit_font_index(i);
            }
        });
    }
    // Weight picked.
    {
        let draft = draft.clone(); let weak = ui.as_weak();
        ui.on_weight_picked(move |i| {
            let Some(ui) = weak.upgrade() else { return };
            let w = WEIGHT_OPTIONS.get(i.max(0) as usize).map(|(_, v)| *v).unwrap_or(400);
            draft.borrow_mut().text.font_weight = w;
            ui.set_edit_weight_index(i);
        });
    }
    // Size changed.
    {
        let draft = draft.clone(); let weak = ui.as_weak();
        ui.on_size_changed(move |v| {
            let Some(ui) = weak.upgrade() else { return };
            draft.borrow_mut().text.font_size_pt = Some(v);
            ui.set_edit_font_size(v);
        });
    }
    // Align picked.
    {
        let draft = draft.clone(); let weak = ui.as_weak();
        ui.on_align_picked(move |a| {
            let Some(ui) = weak.upgrade() else { return };
            draft.borrow_mut().text.h_align = match a.as_str() {
                "left" => HAlign::Left, "right" => HAlign::Right, _ => HAlign::Center,
            };
            ui.set_edit_h_align(a);
        });
    }
    // Text color hex edited (ignore invalid).
    {
        let draft = draft.clone(); let weak = ui.as_weak();
        ui.on_text_hex_edited(move |t| {
            let Some(ui) = weak.upgrade() else { return };
            if let Some(rgba) = hymnal_core::theme::Rgba::from_hex(&t) {
                draft.borrow_mut().text.color = rgba;
                ui.set_edit_text_color(to_color(rgba));
                ui.set_edit_text_hex(rgba.to_hex().into());
            }
        });
    }
    {
        let draft = draft.clone(); let weak = ui.as_weak();
        ui.on_text_preset(move |c| {
            let Some(ui) = weak.upgrade() else { return };
            let rgba = hymnal_core::theme::Rgba::new(c.red(), c.green(), c.blue(), 255);
            draft.borrow_mut().text.color = rgba;
            ui.set_edit_text_color(c);
            ui.set_edit_text_hex(rgba.to_hex().into());
        });
    }
    // Background color hex + preset (sets Solid background).
    {
        let draft = draft.clone(); let weak = ui.as_weak();
        ui.on_bg_hex_edited(move |t| {
            let Some(ui) = weak.upgrade() else { return };
            if let Some(rgba) = hymnal_core::theme::Rgba::from_hex(&t) {
                draft.borrow_mut().background.kind = Background::Solid { color: rgba };
                ui.set_edit_bg_color(to_color(rgba));
                ui.set_edit_bg_hex(rgba.to_hex().into());
            }
        });
    }
    {
        let draft = draft.clone(); let weak = ui.as_weak();
        ui.on_bg_preset(move |c| {
            let Some(ui) = weak.upgrade() else { return };
            let rgba = hymnal_core::theme::Rgba::new(c.red(), c.green(), c.blue(), 255);
            draft.borrow_mut().background.kind = Background::Solid { color: rgba };
            ui.set_edit_bg_color(c);
            ui.set_edit_bg_hex(rgba.to_hex().into());
        });
    }
    // New theme: a fresh draft named "Custom".
    {
        let draft = draft.clone(); let families = families.clone(); let weak = ui.as_weak();
        ui.on_new_theme(move || {
            let Some(ui) = weak.upgrade() else { return };
            let t = Theme { name: "Custom".into(), ..Theme::default() };
            *draft.borrow_mut() = t.clone();
            load_theme_into_editor(&ui, &t, &families);
        });
    }
    // Save the draft (reads edit-name for the title).
    {
        let themes = themes.clone(); let draft = draft.clone();
        let active_theme = active_theme.clone(); let weak = ui.as_weak();
        ui.on_save_theme(move || {
            let Some(ui) = weak.upgrade() else { return };
            let mut t = draft.borrow().clone();
            t.name = ui.get_edit_name().to_string();
            if t.name.is_empty() || t.name == "Default" { t.name = "Custom".into(); }
            if let Some(dir) = hymnal_core::library::themes_dir() {
                match hymnal_core::theme::store::save_theme(&dir, &t) {
                    Ok(()) => info!("saved theme {}", t.name),
                    Err(e) => warn!("save theme failed: {e}"),
                }
            }
            *draft.borrow_mut() = t.clone();
            refresh_theme_list(&ui, &themes, &active_theme.borrow().name);
        });
    }
    // Delete the selected theme; if it was active, reset active to Default.
    {
        let themes = themes.clone(); let active_theme = active_theme.clone();
        let projector = projector.clone(); let present = present.clone(); let weak = ui.as_weak();
        ui.on_delete_theme(move || {
            let Some(ui) = weak.upgrade() else { return };
            let idx = ui.get_selected_index().max(0) as usize;
            let name = themes.borrow().get(idx).map(|t| t.name.clone());
            if let Some(name) = name {
                if let Some(dir) = hymnal_core::library::themes_dir() {
                    if let Err(e) = hymnal_core::theme::store::delete_theme(&dir, &name) {
                        warn!("delete theme failed: {e}");
                    }
                }
                if active_theme.borrow().name == name {
                    *active_theme.borrow_mut() = Theme::default();
                    if let Some(p) = hymnal_core::library::config_path() {
                        let mut cfg = Config::load(&p).unwrap_or_default();
                        cfg.active_theme = Some("Default".into());
                        let _ = cfg.save(&p);
                    }
                    push_to_projector(&projector, &active_theme.borrow(), &present.borrow());
                }
            }
            refresh_theme_list(&ui, &themes, &active_theme.borrow().name);
        });
    }
    // Set the selected theme as the active projection theme; persist + push.
    {
        let themes = themes.clone(); let active_theme = active_theme.clone();
        let projector = projector.clone(); let present = present.clone(); let weak = ui.as_weak();
        ui.on_set_active(move || {
            let Some(ui) = weak.upgrade() else { return };
            let idx = ui.get_selected_index().max(0) as usize;
            if let Some(t) = themes.borrow().get(idx).cloned() {
                *active_theme.borrow_mut() = t.clone();
                if let Some(p) = hymnal_core::library::config_path() {
                    let mut cfg = Config::load(&p).unwrap_or_default();
                    cfg.active_theme = Some(t.name.clone());
                    let _ = cfg.save(&p);
                }
                push_to_projector(&projector, &active_theme.borrow(), &present.borrow());
                refresh_theme_list(&ui, &themes, &t.name);
            }
        });
    }
```
> `projector`, `present`, `active_theme` Rcs are defined later in main.rs today.
> Ensure these handler blocks are placed AFTER those `let` bindings (move the
> theme handlers down, or move the bindings up). The build will tell you.

- [ ] **Step 4: Build + smoke + clippy**

Run: `cargo build -p hymnal-gui 2>&1 | tail -25` → `Finished`. Fix borrow/order issues.
Run: `RUST_LOG=hymnal_gui=info timeout 20 cargo run -q -p hymnal-gui 2>&1 | grep -iE "panic|error" | head` → no panic.
Run: `cargo clippy -p hymnal-gui 2>&1 | grep -E "^error" | head` → none.

- [ ] **Step 5: Commit**

```bash
git add crates/hymnal-gui/src/main.rs
git commit -m "feat(gui): wire rebuilt Themes panel — thumbnails, draft editor, set-active"
```

---

## Phase 3 — Control tab

### Task 6: Rebuild `ControlPanel` markup (remove theme picker, fixed boxes, styled buttons)

**Files:**
- Modify: `crates/hymnal-gui/ui/app.slint` (replace `ControlPanel`)

- [ ] **Step 1: Replace `ControlPanel`**

Replace `component ControlPanel inherits Rectangle { ... }` (lines ~726–804) with a version that removes `theme-names`/`ctl-theme-index`/`theme-picked`, uses `PrimaryButton`/`SecondaryButton`, and fixed-size Live/Next boxes:
```slint
component ControlPanel inherits Rectangle {
    in property <string> current-title;
    in property <string> live-text;
    in property <string> next-text;
    in property <string> slide-pos;
    in property <bool> projecting;
    in property <bool> blank;
    in property <[string]> display-names;
    in-out property <int> display-index;
    in property <[StandardListViewItem]> search-results;
    in-out property <int> search-current;
    callback start();
    callback stop();
    callback blank-toggle();
    callback prev();
    callback next();
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
            padding: 14px; spacing: 12px;
            // Toolbar.
            Rectangle {
                background: Theme.panel; border-radius: Theme.radius;
                border-width: 1px; border-color: Theme.panel-border;
                HorizontalBox {
                    padding: 8px; spacing: 10px; alignment: start;
                    Text { text: I18n.control-output + ":"; color: Theme.text-dim; vertical-alignment: center; }
                    ComboBox { model: root.display-names; current-index <=> root.display-index;
                        selected => { root.display-picked(self.current-index); } }
                    Rectangle { horizontal-stretch: 1; }
                    PrimaryButton { text: root.projecting ? I18n.control-stop : I18n.control-start;
                        clicked => { if (root.projecting) { root.stop(); } else { root.start(); } } }
                    SecondaryButton { text: I18n.control-blank; clicked => { root.blank-toggle(); } }
                }
            }
            // Search.
            LineEdit {
                placeholder-text: I18n.search-placeholder;
                edited(t) => { root.search-changed(t); }
                accepted(t) => { root.search-activated(root.search-current); }
            }
            // Two panes.
            HorizontalBox {
                spacing: 14px;
                StandardListView {
                    width: 40%;
                    model: root.search-results;
                    current-item <=> root.search-current;
                    current-item-changed(i) => { root.search-activated(i); }
                }
                VerticalBox {
                    alignment: start; spacing: 8px;
                    Text { text: root.current-title; color: Theme.accent-soft; font-weight: 700; }
                    HorizontalBox {
                        max-width: 300px;
                        Text { text: I18n.control-live; color: Theme.text-dim; font-size: 11px; }
                        Rectangle { horizontal-stretch: 1; }
                        Text { text: root.slide-pos; color: Theme.text-dim; font-size: 11px; }
                    }
                    Rectangle {
                        width: 300px; height: 169px; background: #0b1f3a;
                        border-radius: Theme.radius; border-width: 1px; border-color: Theme.panel-border;
                        Text { text: root.blank ? "" : root.live-text; color: white; font-weight: 700;
                            horizontal-alignment: center; vertical-alignment: center;
                            width: parent.width - 24px; x: 12px; wrap: word-wrap; }
                    }
                    Text { text: I18n.control-next-label; color: Theme.text-dim; font-size: 11px; }
                    Rectangle {
                        width: 300px; height: 90px; background: #0b1f3a;
                        border-radius: Theme.radius; border-width: 1px; border-color: Theme.panel-border;
                        Text { text: root.next-text; color: #cbd5e1;
                            horizontal-alignment: center; vertical-alignment: center;
                            width: parent.width - 24px; x: 12px; wrap: word-wrap; }
                    }
                    HorizontalBox {
                        max-width: 300px; spacing: 8px; alignment: center;
                        SecondaryButton { text: I18n.control-prev; clicked => { root.prev(); } }
                        PrimaryButton { text: I18n.control-next; clicked => { root.next(); } }
                    }
                }
            }
        }
    }
}
```

- [ ] **Step 2: Update AppWindow Control members + panel block**

In `AppWindow`, remove the Control theme members (`ctl-theme-names`, `ctl-theme-index`, `ctl-theme-picked` callback). Update the `if root.active-tab == 3: ControlPanel { ... }` block to drop `theme-names`/`ctl-theme-index`/`theme-picked` bindings; keep the rest.

- [ ] **Step 3: Build (expect Rust errors for removed theme-picker wiring)**

Run: `cargo build -p hymnal-gui 2>&1 | tail -25`.
Expected: slint compiles; Rust errors only where main.rs still calls
`set_ctl_theme_names` / `on_ctl_theme_picked` (removed in Task 7). If slint
markup errors, fix now.

- [ ] **Step 4: Commit**

```bash
git add crates/hymnal-gui/ui/app.slint
git commit -m "feat(gui): rebuild Control panel — no theme picker, fixed boxes, styled buttons (UI)"
```

---

### Task 7: Remove Control theme-picker wiring in main.rs

**Files:**
- Modify: `crates/hymnal-gui/src/main.rs`

- [ ] **Step 1: Delete the obsolete wiring**

Remove the `set_ctl_theme_names(...)` population block (~line 320) and the entire
`ui.on_ctl_theme_picked(move |i| { ... })` handler (~line 1018). The Control tab
now always projects with `active_theme` (set in the Themes tab), which
`push_to_projector` already reads. Leave all other Control handlers
(`on_ctl_start/stop/go_next/prev/blank_toggle/display_picked/search_changed/
search_activated`, `on_project_current`) intact.

- [ ] **Step 2: Build + smoke + clippy**

Run: `cargo build -p hymnal-gui 2>&1 | tail -20` → `Finished`, no errors. Remove any now-unused imports the compiler flags.
Run: `cargo test -p hymnal-core 2>&1 | grep "test result"` → all pass.
Run: `RUST_LOG=hymnal_gui=info timeout 20 cargo run -q -p hymnal-gui 2>&1 | grep -iE "panic|error" | head` → no panic.
Run: `cargo clippy --workspace 2>&1 | grep -cE "^warning|^error"` → 0.

- [ ] **Step 3: Commit**

```bash
git add crates/hymnal-gui/src/main.rs
git commit -m "refactor(gui): Control tab reads active theme; remove its theme picker wiring"
```

---

## Task 8: Manual end-to-end verification

**Files:** none.

- [ ] **Step 1: Run release build**

Run: `cargo run --release -p hymnal-gui`
Verify:
- **Themes tab:** thumbnails render each theme as a mini-slide; clicking one loads it into the editor; Font/Weight are dropdowns, Size is a slider, Align is L/C/R, Text/Background are swatch + hex + presets; the preview box is fixed-size and text wraps (no overflow); typing an invalid hex doesn't corrupt the swatch; Save persists (survives restart); "Set as active" marks a theme with the ● Active badge; deleting the active theme falls back to Default.
- **Control tab:** no theme selector; toolbar is Output + Start/Stop + Blank, buttons are normal-sized; Live (300×169) and Next (300×90) boxes are fixed-size and wrap text; projecting uses the active theme set in Themes; →/←/Space/B/Esc work.
- Switching tabs doesn't break Library arrow-key nav.

- [ ] **Step 2: Verification only** — fix in earlier tasks and re-run if wrong.

---

## Self-Review Notes

- **Spec coverage:** reusable styled controls (Task 2); hex↔Rgba + ColorField swatch/hex/presets (Tasks 1,2,5); font dropdown from system enumeration + curated fallback (Task 3); weight dropdown + size slider + align toggle (Tasks 4,5); contained fixed preview box + 2-col form (Task 4); thumbnail grid with edit-selection vs active (● Active) (Tasks 2,4,5); "Set as active" persistence + delete-active→Default (Task 5); Control tab fixed Live/Next boxes, compact toolbar, theme-picker removed (Tasks 6,7); error handling for invalid hex / missing font / deleted active (Tasks 3,5); manual E2E (Task 8). All spec sections map to a task.
- **Deferred (per spec):** renderer "5b" features and the controls for them; Library/Downloader/Settings restyle.
- **Type/name consistency:** `Rgba::from_hex/to_hex`, `fonts::families()`, `WEIGHT_OPTIONS`/`weight_index`, `refresh_theme_list(ui, themes, active_name)`, `load_theme_into_editor(ui, t, families)`, `draft`/`active_theme`/`themes` Rcs, and the Slint members (`theme-names:[string]`, `theme-bgs/fgs:[color]`, `theme-fonts:[string]`, `selected-index`/`active-index`, `edit-*`, `font-families`, `weight-options`, callbacks `theme-selected/set-active/new-theme/save-theme/delete-theme/font-picked/weight-picked/size-changed/align-picked/text-hex-edited/text-preset/bg-hex-edited/bg-preset`) are consistent across Tasks 4–5. Control members drop `ctl-theme-*` consistently in Tasks 6–7.
- **Placeholder scan:** no TBD/TODO. The font-kit "if it doesn't build, use curated-only via feature flag" is a concrete, coded fallback (feature gate), not a placeholder. One self-correcting note flagged inline: drop the stray `text_name_for_editor()` line in `load_theme_into_editor`.
```
