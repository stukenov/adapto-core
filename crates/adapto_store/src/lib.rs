//! # adapto_store
//!
//! An embedded, document-oriented database — SQLite for JSON documents.
//!
//! ```rust,no_run
//! use adapto_store::{AdaptoStore, Query};
//! use serde_json::json;
//!
//! let store = AdaptoStore::open(None).unwrap(); // in-memory
//! let users = store.collection("users");
//! let id = users.insert(json!({"name": "Alice", "age": 30})).unwrap();
//! let doc = users.find_by_id(&id).unwrap().unwrap();
//! assert_eq!(doc.get_str("name"), Some("Alice"));
//! ```

pub mod collection;
pub mod cursor;
pub mod document;
pub mod engine;
pub mod error;
pub mod index;
pub mod query;
pub mod tenant;
pub mod wal;

// Re-exports for a clean public API.
pub use cursor::Cursor;
pub use document::Document;
pub use engine::StoreStats;
pub use error::StoreError;
pub use index::{IndexInfo, IndexKey};
pub use query::{Filter, Query, SortDir, Update, UpdateResult};
pub use tenant::{TenantCollection, TenantScope};

use engine::StorageEngine;
use serde_json::Value;

// ---------------------------------------------------------------------------
// AdaptoStore — the top-level entry point
// ---------------------------------------------------------------------------

/// The main entry point. Open a store, get collections, query documents.
///
/// Thread-safe: cloneable handles share the same underlying engine.
pub struct AdaptoStore {
    engine: StorageEngine,
}

impl AdaptoStore {
    /// Open or create a store.
    ///
    /// - `Some("./data")` — persistent, WAL-backed storage at the given path.
    /// - `None` — in-memory only (fast, no disk I/O).
    pub fn open(path: Option<&str>) -> Result<Self, StoreError> {
        let engine = StorageEngine::open(path)?;
        Ok(Self { engine })
    }

    /// Get (or create) a collection by name.
    pub fn collection(&self, name: &str) -> Collection<'_> {
        self.engine.collection_ensure(name);
        Collection {
            engine: &self.engine,
            name: name.to_string(),
        }
    }

    /// Get a tenant-scoped view of the store.
    pub fn tenant(&self, tenant_id: &str) -> TenantScope<'_> {
        TenantScope::new(&self.engine, tenant_id.to_string())
    }

    /// List all collection names.
    pub fn collections(&self) -> Vec<String> {
        self.engine.collection_names()
    }

    /// Drop a collection entirely.
    pub fn drop_collection(&self, name: &str) -> Result<(), StoreError> {
        self.engine.drop_collection(name)
    }

    /// Force WAL compaction: snapshot current state, truncate log.
    pub fn compact(&self) -> Result<(), StoreError> {
        self.engine.compact()
    }

    /// Aggregate statistics about the store.
    pub fn stats(&self) -> StoreStats {
        self.engine.stats()
    }
}

// ---------------------------------------------------------------------------
// Collection — ergonomic handle for a named collection
// ---------------------------------------------------------------------------

/// A handle to a named collection within the store.
///
/// This is a lightweight reference — creating or dropping it does not
/// allocate storage. The underlying data lives in the engine.
pub struct Collection<'a> {
    engine: &'a StorageEngine,
    name: String,
}

impl<'a> Collection<'a> {
    // -----------------------------------------------------------------------
    // CRUD
    // -----------------------------------------------------------------------

    /// Insert a JSON document. Returns the auto-generated document ID.
    pub fn insert(&self, doc: Value) -> Result<String, StoreError> {
        self.engine.insert(&self.name, doc, None)
    }

    /// Insert multiple documents. Returns all generated IDs.
    pub fn insert_many(&self, docs: Vec<Value>) -> Result<Vec<String>, StoreError> {
        self.engine.insert_many(&self.name, docs, None)
    }

    /// Execute a query and return a cursor over results.
    pub fn find(&self, query: Query) -> Cursor {
        self.engine.find(&self.name, &query, None)
    }

    /// Find a single document matching the query.
    pub fn find_one(&self, query: Query) -> Result<Option<Document>, StoreError> {
        self.engine.find_one(&self.name, &query, None)
    }

    /// Find a document by its ID.
    pub fn find_by_id(&self, id: &str) -> Result<Option<Document>, StoreError> {
        self.engine.find_by_id(&self.name, id, None)
    }

    /// Update all documents matching the query.
    pub fn update(&self, query: Query, update: Update) -> Result<UpdateResult, StoreError> {
        self.engine.update(&self.name, &query, &update, None)
    }

    /// Update a single document by ID.
    pub fn update_by_id(&self, id: &str, update: Update) -> Result<bool, StoreError> {
        self.engine.update_by_id(&self.name, id, &update, None)
    }

    /// Delete all documents matching the query. Returns the count of deleted documents.
    pub fn delete(&self, query: Query) -> Result<u64, StoreError> {
        self.engine.delete(&self.name, &query, None)
    }

    /// Delete a single document by ID.
    pub fn delete_by_id(&self, id: &str) -> Result<bool, StoreError> {
        self.engine.delete_by_id(&self.name, id, None)
    }

    /// Count documents matching the query.
    pub fn count(&self, query: Query) -> Result<u64, StoreError> {
        Ok(self.engine.count(&self.name, &query, None))
    }

    // -----------------------------------------------------------------------
    // Indexes
    // -----------------------------------------------------------------------

    /// Create a single-field index.
    pub fn create_index(&self, field: &str, unique: bool) -> Result<(), StoreError> {
        self.engine.create_index(&self.name, field, unique)
    }

    /// Create a compound index over multiple fields.
    pub fn create_compound_index(
        &self,
        fields: &[&str],
        unique: bool,
    ) -> Result<(), StoreError> {
        self.engine
            .create_compound_index(&self.name, fields, unique)
    }

    /// Drop an index by name.
    pub fn drop_index(&self, name: &str) -> Result<(), StoreError> {
        self.engine.drop_index(&self.name, name)
    }

    /// List metadata for all indexes on this collection.
    pub fn indexes(&self) -> Vec<IndexInfo> {
        self.engine.indexes(&self.name)
    }

    // -----------------------------------------------------------------------
    // Info
    // -----------------------------------------------------------------------

    /// The collection name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Total number of documents (unfiltered).
    pub fn count_all(&self) -> u64 {
        self.engine.count_all(&self.name)
    }
}
