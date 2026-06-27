use hymnal_core::refresh::force_clean_paths;
use std::fs;

#[test]
fn force_clean_removes_clone_dir_and_cache_file() {
    let tmp = tempfile::tempdir().unwrap();
    let clone = tmp.path().join("default-library");
    let cache = tmp.path().join("index.bin");
    fs::create_dir_all(clone.join(".git")).unwrap();
    fs::write(clone.join("a.pptx"), b"x").unwrap();
    fs::write(&cache, b"cached").unwrap();

    force_clean_paths(Some(&clone), Some(&cache)).unwrap();

    assert!(!clone.exists(), "clone dir should be deleted");
    assert!(!cache.exists(), "cache file should be deleted");
}

#[test]
fn force_clean_is_ok_when_absent() {
    let tmp = tempfile::tempdir().unwrap();
    let clone = tmp.path().join("nope-dir");
    let cache = tmp.path().join("nope.bin");
    force_clean_paths(Some(&clone), Some(&cache)).unwrap();
    force_clean_paths(None, None).unwrap();
}
