use std::collections::BTreeMap;
use std::ops::Bound;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::document::{resolve_path, Document};
use crate::error::StoreError;

// ---------------------------------------------------------------------------
// IndexKey — orderable wrapper around JSON values
// ---------------------------------------------------------------------------

/// A key extracted from one or more document fields for index storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IndexKey {
    Single(Value),
    Compound(Vec<Value>),
    Null,
}

impl PartialEq for IndexKey {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == std::cmp::Ordering::Equal
    }
}

impl Eq for IndexKey {}

impl PartialOrd for IndexKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for IndexKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (IndexKey::Null, IndexKey::Null) => std::cmp::Ordering::Equal,
            (IndexKey::Null, _) => std::cmp::Ordering::Less,
            (_, IndexKey::Null) => std::cmp::Ordering::Greater,
            (IndexKey::Single(a), IndexKey::Single(b)) => cmp_value(a, b),
            (IndexKey::Compound(a), IndexKey::Compound(b)) => {
                for (va, vb) in a.iter().zip(b.iter()) {
                    let ord = cmp_value(va, vb);
                    if ord != std::cmp::Ordering::Equal {
                        return ord;
                    }
                }
                a.len().cmp(&b.len())
            }
            (IndexKey::Single(_), IndexKey::Compound(_)) => std::cmp::Ordering::Less,
            (IndexKey::Compound(_), IndexKey::Single(_)) => std::cmp::Ordering::Greater,
        }
    }
}

/// Deterministic ordering for arbitrary `serde_json::Value`.
/// Type order: Null < Bool < Number < String < Array < Object.
fn cmp_value(a: &Value, b: &Value) -> std::cmp::Ordering {
    let type_rank = |v: &Value| -> u8 {
        match v {
            Value::Null => 0,
            Value::Bool(_) => 1,
            Value::Number(_) => 2,
            Value::String(_) => 3,
            Value::Array(_) => 4,
            Value::Object(_) => 5,
        }
    };

    let ra = type_rank(a);
    let rb = type_rank(b);
    if ra != rb {
        return ra.cmp(&rb);
    }

    match (a, b) {
        (Value::Null, Value::Null) => std::cmp::Ordering::Equal,
        (Value::Bool(ba), Value::Bool(bb)) => ba.cmp(bb),
        (Value::Number(na), Value::Number(nb)) => {
            let fa = na.as_f64().unwrap_or(0.0);
            let fb = nb.as_f64().unwrap_or(0.0);
            fa.partial_cmp(&fb).unwrap_or(std::cmp::Ordering::Equal)
        }
        (Value::String(sa), Value::String(sb)) => sa.cmp(sb),
        (Value::Array(aa), Value::Array(ab)) => {
            for (ea, eb) in aa.iter().zip(ab.iter()) {
                let ord = cmp_value(ea, eb);
                if ord != std::cmp::Ordering::Equal {
                    return ord;
                }
            }
            aa.len().cmp(&ab.len())
        }
        _ => std::cmp::Ordering::Equal,
    }
}

impl IndexKey {
    /// Format the key as a human-readable string (for error messages).
    pub fn display(&self) -> String {
        match self {
            IndexKey::Null => "null".to_string(),
            IndexKey::Single(v) => format!("{v}"),
            IndexKey::Compound(vs) => {
                let parts: Vec<String> = vs.iter().map(|v| format!("{v}")).collect();
                format!("[{}]", parts.join(", "))
            }
        }
    }
}

// ---------------------------------------------------------------------------
// IndexInfo — public metadata about an index
// ---------------------------------------------------------------------------

/// Public metadata about an index.
#[derive(Debug, Clone)]
pub struct IndexInfo {
    pub name: String,
    pub fields: Vec<String>,
    pub unique: bool,
    pub entries: usize,
}

// ---------------------------------------------------------------------------
// BTreeIndex
// ---------------------------------------------------------------------------

/// A B-tree backed index over one or more document fields.
pub struct BTreeIndex {
    pub(crate) name: String,
    pub(crate) fields: Vec<String>,
    pub(crate) unique: bool,
    tree: BTreeMap<IndexKey, Vec<String>>,
}

impl BTreeIndex {
    /// Create a new empty index.
    pub fn new(name: String, fields: Vec<String>, unique: bool) -> Self {
        Self {
            name,
            fields,
            unique,
            tree: BTreeMap::new(),
        }
    }

    /// Extract the index key from a document.
    pub fn key_for(&self, doc: &Document) -> IndexKey {
        if self.fields.len() == 1 {
            match resolve_path(&doc.data, &self.fields[0]) {
                Some(v) => IndexKey::Single(v.clone()),
                None => IndexKey::Null,
            }
        } else {
            let values: Vec<Value> = self
                .fields
                .iter()
                .map(|f| resolve_path(&doc.data, f).cloned().unwrap_or(Value::Null))
                .collect();
            if values.iter().all(|v| v.is_null()) {
                IndexKey::Null
            } else {
                IndexKey::Compound(values)
            }
        }
    }

    /// Insert a document ID under the given key.
    pub fn insert(&mut self, key: IndexKey, doc_id: &str) -> Result<(), StoreError> {
        if self.unique {
            if let Some(ids) = self.tree.get(&key) {
                if !ids.is_empty() && key != IndexKey::Null {
                    return Err(StoreError::DuplicateKey {
                        index: self.name.clone(),
                        key: key.display(),
                    });
                }
            }
        }
        self.tree
            .entry(key)
            .or_default()
            .push(doc_id.to_string());
        Ok(())
    }

    /// Remove a document ID from a key entry.
    pub fn remove(&mut self, key: &IndexKey, doc_id: &str) {
        if let Some(ids) = self.tree.get_mut(key) {
            ids.retain(|id| id != doc_id);
            if ids.is_empty() {
                self.tree.remove(key);
            }
        }
    }

    /// Find all document IDs with an exact key match.
    pub fn find_eq(&self, key: &IndexKey) -> Vec<&str> {
        self.tree
            .get(key)
            .map(|ids| ids.iter().map(String::as_str).collect())
            .unwrap_or_default()
    }

    /// Find all document IDs within a key range.
    pub fn find_range(
        &self,
        start: Bound<&IndexKey>,
        end: Bound<&IndexKey>,
    ) -> Vec<&str> {
        let owned_start = bound_cloned(start);
        let owned_end = bound_cloned(end);
        self.tree
            .range((owned_start, owned_end))
            .flat_map(|(_, ids)| ids.iter().map(String::as_str))
            .collect()
    }

    /// Check whether a key exists in the index.
    pub fn contains_key(&self, key: &IndexKey) -> bool {
        self.tree.contains_key(key)
    }

    /// Return metadata about this index.
    pub fn info(&self) -> IndexInfo {
        IndexInfo {
            name: self.name.clone(),
            fields: self.fields.clone(),
            unique: self.unique,
            entries: self.tree.len(),
        }
    }

    /// Total number of indexed document IDs.
    pub fn len(&self) -> usize {
        self.tree.values().map(Vec::len).sum()
    }

    /// Whether the index is empty.
    pub fn is_empty(&self) -> bool {
        self.tree.is_empty()
    }
}

/// Clone a `Bound<&T>` into a `Bound<T>`.
fn bound_cloned<T: Clone>(b: Bound<&T>) -> Bound<T> {
    match b {
        Bound::Included(v) => Bound::Included(v.clone()),
        Bound::Excluded(v) => Bound::Excluded(v.clone()),
        Bound::Unbounded => Bound::Unbounded,
    }
}
