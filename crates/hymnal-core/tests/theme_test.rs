use hymnal_core::theme::{Background, Rgba, Theme};

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

#[test]
fn rgba_hex_round_trips() {
    let c = Rgba::new(11, 31, 58, 255);
    assert_eq!(c.to_hex(), "#0B1F3A");
    assert_eq!(Rgba::from_hex("#0B1F3A"), Some(Rgba::new(11, 31, 58, 255)));
}

#[test]
fn rgba_from_hex_accepts_lowercase_and_no_hash() {
    assert_eq!(Rgba::from_hex("ffffff"), Some(Rgba::new(255, 255, 255, 255)));
    assert_eq!(Rgba::from_hex("#abcdef"), Some(Rgba::new(0xab, 0xcd, 0xef, 255)));
}

#[test]
fn rgba_from_hex_rejects_invalid() {
    assert_eq!(Rgba::from_hex("#FF"), None);
    assert_eq!(Rgba::from_hex("#GGGGGG"), None);
    assert_eq!(Rgba::from_hex(""), None);
    assert_eq!(Rgba::from_hex("#1234567"), None);
}
