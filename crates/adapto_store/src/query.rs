use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::document::{resolve_path, Document};

// ---------------------------------------------------------------------------
// Sort direction
// ---------------------------------------------------------------------------

/// Sort direction for query results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortDir {
    Asc,
    Desc,
}

// ---------------------------------------------------------------------------
// Filter
// ---------------------------------------------------------------------------

/// A composable filter expression tree (MongoDB-style operators).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Filter {
    /// Exact equality: `{ field: value }`.
    Eq(String, Value),
    /// Not equal: `{ field: { $ne: value } }`.
    Ne(String, Value),
    /// Greater than.
    Gt(String, Value),
    /// Greater than or equal.
    Gte(String, Value),
    /// Less than.
    Lt(String, Value),
    /// Less than or equal.
    Lte(String, Value),
    /// Value is in the given set.
    In(String, Vec<Value>),
    /// Value is NOT in the given set.
    Nin(String, Vec<Value>),
    /// Field exists (or does not).
    Exists(String, bool),
    /// Regex match (field value must be a string).
    Regex(String, String),
    /// Substring containment (case-sensitive).
    Contains(String, String),
    /// Logical AND of sub-filters.
    And(Vec<Filter>),
    /// Logical OR of sub-filters.
    Or(Vec<Filter>),
    /// Logical NOT.
    Not(Box<Filter>),
    /// Match every document.
    All,
}

impl Filter {
    /// Evaluate whether `doc` satisfies this filter.
    pub fn matches(&self, doc: &Document) -> bool {
        match self {
            Filter::All => true,
            Filter::Eq(field, val) => resolve_path(&doc.data, field)
                .map_or(false, |v| values_equal(v, val)),
            Filter::Ne(field, val) => resolve_path(&doc.data, field)
                .map_or(true, |v| !values_equal(v, val)),
            Filter::Gt(field, val) => resolve_path(&doc.data, field)
                .map_or(false, |v| compare_values(v, val) == Some(std::cmp::Ordering::Greater)),
            Filter::Gte(field, val) => resolve_path(&doc.data, field)
                .map_or(false, |v| matches!(compare_values(v, val), Some(std::cmp::Ordering::Greater | std::cmp::Ordering::Equal))),
            Filter::Lt(field, val) => resolve_path(&doc.data, field)
                .map_or(false, |v| compare_values(v, val) == Some(std::cmp::Ordering::Less)),
            Filter::Lte(field, val) => resolve_path(&doc.data, field)
                .map_or(false, |v| matches!(compare_values(v, val), Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal))),
            Filter::In(field, vals) => resolve_path(&doc.data, field)
                .map_or(false, |v| vals.iter().any(|candidate| values_equal(v, candidate))),
            Filter::Nin(field, vals) => resolve_path(&doc.data, field)
                .map_or(true, |v| !vals.iter().any(|candidate| values_equal(v, candidate))),
            Filter::Exists(field, should_exist) => {
                let found = resolve_path(&doc.data, field).is_some();
                found == *should_exist
            }
            Filter::Regex(field, pattern) => {
                resolve_path(&doc.data, field)
                    .and_then(Value::as_str)
                    .map_or(false, |s| regex_match(s, pattern))
            }
            Filter::Contains(field, substr) => {
                resolve_path(&doc.data, field)
                    .and_then(Value::as_str)
                    .map_or(false, |s| s.contains(substr.as_str()))
            }
            Filter::And(filters) => filters.iter().all(|f| f.matches(doc)),
            Filter::Or(filters) => filters.iter().any(|f| f.matches(doc)),
            Filter::Not(inner) => !inner.matches(doc),
        }
    }
}

// ---------------------------------------------------------------------------
// Value comparison helpers
// ---------------------------------------------------------------------------

/// Structural equality for JSON values (numbers compared numerically).
fn values_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(na), Value::Number(nb)) => {
            na.as_f64().zip(nb.as_f64()).map_or(false, |(x, y)| (x - y).abs() < f64::EPSILON)
        }
        _ => a == b,
    }
}

/// Ordering comparison for JSON values. Returns `None` for incompatible types.
pub fn compare_values(a: &Value, b: &Value) -> Option<std::cmp::Ordering> {
    match (a, b) {
        (Value::Number(na), Value::Number(nb)) => {
            let fa = na.as_f64()?;
            let fb = nb.as_f64()?;
            fa.partial_cmp(&fb)
        }
        (Value::String(sa), Value::String(sb)) => Some(sa.cmp(sb)),
        (Value::Bool(ba), Value::Bool(bb)) => Some(ba.cmp(bb)),
        _ => None,
    }
}

fn regex_match(haystack: &str, pattern: &str) -> bool {
    match regex::Regex::new(pattern) {
        Ok(re) => re.is_match(haystack),
        Err(_) => false,
    }
}

// ---------------------------------------------------------------------------
// Update
// ---------------------------------------------------------------------------

/// Update operators (MongoDB-style).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Update {
    /// `$set` — set fields to given values.
    Set(Vec<(String, Value)>),
    /// `$unset` — remove fields.
    Unset(Vec<String>),
    /// `$inc` — increment a numeric field.
    Inc(String, f64),
    /// `$push` — append a value to an array field.
    Push(String, Value),
    /// `$pull` — remove matching values from an array field.
    Pull(String, Value),
    /// `$rename` — rename a field.
    Rename(String, String),
    /// Apply multiple update operations in sequence.
    Multi(Vec<Update>),
}

// ---------------------------------------------------------------------------
// Query
// ---------------------------------------------------------------------------

/// A complete query: filter + sort + pagination + projection.
#[derive(Debug, Clone)]
pub struct Query {
    pub filter: Filter,
    pub sort: Option<Vec<(String, SortDir)>>,
    pub limit: Option<usize>,
    pub skip: Option<usize>,
    pub projection: Option<Vec<String>>,
}

impl Default for Query {
    fn default() -> Self {
        Self::new()
    }
}

impl Query {
    /// Create a query that matches all documents.
    pub fn new() -> Self {
        Self {
            filter: Filter::All,
            sort: None,
            limit: None,
            skip: None,
            projection: None,
        }
    }

    /// Create a query from a filter.
    pub fn filter(f: Filter) -> Self {
        Self {
            filter: f,
            sort: None,
            limit: None,
            skip: None,
            projection: None,
        }
    }

    /// Shortcut: equality filter on a single field.
    pub fn eq(field: &str, val: impl Into<Value>) -> Self {
        Self::filter(Filter::Eq(field.to_string(), val.into()))
    }

    /// Add a sort clause.
    pub fn sort(mut self, field: &str, dir: SortDir) -> Self {
        self.sort
            .get_or_insert_with(Vec::new)
            .push((field.to_string(), dir));
        self
    }

    /// Set result limit.
    pub fn limit(mut self, n: usize) -> Self {
        self.limit = Some(n);
        self
    }

    /// Set number of results to skip.
    pub fn skip(mut self, n: usize) -> Self {
        self.skip = Some(n);
        self
    }

    /// Set field projection (only include these fields in results).
    pub fn project(mut self, fields: &[&str]) -> Self {
        self.projection = Some(fields.iter().map(|s| s.to_string()).collect());
        self
    }
}

// ---------------------------------------------------------------------------
// UpdateResult
// ---------------------------------------------------------------------------

/// Result of an update operation.
#[derive(Debug, Clone)]
pub struct UpdateResult {
    /// Number of documents matched by the query.
    pub matched: u64,
    /// Number of documents actually modified.
    pub modified: u64,
}
