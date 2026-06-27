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
