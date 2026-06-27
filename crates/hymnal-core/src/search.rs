use crate::fold::fold;
use crate::model::{HymnEntry, MatchField, SearchHit};
use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};

/// Pre-folded searchable text for one entry, kept alongside the entry.
struct Indexed {
    entry: HymnEntry,
    number_str: String,
    title_folded: String,
    filename_folded: String,
    body_folded: String,
}

/// In-memory fuzzy searcher over hymn entries.
pub struct Searcher {
    items: Vec<Indexed>,
}

impl Searcher {
    pub fn new(entries: Vec<HymnEntry>) -> Self {
        let items = entries
            .into_iter()
            .map(|entry| {
                let filename = entry
                    .path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string();
                Indexed {
                    number_str: entry.number.map(|n| n.to_string()).unwrap_or_default(),
                    title_folded: fold(&entry.title),
                    filename_folded: fold(&filename),
                    body_folded: fold(&entry.body),
                    entry,
                }
            })
            .collect();
        Searcher { items }
    }

    /// Rank entries against `query`. Empty query returns all entries sorted by
    /// hymn number. Otherwise fuzzy-matches across number, title, filename and
    /// body, keeping the best-scoring field per entry.
    pub fn search(&self, query: &str) -> Vec<SearchHit> {
        let q = fold(query);
        if q.trim().is_empty() {
            let mut all: Vec<SearchHit> = self
                .items
                .iter()
                .map(|it| SearchHit {
                    entry: it.entry.clone(),
                    score: 0,
                    field: MatchField::Number,
                })
                .collect();
            all.sort_by_key(|h| h.entry.number.unwrap_or(u32::MAX));
            return all;
        }

        let mut matcher = Matcher::new(Config::DEFAULT);
        let pattern = Pattern::parse(&q, CaseMatching::Ignore, Normalization::Smart);

        let mut hits: Vec<SearchHit> = Vec::new();
        for it in &self.items {
            // Field weight nudges ties toward more authoritative fields.
            let candidates = [
                (MatchField::Number, &it.number_str, 4u32),
                (MatchField::Title, &it.title_folded, 3),
                (MatchField::Filename, &it.filename_folded, 2),
                (MatchField::Body, &it.body_folded, 1),
            ];
            let mut best: Option<(MatchField, u32)> = None;
            for (field, haystack, weight) in candidates {
                if haystack.is_empty() {
                    continue;
                }
                let mut buf = Vec::new();
                let hay = Utf32Str::new(haystack, &mut buf);
                if let Some(raw) = pattern.score(hay, &mut matcher) {
                    // Strong bonuses so exact/prefix matches dominate fuzzy ones.
                    // An exact field equality (e.g. number "150" == "150") should
                    // always outrank a substring/fuzzy hit in some other field.
                    let bonus = if *haystack == q {
                        20_000
                    } else if haystack.starts_with(&q) {
                        10_000
                    } else if haystack.contains(&q) {
                        5_000
                    } else {
                        0
                    };
                    let weighted = raw * 10 + bonus + weight;
                    if best.is_none_or(|(_, b)| weighted > b) {
                        best = Some((field, weighted));
                    }
                }
            }
            if let Some((field, score)) = best {
                hits.push(SearchHit {
                    entry: it.entry.clone(),
                    score,
                    field,
                });
            }
        }
        hits.sort_by(|a, b| b.score.cmp(&a.score));
        hits
    }
}
