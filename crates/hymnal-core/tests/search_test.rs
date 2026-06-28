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
    assert_eq!(hits[0].entry.number.as_deref(), Some("1"));
}

#[test]
fn matches_by_number() {
    let s = searcher();
    let hits = s.search("150");
    assert_eq!(hits[0].entry.number.as_deref(), Some("150"));
}

#[test]
fn matches_body_text() {
    let s = searcher();
    let hits = s.search("Popoare");
    assert!(hits.iter().any(|h| h.entry.number.as_deref() == Some("1")));
}

#[test]
fn empty_query_returns_all_sorted_by_number() {
    let s = searcher();
    let hits = s.search("");
    assert!(hits.len() >= 2);
    assert_eq!(hits[0].entry.number.as_deref(), Some("1"));
}

#[test]
fn lettered_number_sorts_numerically_then_by_suffix() {
    use hymnal_core::model::{HymnEntry, MatchField};
    let mk = |n: &str| HymnEntry {
        number: Some(n.into()),
        title: format!("hymn {n}"),
        body: String::new(),
        slides: vec![],
        path: std::path::PathBuf::from(format!("/x/{n}.pptx")),
        library: "L".into(),
        mtime: 0,
    };
    // Deliberately unsorted, with a letter-suffixed number among integers.
    let s = Searcher::new(vec![mk("665"), mk("664b"), mk("70"), mk("664"), mk("664a")]);
    let order: Vec<String> = s
        .search("")
        .into_iter()
        .map(|h| h.entry.number.clone().unwrap())
        .collect();
    assert_eq!(order, vec!["70", "664", "664a", "664b", "665"]);
    // Sanity: an empty-query hit reports the Number field.
    assert_eq!(s.search("").first().unwrap().field, MatchField::Number);
}
