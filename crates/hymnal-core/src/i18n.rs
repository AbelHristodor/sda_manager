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
    pub user_libraries_heading: String,
    pub add_folder_button: String,
    pub user_libraries_description: String,
    pub library_unavailable_suffix: String,
    pub nav_themes: String,
    pub themes_heading: String,
    pub theme_new: String,
    pub theme_save: String,
    pub theme_delete: String,
    pub theme_preview_sample: String,
    pub theme_font_label: String,
    pub theme_size_label: String,
    pub theme_weight_label: String,
    pub theme_align_label: String,
    pub theme_text_color_label: String,
    pub theme_bg_color_label: String,
    pub theme_set_active: String,
    pub theme_name_label: String,
    pub nav_control: String,
    pub control_heading: String,
    pub control_start: String,
    pub control_stop: String,
    pub control_blank: String,
    pub control_prev: String,
    pub control_next: String,
    pub control_next_label: String,
    pub control_live: String,
    pub control_output: String,
    pub control_theme: String,
    pub library_project: String,
    // Dynamic status (main.rs). `{}` placeholders filled with format!.
    pub status_loading: String,
    pub status_library_ready: String,
    pub status_re_cloning: String,
    pub update_checking: String,
    pub update_up_to_date: String,
    pub update_failed: String,
    pub update_unavailable: String,
    pub status_synced_fmt: String,      // one `{}` = hymn count
    pub status_sync_failed_fmt: String, // one `{}` = error
    pub update_staged_fmt: String,      // one `{}` = version
    pub slide_counter_fmt: String,      // "{} — slide {}/{}" = title, idx, count
    pub slide_zero_fmt: String,         // "{} — 0 slides" = title
    pub status_indexing: String,        // "Indexing…"
    pub status_indexed_fmt: String,     // one `{}` = hymn count
    pub status_indexing_failed_fmt: String, // one `{}` = error
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
            &self.user_libraries_heading,
            &self.add_folder_button,
            &self.user_libraries_description,
            &self.library_unavailable_suffix,
            &self.nav_themes,
            &self.themes_heading,
            &self.theme_new,
            &self.theme_save,
            &self.theme_delete,
            &self.theme_preview_sample,
            &self.theme_font_label,
            &self.theme_size_label,
            &self.theme_weight_label,
            &self.theme_align_label,
            &self.theme_text_color_label,
            &self.theme_bg_color_label,
            &self.theme_set_active,
            &self.theme_name_label,
            &self.nav_control,
            &self.control_heading,
            &self.control_start,
            &self.control_stop,
            &self.control_blank,
            &self.control_prev,
            &self.control_next,
            &self.control_next_label,
            &self.control_live,
            &self.control_output,
            &self.control_theme,
            &self.library_project,
            &self.status_loading,
            &self.status_library_ready,
            &self.status_re_cloning,
            &self.update_checking,
            &self.update_up_to_date,
            &self.update_failed,
            &self.update_unavailable,
            &self.status_synced_fmt,
            &self.status_sync_failed_fmt,
            &self.update_staged_fmt,
            &self.slide_counter_fmt,
            &self.slide_zero_fmt,
            &self.status_indexing,
            &self.status_indexed_fmt,
            &self.status_indexing_failed_fmt,
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
            user_libraries_heading: "Your library folders".into(),
            add_folder_button: "Add folder…".into(),
            user_libraries_description: "Add your own folders of .pptx hymns. They're searched alongside the built-in library.".into(),
            library_unavailable_suffix: "  (unavailable)".into(),
            nav_themes: "Themes".into(),
            themes_heading: "Themes".into(),
            theme_new: "New theme".into(),
            theme_save: "Save".into(),
            theme_delete: "Delete".into(),
            theme_preview_sample: "Plecaţi-vă lui Dumnezeu".into(),
            theme_font_label: "Font".into(),
            theme_size_label: "Size".into(),
            theme_weight_label: "Weight".into(),
            theme_align_label: "Align".into(),
            theme_text_color_label: "Text color".into(),
            theme_bg_color_label: "Background".into(),
            theme_set_active: "Set as active".into(),
            theme_name_label: "Name".into(),
            nav_control: "Control".into(),
            control_heading: "Control".into(),
            control_start: "▶ Start projecting".into(),
            control_stop: "■ Stop".into(),
            control_blank: "Blank (B)".into(),
            control_prev: "◀ Prev".into(),
            control_next: "Next ▶".into(),
            control_next_label: "NEXT".into(),
            control_live: "LIVE".into(),
            control_output: "Output".into(),
            control_theme: "Theme".into(),
            library_project: "Project".into(),
            status_loading: "Loading hymn library…".into(),
            status_library_ready: "Library ready.".into(),
            status_re_cloning: "Re-cloning and reindexing…".into(),
            update_checking: "Checking for updates…".into(),
            update_up_to_date: "Up to date.".into(),
            update_failed: "Update check failed.".into(),
            update_unavailable: "Couldn't check for updates — try later.".into(),
            status_synced_fmt: "Synced — indexed {} hymns.".into(),
            status_sync_failed_fmt: "Sync failed: {}".into(),
            update_staged_fmt: "Update {} staged — restart to apply.".into(),
            slide_counter_fmt: "{} — slide {}/{}".into(),
            slide_zero_fmt: "{} — 0 slides".into(),
            status_indexing: "Indexing…".into(),
            status_indexed_fmt: "Indexed {} hymns.".into(),
            status_indexing_failed_fmt: "Indexing failed: {}".into(),
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
            user_libraries_heading: "Le tue cartelle".into(),
            add_folder_button: "Aggiungi cartella…".into(),
            user_libraries_description: "Aggiungi le tue cartelle di inni .pptx. Vengono cercate insieme all'innario integrato.".into(),
            library_unavailable_suffix: "  (non disponibile)".into(),
            nav_themes: "Temi".into(),
            themes_heading: "Temi".into(),
            theme_new: "Nuovo tema".into(),
            theme_save: "Salva".into(),
            theme_delete: "Elimina".into(),
            theme_preview_sample: "Plecaţi-vă lui Dumnezeu".into(),
            theme_font_label: "Carattere".into(),
            theme_size_label: "Dimensione".into(),
            theme_weight_label: "Spessore".into(),
            theme_align_label: "Allinea".into(),
            theme_text_color_label: "Colore testo".into(),
            theme_bg_color_label: "Sfondo".into(),
            theme_set_active: "Imposta come attivo".into(),
            theme_name_label: "Nome".into(),
            nav_control: "Controllo".into(),
            control_heading: "Controllo".into(),
            control_start: "▶ Avvia proiezione".into(),
            control_stop: "■ Ferma".into(),
            control_blank: "Schermo nero (B)".into(),
            control_prev: "◀ Prec".into(),
            control_next: "Succ ▶".into(),
            control_next_label: "PROSSIMO".into(),
            control_live: "IN ONDA".into(),
            control_output: "Uscita".into(),
            control_theme: "Tema".into(),
            library_project: "Proietta".into(),
            status_loading: "Caricamento dell'innario…".into(),
            status_library_ready: "Innario pronto.".into(),
            status_re_cloning: "Riscaricamento e reindicizzazione…".into(),
            update_checking: "Ricerca aggiornamenti…".into(),
            update_up_to_date: "Aggiornato.".into(),
            update_failed: "Controllo aggiornamenti non riuscito.".into(),
            update_unavailable: "Impossibile controllare gli aggiornamenti — riprova più tardi.".into(),
            status_synced_fmt: "Sincronizzato — {} inni indicizzati.".into(),
            status_sync_failed_fmt: "Sincronizzazione non riuscita: {}".into(),
            update_staged_fmt: "Aggiornamento {} pronto — riavvia per applicarlo.".into(),
            slide_counter_fmt: "{} — diapositiva {}/{}".into(),
            slide_zero_fmt: "{} — 0 diapositive".into(),
            status_indexing: "Indicizzazione…".into(),
            status_indexed_fmt: "Indicizzati {} inni.".into(),
            status_indexing_failed_fmt: "Indicizzazione non riuscita: {}".into(),
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
            user_libraries_heading: "Folderele tale".into(),
            add_folder_button: "Adaugă folder…".into(),
            user_libraries_description: "Adaugă propriile foldere cu imnuri .pptx. Sunt căutate împreună cu imnarul integrat.".into(),
            library_unavailable_suffix: "  (indisponibil)".into(),
            nav_themes: "Teme".into(),
            themes_heading: "Teme".into(),
            theme_new: "Temă nouă".into(),
            theme_save: "Salvează".into(),
            theme_delete: "Șterge".into(),
            theme_preview_sample: "Plecaţi-vă lui Dumnezeu".into(),
            theme_font_label: "Font".into(),
            theme_size_label: "Dimensiune".into(),
            theme_weight_label: "Grosime".into(),
            theme_align_label: "Aliniere".into(),
            theme_text_color_label: "Culoare text".into(),
            theme_bg_color_label: "Fundal".into(),
            theme_set_active: "Setează ca activ".into(),
            theme_name_label: "Nume".into(),
            nav_control: "Control".into(),
            control_heading: "Control".into(),
            control_start: "▶ Începe proiecția".into(),
            control_stop: "■ Oprește".into(),
            control_blank: "Ecran negru (B)".into(),
            control_prev: "◀ Înapoi".into(),
            control_next: "Înainte ▶".into(),
            control_next_label: "URMĂTORUL".into(),
            control_live: "ÎN DIRECT".into(),
            control_output: "Ieșire".into(),
            control_theme: "Temă".into(),
            library_project: "Proiectează".into(),
            status_loading: "Se încarcă imnarul…".into(),
            status_library_ready: "Imnar pregătit.".into(),
            status_re_cloning: "Se reclonează și se reindexează…".into(),
            update_checking: "Se caută actualizări…".into(),
            update_up_to_date: "La zi.".into(),
            update_failed: "Verificarea actualizărilor a eșuat.".into(),
            update_unavailable: "Nu s-au putut verifica actualizările — încearcă mai târziu.".into(),
            status_synced_fmt: "Sincronizat — {} imnuri indexate.".into(),
            status_sync_failed_fmt: "Sincronizare eșuată: {}".into(),
            update_staged_fmt: "Actualizarea {} este pregătită — repornește pentru a o aplica.".into(),
            slide_counter_fmt: "{} — diapozitivul {}/{}".into(),
            slide_zero_fmt: "{} — 0 diapozitive".into(),
            status_indexing: "Se indexează…".into(),
            status_indexed_fmt: "{} imnuri indexate.".into(),
            status_indexing_failed_fmt: "Indexare eșuată: {}".into(),
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
