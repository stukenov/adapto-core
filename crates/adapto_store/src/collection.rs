use std::collections::HashMap;

use chrono::Utc;
use serde_json::Value;

use crate::cursor::Cursor;
use crate::document::{resolve_path, resolve_path_mut, Document};
use crate::error::StoreError;
use std::ops::Bound;

use crate::index::{BTreeIndex, IndexInfo, IndexKey};
use crate::query::{compare_values, Filter, Query, SortDir, Update, UpdateResult};

// ---------------------------------------------------------------------------
// CollectionInner — the mutable state behind a Collection handle
// ---------------------------------------------------------------------------

/// The mutable interior of a collection, protected by the engine's lock.
pub struct CollectionInner {
    name: String,
    docs: HashMap<String, Document>,
    indexes: Vec<BTreeIndex>,
}

impl CollectionInner {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            docs: HashMap::new(),
            indexes: Vec::new(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    // -----------------------------------------------------------------------
    // CRUD
    // -----------------------------------------------------------------------

    /// Insert a single document. Returns the generated document ID.
    pub fn insert(
        &mut self,
        doc_value: Value,
        tenant_id: Option<String>,
    ) -> Result<String, StoreError> {
        let doc = Document::new(doc_value, tenant_id);
        let id = doc.id.clone();

        // Update indexes first (may fail on unique violation).
        for idx in &mut self.indexes {
            let key = idx.key_for(&doc);
            idx.insert(key, &id)?;
        }

        self.docs.insert(id.clone(), doc);
        Ok(id)
    }

    /// Insert a document that was replayed from the WAL (preserves original id/timestamps).
    pub fn insert_raw(&mut self, doc: Document) -> Result<(), StoreError> {
        let id = doc.id.clone();
        for idx in &mut self.indexes {
            let key = idx.key_for(&doc);
            idx.insert(key, &id)?;
        }
        self.docs.insert(id, doc);
        Ok(())
    }

    /// Insert many documents. Returns all generated IDs.
    pub fn insert_many(
        &mut self,
        docs: Vec<Value>,
        tenant_id: Option<String>,
    ) -> Result<Vec<String>, StoreError> {
        let mut ids = Vec::with_capacity(docs.len());
        for doc_value in docs {
            ids.push(self.insert(doc_value, tenant_id.clone())?);
        }
        Ok(ids)
    }

    /// Execute a query and return a cursor over matching documents.
    pub fn find(&self, query: &Query, tenant_id: Option<&str>) -> Cursor {
        let mut results = self.scan(query, tenant_id);

        // Sort
        if let Some(ref sorts) = query.sort {
            results.sort_by(|a, b| {
                for (field, dir) in sorts {
                    let va = resolve_path(&a.data, field);
                    let vb = resolve_path(&b.data, field);
                    let ord = match (va, vb) {
                        (Some(va), Some(vb)) => {
                            compare_values(va, vb).unwrap_or(std::cmp::Ordering::Equal)
                        }
                        (Some(_), None) => std::cmp::Ordering::Less,
                        (None, Some(_)) => std::cmp::Ordering::Greater,
                        (None, None) => std::cmp::Ordering::Equal,
                    };
                    let ord = match dir {
                        SortDir::Asc => ord,
                        SortDir::Desc => ord.reverse(),
                    };
                    if ord != std::cmp::Ordering::Equal {
                        return ord;
                    }
                }
                std::cmp::Ordering::Equal
            });
        }

        // Skip
        if let Some(skip) = query.skip {
            results = results.into_iter().skip(skip).collect();
        }

        // Limit
        if let Some(limit) = query.limit {
            results.truncate(limit);
        }

        // Projection
        if let Some(ref proj_fields) = query.projection {
            for doc in &mut results {
                if let Value::Object(map) = &doc.data {
                    let mut projected = serde_json::Map::new();
                    for field in proj_fields {
                        if let Some(v) = map.get(field) {
                            projected.insert(field.clone(), v.clone());
                        }
                    }
                    doc.data = Value::Object(projected);
                }
            }
        }

        Cursor::new(results)
    }

    /// Find a single document matching the query.
    pub fn find_one(
        &self,
        query: &Query,
        tenant_id: Option<&str>,
    ) -> Result<Option<Document>, StoreError> {
        let limited = Query {
            filter: query.filter.clone(),
            sort: query.sort.clone(),
            limit: Some(1),
            skip: query.skip,
            projection: query.projection.clone(),
        };
        Ok(self.find(&limited, tenant_id).first())
    }

    /// Find a document by its ID.
    pub fn find_by_id(
        &self,
        id: &str,
        tenant_id: Option<&str>,
    ) -> Result<Option<Document>, StoreError> {
        match self.docs.get(id) {
            Some(doc) => {
                if let Some(tid) = tenant_id {
                    if doc.tenant_id.as_deref() != Some(tid) {
                        return Ok(None);
                    }
                }
                Ok(Some(doc.clone()))
            }
            None => Ok(None),
        }
    }

    /// Update documents matching a query. Returns counts of matched/modified.
    pub fn update(
        &mut self,
        query: &Query,
        update: &Update,
        tenant_id: Option<&str>,
    ) -> Result<UpdateResult, StoreError> {
        let matching_ids: Vec<String> = self
            .scan(query, tenant_id)
            .iter()
            .map(|d| d.id.clone())
            .collect();

        let matched = matching_ids.len() as u64;
        let mut modified = 0u64;

        for id in &matching_ids {
            if self.apply_update(id, update)? {
                modified += 1;
            }
        }

        Ok(UpdateResult { matched, modified })
    }

    /// Update a single document by ID.
    pub fn update_by_id(
        &mut self,
        id: &str,
        update: &Update,
        tenant_id: Option<&str>,
    ) -> Result<bool, StoreError> {
        if let Some(doc) = self.docs.get(id) {
            if let Some(tid) = tenant_id {
                if doc.tenant_id.as_deref() != Some(tid) {
                    return Ok(false);
                }
            }
        } else {
            return Ok(false);
        }
        self.apply_update(id, update)
    }

    /// Delete documents matching a query. Returns the count of deleted documents.
    pub fn delete(
        &mut self,
        query: &Query,
        tenant_id: Option<&str>,
    ) -> Result<u64, StoreError> {
        let ids: Vec<String> = self
            .scan(query, tenant_id)
            .iter()
            .map(|d| d.id.clone())
            .collect();

        let count = ids.len() as u64;
        for id in &ids {
            self.remove_doc(id);
        }
        Ok(count)
    }

    /// Delete a single document by ID.
    pub fn delete_by_id(&mut self, id: &str, tenant_id: Option<&str>) -> Result<bool, StoreError> {
        if let Some(doc) = self.docs.get(id) {
            if let Some(tid) = tenant_id {
                if doc.tenant_id.as_deref() != Some(tid) {
                    return Ok(false);
                }
            }
        } else {
            return Ok(false);
        }
        self.remove_doc(id);
        Ok(true)
    }

    /// Count documents matching a query.
    pub fn count(&self, query: &Query, tenant_id: Option<&str>) -> u64 {
        self.scan(query, tenant_id).len() as u64
    }

    /// Total number of documents in the collection.
    pub fn count_all(&self) -> u64 {
        self.docs.len() as u64
    }

    // -----------------------------------------------------------------------
    // Indexes
    // -----------------------------------------------------------------------

    /// Create a single-field index.
    pub fn create_index(&mut self, field: &str, unique: bool) -> Result<(), StoreError> {
        self.create_compound_index(&[field], unique)
    }

    /// Create a compound index over multiple fields.
    pub fn create_compound_index(
        &mut self,
        fields: &[&str],
        unique: bool,
    ) -> Result<(), StoreError> {
        let name = format!("idx_{}", fields.join("_"));

        // Don't create duplicates.
        if self.indexes.iter().any(|i| i.name == name) {
            return Ok(());
        }

        let field_strings: Vec<String> = fields.iter().map(|f| f.to_string()).collect();
        let mut index = BTreeIndex::new(name, field_strings, unique);

        // Backfill existing documents.
        for doc in self.docs.values() {
            let key = index.key_for(doc);
            index.insert(key, &doc.id)?;
        }

        self.indexes.push(index);
        Ok(())
    }

    /// Drop an index by name.
    pub fn drop_index(&mut self, name: &str) -> Result<(), StoreError> {
        let before = self.indexes.len();
        self.indexes.retain(|i| i.name != name);
        if self.indexes.len() == before {
            return Err(StoreError::IndexNotFound(name.to_string()));
        }
        Ok(())
    }

    /// List metadata for all indexes.
    pub fn indexes(&self) -> Vec<IndexInfo> {
        self.indexes.iter().map(BTreeIndex::info).collect()
    }

    // -----------------------------------------------------------------------
    // Serialization for WAL snapshots
    // -----------------------------------------------------------------------

    /// Serialize all documents to a JSON array (for WAL snapshot).
    pub fn snapshot_docs(&self) -> Value {
        let docs: Vec<Value> = self
            .docs
            .values()
            .map(|d| serde_json::to_value(d).unwrap_or(Value::Null))
            .collect();
        Value::Array(docs)
    }

    /// Serialize index definitions (not data — indexes are rebuilt from documents).
    pub fn snapshot_index_defs(&self) -> Vec<(Vec<String>, bool)> {
        self.indexes
            .iter()
            .map(|i| (i.fields.clone(), i.unique))
            .collect()
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    /// Try to use an index for the query filter. Falls back to full scan.
    fn scan(&self, query: &Query, tenant_id: Option<&str>) -> Vec<Document> {
        if let Some(candidates) = self.index_candidates(&query.filter) {
            return candidates
                .into_iter()
                .filter_map(|id| self.docs.get(&id))
                .filter(|doc| {
                    if let Some(tid) = tenant_id {
                        if doc.tenant_id.as_deref() != Some(tid) {
                            return false;
                        }
                    }
                    query.filter.matches(doc)
                })
                .cloned()
                .collect();
        }

        self.docs
            .values()
            .filter(|doc| {
                if let Some(tid) = tenant_id {
                    if doc.tenant_id.as_deref() != Some(tid) {
                        return false;
                    }
                }
                query.filter.matches(doc)
            })
            .cloned()
            .collect()
    }

    /// Check if any index can narrow down candidates for this filter.
    /// Returns Some(doc_ids) if an index can be used, None for full scan.
    fn index_candidates(&self, filter: &Filter) -> Option<Vec<String>> {
        match filter {
            Filter::Eq(field, val) => {
                self.find_single_field_index(field).map(|idx| {
                    let key = IndexKey::Single(val.clone());
                    idx.find_eq(&key).into_iter().map(|s| s.to_string()).collect()
                })
            }
            Filter::In(field, vals) => {
                self.find_single_field_index(field).map(|idx| {
                    let mut ids = Vec::new();
                    for val in vals {
                        let key = IndexKey::Single(val.clone());
                        for id in idx.find_eq(&key) {
                            ids.push(id.to_string());
                        }
                    }
                    ids
                })
            }
            Filter::Gt(field, val) => {
                self.find_single_field_index(field).map(|idx| {
                    let key = IndexKey::Single(val.clone());
                    idx.find_range(
                        std::ops::Bound::Excluded(&key),
                        std::ops::Bound::Unbounded,
                    ).into_iter().map(|s| s.to_string()).collect()
                })
            }
            Filter::Gte(field, val) => {
                self.find_single_field_index(field).map(|idx| {
                    let key = IndexKey::Single(val.clone());
                    idx.find_range(
                        std::ops::Bound::Included(&key),
                        std::ops::Bound::Unbounded,
                    ).into_iter().map(|s| s.to_string()).collect()
                })
            }
            Filter::Lt(field, val) => {
                self.find_single_field_index(field).map(|idx| {
                    let key = IndexKey::Single(val.clone());
                    idx.find_range(
                        std::ops::Bound::Unbounded,
                        std::ops::Bound::Excluded(&key),
                    ).into_iter().map(|s| s.to_string()).collect()
                })
            }
            Filter::Lte(field, val) => {
                self.find_single_field_index(field).map(|idx| {
                    let key = IndexKey::Single(val.clone());
                    idx.find_range(
                        std::ops::Bound::Unbounded,
                        std::ops::Bound::Included(&key),
                    ).into_iter().map(|s| s.to_string()).collect()
                })
            }
            Filter::And(filters) => {
                // Try to merge range filters on same indexed field into a single range scan
                if let Some(ids) = self.try_merged_range(filters) {
                    return Some(ids);
                }
                // Fallback: use index on first filterable sub-expression
                for f in filters {
                    if let Some(ids) = self.index_candidates(f) {
                        return Some(ids);
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// Try to merge multiple range/eq filters on the same indexed field into one range scan.
    fn try_merged_range(&self, filters: &[Filter]) -> Option<Vec<String>> {
        // Collect all range bounds targeting the same field
        let mut field_bounds: std::collections::HashMap<&str, (
            std::ops::Bound<IndexKey>,
            std::ops::Bound<IndexKey>,
        )> = std::collections::HashMap::new();

        for f in filters {
            let (field, lower, upper) = match f {
                Filter::Gt(field, val) => (
                    field.as_str(),
                    std::ops::Bound::Excluded(IndexKey::Single(val.clone())),
                    std::ops::Bound::Unbounded,
                ),
                Filter::Gte(field, val) => (
                    field.as_str(),
                    std::ops::Bound::Included(IndexKey::Single(val.clone())),
                    std::ops::Bound::Unbounded,
                ),
                Filter::Lt(field, val) => (
                    field.as_str(),
                    std::ops::Bound::Unbounded,
                    std::ops::Bound::Excluded(IndexKey::Single(val.clone())),
                ),
                Filter::Lte(field, val) => (
                    field.as_str(),
                    std::ops::Bound::Unbounded,
                    std::ops::Bound::Included(IndexKey::Single(val.clone())),
                ),
                Filter::Eq(field, val) => (
                    field.as_str(),
                    std::ops::Bound::Included(IndexKey::Single(val.clone())),
                    std::ops::Bound::Included(IndexKey::Single(val.clone())),
                ),
                _ => continue,
            };

            let entry = field_bounds.entry(field).or_insert((
                std::ops::Bound::Unbounded,
                std::ops::Bound::Unbounded,
            ));
            // Tighten lower bound
            if !matches!(lower, std::ops::Bound::Unbounded) {
                entry.0 = lower;
            }
            // Tighten upper bound
            if !matches!(upper, std::ops::Bound::Unbounded) {
                entry.1 = upper;
            }
        }

        // Find first field that has both bounds AND an index
        for (field, (lower, upper)) in &field_bounds {
            if matches!(lower, std::ops::Bound::Unbounded) && matches!(upper, std::ops::Bound::Unbounded) {
                continue;
            }
            if let Some(idx) = self.find_single_field_index(field) {
                let ids = idx.find_range(
                    bound_ref(lower),
                    bound_ref(upper),
                ).into_iter().map(|s| s.to_string()).collect();
                return Some(ids);
            }
        }

        None
    }

    fn find_single_field_index(&self, field: &str) -> Option<&BTreeIndex> {
        self.indexes.iter().find(|idx| {
            idx.fields.len() == 1 && idx.fields[0] == field
        })
    }

    /// Apply an update to a document by ID. Returns true if the document was modified.
    fn apply_update(&mut self, id: &str, update: &Update) -> Result<bool, StoreError> {
        let doc = self.docs.get(id).cloned();
        let Some(mut doc) = doc else {
            return Ok(false);
        };

        // Remove from indexes before mutation.
        for idx in &mut self.indexes {
            let key = idx.key_for(&doc);
            idx.remove(&key, id);
        }

        let changed = apply_update_to_value(&mut doc.data, update)?;

        if changed {
            doc.updated_at = Utc::now();
        }

        // Re-insert into indexes.
        for idx in &mut self.indexes {
            let key = idx.key_for(&doc);
            idx.insert(key, id)?;
        }

        self.docs.insert(id.to_string(), doc);
        Ok(changed)
    }

    /// Remove a document and clean up indexes.
    fn remove_doc(&mut self, id: &str) {
        if let Some(doc) = self.docs.remove(id) {
            for idx in &mut self.indexes {
                let key = idx.key_for(&doc);
                idx.remove(&key, id);
            }
        }
    }
}

fn bound_ref(b: &Bound<IndexKey>) -> Bound<&IndexKey> {
    match b {
        Bound::Included(v) => Bound::Included(v),
        Bound::Excluded(v) => Bound::Excluded(v),
        Bound::Unbounded => Bound::Unbounded,
    }
}

// ---------------------------------------------------------------------------
// Update application
// ---------------------------------------------------------------------------

/// Apply an `Update` to a mutable `Value`. Returns `true` if something changed.
fn apply_update_to_value(data: &mut Value, update: &Update) -> Result<bool, StoreError> {
    match update {
        Update::Set(fields) => {
            let mut changed = false;
            for (path, new_val) in fields {
                if let Some((parent, key)) = resolve_path_mut(data, path) {
                    let old = parent.get(&key);
                    if old != Some(new_val) {
                        parent.insert(key, new_val.clone());
                        changed = true;
                    }
                }
            }
            Ok(changed)
        }
        Update::Unset(fields) => {
            let mut changed = false;
            for path in fields {
                if let Some((parent, key)) = resolve_path_mut(data, path) {
                    if parent.remove(&key).is_some() {
                        changed = true;
                    }
                }
            }
            Ok(changed)
        }
        Update::Inc(path, amount) => {
            if let Some((parent, key)) = resolve_path_mut(data, path) {
                let current = parent
                    .get(&key)
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);
                let new_val = current + amount;
                // Preserve integer representation when possible.
                let json_val = if new_val.fract() == 0.0 && new_val.abs() < i64::MAX as f64 {
                    Value::from(new_val as i64)
                } else {
                    serde_json::Number::from_f64(new_val)
                        .map(Value::Number)
                        .unwrap_or(Value::Null)
                };
                parent.insert(key, json_val);
                Ok(true)
            } else {
                Ok(false)
            }
        }
        Update::Push(path, val) => {
            if let Some((parent, key)) = resolve_path_mut(data, path) {
                let arr = parent
                    .entry(&key)
                    .or_insert_with(|| Value::Array(Vec::new()));
                if let Value::Array(ref mut vec) = arr {
                    vec.push(val.clone());
                    Ok(true)
                } else {
                    Err(StoreError::InvalidUpdate(format!(
                        "field `{path}` is not an array"
                    )))
                }
            } else {
                Ok(false)
            }
        }
        Update::Pull(path, val) => {
            if let Some(field_val) = resolve_path(data, path).cloned() {
                if let Value::Array(arr) = field_val {
                    let new_arr: Vec<Value> =
                        arr.into_iter().filter(|v| v != val).collect();
                    if let Some((parent, key)) = resolve_path_mut(data, path) {
                        parent.insert(key, Value::Array(new_arr));
                        return Ok(true);
                    }
                }
            }
            Ok(false)
        }
        Update::Rename(old_path, new_path) => {
            // Read value at old path.
            let val = resolve_path(data, old_path).cloned();
            if let Some(val) = val {
                // Remove old.
                if let Some((parent, key)) = resolve_path_mut(data, old_path) {
                    parent.remove(&key);
                }
                // Set new.
                if let Some((parent, key)) = resolve_path_mut(data, new_path) {
                    parent.insert(key, val);
                }
                Ok(true)
            } else {
                Ok(false)
            }
        }
        Update::Multi(updates) => {
            let mut any_changed = false;
            for u in updates {
                if apply_update_to_value(data, u)? {
                    any_changed = true;
                }
            }
            Ok(any_changed)
        }
    }
}
