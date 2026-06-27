use hymnal_core::pptx::extract;
use std::path::Path;

#[test]
fn extracts_number_title_and_body() {
    let path = Path::new("tests/fixtures/001.pptx");
    let parsed = extract(path).expect("should parse");

    // Number comes from the filename stem.
    assert_eq!(parsed.number, Some(1));
    // Title is the first meaningful line of the title slide.
    assert!(parsed.title.contains("Plecaţi-vă lui Dumnezeu"));
    // Body contains verse text from later slides.
    assert!(parsed.body.contains("Popoare-oriunde"));
    // The "Imnul" marker and counter lines are not the title.
    assert!(!parsed.title.starts_with("Imnul"));
}

#[test]
fn number_from_three_digit_filename() {
    let parsed = extract(Path::new("tests/fixtures/150.pptx")).unwrap();
    assert_eq!(parsed.number, Some(150));
    assert!(parsed.title.contains("Cerul, pământul"));
}

#[test]
fn title_joins_runs_split_within_a_paragraph() {
    // Hymn 356's title is split across four <a:t> runs in one <a:p>:
    // "Ca un " + "cerb" + " setos de " + "ape". The parser must join runs
    // within a paragraph so the full title survives, not just "Ca un".
    let parsed = extract(Path::new("tests/fixtures/356.pptx")).unwrap();
    assert_eq!(parsed.number, Some(356));
    assert_eq!(parsed.title, "Ca un cerb setos de ape");
    assert!(!parsed.title.starts_with("Imnul"));
}

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

#[test]
fn strips_slide_chrome_footer_counter_and_imnul() {
    // Each slide carries chrome that should not appear in the preview/search:
    // the "IMNURI CREȘTINE 2013" footer, a "X/920" (or "X/300") counter, and
    // on the first slide the "Imnul X" marker.
    let parsed = extract(Path::new("tests/fixtures/001.pptx")).unwrap();
    let all = parsed.slides.join("\n");
    assert!(!all.contains("IMNURI CREȘTINE"), "footer must be stripped");
    assert!(!all.contains("Imnul"), "'Imnul X' marker must be stripped");
    // No bare "N/M" counter lines remain.
    assert!(
        !parsed.slides.iter().any(|s| s.lines().any(is_counter_line)),
        "counter lines like '1/300' must be stripped"
    );
    // Real lyrics survive.
    assert!(all.contains("Popoare-oriunde"));
    // The title is still intact on slide 0.
    assert!(parsed.slides[0].contains("Plecaţi-vă lui Dumnezeu"));
}

/// A line that is only digits and slashes, e.g. "1/300" or "150/920".
fn is_counter_line(line: &str) -> bool {
    let l = line.trim();
    !l.is_empty() && l.contains('/') && l.chars().all(|c| c.is_ascii_digit() || c == '/')
}

#[test]
fn strips_standalone_slide_numbers_but_keeps_numbered_lyrics() {
    // Hymn 1's slides carry a standalone slide-number line ("1.", "2.", …).
    let one = extract(Path::new("tests/fixtures/001.pptx")).unwrap();
    for slide in &one.slides {
        for line in slide.lines() {
            let l = line.trim();
            let standalone_number = !l.is_empty()
                && l.trim_end_matches('.').chars().all(|c| c.is_ascii_digit())
                && l.trim_end_matches('.').chars().next().is_some();
            assert!(
                !standalone_number,
                "standalone slide number {l:?} must be stripped"
            );
        }
    }
    // Hymn 356's verse lines START with "1. " followed by lyrics — keep those.
    let h356 = extract(Path::new("tests/fixtures/356.pptx")).unwrap();
    let body = h356.slides.join("\n");
    assert!(
        body.contains("Ca un cerb setos de ape,"),
        "numbered lyric lines must survive"
    );
}
