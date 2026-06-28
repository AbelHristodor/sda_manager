use hymnal_core::present::PresentationState;

fn slides() -> Vec<String> {
    vec!["title".into(), "verse 1".into(), "verse 2".into()]
}

#[test]
fn load_hymn_resets_to_first_slide_unblanked() {
    let mut p = PresentationState::default();
    p.blank = true;
    p.load_hymn(Some("150".into()), "T".into(), slides());
    assert_eq!(p.index, 0);
    assert!(!p.blank);
    assert_eq!(p.current_slide(), Some("title"));
}

#[test]
fn next_and_prev_clamp_at_bounds() {
    let mut p = PresentationState::default();
    p.load_hymn(None, "T".into(), slides());
    assert_eq!(p.index, 0);
    p.prev();
    assert_eq!(p.index, 0);
    p.next();
    p.next();
    assert_eq!(p.index, 2);
    p.next();
    assert_eq!(p.index, 2);
    p.prev();
    assert_eq!(p.index, 1);
}

#[test]
fn blank_toggles() {
    let mut p = PresentationState::default();
    p.load_hymn(None, "T".into(), slides());
    assert!(!p.blank);
    p.toggle_blank();
    assert!(p.blank);
    p.toggle_blank();
    assert!(!p.blank);
}

#[test]
fn current_slide_none_when_empty() {
    let p = PresentationState::default();
    assert_eq!(p.current_slide(), None);
    assert_eq!(p.slide_count(), 0);
}

#[test]
fn next_slide_peek() {
    let mut p = PresentationState::default();
    p.load_hymn(None, "T".into(), slides());
    assert_eq!(p.next_slide(), Some("verse 1"));
    p.next();
    p.next();
    assert_eq!(p.next_slide(), None);
}
