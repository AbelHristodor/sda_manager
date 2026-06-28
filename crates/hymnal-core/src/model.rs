use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// One hymn extracted from a .pptx file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HymnEntry {
    /// Hymn number parsed from the filename stem (authoritative), e.g. "150"
    /// or "664b" (some hymns have a letter suffix). `None` if unparseable.
    pub number: Option<String>,
    /// Title line (first meaningful text on the title slide).
    pub title: String,
    /// Concatenated verse text from all slides, for full-text search.
    pub body: String,
    /// Text of each slide (paragraphs joined by `\n`), in presentation order.
    pub slides: Vec<String>,
    /// Absolute path to the source .pptx.
    pub path: PathBuf,
    /// Name of the library this hymn belongs to.
    pub library: String,
    /// File modification time (unix seconds) used for cache invalidation.
    pub mtime: i64,
}

/// Which field a search query matched, for display/ranking hints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchField {
    Number,
    Title,
    Filename,
    Body,
}

/// A ranked search result. Borrows the matched entry from the `Searcher` to
/// avoid cloning hymn bodies on every keystroke; `index` identifies the entry
/// within the searcher for later lookup (preview/open).
#[derive(Debug, Clone)]
pub struct SearchHit<'a> {
    pub index: usize,
    pub entry: &'a HymnEntry,
    pub score: u32,
    pub field: MatchField,
}
