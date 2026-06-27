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
