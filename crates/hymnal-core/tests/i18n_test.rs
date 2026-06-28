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
    // Guard: placeholders must survive translation in all languages.
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
