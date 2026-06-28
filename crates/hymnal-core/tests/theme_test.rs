use hymnal_core::theme::{Background, Theme};

#[test]
fn theme_json_round_trips() {
    let t = Theme::default();
    let json = t.to_json().unwrap();
    let back = Theme::from_json(&json).unwrap();
    assert_eq!(back, t);
}

#[test]
fn default_theme_has_name_and_dark_background() {
    let t = Theme::default();
    assert_eq!(t.name, "Default");
    assert!(matches!(t.background.kind, Background::Solid { .. }));
}

#[test]
fn custom_theme_round_trips_all_fields() {
    let mut t = Theme::default();
    t.name = "Christmas".into();
    t.text.font_family = "Georgia".into();
    t.text.font_size_pt = Some(48.0);
    t.text.font_weight = 700;
    t.background.kind = Background::Image { path: "/tmp/bg.jpg".into() };
    t.background.overlay_opacity = 0.4;
    let back = Theme::from_json(&t.to_json().unwrap()).unwrap();
    assert_eq!(back, t);
}
