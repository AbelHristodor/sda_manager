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
