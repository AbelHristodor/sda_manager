//! System font family enumeration with a curated fallback.

/// Curated families that exist on virtually all systems — used as a fallback
/// when enumeration is unavailable, and merged ahead of enumerated families.
const CURATED: &[&str] = &[
    "sans-serif", "serif", "monospace",
    "Arial", "Helvetica", "Times New Roman", "Georgia", "Verdana",
];

/// Return a sorted, de-duplicated list of font family names. Always begins with
/// the curated families (so common picks are at the top), followed by any
/// additional system families discovered.
pub fn families() -> Vec<String> {
    let mut out: Vec<String> = CURATED.iter().map(|s| s.to_string()).collect();
    out.extend(enumerate_system());
    let mut seen = std::collections::HashSet::new();
    out.retain(|f| seen.insert(f.to_lowercase()));
    out
}

#[cfg(feature = "system-fonts")]
fn enumerate_system() -> Vec<String> {
    use font_kit::source::SystemSource;
    match SystemSource::new().all_families() {
        Ok(mut v) => { v.sort(); v }
        Err(_) => Vec::new(),
    }
}

#[cfg(not(feature = "system-fonts"))]
fn enumerate_system() -> Vec<String> {
    Vec::new()
}
