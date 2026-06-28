# App Translations (English / Italian / Romanian) — Design

Date: 2026-06-28

## Goal

Make all user-facing text in SDA Manager available in English, Italian, and
Romanian, switchable live from a language picker in the Settings tab. The chosen
language persists across launches. On first run (no saved choice) the app
detects the OS locale and uses it if it is one of the three, else English.

Translations ship with the app and are **not** user-editable (a language picker
only — no in-app string editor).

## Approach

Typed translation strings live in `hymnal-core` (testable, no UI dependency); a
Slint `global I18n` exposes one string property per UI label; `main.rs` maps a
`Strings` struct onto that global and also uses it for dynamic status messages.
Switching language is live — it re-fills the global, no restart.

This is a **runtime** system, deliberately not Slint's compile-time `@tr()`
gettext: strings must swap at runtime when the user picks a language.

## Components

### 1. `hymnal-core/src/i18n.rs` (new, unit-tested)

```rust
pub enum Language { En, It, Ro }          // derive Copy, Clone, PartialEq, Eq

impl Language {
    pub fn from_locale(loc: &str) -> Language;  // "it_IT"/"it" → It, "ro*" → Ro, else En
    pub fn code(self) -> &'static str;          // "en" | "it" | "ro"  (persisted in Config)
    pub fn from_code(s: &str) -> Language;       // parse persisted value; unknown → En
    pub fn label(self) -> &'static str;          // "English" | "Italiano" | "Română"
    pub fn all() -> [Language; 3];               // for building the picker
}

pub struct Strings {                       // one field per user-facing string
    // static UI (from app.slint)
    pub app_title: String,
    pub nav_library: String,
    pub nav_downloader: String,
    pub nav_settings: String,
    pub search_placeholder: String,
    pub prev_slide: String,
    pub next_slide: String,
    pub open_in_powerpoint: String,
    pub reveal_in_folder: String,
    pub downloader_heading: String,
    pub downloader_subtitle: String,
    pub downloader_url_placeholder: String,
    pub choose: String,
    pub setting_up_downloader: String,
    pub download_complete: String,
    pub download_failed_prefix: String,   // "Download failed: " + reason
    pub settings_heading: String,
    pub version_prefix: String,           // "Version " + number
    pub library_heading: String,
    pub force_sync_description: String,
    pub force_sync_button: String,
    pub syncing_button: String,
    pub language_heading: String,         // new Settings section label
    // dynamic status (from main.rs)
    pub status_loading: String,
    pub status_library_ready: String,
    pub status_re_cloning: String,
    pub update_checking: String,
    pub update_up_to_date: String,
    pub update_failed: String,
}

impl Strings {
    pub fn for_language(lang: Language) -> Strings;  // dispatches to en()/it()/ro()
}
```

String values with interpolation stay as templates assembled in Rust/Slint
(e.g. `version_prefix` is concatenated with the version; the few `format!`
messages with args — `Synced — indexed {n} hymns.`, `Update {version} staged
— restart to apply.`, `Sync failed: {e}`, `{title} — slide {i}/{n}` — keep
their format strings as `Strings` fields with `{}` placeholders, filled in
`main.rs`).

Because `Strings` has named fields, adding a new UI string is a **compile
error** until `en()`, `it()`, and `ro()` all supply it.

### 2. Config (`library.rs`)

Add `#[serde(default)] pub language: Option<String>` (stores `"en"`/`"it"`/
`"ro"`; `None` = not yet chosen). Round-trips through the existing
`to_toml`/`from_toml`; covered by a new persistence test.

### 3. Slint `global I18n` (`app.slint`)

A `global I18n { in property <string> <field>; … }` with one property per
`Strings` field. Every hardcoded literal in the UI is replaced by
`I18n.<field>`. The window `title`, nav labels, all panel text, button labels,
and Settings descriptions bind to the global.

### 4. `main.rs`

- `fn apply_language(ui: &AppWindow, lang: Language)`: builds
  `Strings::for_language(lang)` and calls the generated `I18n` setters
  (`ui.global::<I18n>().set_…`). Returns the `Strings` so the caller can keep it
  for dynamic status messages.
- The current language + its `Strings` live in the UI-thread state
  (`Rc<RefCell<…>>` alongside `searcher`), so status-setting code uses the
  active translation.
- Boot: language = `Config.language` (via `from_code`) if set, else
  `Language::from_locale(<OS locale>)`. Apply before first content render.
- OS locale via the `sys-locale` crate (cross-platform; avoids per-OS env
  parsing). Add to `hymnal-gui/Cargo.toml`.

### 5. Settings tab language picker

A new "Language" section in `SettingsPanel` with three selectable pills (reusing
the existing `TouchArea` pill style), one per `Language::all()`, highlighting the
active one. An `in property <string> active-language` drives the highlight; a new
`callback set-language(string)` fires on click. `main.rs` wires `set-language` →
`apply_language` + persist `Config.language`. The switch is immediate.

## Error handling

- Unknown/absent OS locale or saved code → English (`from_locale`/`from_code`
  fall back).
- Config save failure → log and keep the in-memory choice (matches existing
  `force-sync` behavior). No new failure modes.

## Testing (hymnal-core)

- `from_locale`: `"it_IT"`→It, `"it"`→It, `"ro_RO"`→Ro, `"en_US"`→En,
  `"de_DE"`→En, `""`→En.
- `code` ↔ `from_code` round-trip for all three; unknown code → En.
- `Config` persists and reloads `language`.
- Completeness: for every `Language`, `Strings::for_language` returns non-empty
  values in every field (guards against an empty/forgotten translation). Done by
  iterating the fields — implemented as an explicit assertion list or a small
  helper returning all fields as a slice.

GUI wiring (`apply_language`, picker) is not unit-tested (consistent with the
rest of `hymnal-gui`); verified by running the app.

## Out of scope (YAGNI)

- User-editable / custom translations and any in-app string editor.
- Languages beyond the three.
- Translating the hymn content itself (only the app chrome is translated).
- Right-to-left layouts.
