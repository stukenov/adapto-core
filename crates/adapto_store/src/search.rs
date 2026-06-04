//! In-memory full-text search engine: inverted index + BM25F ranking + prefix suggest.
//!
//! Two query paths share one index:
//! - [`SearchIndex::suggest`] — tier-1 prefix completion over title/keywords, for typeahead widgets.
//! - [`SearchIndex::search`] — tier-2 BM25F relevance ranking over all fields, for results pages.
//!
//! The engine is generic: each document carries an opaque `payload` ([`serde_json::Value`]) that the
//! caller supplies and the engine echoes back in results. The engine never interprets the payload —
//! URL schemes, content types, and display data are entirely the caller's concern. Adapto provides
//! ranking; the application provides meaning.
//!
//! Workflow: [`SearchIndex::new`] → [`SearchIndex::add`] (per document) → [`SearchIndex::build`] →
//! [`SearchIndex::suggest`] / [`SearchIndex::search`].

use serde_json::Value;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// The three weighted fields of a document. Title matters most, body least.
pub struct TextFields<'a> {
    pub title: &'a str,
    pub keywords: &'a str,
    pub body: &'a str,
}

/// Tokenization and BM25F tuning.
pub struct SearchConfig {
    /// BM25 term-frequency saturation. Higher → repeated terms keep mattering. ~1.2.
    pub k1: f32,
    /// BM25 length normalization. 0 = none, 1 = full. ~0.75.
    pub b: f32,
    /// Field boost for title matches.
    pub w_title: f32,
    /// Field boost for keyword matches.
    pub w_keywords: f32,
    /// Field boost for body matches.
    pub w_body: f32,
    /// Emit character n-grams for typo/morphology tolerance.
    pub ngrams: bool,
    /// Character n-gram size.
    pub ngram_size: usize,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            k1: 1.2,
            b: 0.75,
            w_title: 5.0,
            w_keywords: 3.0,
            w_body: 1.0,
            ngrams: true,
            ngram_size: 3,
        }
    }
}

/// A typeahead completion: the matched document's id, a rank score, and its echoed payload.
#[derive(Clone, Debug)]
pub struct Suggestion {
    pub id: String,
    pub score: f32,
    pub payload: Value,
}

/// A search hit: the matched document's id, BM25F score, and its echoed payload.
#[derive(Clone, Debug)]
pub struct Hit {
    pub id: String,
    pub score: f32,
    pub payload: Value,
}

// ---------------------------------------------------------------------------
// Internal document representation
// ---------------------------------------------------------------------------

/// Number of weighted fields: index 0 = title, 1 = keywords, 2 = body.
const N_FIELDS: usize = 3;

struct Doc {
    id: String,
    payload: Value,
    /// Lowercased, normalized title + keywords text, for prefix suggest.
    suggest_text: String,
    /// Per-field token lists (index 0 = title, 1 = keywords, 2 = body), with n-grams.
    field_tokens: [Vec<String>; N_FIELDS],
    /// BM25F weighted document length: Σ_field w_field · |tokens_field|. Set in `build`.
    weighted_len: f32,
    /// Whether this doc participates in prefix [`SearchIndex::suggest`]. Search always sees it;
    /// huge low-priority corpora (companies, deep document trees) opt out so the typeahead scan
    /// stays small while the document is still findable on the full results page.
    suggestable: bool,
}

/// One token occurrence in one document: per-field term frequencies.
struct Posting {
    doc: u32,
    tf: [u32; N_FIELDS],
}

// ---------------------------------------------------------------------------
// SearchIndex
// ---------------------------------------------------------------------------

/// In-memory inverted index with BM25F ranking and prefix suggest.
pub struct SearchIndex {
    config: SearchConfig,
    docs: Vec<Doc>,
    /// token → postings across all documents.
    postings: HashMap<String, Vec<Posting>>,
    /// token → inverse document frequency.
    idf: HashMap<String, f32>,
    /// Mean weighted document length, for BM25 length normalization.
    avg_weighted_len: f32,
    /// Indices of suggestable docs, in insertion order. `suggest` scans only these so a large
    /// search-only corpus doesn't slow the typeahead.
    suggest_ids: Vec<u32>,
    built: bool,
}

impl SearchIndex {
    pub fn new(config: SearchConfig) -> Self {
        Self {
            config,
            docs: Vec::new(),
            postings: HashMap::new(),
            idf: HashMap::new(),
            avg_weighted_len: 0.0,
            suggest_ids: Vec::new(),
            built: false,
        }
    }

    /// Add a document that appears in both `suggest` and `search`. `id` and `payload` are returned
    /// verbatim in results.
    pub fn add(&mut self, id: &str, fields: TextFields, payload: Value) {
        self.add_doc(id, fields, payload, true);
    }

    /// Add a search-only document: found by [`SearchIndex::search`] but excluded from the typeahead
    /// [`SearchIndex::suggest`] scan. Use for large, low-priority corpora.
    pub fn add_search_only(&mut self, id: &str, fields: TextFields, payload: Value) {
        self.add_doc(id, fields, payload, false);
    }

    fn add_doc(&mut self, id: &str, fields: TextFields, payload: Value, suggestable: bool) {
        let suggest_text = normalize(&format!("{} {}", fields.title, fields.keywords));
        let field_tokens = [
            index_tokens(fields.title, &self.config),
            index_tokens(fields.keywords, &self.config),
            index_tokens(fields.body, &self.config),
        ];
        self.docs.push(Doc {
            id: id.to_string(),
            payload,
            suggest_text,
            field_tokens,
            weighted_len: 0.0,
            suggestable,
        });
        self.built = false;
    }

    /// Build the inverted index and BM25 statistics. Call after all `add`s.
    pub fn build(&mut self) {
        self.postings.clear();
        self.idf.clear();

        let field_weights = [self.config.w_title, self.config.w_keywords, self.config.w_body];

        // Pass 1: postings (per-field term frequencies) + weighted document lengths.
        let mut total_len = 0.0f32;
        for (doc_idx, doc) in self.docs.iter_mut().enumerate() {
            let mut tf_by_token: HashMap<&str, [u32; N_FIELDS]> = HashMap::new();
            let mut weighted_len = 0.0f32;
            for field in 0..N_FIELDS {
                weighted_len += field_weights[field] * doc.field_tokens[field].len() as f32;
                for token in &doc.field_tokens[field] {
                    tf_by_token.entry(token).or_default()[field] += 1;
                }
            }
            doc.weighted_len = weighted_len;
            total_len += weighted_len;

            for (token, tf) in tf_by_token {
                self.postings
                    .entry(token.to_string())
                    .or_default()
                    .push(Posting {
                        doc: doc_idx as u32,
                        tf,
                    });
            }
        }

        // Pass 2: IDF per token. Robertson/Sparck-Jones BM25 idf (always positive via +1).
        let n = self.docs.len().max(1) as f32;
        for (token, plist) in &self.postings {
            let df = plist.len() as f32;
            let idf = ((n - df + 0.5) / (df + 0.5) + 1.0).ln();
            self.idf.insert(token.clone(), idf);
        }

        self.avg_weighted_len = if self.docs.is_empty() {
            0.0
        } else {
            total_len / self.docs.len() as f32
        };

        // Suggest scans only suggestable docs, so a large search-only corpus doesn't slow typeahead.
        self.suggest_ids = self
            .docs
            .iter()
            .enumerate()
            .filter(|(_, d)| d.suggestable)
            .map(|(i, _)| i as u32)
            .collect();

        // Per-doc token lists were only needed to build postings; search uses the inverted index
        // and suggest uses `suggest_text`. Drop them so a large corpus doesn't hold them in RAM.
        for doc in &mut self.docs {
            doc.field_tokens = std::array::from_fn(|_| Vec::new());
        }

        self.built = true;
    }

    /// Tier-1 prefix completion over title + keywords. Cheap; for typeahead widgets.
    ///
    /// A document matches when every whitespace-separated query token is a prefix of some word in
    /// its title/keywords (AND semantics). Ranked by caller-supplied `payload.weight` (popularity),
    /// then a bonus for a leading match, then shorter (more specific) text first.
    pub fn suggest(&self, prefix: &str, limit: usize) -> Vec<Suggestion> {
        let q = normalize(prefix);
        let q = q.trim();
        if q.is_empty() || limit == 0 {
            return Vec::new();
        }
        let qtokens: Vec<&str> = q.split_whitespace().collect();

        let mut ranked: Vec<(f32, usize)> = Vec::new();
        for &doc_id in &self.suggest_ids {
            let i = doc_id as usize;
            let doc = &self.docs[i];
            let words: Vec<&str> = doc.suggest_text.split_whitespace().collect();
            let matches = qtokens
                .iter()
                .all(|qt| words.iter().any(|w| w.starts_with(qt)));
            if !matches {
                continue;
            }
            let mut score = payload_weight(&doc.payload);
            if doc.suggest_text.starts_with(q) {
                score += 1_000.0; // leading match — "закон…" beats "…закон…"
            }
            score -= doc.suggest_text.chars().count() as f32 * 0.001; // prefer concise titles
            ranked.push((score, i));
        }

        ranked.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        ranked.truncate(limit);
        ranked
            .into_iter()
            .map(|(score, i)| {
                let d = &self.docs[i];
                Suggestion {
                    id: d.id.clone(),
                    score,
                    payload: d.payload.clone(),
                }
            })
            .collect()
    }

    /// Tier-2 BM25F relevance ranking over all fields. For results pages.
    pub fn search(&self, query: &str, limit: usize) -> Vec<Hit> {
        debug_assert!(self.built, "call build() before search()");
        if !self.built || limit == 0 {
            return Vec::new();
        }

        let field_weights = [self.config.w_title, self.config.w_keywords, self.config.w_body];
        let (k1, b) = (self.config.k1, self.config.b);
        let avgdl = self.avg_weighted_len.max(1.0);

        // Accumulate BM25F score per candidate document.
        let mut scores: HashMap<u32, f32> = HashMap::new();
        let mut seen_query_token = std::collections::HashSet::new();
        for token in index_tokens(query, &self.config) {
            if !seen_query_token.insert(token.clone()) {
                continue; // count each distinct query token once
            }
            let Some(plist) = self.postings.get(&token) else {
                continue;
            };
            let idf = self.idf.get(&token).copied().unwrap_or(0.0);
            for posting in plist {
                // Weighted term frequency across fields (BM25F: sum then saturate once).
                let weighted_tf: f32 = (0..N_FIELDS)
                    .map(|f| field_weights[f] * posting.tf[f] as f32)
                    .sum();
                let dl = self.docs[posting.doc as usize].weighted_len;
                let denom = weighted_tf + k1 * (1.0 - b + b * dl / avgdl);
                let term_score = idf * (weighted_tf * (k1 + 1.0)) / denom.max(f32::EPSILON);
                *scores.entry(posting.doc).or_default() += term_score;
            }
        }

        let mut ranked: Vec<(u32, f32)> = scores.into_iter().filter(|(_, s)| *s > 0.0).collect();
        ranked.sort_unstable_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        ranked.truncate(limit);
        ranked
            .into_iter()
            .map(|(doc, score)| {
                let d = &self.docs[doc as usize];
                Hit {
                    id: d.id.clone(),
                    score,
                    payload: d.payload.clone(),
                }
            })
            .collect()
    }

    pub fn len(&self) -> usize {
        self.docs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.docs.is_empty()
    }
}

impl SearchConfig {
    fn min_or_default(&self) -> usize {
        2
    }
}

// ---------------------------------------------------------------------------
// Tokenization
// ---------------------------------------------------------------------------

/// Lowercase, fold `ё→е`, strip dashes. Shared normalization for all text.
fn normalize(text: &str) -> String {
    text.to_lowercase()
        .replace('ё', "е")
        .replace(['-', '\u{2013}', '\u{2014}'], "")
}

/// Word tokens (length ≥ `min` chars) from normalized text. No n-grams.
fn word_tokens(text: &str, min: usize) -> Vec<String> {
    let normalized = normalize(text);
    normalized
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.chars().count() >= min)
        .map(|w| w.to_string())
        .collect()
}

/// Character n-grams (prefixed `#` to avoid collision with words), over alphanumeric+space.
/// These give morphology/typo tolerance for languages with no stemmer (e.g. Kazakh): inflected
/// forms of the same stem share most n-grams, so they match partially.
fn char_ngrams(text: &str, n: usize, min: usize) -> Vec<String> {
    let chars: Vec<char> = normalize(text)
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == ' ')
        .collect();
    if chars.len() < n {
        return Vec::new();
    }
    chars
        .windows(n)
        .map(|w| w.iter().collect::<String>())
        .filter(|g| g.trim().chars().count() >= min)
        .map(|g| format!("#{g}"))
        .collect()
}

/// Popularity weight a caller may attach as `payload.weight` (number). Absent → 0.
fn payload_weight(payload: &Value) -> f32 {
    payload.get("weight").and_then(Value::as_f64).unwrap_or(0.0) as f32
}

/// Full indexing/query token set: word tokens plus (optionally) character n-grams.
fn index_tokens(text: &str, config: &SearchConfig) -> Vec<String> {
    let min = config.min_or_default();
    let mut tokens = word_tokens(text, min);
    if config.ngrams {
        tokens.extend(char_ngrams(text, config.ngram_size, min));
    }
    tokens
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn add(idx: &mut SearchIndex, id: &str, title: &str, keywords: &str, body: &str) {
        idx.add(
            id,
            TextFields {
                title,
                keywords,
                body,
            },
            json!({ "title": title }),
        );
    }

    #[test]
    fn exact_title_match_ranks_first() {
        let mut idx = SearchIndex::new(SearchConfig::default());
        add(&mut idx, "edu", "Закон об образовании", "", "");
        add(&mut idx, "health", "Закон о здравоохранении", "", "");
        idx.build();

        let hits = idx.search("образование", 5);

        assert!(!hits.is_empty(), "expected at least one hit");
        assert_eq!(hits[0].id, "edu", "education law should rank first");
    }

    #[test]
    fn suggest_completes_prefix_ranked_by_weight() {
        let mut idx = SearchIndex::new(SearchConfig::default());
        idx.add(
            "edu",
            TextFields { title: "Закон об образовании", keywords: "", body: "" },
            json!({ "title": "Закон об образовании", "url": "/law/laws/education/", "weight": 50 }),
        );
        idx.add(
            "rare",
            TextFields { title: "Закон о рекламе и спонсорстве спорта", keywords: "", body: "" },
            json!({ "title": "Закон о рекламе", "url": "/law/laws/ads/", "weight": 1 }),
        );
        idx.add(
            "traffic",
            TextFields { title: "Правила дорожного движения", keywords: "", body: "" },
            json!({ "title": "ПДД", "url": "/law/codes/traffic/", "weight": 99 }),
        );
        idx.build();

        let s = idx.suggest("закон", 8);

        assert!(!s.is_empty(), "prefix 'закон' should complete");
        assert!(
            s.iter().all(|x| x.id != "traffic"),
            "ПДД has no word starting with 'закон' — must not appear"
        );
        assert_eq!(s[0].id, "edu", "higher-weight matching law ranks first");
    }

    #[test]
    fn title_match_outranks_body_match() {
        let mut idx = SearchIndex::new(SearchConfig::default());
        add(&mut idx, "title_hit", "Лицензия на алкоголь", "", "общие положения");
        add(&mut idx, "body_hit", "Общие положения", "", "порядок выдачи лицензия");
        idx.build();

        let hits = idx.search("лицензия", 5);

        assert_eq!(hits[0].id, "title_hit", "match in title must outrank match in body");
    }

    #[test]
    fn shorter_document_wins_on_equal_term() {
        let mut idx = SearchIndex::new(SearchConfig::default());
        add(&mut idx, "short", "", "", "договор");
        let filler = "lorem ipsum dolor sit amet consectetur adipiscing elit sed do eiusmod";
        add(&mut idx, "long", "", "", &format!("договор {filler} {filler} {filler}"));
        idx.build();

        let hits = idx.search("договор", 5);

        assert_eq!(hits[0].id, "short", "BM25 length normalization: concise doc wins");
    }

    #[test]
    fn typo_still_finds_via_ngrams() {
        let mut idx = SearchIndex::new(SearchConfig::default());
        add(&mut idx, "const", "Конституция Республики Казахстан", "", "");
        add(&mut idx, "weather", "Погода в Астане", "", "");
        idx.build();

        let hits = idx.search("конститция", 5); // missing у

        assert!(!hits.is_empty(), "typo should still match via shared n-grams");
        assert_eq!(hits[0].id, "const");
    }

    #[test]
    fn payload_is_echoed_verbatim() {
        let mut idx = SearchIndex::new(SearchConfig::default());
        let payload = json!({ "title": "Закон", "url": "/law/laws/x/", "type": "law" });
        idx.add(
            "x",
            TextFields { title: "Закон икс", keywords: "", body: "" },
            payload.clone(),
        );
        idx.build();

        let hits = idx.search("закон", 5);

        assert_eq!(hits[0].payload, payload, "payload must be returned unchanged");
    }

    #[test]
    fn empty_query_and_empty_index_are_safe() {
        let mut empty = SearchIndex::new(SearchConfig::default());
        empty.build();
        assert!(empty.search("что угодно", 5).is_empty());
        assert!(empty.suggest("что", 5).is_empty());

        let mut idx = SearchIndex::new(SearchConfig::default());
        add(&mut idx, "a", "Закон", "", "");
        idx.build();
        assert!(idx.search("", 5).is_empty(), "empty query → no hits");
        assert!(idx.suggest("", 5).is_empty(), "empty prefix → no suggestions");
        assert!(idx.search("закон", 0).is_empty(), "limit 0 → no hits");
    }

    #[test]
    fn search_only_doc_is_searchable_but_not_suggested() {
        let mut idx = SearchIndex::new(SearchConfig::default());
        add(&mut idx, "top", "Закон об образовании", "", "");
        idx.add_search_only(
            "deep",
            TextFields {
                title: "Статья 5 Закона об образовании",
                keywords: "",
                body: "",
            },
            json!({ "title": "Статья 5" }),
        );
        idx.build();

        let hits = idx.search("образование", 10);
        assert!(
            hits.iter().any(|h| h.id == "deep"),
            "search-only doc must be findable via search"
        );

        let s = idx.suggest("закон", 10);
        assert!(s.iter().any(|x| x.id == "top"), "suggestable doc appears in suggest");
        assert!(
            s.iter().all(|x| x.id != "deep"),
            "search-only doc must NOT appear in suggest"
        );
    }
}
