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

/// Sort key for a hymn number that orders numerically with a letter suffix as
/// the tiebreaker: "664" < "664a" < "664b" < "665". Numbers come before any
/// non-numeric/`None` value. Returns `(numeric, suffix)`; unparseable or `None`
/// sort last via `u32::MAX`.
fn number_sort_key(number: Option<&str>) -> (u32, String) {
    match number {
        Some(s) => {
            let digits: String = s.chars().take_while(|c| c.is_ascii_digit()).collect();
            let suffix = s[digits.len()..].to_string();
            (digits.parse::<u32>().unwrap_or(u32::MAX), suffix)
        }
        None => (u32::MAX, String::new()),
    }
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
                    number_str: entry.number.clone().unwrap_or_default(),
                    title_folded: fold(&entry.title),
                    filename_folded: fold(&filename),
                    body_folded: fold(&entry.body),
                    entry,
                }
            })
            .collect();
        Searcher { items }
    }

    /// Number of indexed hymns.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// True when no hymns are indexed.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Look up an entry by the `index` carried on a [`SearchHit`].
    pub fn entry(&self, index: usize) -> Option<&HymnEntry> {
        self.items.get(index).map(|it| &it.entry)
    }

    /// Rank entries against `query`. Empty query returns all entries sorted by
    /// hymn number. Otherwise fuzzy-matches across number, title, filename and
    /// body, keeping the best-scoring field per entry.
    ///
    /// Hits borrow their entry from the searcher (no body cloning per keystroke).
    pub fn search(&self, query: &str) -> Vec<SearchHit<'_>> {
        let q = fold(query);
        if q.trim().is_empty() {
            let mut all: Vec<SearchHit<'_>> = self
                .items
                .iter()
                .enumerate()
                .map(|(index, it)| SearchHit {
                    index,
                    entry: &it.entry,
                    score: 0,
                    field: MatchField::Number,
                })
                .collect();
            all.sort_by(|a, b| {
                number_sort_key(a.entry.number.as_deref())
                    .cmp(&number_sort_key(b.entry.number.as_deref()))
            });
            return all;
        }

        let mut matcher = Matcher::new(Config::DEFAULT);
        let pattern = Pattern::parse(&q, CaseMatching::Ignore, Normalization::Smart);

        let mut hits: Vec<SearchHit<'_>> = Vec::new();
        for (index, it) in self.items.iter().enumerate() {
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
                    index,
                    entry: &it.entry,
                    score,
                    field,
                });
            }
        }
        hits.sort_by(|a, b| b.score.cmp(&a.score));
        hits
    }
}
