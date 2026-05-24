use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::collection::CollectionInner;
use crate::cursor::Cursor;
use crate::document::Document;
use crate::error::StoreError;
use crate::index::IndexInfo;
use crate::query::{Query, Update, UpdateResult};
use crate::wal::{WalEntry, WriteAheadLog};

// ---------------------------------------------------------------------------
// StoreStats
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct StoreStats {
    pub collections: usize,
    pub total_documents: u64,
    pub total_indexes: usize,
    pub wal_size_bytes: u64,
    pub in_memory: bool,
}

// ---------------------------------------------------------------------------
// SnapshotData
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize)]
struct SnapshotCollection {
    name: String,
    docs: Vec<Document>,
    indexes: Vec<(Vec<String>, bool)>,
}

#[derive(Serialize, Deserialize)]
struct SnapshotData {
    collections: Vec<SnapshotCollection>,
}

// ---------------------------------------------------------------------------
// StorageEngine — per-collection RwLock for concurrent access
// ---------------------------------------------------------------------------

/// Each collection gets its own RwLock, so writes to "users" don't block
/// reads from "orders". The registry itself is behind a RwLock that's only
/// write-locked for collection creation/deletion (rare).
pub struct StorageEngine {
    registry: Arc<RwLock<HashMap<String, Arc<RwLock<CollectionInner>>>>>,
    wal: Option<Arc<RwLock<WriteAheadLog>>>,
    #[allow(dead_code)]
    path: Option<PathBuf>,
}

impl StorageEngine {
    pub fn open(path: Option<&str>) -> Result<Self, StoreError> {
        let (wal, base_path) = if let Some(p) = path {
            let base = PathBuf::from(p);
            std::fs::create_dir_all(&base)?;
            let wal_path = base.join("store.wal");
            let wal = WriteAheadLog::open(wal_path.to_str().unwrap())?;
            (Some(Arc::new(RwLock::new(wal))), Some(base))
        } else {
            (None, None)
        };

        let engine = Self {
            registry: Arc::new(RwLock::new(HashMap::new())),
            wal,
            path: base_path,
        };

        engine.replay_wal()?;
        Ok(engine)
    }

    // -----------------------------------------------------------------------
    // Collection management
    // -----------------------------------------------------------------------

    fn get_or_create(&self, name: &str) -> Arc<RwLock<CollectionInner>> {
        // Fast path: read lock
        {
            let reg = self.registry.read().unwrap();
            if let Some(col) = reg.get(name) {
                return Arc::clone(col);
            }
        }
        // Slow path: write lock to insert
        let mut reg = self.registry.write().unwrap();
        reg.entry(name.to_string())
            .or_insert_with(|| Arc::new(RwLock::new(CollectionInner::new(name))))
            .clone()
    }

    pub fn collection_ensure(&self, name: &str) {
        self.get_or_create(name);
    }

    pub fn collection_names(&self) -> Vec<String> {
        self.registry.read().unwrap().keys().cloned().collect()
    }

    pub fn drop_collection(&self, name: &str) -> Result<(), StoreError> {
        let mut reg = self.registry.write().unwrap();
        reg.remove(name)
            .ok_or_else(|| StoreError::CollectionNotFound(name.to_string()))?;
        drop(reg);
        self.wal_append(&WalEntry::DropCollection {
            name: name.to_string(),
        })?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // CRUD — each operation locks only the target collection
    // -----------------------------------------------------------------------

    pub fn insert(
        &self,
        collection: &str,
        doc: Value,
        tenant_id: Option<String>,
    ) -> Result<String, StoreError> {
        let col = self.get_or_create(collection);
        let id = {
            let mut inner = col.write().unwrap();
            inner.insert(doc.clone(), tenant_id.clone())?
        };
        self.wal_append(&WalEntry::Insert {
            collection: collection.to_string(),
            doc_id: id.clone(),
            data: doc,
            tenant_id,
        })?;
        Ok(id)
    }

    pub fn insert_many(
        &self,
        collection: &str,
        docs: Vec<Value>,
        tenant_id: Option<String>,
    ) -> Result<Vec<String>, StoreError> {
        let col = self.get_or_create(collection);
        let mut ids = Vec::with_capacity(docs.len());
        // Hold lock once for all inserts — much faster than per-doc lock
        {
            let mut inner = col.write().unwrap();
            for doc in &docs {
                ids.push(inner.insert(doc.clone(), tenant_id.clone())?);
            }
        }
        // WAL entries
        for (doc, id) in docs.into_iter().zip(ids.iter()) {
            self.wal_append(&WalEntry::Insert {
                collection: collection.to_string(),
                doc_id: id.clone(),
                data: doc,
                tenant_id: tenant_id.clone(),
            })?;
        }
        Ok(ids)
    }

    pub fn find(
        &self,
        collection: &str,
        query: &Query,
        tenant_id: Option<&str>,
    ) -> Cursor {
        let col = self.get_or_create(collection);
        let inner = col.read().unwrap();
        inner.find(query, tenant_id)
    }

    pub fn find_one(
        &self,
        collection: &str,
        query: &Query,
        tenant_id: Option<&str>,
    ) -> Result<Option<Document>, StoreError> {
        let col = self.get_or_create(collection);
        let inner = col.read().unwrap();
        inner.find_one(query, tenant_id)
    }

    pub fn find_by_id(
        &self,
        collection: &str,
        id: &str,
        tenant_id: Option<&str>,
    ) -> Result<Option<Document>, StoreError> {
        let reg = self.registry.read().unwrap();
        match reg.get(collection) {
            Some(col) => {
                let inner = col.read().unwrap();
                inner.find_by_id(id, tenant_id)
            }
            None => Ok(None),
        }
    }

    pub fn update(
        &self,
        collection: &str,
        query: &Query,
        update: &Update,
        tenant_id: Option<&str>,
    ) -> Result<UpdateResult, StoreError> {
        let col = self.get_or_create(collection);
        let (result, affected) = {
            let mut inner = col.write().unwrap();
            let result = inner.update(query, update, tenant_id)?;
            let affected: Vec<Document> = if result.modified > 0 {
                inner.find(query, tenant_id).collect_docs()
            } else {
                Vec::new()
            };
            (result, affected)
        };
        for doc in affected {
            self.wal_append(&WalEntry::Update {
                collection: collection.to_string(),
                doc_id: doc.id.clone(),
                data: serde_json::to_value(&doc).unwrap_or(Value::Null),
            })?;
        }
        Ok(result)
    }

    pub fn update_by_id(
        &self,
        collection: &str,
        id: &str,
        update: &Update,
        tenant_id: Option<&str>,
    ) -> Result<bool, StoreError> {
        let col = self.get_or_create(collection);
        let (changed, doc_snapshot) = {
            let mut inner = col.write().unwrap();
            let changed = inner.update_by_id(id, update, tenant_id)?;
            let snap = if changed {
                inner.find_by_id(id, None)?
            } else {
                None
            };
            (changed, snap)
        };
        if let Some(doc) = doc_snapshot {
            self.wal_append(&WalEntry::Update {
                collection: collection.to_string(),
                doc_id: id.to_string(),
                data: serde_json::to_value(&doc).unwrap_or(Value::Null),
            })?;
        }
        Ok(changed)
    }

    pub fn delete(
        &self,
        collection: &str,
        query: &Query,
        tenant_id: Option<&str>,
    ) -> Result<u64, StoreError> {
        let col = self.get_or_create(collection);
        let deleted_ids: Vec<String>;
        {
            let mut inner = col.write().unwrap();
            // Collect + delete in one lock acquisition
            let ids: Vec<String> = inner
                .find(query, tenant_id)
                .map(|d| d.id.clone())
                .collect();
            let mut count_ids = Vec::new();
            for id in &ids {
                if inner.delete_by_id(id, None).unwrap_or(false) {
                    count_ids.push(id.clone());
                }
            }
            deleted_ids = count_ids;
        }
        for id in &deleted_ids {
            self.wal_append(&WalEntry::Delete {
                collection: collection.to_string(),
                doc_id: id.clone(),
            })?;
        }
        Ok(deleted_ids.len() as u64)
    }

    pub fn delete_by_id(
        &self,
        collection: &str,
        id: &str,
        tenant_id: Option<&str>,
    ) -> Result<bool, StoreError> {
        let reg = self.registry.read().unwrap();
        match reg.get(collection) {
            Some(col) => {
                let deleted = {
                    let mut inner = col.write().unwrap();
                    inner.delete_by_id(id, tenant_id)?
                };
                drop(reg);
                if deleted {
                    self.wal_append(&WalEntry::Delete {
                        collection: collection.to_string(),
                        doc_id: id.to_string(),
                    })?;
                }
                Ok(deleted)
            }
            None => Ok(false),
        }
    }

    pub fn count(
        &self,
        collection: &str,
        query: &Query,
        tenant_id: Option<&str>,
    ) -> u64 {
        let reg = self.registry.read().unwrap();
        match reg.get(collection) {
            Some(col) => {
                let inner = col.read().unwrap();
                inner.count(query, tenant_id)
            }
            None => 0,
        }
    }

    pub fn count_all(&self, collection: &str) -> u64 {
        let reg = self.registry.read().unwrap();
        match reg.get(collection) {
            Some(col) => {
                let inner = col.read().unwrap();
                inner.count_all()
            }
            None => 0,
        }
    }

    // -----------------------------------------------------------------------
    // Index management
    // -----------------------------------------------------------------------

    pub fn create_index(
        &self,
        collection: &str,
        field: &str,
        unique: bool,
    ) -> Result<(), StoreError> {
        let col = self.get_or_create(collection);
        {
            let mut inner = col.write().unwrap();
            inner.create_index(field, unique)?;
        }
        self.wal_append(&WalEntry::CreateIndex {
            collection: collection.to_string(),
            fields: vec![field.to_string()],
            unique,
        })?;
        Ok(())
    }

    pub fn create_compound_index(
        &self,
        collection: &str,
        fields: &[&str],
        unique: bool,
    ) -> Result<(), StoreError> {
        let col = self.get_or_create(collection);
        {
            let mut inner = col.write().unwrap();
            inner.create_compound_index(fields, unique)?;
        }
        self.wal_append(&WalEntry::CreateIndex {
            collection: collection.to_string(),
            fields: fields.iter().map(|s| s.to_string()).collect(),
            unique,
        })?;
        Ok(())
    }

    pub fn drop_index(&self, collection: &str, name: &str) -> Result<(), StoreError> {
        let reg = self.registry.read().unwrap();
        match reg.get(collection) {
            Some(col) => {
                let mut inner = col.write().unwrap();
                inner.drop_index(name)
            }
            None => Err(StoreError::CollectionNotFound(collection.to_string())),
        }
    }

    pub fn indexes(&self, collection: &str) -> Vec<IndexInfo> {
        let reg = self.registry.read().unwrap();
        match reg.get(collection) {
            Some(col) => {
                let inner = col.read().unwrap();
                inner.indexes()
            }
            None => Vec::new(),
        }
    }

    // -----------------------------------------------------------------------
    // Stats & compaction
    // -----------------------------------------------------------------------

    pub fn stats(&self) -> StoreStats {
        let reg = self.registry.read().unwrap();
        let mut total_documents = 0u64;
        let mut total_indexes = 0usize;
        for col in reg.values() {
            let inner = col.read().unwrap();
            total_documents += inner.count_all();
            total_indexes += inner.indexes().len();
        }
        let wal_size_bytes = self
            .wal
            .as_ref()
            .map(|w| w.read().unwrap().size_bytes())
            .unwrap_or(0);

        StoreStats {
            collections: reg.len(),
            total_documents,
            total_indexes,
            wal_size_bytes,
            in_memory: self.wal.is_none(),
        }
    }

    pub fn compact(&self) -> Result<(), StoreError> {
        let Some(ref wal) = self.wal else {
            return Ok(());
        };

        let snapshot = self.build_snapshot();
        let snapshot_val = serde_json::to_value(&snapshot)
            .map_err(|e| StoreError::Serialization(e.to_string()))?;

        let mut wal = wal.write().unwrap();
        wal.compact(snapshot_val)?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // WAL internals
    // -----------------------------------------------------------------------

    fn wal_append(&self, entry: &WalEntry) -> Result<(), StoreError> {
        if let Some(ref wal) = self.wal {
            let mut wal = wal.write().unwrap();
            wal.append(entry)?;
        }
        Ok(())
    }

    fn build_snapshot(&self) -> SnapshotData {
        let reg = self.registry.read().unwrap();
        let collections = reg
            .values()
            .map(|col_lock| {
                let col = col_lock.read().unwrap();
                SnapshotCollection {
                    name: col.name().to_string(),
                    docs: {
                        let query = Query::new();
                        col.find(&query, None).collect_docs()
                    },
                    indexes: col.snapshot_index_defs(),
                }
            })
            .collect();
        SnapshotData { collections }
    }

    fn replay_wal(&self) -> Result<(), StoreError> {
        let Some(ref wal) = self.wal else {
            return Ok(());
        };

        let entries = {
            let wal = wal.read().unwrap();
            wal.replay()?
        };

        for entry in entries {
            match entry {
                WalEntry::Snapshot { data } => {
                    self.apply_snapshot(data)?;
                }
                WalEntry::Insert {
                    collection,
                    doc_id,
                    data,
                    tenant_id,
                } => {
                    let col = self.get_or_create(&collection);
                    let mut inner = col.write().unwrap();
                    let doc = Document {
                        id: doc_id,
                        data,
                        created_at: chrono::Utc::now(),
                        updated_at: chrono::Utc::now(),
                        tenant_id,
                    };
                    let _ = inner.insert_raw(doc);
                }
                WalEntry::Update {
                    collection,
                    doc_id,
                    data,
                } => {
                    let col = self.get_or_create(&collection);
                    let mut inner = col.write().unwrap();
                    if let Ok(full_doc) = serde_json::from_value::<Document>(data.clone()) {
                        let _ = inner.delete_by_id(&doc_id, None);
                        let _ = inner.insert_raw(full_doc);
                    }
                }
                WalEntry::Delete {
                    collection,
                    doc_id,
                } => {
                    let reg = self.registry.read().unwrap();
                    if let Some(col) = reg.get(&collection) {
                        let mut inner = col.write().unwrap();
                        let _ = inner.delete_by_id(&doc_id, None);
                    }
                }
                WalEntry::CreateCollection { name } => {
                    self.get_or_create(&name);
                }
                WalEntry::DropCollection { name } => {
                    let mut reg = self.registry.write().unwrap();
                    reg.remove(&name);
                }
                WalEntry::CreateIndex {
                    collection,
                    fields,
                    unique,
                } => {
                    let col = self.get_or_create(&collection);
                    let mut inner = col.write().unwrap();
                    let field_refs: Vec<&str> = fields.iter().map(String::as_str).collect();
                    let _ = inner.create_compound_index(&field_refs, unique);
                }
            }
        }

        Ok(())
    }

    fn apply_snapshot(&self, data: Value) -> Result<(), StoreError> {
        let snap: SnapshotData = serde_json::from_value(data)
            .map_err(|e| StoreError::WalCorrupted(format!("invalid snapshot: {e}")))?;

        let mut reg = self.registry.write().unwrap();
        reg.clear();

        for sc in snap.collections {
            let mut col = CollectionInner::new(&sc.name);
            for doc in sc.docs {
                col.insert_raw(doc)?;
            }
            for (fields, unique) in &sc.indexes {
                let field_refs: Vec<&str> = fields.iter().map(String::as_str).collect();
                col.create_compound_index(&field_refs, *unique)?;
            }
            reg.insert(sc.name, Arc::new(RwLock::new(col)));
        }

        Ok(())
    }
}
