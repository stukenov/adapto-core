//! TF-IDF vector index for fuzzy text matching and data normalization.
//!
//! Enabled via `vector` feature flag. Zero external dependencies.
//!
//! ```rust
//! use adapto_store::vector::VectorIndex;
//!
//! let mut idx = VectorIndex::new();
//! idx.add("47110", "Розничная торговля в неспециализированных магазинах");
//! idx.add("47190", "Прочая розничная торговля в неспециализированных магазинах");
//! idx.build();
//!
//! let results = idx.search("ПРОЧАЯ РОЗНИЧНАЯ ТОРГОВЛЯ НЕ В МАГАЗИНАХ", 3);
//! assert_eq!(results[0].0, "47190");
//! ```

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Search result: (id, similarity score 0.0–1.0).
pub type SearchResult = (String, f32);

/// Tokenization and matching configuration.
pub struct VectorConfig {
    /// Minimum token length in chars (default: 2).
    pub min_token_len: usize,
    /// Include character n-grams for fuzzy matching (default: true).
    pub ngrams: bool,
    /// Character n-gram size (default: 3).
    pub ngram_size: usize,
}

impl Default for VectorConfig {
    fn default() -> Self {
        Self {
            min_token_len: 2,
            ngrams: true,
            ngram_size: 3,
        }
    }
}

// ---------------------------------------------------------------------------
// VectorIndex
// ---------------------------------------------------------------------------

/// TF-IDF vector index for matching messy text against a clean reference set.
///
/// Workflow: add reference entries → build → search queries against them.
pub struct VectorIndex {
    docs: Vec<DocEntry>,
    vocab: HashMap<String, u32>,
    idf: Vec<f32>,
    vectors: Vec<SparseVec>,
    built: bool,
    config: VectorConfig,
}

struct DocEntry {
    id: String,
    tokens: Vec<String>,
}

impl VectorIndex {
    pub fn new() -> Self {
        Self::with_config(VectorConfig::default())
    }

    pub fn with_config(config: VectorConfig) -> Self {
        Self {
            docs: Vec::new(),
            vocab: HashMap::new(),
            idf: Vec::new(),
            vectors: Vec::new(),
            built: false,
            config,
        }
    }

    /// Add a reference entry. `id` is returned in search results.
    pub fn add(&mut self, id: &str, text: &str) {
        self.docs.push(DocEntry {
            id: id.to_string(),
            tokens: self.tokenize(text),
        });
        self.built = false;
    }

    /// Build the index. Call after all entries are added.
    pub fn build(&mut self) {
        self.vocab.clear();
        let mut df: HashMap<String, u32> = HashMap::new();

        // Pass 1: build vocabulary and document frequencies
        for doc in &self.docs {
            let mut seen = std::collections::HashSet::new();
            for token in &doc.tokens {
                if !self.vocab.contains_key(token) {
                    let idx = self.vocab.len() as u32;
                    self.vocab.insert(token.clone(), idx);
                }
                if seen.insert(token) {
                    *df.entry(token.clone()).or_default() += 1;
                }
            }
        }

        // Compute IDF: ln(N / df) + 1
        let n = self.docs.len() as f32;
        self.idf = vec![0.0; self.vocab.len()];
        for (token, &idx) in &self.vocab {
            let doc_freq = df.get(token).copied().unwrap_or(1) as f32;
            self.idf[idx as usize] = (n / doc_freq).ln() + 1.0;
        }

        // Pass 2: build sparse TF-IDF vectors
        self.vectors.clear();
        self.vectors.reserve(self.docs.len());
        for doc in &self.docs {
            self.vectors.push(self.tfidf(&doc.tokens));
        }

        self.built = true;
    }

    /// Find top-k most similar reference entries.
    pub fn search(&self, query: &str, top_k: usize) -> Vec<SearchResult> {
        debug_assert!(self.built, "call build() before search()");
        if !self.built {
            return Vec::new();
        }
        let tokens = self.tokenize(query);
        let qvec = self.tfidf(&tokens);

        let mut scores: Vec<(usize, f32)> = self
            .vectors
            .iter()
            .enumerate()
            .map(|(i, v)| (i, sparse_dot(&qvec, v)))
            .collect();

        scores.sort_unstable_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        scores
            .into_iter()
            .take(top_k)
            .filter(|(_, s)| *s > 0.0)
            .map(|(i, s)| (self.docs[i].id.clone(), s))
            .collect()
    }

    /// Best match above threshold. Returns `None` if nothing scores high enough.
    pub fn search_one(&self, query: &str, threshold: f32) -> Option<SearchResult> {
        let results = self.search(query, 1);
        results.into_iter().next().filter(|(_, s)| *s >= threshold)
    }

    /// Batch search: match many texts at once. Returns `(query_id, results)` pairs.
    pub fn batch_search(
        &self,
        queries: &[(&str, &str)],
        top_k: usize,
    ) -> Vec<(String, Vec<SearchResult>)> {
        queries
            .iter()
            .map(|(qid, text)| (qid.to_string(), self.search(text, top_k)))
            .collect()
    }

    /// Normalize a batch of texts: for each, return the best match above threshold.
    /// Input: `[(id, text)]`. Output: `[(id, matched_ref_id, score)]`.
    pub fn normalize(
        &self,
        items: &[(&str, &str)],
        threshold: f32,
    ) -> Vec<(String, Option<String>, f32)> {
        items
            .iter()
            .map(|(id, text)| {
                let best = self.search_one(text, threshold);
                match best {
                    Some((ref_id, score)) => (id.to_string(), Some(ref_id), score),
                    None => (id.to_string(), None, 0.0),
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

    pub fn vocab_size(&self) -> usize {
        self.vocab.len()
    }

    // -----------------------------------------------------------------------
    // Internal
    // -----------------------------------------------------------------------

    fn tokenize(&self, text: &str) -> Vec<String> {
        let normalized = text
            .to_lowercase()
            .replace('-', "")
            .replace('\u{2013}', "")
            .replace('\u{2014}', "");

        let min = self.config.min_token_len;
        let mut tokens: Vec<String> = Vec::new();

        // Word tokens
        for word in normalized.split(|c: char| !c.is_alphanumeric()) {
            if word.chars().count() >= min {
                tokens.push(word.to_string());
            }
        }

        // Character n-grams (prefixed with # to avoid collision with words)
        if self.config.ngrams {
            let chars: Vec<char> = normalized
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == ' ')
                .collect();
            let n = self.config.ngram_size;
            if chars.len() >= n {
                for w in chars.windows(n) {
                    let gram: String = w.iter().collect();
                    if gram.trim().chars().count() >= min {
                        tokens.push(format!("#{gram}"));
                    }
                }
            }
        }

        tokens
    }

    fn tfidf(&self, tokens: &[String]) -> SparseVec {
        let mut tf: HashMap<u32, f32> = HashMap::new();
        let len = tokens.len().max(1) as f32;

        for token in tokens {
            if let Some(&idx) = self.vocab.get(token) {
                *tf.entry(idx).or_default() += 1.0;
            }
        }

        let mut indices: Vec<u32> = tf.keys().copied().collect();
        indices.sort_unstable();

        let mut values: Vec<f32> = indices
            .iter()
            .map(|&idx| (tf[&idx] / len) * self.idf[idx as usize])
            .collect();

        // L2-normalize so dot product = cosine similarity
        let norm: f32 = values.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for v in &mut values {
                *v /= norm;
            }
        }

        SparseVec { indices, values }
    }
}

impl Default for VectorIndex {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Sparse vector
// ---------------------------------------------------------------------------

#[derive(Clone, Default)]
struct SparseVec {
    indices: Vec<u32>,
    values: Vec<f32>,
}

/// Dot product of two sorted sparse vectors. Both are L2-normalized → result is cosine.
fn sparse_dot(a: &SparseVec, b: &SparseVec) -> f32 {
    let mut dot = 0.0f32;
    let (mut i, mut j) = (0, 0);
    while i < a.indices.len() && j < b.indices.len() {
        match a.indices[i].cmp(&b.indices[j]) {
            std::cmp::Ordering::Less => i += 1,
            std::cmp::Ordering::Greater => j += 1,
            std::cmp::Ordering::Equal => {
                dot += a.values[i] * b.values[j];
                i += 1;
                j += 1;
            }
        }
    }
    dot
}

// ---------------------------------------------------------------------------
// Text extraction utilities
// ---------------------------------------------------------------------------

/// Extract text between the first pair of quotes found in the input.
///
/// Recognizes directional pairs `«»`, `""`, `''` and plain `""`. For the
/// directional pairs, nesting is balanced: `«A «B»»` yields `A «B»` (the span
/// of the OUTERMOST pair), not `A «B` truncated at the first inner close.
/// Returns `None` if no non-empty quoted substring is found.
pub fn extract_quoted(text: &str) -> Option<&str> {
    const PAIRS: &[(char, char)] = &[
        ('\u{00ab}', '\u{00bb}'), // « »
        ('\u{201c}', '\u{201d}'), // " "
        ('\u{2018}', '\u{2019}'), // ' '
    ];
    for &(open, close) in PAIRS {
        if let Some(start) = text.find(open) {
            let inner_start = start + open.len_utf8();
            // Walk forward tracking nesting depth so the matching close is the
            // one that returns depth to zero, not the first inner close.
            let mut depth = 1usize;
            let mut last_close: Option<usize> = None;
            for (off, ch) in text[inner_start..].char_indices() {
                if ch == open {
                    depth += 1;
                } else if ch == close {
                    last_close = Some(off);
                    depth -= 1;
                    if depth == 0 {
                        let inner = text[inner_start..inner_start + off].trim();
                        if !inner.is_empty() {
                            return Some(inner);
                        }
                        break;
                    }
                }
            }
            // Unbalanced source (more opens than closes): fall back to the span
            // from the first open to the LAST close, dropping any prefix.
            if depth > 0 {
                if let Some(off) = last_close {
                    let inner = text[inner_start..inner_start + off].trim();
                    if !inner.is_empty() {
                        return Some(inner);
                    }
                }
            }
        }
    }
    // Plain double quotes (no directional nesting possible)
    let bytes = text.as_bytes();
    let mut first = None;
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'"' {
            match first {
                None => first = Some(i),
                Some(start) => {
                    let inner = text[start + 1..i].trim();
                    if !inner.is_empty() {
                        return Some(inner);
                    }
                    first = None;
                }
            }
        }
    }
    None
}

/// Extract the prefix of `text` before the first quotation mark.
///
/// Useful for getting the legal-form portion of a company name.
pub fn extract_prefix(text: &str) -> &str {
    const QUOTES: &[char] = &['\u{00ab}', '\u{00bb}', '"', '\u{201c}', '\u{201d}', '\u{2018}', '\u{2019}'];
    let pos = text.find(QUOTES).unwrap_or(text.len());
    let prefix = text[..pos].trim();
    if prefix.is_empty() { text } else { prefix }
}

// ---------------------------------------------------------------------------
// Address parsing
// ---------------------------------------------------------------------------

/// A parsed, structured address.
///
/// Produced by [`parse_address`]. Geographic components (`region`/`city`) are
/// matched IDs from the reference indexes; `street` is the unmatched remainder.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ParsedAddress {
    /// Leading all-digit field (postal/zip code), if present.
    pub postal_code: Option<String>,
    /// Matched region reference ID.
    pub region: Option<String>,
    /// Region match score (0.0 if no match).
    pub region_score: f32,
    /// Matched city/settlement reference ID.
    pub city: Option<String>,
    /// City match score (0.0 if no match).
    pub city_score: f32,
    /// Remaining address text (street, building, etc.), comma-joined.
    pub street: Option<String>,
}

/// Parse a comma-separated address into structured components using vector indexes.
///
/// - Postal code: first comma-field that is all ASCII digits.
/// - Region: best match of any field against `region_index`.
/// - City: best match of any field against `city_index` (skips the field used for region).
/// - Street: remaining fields after region/city/postal, comma-joined.
///
/// Both indexes must be built. `threshold` filters weak matches.
///
/// ```rust
/// use adapto_store::vector::{VectorIndex, parse_address};
///
/// let mut regions = VectorIndex::new();
/// regions.add("75", "город Алматы");
/// regions.build();
///
/// let mut cities = VectorIndex::new();
/// cities.add("750000", "Алмалинский район");
/// cities.build();
///
/// let parsed = parse_address(
///     "050000, Г.АЛМАТЫ, АЛМАЛИНСКИЙ РАЙОН, УЛ. КУНАЕВА, д. 43",
///     &regions, &cities, 0.3,
/// );
/// assert_eq!(parsed.postal_code.as_deref(), Some("050000"));
/// assert_eq!(parsed.region.as_deref(), Some("75"));
/// assert_eq!(parsed.city.as_deref(), Some("750000"));
/// ```
pub fn parse_address(
    text: &str,
    region_index: &VectorIndex,
    city_index: &VectorIndex,
    threshold: f32,
) -> ParsedAddress {
    let fields: Vec<&str> = text.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    if fields.is_empty() {
        return ParsedAddress::default();
    }

    let mut result = ParsedAddress::default();
    let mut consumed = vec![false; fields.len()];

    // Postal code: first all-digit field
    for (i, f) in fields.iter().enumerate() {
        if !f.is_empty() && f.bytes().all(|b| b.is_ascii_digit()) {
            result.postal_code = Some((*f).to_string());
            consumed[i] = true;
            break;
        }
    }

    // Region: the FIRST non-consumed field only (positional). In postal-prefixed
    // addresses the region always leads, so scanning every field risks an
    // intra-city district name (e.g. "район Алматы" inside Astana) hijacking the
    // region match against a same-named city region.
    if let Some((i, f)) = fields.iter().enumerate().find(|(i, _)| !consumed[*i]) {
        if let Some((id, score)) = region_index.search_one(f, threshold) {
            result.region = Some(id);
            result.region_score = score;
            consumed[i] = true;
        }
    }

    // City: prefer the field immediately after the region (positional); if that
    // field doesn't match, fall back to the best match among the rest. The
    // positional preference stops a weakly-matching street field from beating the
    // real city/district that follows the region.
    let mut city_pick: Option<(usize, String, f32)> = None;
    if let Some((i, f)) = fields.iter().enumerate().find(|(i, _)| !consumed[*i]) {
        if let Some((id, score)) = city_index.search_one(f, threshold) {
            city_pick = Some((i, id, score));
        }
    }
    if city_pick.is_none() {
        for (i, f) in fields.iter().enumerate() {
            if consumed[i] {
                continue;
            }
            if let Some((id, score)) = city_index.search_one(f, threshold) {
                if city_pick.as_ref().map_or(true, |(_, _, s)| score > *s) {
                    city_pick = Some((i, id, score));
                }
            }
        }
    }
    if let Some((i, id, score)) = city_pick {
        result.city = Some(id);
        result.city_score = score;
        consumed[i] = true;
    }

    // Street: remaining unconsumed fields
    let street: Vec<&str> = fields
        .iter()
        .enumerate()
        .filter(|(i, _)| !consumed[*i])
        .map(|(_, f)| *f)
        .collect();
    if !street.is_empty() {
        result.street = Some(street.join(", "));
    }

    result
}

// ---------------------------------------------------------------------------
// Collection integration — normalize fields at DB level
// ---------------------------------------------------------------------------

/// Result of a `normalize_field` operation.
#[derive(Debug, Clone)]
pub struct NormalizeResult {
    /// Total documents processed.
    pub total: u64,
    /// Documents where a match was found above threshold.
    pub matched: u64,
    /// Documents with no match or empty source field.
    pub unmatched: u64,
}

impl<'a> crate::Collection<'a> {
    /// Normalize a text field using a VectorIndex.
    ///
    /// For each document, reads `source_field`, searches the index for the
    /// best match, and writes the result to `target_field` (plus
    /// `{target_field}_score` with the confidence).
    ///
    /// ```rust,no_run
    /// use adapto_store::{AdaptoStore, VectorIndex};
    ///
    /// let store = AdaptoStore::open(None).unwrap();
    ///
    /// // Build reference index
    /// let mut idx = VectorIndex::new();
    /// idx.add("682", "Аренда и управление недвижимостью");
    /// idx.add("561", "Деятельность ресторанов");
    /// idx.build();
    ///
    /// // Normalize companies
    /// let companies = store.collection("companies");
    /// let result = companies.normalize_field("okedru", "oked_code", &idx, 0.4).unwrap();
    /// eprintln!("Matched {}/{}", result.matched, result.total);
    /// ```
    pub fn normalize_field(
        &self,
        source_field: &str,
        target_field: &str,
        index: &VectorIndex,
        threshold: f32,
    ) -> Result<NormalizeResult, crate::StoreError> {
        let docs: Vec<crate::Document> = self.find(crate::Query::new()).collect();
        let mut matched = 0u64;
        let mut unmatched = 0u64;
        let score_field = format!("{target_field}_score");

        for doc in &docs {
            let text = match doc.get_str(source_field) {
                Some(t) if !t.is_empty() => t,
                _ => {
                    unmatched += 1;
                    continue;
                }
            };

            match index.search_one(text, threshold) {
                Some((ref_id, score)) => {
                    let score_num = serde_json::Number::from_f64(score as f64)
                        .unwrap_or_else(|| serde_json::Number::from(0));
                    self.update_by_id(
                        &doc.id,
                        crate::Update::Set(vec![
                            (target_field.to_string(), serde_json::Value::String(ref_id)),
                            (score_field.clone(), serde_json::Value::Number(score_num)),
                        ]),
                    )?;
                    matched += 1;
                }
                None => {
                    unmatched += 1;
                }
            }
        }

        Ok(NormalizeResult {
            total: matched + unmatched,
            matched,
            unmatched,
        })
    }

    /// Normalize a name field: extract legal form via VectorIndex + clean name from quotes.
    ///
    /// For each document, reads `source_field` (e.g. `"nameru"`), then:
    /// - Extracts prefix before quotes → searches `index` → writes `form_field` + `{form_field}_score`
    /// - Extracts quoted substring → writes `name_field`
    ///
    /// ```rust,no_run
    /// use adapto_store::{AdaptoStore, VectorIndex};
    ///
    /// let store = AdaptoStore::open(None).unwrap();
    /// let mut idx = VectorIndex::new();
    /// idx.add("ТОО", "Товарищество с ограниченной ответственностью");
    /// idx.add("АО", "Акционерное общество");
    /// idx.build();
    ///
    /// let col = store.collection("companies");
    /// let result = col.normalize_name_field("nameru", "legal_form", "clean_name", &idx, 0.3).unwrap();
    /// ```
    pub fn normalize_name_field(
        &self,
        source_field: &str,
        form_field: &str,
        name_field: &str,
        index: &VectorIndex,
        threshold: f32,
    ) -> Result<NormalizeResult, crate::StoreError> {
        let docs: Vec<crate::Document> = self.find(crate::Query::new()).collect();
        let mut matched = 0u64;
        let mut unmatched = 0u64;
        let score_field = format!("{form_field}_score");

        for doc in &docs {
            let full_name = match doc.get_str(source_field) {
                Some(t) if !t.is_empty() => t,
                _ => {
                    unmatched += 1;
                    continue;
                }
            };

            let mut updates: Vec<(String, serde_json::Value)> = Vec::new();

            // Extract clean name from quotes
            if let Some(clean) = extract_quoted(full_name) {
                updates.push((name_field.to_string(), serde_json::Value::String(clean.to_string())));
            }

            // Extract legal form via vector search on prefix
            let prefix = extract_prefix(full_name);
            match index.search_one(prefix, threshold) {
                Some((ref_id, score)) => {
                    let score_num = serde_json::Number::from_f64(score as f64)
                        .unwrap_or_else(|| serde_json::Number::from(0));
                    updates.push((form_field.to_string(), serde_json::Value::String(ref_id)));
                    updates.push((score_field.clone(), serde_json::Value::Number(score_num)));
                    matched += 1;
                }
                None => {
                    unmatched += 1;
                }
            }

            if !updates.is_empty() {
                self.update_by_id(&doc.id, crate::Update::Set(updates))?;
            }
        }

        Ok(NormalizeResult {
            total: matched + unmatched,
            matched,
            unmatched,
        })
    }

    /// Parse an address field into structured geo components at the DB level.
    ///
    /// For each document, reads `source_field`, runs [`parse_address`], and writes:
    /// `region_field`, `city_field`, `postal_field`, `street_field`
    /// (plus `{region_field}_score` and `{city_field}_score`).
    ///
    /// Any target whose value is `None` is skipped (left untouched).
    /// Returns counts of documents where region / city were matched.
    ///
    /// ```rust,no_run
    /// use adapto_store::{AdaptoStore, VectorIndex};
    ///
    /// let store = AdaptoStore::open(None).unwrap();
    /// let mut regions = VectorIndex::new();
    /// regions.add("75", "город Алматы");
    /// regions.build();
    /// let mut cities = VectorIndex::new();
    /// cities.add("750000", "Алмалинский район");
    /// cities.build();
    ///
    /// let col = store.collection("companies");
    /// let r = col.parse_address_field(
    ///     "addressru", &regions, &cities,
    ///     "region_code", "city_code", "postal_code", "street", 0.3,
    /// ).unwrap();
    /// eprintln!("region {}/{}, city {}/{}", r.region_matched, r.total, r.city_matched, r.total);
    /// ```
    #[allow(clippy::too_many_arguments)]
    pub fn parse_address_field(
        &self,
        source_field: &str,
        region_index: &VectorIndex,
        city_index: &VectorIndex,
        region_field: &str,
        city_field: &str,
        postal_field: &str,
        street_field: &str,
        threshold: f32,
    ) -> Result<AddressResult, crate::StoreError> {
        let docs: Vec<crate::Document> = self.find(crate::Query::new()).collect();
        let mut total = 0u64;
        let mut region_matched = 0u64;
        let mut city_matched = 0u64;
        let region_score_field = format!("{region_field}_score");
        let city_score_field = format!("{city_field}_score");

        for doc in &docs {
            let text = match doc.get_str(source_field) {
                Some(t) if !t.is_empty() => t,
                _ => continue,
            };
            total += 1;

            let parsed = parse_address(text, region_index, city_index, threshold);
            let mut updates: Vec<(String, serde_json::Value)> = Vec::new();

            if let Some(postal) = parsed.postal_code {
                updates.push((postal_field.to_string(), serde_json::Value::String(postal)));
            }
            if let Some(region) = parsed.region {
                updates.push((region_field.to_string(), serde_json::Value::String(region)));
                if let Some(n) = serde_json::Number::from_f64(parsed.region_score as f64) {
                    updates.push((region_score_field.clone(), serde_json::Value::Number(n)));
                }
                region_matched += 1;
            }
            if let Some(city) = parsed.city {
                updates.push((city_field.to_string(), serde_json::Value::String(city)));
                if let Some(n) = serde_json::Number::from_f64(parsed.city_score as f64) {
                    updates.push((city_score_field.clone(), serde_json::Value::Number(n)));
                }
                city_matched += 1;
            }
            if let Some(street) = parsed.street {
                updates.push((street_field.to_string(), serde_json::Value::String(street)));
            }

            if !updates.is_empty() {
                self.update_by_id(&doc.id, crate::Update::Set(updates))?;
            }
        }

        Ok(AddressResult {
            total,
            region_matched,
            city_matched,
        })
    }
}

/// Result of a `parse_address_field` operation.
#[derive(Debug, Clone)]
pub struct AddressResult {
    /// Total documents with a non-empty source field.
    pub total: u64,
    /// Documents where a region was matched.
    pub region_matched: u64,
    /// Documents where a city was matched.
    pub city_matched: u64,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_match_scores_highest() {
        let mut idx = VectorIndex::new();
        idx.add("A", "Сельское хозяйство");
        idx.add("B", "Рыбное хозяйство");
        idx.add("C", "Строительство зданий");
        idx.build();

        let results = idx.search("Сельское хозяйство", 3);
        assert_eq!(results[0].0, "A");
        assert!(results[0].1 > 0.8);
    }

    #[test]
    fn case_insensitive() {
        let mut idx = VectorIndex::new();
        idx.add("1", "Розничная торговля");
        idx.build();

        let results = idx.search("РОЗНИЧНАЯ ТОРГОВЛЯ", 1);
        assert_eq!(results[0].0, "1");
        assert!(results[0].1 > 0.8);
    }

    #[test]
    fn partial_overlap() {
        let mut idx = VectorIndex::new();
        idx.add("47110", "Розничная торговля в неспециализированных магазинах");
        idx.add("47190", "Прочая розничная торговля в неспециализированных магазинах");
        idx.add("56100", "Деятельность ресторанов");
        idx.build();

        let results = idx.search("ПРОЧАЯ РОЗНИЧНАЯ ТОРГОВЛЯ НЕ В МАГАЗИНАХ", 3);
        assert_eq!(results[0].0, "47190");
    }

    #[test]
    fn hyphen_normalization() {
        let mut idx = VectorIndex::new();
        idx.add("1", "Строительно-монтажные работы");
        idx.build();

        let results = idx.search("СТРОИТЕЛЬНОМОНТАЖНЫЕ РАБОТЫ", 1);
        assert_eq!(results[0].0, "1");
        assert!(results[0].1 > 0.5);
    }

    #[test]
    fn search_one_with_threshold() {
        let mut idx = VectorIndex::new();
        idx.add("A", "Сельское хозяйство");
        idx.build();

        assert!(idx.search_one("Сельское хозяйство", 0.5).is_some());
        assert!(idx.search_one("Квантовая физика плазмы", 0.5).is_none());
    }

    #[test]
    fn normalize_batch() {
        let mut idx = VectorIndex::new();
        idx.add("01", "Сельское хозяйство");
        idx.add("41", "Строительство зданий");
        idx.build();

        let items = vec![
            ("c1", "СЕЛЬСКОЕ ХОЗЯЙСТВО"),
            ("c2", "строительство зданий и сооружений"),
            ("c3", "абсолютно неизвестный текст xyz"),
        ];
        let results = idx.normalize(&items, 0.5);
        assert_eq!(results[0].1, Some("01".to_string()));
        assert!(results[2].1.is_none());
    }

    #[test]
    fn empty_index() {
        let mut idx = VectorIndex::new();
        idx.build();
        let results = idx.search("test", 5);
        assert!(results.is_empty());
    }

    #[test]
    fn no_ngrams_config() {
        let mut idx = VectorIndex::with_config(VectorConfig {
            min_token_len: 2,
            ngrams: false,
            ngram_size: 3,
        });
        idx.add("1", "тестовый документ");
        idx.build();

        assert!(idx.vocab_size() < 10);
        let results = idx.search("тестовый", 1);
        assert!(!results.is_empty());
    }

    #[test]
    fn extract_quoted_various() {
        assert_eq!(extract_quoted("ТОО «Рога и Копыта»"), Some("Рога и Копыта"));
        assert_eq!(extract_quoted("АО \"Казахтелеком\""), Some("Казахтелеком"));
        assert_eq!(extract_quoted("\u{201c}Test\u{201d}"), Some("Test"));
        assert_eq!(extract_quoted("No quotes here"), None);
        assert_eq!(extract_quoted("ТОО \"\""), None);
    }

    #[test]
    fn extract_quoted_nested() {
        // Outermost balanced span, not truncated at the first inner close.
        assert_eq!(
            extract_quoted("Филиал «Казахстанская партия зелёных «Байтақ»» по области"),
            Some("Казахстанская партия зелёных «Байтақ»")
        );
        assert_eq!(
            extract_quoted("Объединение «Ассоциация «Спорт» Казахстана»"),
            Some("Ассоциация «Спорт» Казахстана")
        );
        // Unbalanced source (one close missing) → first-open..last-close span,
        // dropping the legal-form prefix.
        assert_eq!(
            extract_quoted("Общественное объединение «Профсоюз «Управление транспорта»"),
            Some("Профсоюз «Управление транспорта")
        );
    }

    #[test]
    fn extract_prefix_before_quotes() {
        assert_eq!(
            extract_prefix("Товарищество с ограниченной ответственностью «Рога»"),
            "Товарищество с ограниченной ответственностью"
        );
        assert_eq!(
            extract_prefix("Филиал АО \"Компания\""),
            "Филиал АО"
        );
        assert_eq!(extract_prefix("No quotes"), "No quotes");
    }

    #[test]
    fn collection_normalize_name_field() {
        let store = crate::AdaptoStore::open(None).unwrap();
        let col = store.collection("test_names");

        col.insert(serde_json::json!({
            "nameru": "Товарищество с ограниченной ответственностью «Алмаз»"
        })).unwrap();
        col.insert(serde_json::json!({
            "nameru": "Акционерное общество \"Казахтелеком\""
        })).unwrap();

        let mut idx = VectorIndex::new();
        idx.add("ТОО", "Товарищество с ограниченной ответственностью");
        idx.add("АО", "Акционерное общество");
        idx.build();

        let result = col.normalize_name_field("nameru", "legal_form", "clean_name", &idx, 0.3).unwrap();
        assert_eq!(result.matched, 2);

        let doc = col.find_one(crate::Query::eq("clean_name", "Алмаз")).unwrap().unwrap();
        assert_eq!(doc.get_str("legal_form"), Some("ТОО"));
        assert_eq!(doc.get_str("clean_name"), Some("Алмаз"));

        let doc2 = col.find_one(crate::Query::eq("clean_name", "Казахтелеком")).unwrap().unwrap();
        assert_eq!(doc2.get_str("legal_form"), Some("АО"));
    }

    #[test]
    fn parse_address_components() {
        let mut regions = VectorIndex::new();
        regions.add("75", "город Алматы");
        regions.add("19", "Восточно-Казахстанская область");
        regions.build();

        let mut cities = VectorIndex::new();
        cities.add("750000", "Алмалинский район");
        cities.add("190000", "город Семей");
        cities.build();

        let p = parse_address(
            "050000, Г.АЛМАТЫ, АЛМАЛИНСКИЙ РАЙОН, УЛ. КУНАЕВА, д. 43",
            &regions, &cities, 0.3,
        );
        assert_eq!(p.postal_code.as_deref(), Some("050000"));
        assert_eq!(p.region.as_deref(), Some("75"));
        assert_eq!(p.city.as_deref(), Some("750000"));
        assert!(p.street.as_deref().unwrap().contains("КУНАЕВА"));
    }

    #[test]
    fn parse_address_word_order_invariant() {
        let mut regions = VectorIndex::new();
        regions.add("19", "Восточно-Казахстанская область");
        regions.build();
        let mut cities = VectorIndex::new();
        cities.add("190000", "город Семей");
        cities.build();

        // "ОБЛАСТЬ ВОСТОЧНО-КАЗАХСТАНСКАЯ" — reversed word order
        let p = parse_address(
            "070000, ОБЛАСТЬ ВОСТОЧНО-КАЗАХСТАНСКАЯ, Г.СЕМЕЙ, УЛ. ШУГАЕВА, д. 4",
            &regions, &cities, 0.3,
        );
        assert_eq!(p.region.as_deref(), Some("19"));
    }

    #[test]
    fn parse_address_empty() {
        let mut idx = VectorIndex::new();
        idx.add("x", "dummy");
        idx.build();
        let p = parse_address("", &idx, &idx, 0.3);
        assert_eq!(p, ParsedAddress::default());
    }

    #[test]
    fn collection_parse_address_field() {
        let store = crate::AdaptoStore::open(None).unwrap();
        let col = store.collection("test_addr");

        col.insert(serde_json::json!({
            "addressru": "050000, Г.АЛМАТЫ, АЛМАЛИНСКИЙ РАЙОН, УЛ. КУНАЕВА, д. 43"
        })).unwrap();
        col.insert(serde_json::json!({
            "addressru": "100000, КАРАГАНДИНСКАЯ ОБЛАСТЬ, Г.КАРАГАНДА, УЛ. ЛЕНИНА, д. 1"
        })).unwrap();

        let mut regions = VectorIndex::new();
        regions.add("75", "город Алматы");
        regions.add("35", "Карагандинская область");
        regions.build();

        let mut cities = VectorIndex::new();
        cities.add("750000", "Алмалинский район");
        cities.add("350000", "город Караганда");
        cities.build();

        let r = col.parse_address_field(
            "addressru", &regions, &cities,
            "region_code", "city_code", "postal_code", "street", 0.3,
        ).unwrap();

        assert_eq!(r.total, 2);
        assert_eq!(r.region_matched, 2);

        let doc = col.find_one(crate::Query::eq("postal_code", "050000")).unwrap().unwrap();
        assert_eq!(doc.get_str("region_code"), Some("75"));
        assert_eq!(doc.get_str("city_code"), Some("750000"));
        assert!(doc.get_str("street").unwrap().contains("КУНАЕВА"));
        assert!(doc.get("region_code_score").is_some());
    }

    #[test]
    fn collection_normalize_field() {
        let store = crate::AdaptoStore::open(None).unwrap();
        let col = store.collection("test_companies");

        col.insert(serde_json::json!({"name": "Comp A", "okedru": "СЕЛЬСКОЕ ХОЗЯЙСТВО"})).unwrap();
        col.insert(serde_json::json!({"name": "Comp B", "okedru": "СТРОИТЕЛЬСТВО ЗДАНИЙ"})).unwrap();
        col.insert(serde_json::json!({"name": "Comp C", "okedru": ""})).unwrap();
        col.insert(serde_json::json!({"name": "Comp D"})).unwrap();

        let mut idx = VectorIndex::new();
        idx.add("01", "Сельское хозяйство");
        idx.add("41", "Строительство зданий");
        idx.build();

        let result = col.normalize_field("okedru", "oked_code", &idx, 0.4).unwrap();
        assert_eq!(result.total, 4);
        assert_eq!(result.matched, 2);
        assert_eq!(result.unmatched, 2);

        let doc_a = col.find_one(crate::Query::eq("name", "Comp A")).unwrap().unwrap();
        assert_eq!(doc_a.get_str("oked_code"), Some("01"));
        assert!(doc_a.get("oked_code_score").is_some());

        let doc_b = col.find_one(crate::Query::eq("name", "Comp B")).unwrap().unwrap();
        assert_eq!(doc_b.get_str("oked_code"), Some("41"));

        let doc_c = col.find_one(crate::Query::eq("name", "Comp C")).unwrap().unwrap();
        assert!(doc_c.get_str("oked_code").is_none());
    }

    #[test]
    fn large_index_perf() {
        let mut idx = VectorIndex::new();
        for i in 0..2000 {
            idx.add(&format!("{i}"), &format!("запись номер {i} тестовая строка"));
        }
        idx.build();

        let start = std::time::Instant::now();
        for _ in 0..100 {
            idx.search("тестовая строка номер 500", 5);
        }
        let elapsed = start.elapsed();
        assert!(elapsed.as_millis() < 5000, "100 searches in 2K index took {elapsed:?}");
    }
}
