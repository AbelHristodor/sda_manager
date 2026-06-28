use hymnal_core::theme::store::{delete_theme, list_themes, load_theme, save_theme};
use hymnal_core::theme::Theme;

#[test]
fn save_then_list_and_load_includes_default_and_custom() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let names: Vec<String> = list_themes(root).iter().map(|t| t.name.clone()).collect();
    assert!(names.contains(&"Default".to_string()));
    let mut t = Theme::default();
    t.name = "Christmas".into();
    save_theme(root, &t).unwrap();
    let names: Vec<String> = list_themes(root).iter().map(|t| t.name.clone()).collect();
    assert!(names.contains(&"Christmas".to_string()));
    assert!(names.contains(&"Default".to_string()));
    let loaded = load_theme(root, "Christmas").unwrap();
    assert_eq!(loaded.name, "Christmas");
}

#[test]
fn corrupt_theme_file_is_skipped_default_remains() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir_all(root).unwrap();
    std::fs::write(root.join("broken.json"), b"{ not valid json").unwrap();
    let names: Vec<String> = list_themes(root).iter().map(|t| t.name.clone()).collect();
    assert!(names.contains(&"Default".to_string()));
    assert!(!names.contains(&"broken".to_string()));
}

#[test]
fn cannot_delete_builtin_default() {
    let dir = tempfile::tempdir().unwrap();
    assert!(delete_theme(dir.path(), "Default").is_err());
}

#[test]
fn delete_removes_custom_theme() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let mut t = Theme::default();
    t.name = "Temp".into();
    save_theme(root, &t).unwrap();
    delete_theme(root, "Temp").unwrap();
    let names: Vec<String> = list_themes(root).iter().map(|t| t.name.clone()).collect();
    assert!(!names.contains(&"Temp".to_string()));
}
