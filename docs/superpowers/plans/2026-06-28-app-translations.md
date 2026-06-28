# App Translations (en/it/ro) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add English/Italian/Romanian translations to all user-facing text, switchable live from a language picker in the Settings tab, with the choice persisted and the OS locale used on first run.

**Architecture:** Translation data and language logic live in `hymnal-core` (`i18n.rs`, unit-tested). A Slint `global I18n` exposes one string property per UI label. `main.rs` maps a `Strings` struct onto that global (`apply_language`) and reuses it for dynamic status messages. Switching re-fills the global — no restart.

**Tech Stack:** Rust, Slint 1.8, serde/toml (config), `sys-locale` (OS locale detection).

---

## File structure

- Create `crates/hymnal-core/src/i18n.rs` — `Language` enum, `Strings` struct, per-language constructors, locale mapping.
- Modify `crates/hymnal-core/src/lib.rs` — register `pub mod i18n;`.
- Modify `crates/hymnal-core/src/library.rs` — add `Config.language: Option<String>` + persistence test.
- Create `crates/hymnal-core/tests/i18n_test.rs` — integration tests for locale mapping, code round-trip, completeness.
- Modify `crates/hymnal-gui/Cargo.toml` — add `sys-locale`.
- Modify `crates/hymnal-gui/ui/app.slint` — add `export global I18n`, replace literals with `I18n.<field>`, add language picker + `set-language` callback + `active-language` property.
- Modify `crates/hymnal-gui/src/main.rs` — `apply_language`, boot language resolution, status messages via active `Strings`, wire `set-language`.

---

## Task 1: `Language` enum + locale mapping

**Files:**
- Create: `crates/hymnal-core/src/i18n.rs`
- Modify: `crates/hymnal-core/src/lib.rs`

- [ ] **Step 1: Register the module**

In `crates/hymnal-core/src/lib.rs`, add after the existing `pub mod` lines:

```rust
pub mod i18n;
```

- [ ] **Step 2: Write `i18n.rs` with `Language` + tests**

Create `crates/hymnal-core/src/i18n.rs`:

```rust
//! Runtime app translations. `Language` selects a set of UI strings; `Strings`
//! holds one field per user-facing label/message. All data lives here (no UI
//! dependency) so translations are unit-testable.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    En,
    It,
    Ro,
}

impl Language {
    /// All languages in display order (used to build the picker).
    pub fn all() -> [Language; 3] {
        [Language::En, Language::It, Language::Ro]
    }

    /// Map an OS locale string (e.g. "it_IT.UTF-8", "ro", "en-US") to a
    /// language. Matches on the leading two-letter code; unknown → English.
    pub fn from_locale(loc: &str) -> Language {
        let lc = loc.trim().to_ascii_lowercase();
        if lc.starts_with("it") {
            Language::It
        } else if lc.starts_with("ro") {
            Language::Ro
        } else {
            Language::En
        }
    }

    /// Stable code persisted in config.
    pub fn code(self) -> &'static str {
        match self {
            Language::En => "en",
            Language::It => "it",
            Language::Ro => "ro",
        }
    }

    /// Parse a persisted code; unknown → English.
    pub fn from_code(s: &str) -> Language {
        match s {
            "it" => Language::It,
            "ro" => Language::Ro,
            _ => Language::En,
        }
    }

    /// Native display label for the picker.
    pub fn label(self) -> &'static str {
        match self {
            Language::En => "English",
            Language::It => "Italiano",
            Language::Ro => "Română",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_locale_maps_known_languages() {
        assert_eq!(Language::from_locale("it_IT.UTF-8"), Language::It);
        assert_eq!(Language::from_locale("it"), Language::It);
        assert_eq!(Language::from_locale("ro_RO"), Language::Ro);
        assert_eq!(Language::from_locale("en_US"), Language::En);
    }

    #[test]
    fn from_locale_unknown_falls_back_to_english() {
        assert_eq!(Language::from_locale("de_DE"), Language::En);
        assert_eq!(Language::from_locale(""), Language::En);
    }

    #[test]
    fn code_round_trips() {
        for lang in Language::all() {
            assert_eq!(Language::from_code(lang.code()), lang);
        }
        assert_eq!(Language::from_code("xx"), Language::En);
    }
}
```

- [ ] **Step 3: Run tests, expect PASS**

Run: `cargo test -p hymnal-core i18n`
Expected: the three `i18n::tests` pass.

- [ ] **Step 4: Commit**

```bash
git add crates/hymnal-core/src/i18n.rs crates/hymnal-core/src/lib.rs
git commit -m "feat(core): add Language enum with locale mapping"
```

---

## Task 2: `Strings` struct + per-language translations

**Files:**
- Modify: `crates/hymnal-core/src/i18n.rs`
- Create: `crates/hymnal-core/tests/i18n_test.rs`

- [ ] **Step 1: Add `Strings` + constructors**

In `crates/hymnal-core/src/i18n.rs`, add below the `Language` impl. The
`as_fields` helper exists so a test can assert no field is empty without
listing all fields.

```rust
/// One user-facing string per field. Named fields make adding a string a
/// compile error until every language provides it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Strings {
    // Static UI (app.slint)
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
    pub download_failed_prefix: String,
    pub settings_heading: String,
    pub version_prefix: String,
    pub library_heading: String,
    pub force_sync_description: String,
    pub force_sync_button: String,
    pub syncing_button: String,
    pub language_heading: String,
    // Dynamic status (main.rs). `{}` placeholders filled with format!.
    pub status_loading: String,
    pub status_library_ready: String,
    pub status_re_cloning: String,
    pub update_checking: String,
    pub update_up_to_date: String,
    pub update_failed: String,
    pub status_synced_fmt: String,   // one `{}` = hymn count
    pub status_sync_failed_fmt: String, // one `{}` = error
    pub update_staged_fmt: String,   // one `{}` = version
    pub slide_counter_fmt: String,   // "{} — slide {}/{}" = title, idx, count
    pub slide_zero_fmt: String,      // "{} — 0 slides" = title
}

impl Strings {
    pub fn for_language(lang: Language) -> Strings {
        match lang {
            Language::En => Strings::en(),
            Language::It => Strings::it(),
            Language::Ro => Strings::ro(),
        }
    }

    /// All field values, for completeness checks.
    pub fn as_fields(&self) -> Vec<&str> {
        vec![
            &self.app_title, &self.nav_library, &self.nav_downloader,
            &self.nav_settings, &self.search_placeholder, &self.prev_slide,
            &self.next_slide, &self.open_in_powerpoint, &self.reveal_in_folder,
            &self.downloader_heading, &self.downloader_subtitle,
            &self.downloader_url_placeholder, &self.choose,
            &self.setting_up_downloader, &self.download_complete,
            &self.download_failed_prefix, &self.settings_heading,
            &self.version_prefix, &self.library_heading,
            &self.force_sync_description, &self.force_sync_button,
            &self.syncing_button, &self.language_heading, &self.status_loading,
            &self.status_library_ready, &self.status_re_cloning,
            &self.update_checking, &self.update_up_to_date, &self.update_failed,
            &self.status_synced_fmt, &self.status_sync_failed_fmt,
            &self.update_staged_fmt, &self.slide_counter_fmt,
            &self.slide_zero_fmt,
        ]
    }

    fn en() -> Strings {
        Strings {
            app_title: "SDA Manager".into(),
            nav_library: "Library".into(),
            nav_downloader: "Video Downloader".into(),
            nav_settings: "Settings".into(),
            search_placeholder: "Search by number, title, or lyrics…".into(),
            prev_slide: "‹ Prev".into(),
            next_slide: "Next ›".into(),
            open_in_powerpoint: "Open in PowerPoint".into(),
            reveal_in_folder: "Reveal in folder".into(),
            downloader_heading: "Video Downloader".into(),
            downloader_subtitle: "Paste a YouTube link and choose where to save it.".into(),
            downloader_url_placeholder: "Paste a YouTube URL…".into(),
            choose: "Choose…".into(),
            setting_up_downloader: "Setting up downloader…".into(),
            download_complete: "✓ Download complete".into(),
            download_failed_prefix: "Download failed: ".into(),
            settings_heading: "Settings".into(),
            version_prefix: "Version ".into(),
            library_heading: "Library".into(),
            force_sync_description: "Force sync deletes the local hymn library and cache, then re-downloads and reindexes everything.".into(),
            force_sync_button: "Force sync library".into(),
            syncing_button: "Syncing…".into(),
            language_heading: "Language".into(),
            status_loading: "Loading hymn library…".into(),
            status_library_ready: "Library ready.".into(),
            status_re_cloning: "Re-cloning and reindexing…".into(),
            update_checking: "Checking for updates…".into(),
            update_up_to_date: "Up to date.".into(),
            update_failed: "Update check failed.".into(),
            status_synced_fmt: "Synced — indexed {} hymns.".into(),
            status_sync_failed_fmt: "Sync failed: {}".into(),
            update_staged_fmt: "Update {} staged — restart to apply.".into(),
            slide_counter_fmt: "{} — slide {}/{}".into(),
            slide_zero_fmt: "{} — 0 slides".into(),
        }
    }

    fn it() -> Strings {
        Strings {
            app_title: "SDA Manager".into(),
            nav_library: "Innario".into(),
            nav_downloader: "Scarica video".into(),
            nav_settings: "Impostazioni".into(),
            search_placeholder: "Cerca per numero, titolo o testo…".into(),
            prev_slide: "‹ Prec".into(),
            next_slide: "Succ ›".into(),
            open_in_powerpoint: "Apri in PowerPoint".into(),
            reveal_in_folder: "Mostra nella cartella".into(),
            downloader_heading: "Scarica video".into(),
            downloader_subtitle: "Incolla un link di YouTube e scegli dove salvarlo.".into(),
            downloader_url_placeholder: "Incolla un URL di YouTube…".into(),
            choose: "Scegli…".into(),
            setting_up_downloader: "Preparazione del downloader…".into(),
            download_complete: "✓ Download completato".into(),
            download_failed_prefix: "Download non riuscito: ".into(),
            settings_heading: "Impostazioni".into(),
            version_prefix: "Versione ".into(),
            library_heading: "Innario".into(),
            force_sync_description: "La sincronizzazione forzata elimina l'innario locale e la cache, poi riscarica e reindicizza tutto.".into(),
            force_sync_button: "Sincronizza innario".into(),
            syncing_button: "Sincronizzazione…".into(),
            language_heading: "Lingua".into(),
            status_loading: "Caricamento dell'innario…".into(),
            status_library_ready: "Innario pronto.".into(),
            status_re_cloning: "Riscaricamento e reindicizzazione…".into(),
            update_checking: "Ricerca aggiornamenti…".into(),
            update_up_to_date: "Aggiornato.".into(),
            update_failed: "Controllo aggiornamenti non riuscito.".into(),
            status_synced_fmt: "Sincronizzato — {} inni indicizzati.".into(),
            status_sync_failed_fmt: "Sincronizzazione non riuscita: {}".into(),
            update_staged_fmt: "Aggiornamento {} pronto — riavvia per applicarlo.".into(),
            slide_counter_fmt: "{} — diapositiva {}/{}".into(),
            slide_zero_fmt: "{} — 0 diapositive".into(),
        }
    }

    fn ro() -> Strings {
        Strings {
            app_title: "SDA Manager".into(),
            nav_library: "Imnar".into(),
            nav_downloader: "Descărcare video".into(),
            nav_settings: "Setări".into(),
            search_placeholder: "Caută după număr, titlu sau vers…".into(),
            prev_slide: "‹ Înapoi".into(),
            next_slide: "Înainte ›".into(),
            open_in_powerpoint: "Deschide în PowerPoint".into(),
            reveal_in_folder: "Arată în folder".into(),
            downloader_heading: "Descărcare video".into(),
            downloader_subtitle: "Lipește un link YouTube și alege unde să-l salvezi.".into(),
            downloader_url_placeholder: "Lipește un URL YouTube…".into(),
            choose: "Alege…".into(),
            setting_up_downloader: "Se pregătește descărcătorul…".into(),
            download_complete: "✓ Descărcare finalizată".into(),
            download_failed_prefix: "Descărcare eșuată: ".into(),
            settings_heading: "Setări".into(),
            version_prefix: "Versiunea ".into(),
            library_heading: "Imnar".into(),
            force_sync_description: "Sincronizarea forțată șterge imnarul local și memoria cache, apoi descarcă și reindexează totul.".into(),
            force_sync_button: "Sincronizează imnarul".into(),
            syncing_button: "Se sincronizează…".into(),
            language_heading: "Limbă".into(),
            status_loading: "Se încarcă imnarul…".into(),
            status_library_ready: "Imnar pregătit.".into(),
            status_re_cloning: "Se reclonează și se reindexează…".into(),
            update_checking: "Se caută actualizări…".into(),
            update_up_to_date: "La zi.".into(),
            update_failed: "Verificarea actualizărilor a eșuat.".into(),
            status_synced_fmt: "Sincronizat — {} imnuri indexate.".into(),
            status_sync_failed_fmt: "Sincronizare eșuată: {}".into(),
            update_staged_fmt: "Actualizarea {} este pregătită — repornește pentru a o aplica.".into(),
            slide_counter_fmt: "{} — diapozitivul {}/{}".into(),
            slide_zero_fmt: "{} — 0 diapozitive".into(),
        }
    }
}
```

- [ ] **Step 2: Write the completeness integration test**

Create `crates/hymnal-core/tests/i18n_test.rs`:

```rust
use hymnal_core::i18n::{Language, Strings};

#[test]
fn every_language_has_no_empty_strings() {
    for lang in Language::all() {
        let s = Strings::for_language(lang);
        for (i, field) in s.as_fields().iter().enumerate() {
            assert!(
                !field.trim().is_empty(),
                "language {:?} has an empty string at field index {}",
                lang,
                i
            );
        }
    }
}

#[test]
fn format_strings_keep_their_placeholders() {
    // Guard: the count placeholder must survive translation in all languages.
    for lang in Language::all() {
        let s = Strings::for_language(lang);
        assert!(s.status_synced_fmt.contains("{}"), "{:?} synced fmt", lang);
        assert!(s.update_staged_fmt.contains("{}"), "{:?} staged fmt", lang);
        assert_eq!(
            s.slide_counter_fmt.matches("{}").count(),
            3,
            "{:?} slide counter needs 3 placeholders",
            lang
        );
    }
}
```

- [ ] **Step 3: Run tests, expect PASS**

Run: `cargo test -p hymnal-core i18n`
Expected: `i18n::tests` (3) + `i18n_test` (2) pass.

- [ ] **Step 4: Commit**

```bash
git add crates/hymnal-core/src/i18n.rs crates/hymnal-core/tests/i18n_test.rs
git commit -m "feat(core): add Strings with en/it/ro translations"
```

---

## Task 3: Persist language choice in Config

**Files:**
- Modify: `crates/hymnal-core/src/library.rs`

- [ ] **Step 1: Add the field**

In `crates/hymnal-core/src/library.rs`, in `struct Config` after `download_dir`:

```rust
    /// Selected UI language code ("en"/"it"/"ro"). `None` => not yet chosen
    /// (detect from OS locale on first run).
    #[serde(default)]
    pub language: Option<String>,
```

In `impl Default for Config`, add `language: None,` to the returned struct.

- [ ] **Step 2: Add the persistence test**

In the `#[cfg(test)] mod tests` block of `library.rs`, add:

```rust
    #[test]
    fn config_persists_language() {
        let cfg = Config {
            default_repo_url: "https://example.com/hymns.git".into(),
            libraries: vec![],
            download_dir: None,
            language: Some("ro".into()),
        };
        let back = Config::from_toml(&cfg.to_toml().unwrap()).unwrap();
        assert_eq!(back.language, Some("ro".into()));
    }
```

Note: the other tests in this file construct `Config { … }` literally — add
`language: None,` to each existing literal (`config_toml_round_trips`,
`config_persists_download_dir`) so they still compile.

- [ ] **Step 3: Run tests, expect PASS**

Run: `cargo test -p hymnal-core library`
Expected: all `library::tests` pass, including `config_persists_language`.

- [ ] **Step 4: Commit**

```bash
git add crates/hymnal-core/src/library.rs
git commit -m "feat(core): persist selected language in Config"
```

---

## Task 4: Add `sys-locale` dependency

**Files:**
- Modify: `crates/hymnal-gui/Cargo.toml`

- [ ] **Step 1: Add the dep**

In `crates/hymnal-gui/Cargo.toml`, under `[dependencies]`:

```toml
sys-locale = "0.3"
```

- [ ] **Step 2: Verify it resolves**

Run: `cargo build -p hymnal-gui`
Expected: builds (downloads `sys-locale`); no code uses it yet, so no behavior change.

- [ ] **Step 3: Commit**

```bash
git add crates/hymnal-gui/Cargo.toml Cargo.lock
git commit -m "build(gui): add sys-locale for OS locale detection"
```

Note: `Cargo.lock` is git-ignored in this repo; `git add Cargo.lock` is a no-op if so — that's fine, drop it from the command.

---

## Task 5: Add `I18n` global to Slint and bind all literals

**Files:**
- Modify: `crates/hymnal-gui/ui/app.slint`

- [ ] **Step 1: Declare the global**

At the top of `app.slint`, after the `DownloadState` struct (before `global Theme`), add:

```slint
export global I18n {
    in property <string> app-title: "SDA Manager";
    in property <string> nav-library: "Library";
    in property <string> nav-downloader: "Video Downloader";
    in property <string> nav-settings: "Settings";
    in property <string> search-placeholder: "Search by number, title, or lyrics…";
    in property <string> prev-slide: "‹ Prev";
    in property <string> next-slide: "Next ›";
    in property <string> open-in-powerpoint: "Open in PowerPoint";
    in property <string> reveal-in-folder: "Reveal in folder";
    in property <string> downloader-heading: "Video Downloader";
    in property <string> downloader-subtitle: "Paste a YouTube link and choose where to save it.";
    in property <string> downloader-url-placeholder: "Paste a YouTube URL…";
    in property <string> choose: "Choose…";
    in property <string> setting-up-downloader: "Setting up downloader…";
    in property <string> download-complete: "✓ Download complete";
    in property <string> download-failed-prefix: "Download failed: ";
    in property <string> settings-heading: "Settings";
    in property <string> version-prefix: "Version ";
    in property <string> library-heading: "Library";
    in property <string> force-sync-description: "Force sync deletes the local hymn library and cache, then re-downloads and reindexes everything.";
    in property <string> force-sync-button: "Force sync library";
    in property <string> syncing-button: "Syncing…";
    in property <string> language-heading: "Language";
}
```

(Defaults mirror English so the design previews sensibly before Rust sets them.)

- [ ] **Step 2: Replace literals with `I18n.<field>`**

Edit each existing literal in `app.slint` (line numbers approximate — match on text):

| Current text | Replace with |
|---|---|
| `text: "SDA MANAGER";` (sidebar) | `text: I18n.app-title;` |
| `label: "Library";` | `label: I18n.nav-library;` |
| `label: "Video Downloader";` | `label: I18n.nav-downloader;` |
| `label: "Settings";` | `label: I18n.nav-settings;` |
| `placeholder-text: "Search by number, title, or lyrics…";` | `placeholder-text: I18n.search-placeholder;` |
| `text: "‹ Prev";` | `text: I18n.prev-slide;` |
| `text: "Next ›";` | `text: I18n.next-slide;` |
| `text: "Open in PowerPoint";` | `text: I18n.open-in-powerpoint;` |
| `text: "Reveal in folder";` (both occurrences) | `text: I18n.reveal-in-folder;` |
| `text: "Video Downloader";` (heading) | `text: I18n.downloader-heading;` |
| `text: "Paste a YouTube link and choose where to save it.";` | `text: I18n.downloader-subtitle;` |
| `placeholder-text: "Paste a YouTube URL…";` | `placeholder-text: I18n.downloader-url-placeholder;` |
| `text: "Choose…";` | `text: I18n.choose;` |
| `text: "Setting up downloader…";` | `text: I18n.setting-up-downloader;` |
| `text: "✓ Download complete";` | `text: I18n.download-complete;` |
| `text: "Download failed: " + root.state.message;` | `text: I18n.download-failed-prefix + root.state.message;` |
| `text: "Settings";` (heading) | `text: I18n.settings-heading;` |
| `text: "Version " + root.app-version;` | `text: I18n.version-prefix + root.app-version;` |
| `text: "Library";` (settings) | `text: I18n.library-heading;` |
| `text: "Force sync deletes…everything.";` | `text: I18n.force-sync-description;` |
| `text: root.syncing ? "Syncing…" : "Force sync library";` | `text: root.syncing ? I18n.syncing-button : I18n.force-sync-button;` |
| `title: "SDA Manager";` (Window) | `title: I18n.app-title;` |

- [ ] **Step 3: Build to verify the Slint compiles**

Run: `cargo build -p hymnal-gui`
Expected: compiles. (App still shows English — Rust doesn't set the global yet.)

- [ ] **Step 4: Commit**

```bash
git add crates/hymnal-gui/ui/app.slint
git commit -m "feat(gui): route UI strings through I18n global"
```

---

## Task 6: Language picker in Settings + `set-language` callback

**Files:**
- Modify: `crates/hymnal-gui/ui/app.slint`

- [ ] **Step 1: Add picker properties/callback to SettingsPanel**

In `component SettingsPanel`, add alongside its existing `in property`s:

```slint
    in property <string> active-language;   // "en"/"it"/"ro"
    callback set-language(string);
```

- [ ] **Step 2: Add the Language section UI**

Inside `SettingsPanel`'s `VerticalBox`, after the Library card `Rectangle { … }`, add a new card. The three buttons are built inline (Slint `for` over a model):

```slint
        Rectangle {
            background: Theme.panel;
            border-radius: Theme.radius;
            border-width: 1px;
            border-color: Theme.panel-border;
            VerticalBox {
                padding: 14px;
                spacing: 8px;
                Text {
                    text: I18n.language-heading;
                    color: Theme.text;
                    font-weight: 600;
                }
                HorizontalBox {
                    spacing: 8px;
                    alignment: start;
                    for lang in [
                        { code: "en", label: "English" },
                        { code: "it", label: "Italiano" },
                        { code: "ro", label: "Română" },
                    ]: Rectangle {
                        height: 34px;
                        width: 110px;
                        border-radius: Theme.radius;
                        background: root.active-language == lang.code ? Theme.accent : Theme.field;
                        Text {
                            text: lang.label;
                            color: white;
                            vertical-alignment: center;
                            horizontal-alignment: center;
                        }
                        TouchArea {
                            clicked => { root.set-language(lang.code); }
                        }
                    }
                }
            }
        }
```

- [ ] **Step 3: Forward the property/callback from AppWindow**

In `AppWindow`, add to the property/callback list:

```slint
    in property <string> active-language;
    callback set-language(string);
```

And in the `if root.active-tab == 2: SettingsPanel { … }` block, add:

```slint
            active-language: root.active-language;
            set-language(code) => { root.set-language(code); }
```

- [ ] **Step 4: Build to verify**

Run: `cargo build -p hymnal-gui`
Expected: compiles. (Clicking does nothing yet — wired in Task 7.)

- [ ] **Step 5: Commit**

```bash
git add crates/hymnal-gui/ui/app.slint
git commit -m "feat(gui): language picker UI in Settings tab"
```

---

## Task 7: Wire language into `main.rs` (apply, persist, boot-detect, status)

**Files:**
- Modify: `crates/hymnal-gui/src/main.rs`

- [ ] **Step 1: Imports + `apply_language` helper**

Near the top of `main.rs`, add to the `use` block:

```rust
use hymnal_core::i18n::{Language, Strings};
```

Add this free function above `fn main`:

```rust
/// Fill the Slint `I18n` global from `Strings::for_language(lang)`. Returns the
/// built `Strings` so callers can reuse it for dynamic status messages.
fn apply_language(ui: &AppWindow, lang: Language) -> Strings {
    let s = Strings::for_language(lang);
    let g = ui.global::<I18n>();
    g.set_app_title(s.app_title.clone().into());
    g.set_nav_library(s.nav_library.clone().into());
    g.set_nav_downloader(s.nav_downloader.clone().into());
    g.set_nav_settings(s.nav_settings.clone().into());
    g.set_search_placeholder(s.search_placeholder.clone().into());
    g.set_prev_slide(s.prev_slide.clone().into());
    g.set_next_slide(s.next_slide.clone().into());
    g.set_open_in_powerpoint(s.open_in_powerpoint.clone().into());
    g.set_reveal_in_folder(s.reveal_in_folder.clone().into());
    g.set_downloader_heading(s.downloader_heading.clone().into());
    g.set_downloader_subtitle(s.downloader_subtitle.clone().into());
    g.set_downloader_url_placeholder(s.downloader_url_placeholder.clone().into());
    g.set_choose(s.choose.clone().into());
    g.set_setting_up_downloader(s.setting_up_downloader.clone().into());
    g.set_download_complete(s.download_complete.clone().into());
    g.set_download_failed_prefix(s.download_failed_prefix.clone().into());
    g.set_settings_heading(s.settings_heading.clone().into());
    g.set_version_prefix(s.version_prefix.clone().into());
    g.set_library_heading(s.library_heading.clone().into());
    g.set_force_sync_description(s.force_sync_description.clone().into());
    g.set_force_sync_button(s.force_sync_button.clone().into());
    g.set_syncing_button(s.syncing_button.clone().into());
    g.set_language_heading(s.language_heading.clone().into());
    s
}
```

`I18n` is generated by `slint::include_modules!()` because the global is
`export`ed; no extra import beyond what `include_modules!` already brings in.

- [ ] **Step 2: Resolve boot language and store shared `Strings`**

After `let ui = AppWindow::new()?;` and after `dl_cfg` is loaded, add:

```rust
    // Resolve language: saved choice, else OS locale, else English.
    let boot_lang = dl_cfg
        .borrow()
        .language
        .as_deref()
        .map(Language::from_code)
        .unwrap_or_else(|| {
            sys_locale::get_locale()
                .map(|l| Language::from_locale(&l))
                .unwrap_or(Language::En)
        });
    let strings = Rc::new(std::cell::RefCell::new(apply_language(&ui, boot_lang)));
    ui.set_active_language(boot_lang.code().into());
```

- [ ] **Step 3: Replace hardcoded status strings with the active `Strings`**

Replace these literals (search for each) with reads from `strings`. For the
plain ones set on the UI thread, e.g.:

```rust
// was: ui.set_update_status("Checking for updates…".into());
ui.set_update_status(strings.borrow().update_checking.clone().into());
// was: ui.set_status("Loading hymn library…".into());
ui.set_status(strings.borrow().status_loading.clone().into());
```

For the worker-thread messages (inside `upgrade_in_event_loop`/timer where
`strings` must cross threads), clone the needed `String`s **before** spawning,
and move them in. Concretely, before the boot worker `std::thread::spawn`, add:

```rust
    let s_ready = strings.borrow().status_library_ready.clone();
    let s_uptodate = strings.borrow().update_up_to_date.clone();
    let s_updatefail = strings.borrow().update_failed.clone();
    let s_staged = strings.borrow().update_staged_fmt.clone();
```

Then inside the worker, replace:
- `ui.set_status("Library ready.".into())` → `ui.set_status(s_ready.clone().into())` (clone per closure use)
- `"Up to date."` → `s_uptodate.clone().into()`
- `"Update check failed."` → `s_updatefail.clone().into()`
- `format!("Update {version} staged — restart to apply.")` → `s_staged.replace("{}", &version).into()`

For the timer/force-sync block (runs on UI thread, `strings` in scope via
clone into the timer closure — clone `let strings_timer = strings.clone();`
before `timer.start`):
- `format!("Synced — indexed {n} hymns.")` → `strings_timer.borrow().status_synced_fmt.replace("{}", &n.to_string()).into()`
- `format!("Sync failed: {e}")` → `strings_timer.borrow().status_sync_failed_fmt.replace("{}", &e).into()`
- `"Re-cloning and reindexing…"` (in `on_force_sync`) → `strings.borrow().status_re_cloning.clone().into()`

For `show_slide` (free fn), pass the two format templates in. Change its
signature to accept them, or simpler: read from a `&Strings`. Update
`show_slide` to take `strings: &Strings` and build:
- zero case: `strings.slide_zero_fmt.replace("{}", title)`
- counter: a small helper that fills 3 placeholders in order:

```rust
fn fill3(fmt: &str, a: &str, b: &str, c: &str) -> String {
    fmt.replacen("{}", a, 1).replacen("{}", b, 1).replacen("{}", c, 1)
}
```

Call sites of `show_slide` already have access to `strings` (clone the `Rc`
into those callback closures, like the other shared state).

- [ ] **Step 4: Wire `on_set_language`**

After the other `ui.on_*` wiring, add:

```rust
    let weak_lang = ui.as_weak();
    let strings_lang = strings.clone();
    let cfg_lang = dl_cfg.clone();
    let cfg_path_lang = cfg_path.clone();
    ui.on_set_language(move |code| {
        let Some(ui) = weak_lang.upgrade() else { return };
        let lang = Language::from_code(&code);
        *strings_lang.borrow_mut() = apply_language(&ui, lang);
        ui.set_active_language(lang.code().into());
        // Persist; log on failure (matches force-sync error handling).
        cfg_lang.borrow_mut().language = Some(lang.code().to_string());
        if let Some(p) = cfg_path_lang.as_ref() {
            if let Err(e) = cfg_lang.borrow().save(p) {
                warn!("failed to save language: {e}");
            }
        }
    });
```

- [ ] **Step 5: Build and run**

Run: `cargo build -p hymnal-gui`
Expected: compiles with no errors.

Run: `cargo run -p hymnal-gui`
Expected: app launches; Settings tab shows a Language section with English/Italiano/Română; clicking Italiano or Română immediately re-labels the nav, search placeholder, buttons, and Settings text; relaunching keeps the chosen language.

- [ ] **Step 6: Commit**

```bash
git add crates/hymnal-gui/src/main.rs
git commit -m "feat(gui): live language switching wired to I18n + Config"
```

---

## Task 8: Update docs

**Files:**
- Modify: `CLAUDE.md`
- Modify: `README.md`

- [ ] **Step 1: CLAUDE.md**

In the "Conventions / non-obvious decisions" list, add a bullet:

```markdown
- **Translations** (`i18n.rs`): `Language` (En/It/Ro) selects a `Strings` struct
  (one field per UI string). `main.rs::apply_language` maps it onto the Slint
  `export global I18n` and reuses it for dynamic status messages. The choice
  persists in `Config.language`; first run detects the OS locale (`sys-locale`),
  falling back to English. Adding a UI string is a compile error until all three
  languages supply it. Switch is live (no restart).
```

- [ ] **Step 2: README.md**

In the feature list (the Library/Search bullets area), add:

```markdown
- **Languages:** the app UI is available in English, Italian, and Romanian,
  selectable in the Settings tab (auto-detected from your OS on first run).
```

- [ ] **Step 3: Commit**

```bash
git add CLAUDE.md README.md
git commit -m "docs: document translations feature"
```

---

## Self-review notes

- **Spec coverage:** Language enum + locale mapping (T1), Strings + en/it/ro (T2), Config persistence (T3), sys-locale (T4), I18n global + literal binding (T5), Settings picker (T6), apply/persist/boot-detect/status (T7), docs (T8). All spec sections covered, including dynamic status strings (T7 step 3) and the completeness test (T2).
- **Type consistency:** `Strings` field names (snake_case in Rust) map to Slint `I18n` kebab-case properties, and the generated Rust setters are snake_case (`set_search_placeholder`) — consistent across T2/T5/T7. `Language::code`/`from_code` used identically in T1/T3/T7.
- **Known follow-up for the implementer:** T7 step 3 touches several existing call sites; if any worker closure needs a `String` it doesn't yet capture, clone it before the `spawn`/closure as shown. Verify with `cargo run`, not just `cargo build`, since focus/threading issues don't surface at compile time.
```
