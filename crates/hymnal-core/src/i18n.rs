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
    pub status_synced_fmt: String,      // one `{}` = hymn count
    pub status_sync_failed_fmt: String, // one `{}` = error
    pub update_staged_fmt: String,      // one `{}` = version
    pub slide_counter_fmt: String,      // "{} — slide {}/{}" = title, idx, count
    pub slide_zero_fmt: String,         // "{} — 0 slides" = title
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
            &self.app_title,
            &self.nav_library,
            &self.nav_downloader,
            &self.nav_settings,
            &self.search_placeholder,
            &self.prev_slide,
            &self.next_slide,
            &self.open_in_powerpoint,
            &self.reveal_in_folder,
            &self.downloader_heading,
            &self.downloader_subtitle,
            &self.downloader_url_placeholder,
            &self.choose,
            &self.setting_up_downloader,
            &self.download_complete,
            &self.download_failed_prefix,
            &self.settings_heading,
            &self.version_prefix,
            &self.library_heading,
            &self.force_sync_description,
            &self.force_sync_button,
            &self.syncing_button,
            &self.language_heading,
            &self.status_loading,
            &self.status_library_ready,
            &self.status_re_cloning,
            &self.update_checking,
            &self.update_up_to_date,
            &self.update_failed,
            &self.status_synced_fmt,
            &self.status_sync_failed_fmt,
            &self.update_staged_fmt,
            &self.slide_counter_fmt,
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
