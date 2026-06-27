use hymnal_core::index::build_index;
use hymnal_core::search::Searcher;
use std::path::Path;

fn searcher() -> Searcher {
    let entries = build_index(Path::new("tests/fixtures"), "test-lib");
    Searcher::new(entries)
}

#[test]
fn matches_without_diacritics() {
    let s = searcher();
    let hits = s.search("plecati");
    assert!(!hits.is_empty());
    assert_eq!(hits[0].entry.number, Some(1));
}

#[test]
fn matches_by_number() {
    let s = searcher();
    let hits = s.search("150");
    assert_eq!(hits[0].entry.number, Some(150));
}

#[test]
fn matches_body_text() {
    let s = searcher();
    let hits = s.search("Popoare");
    assert!(hits.iter().any(|h| h.entry.number == Some(1)));
}

#[test]
fn empty_query_returns_all_sorted_by_number() {
    let s = searcher();
    let hits = s.search("");
    assert!(hits.len() >= 2);
    assert_eq!(hits[0].entry.number, Some(1));
}
