use serde_json::Value;

use crate::cursor::Cursor;
use crate::document::Document;
use crate::engine::StorageEngine;
use crate::error::StoreError;
use crate::query::{Query, Update, UpdateResult};

// ---------------------------------------------------------------------------
// TenantScope
// ---------------------------------------------------------------------------

/// A tenant-scoped view of the store.
///
/// All operations performed through this handle automatically filter
/// by the tenant ID — there is no way to accidentally access another
/// tenant's documents.
pub struct TenantScope<'a> {
    engine: &'a StorageEngine,
    tenant_id: String,
}

impl<'a> TenantScope<'a> {
    pub(crate) fn new(engine: &'a StorageEngine, tenant_id: String) -> Self {
        Self { engine, tenant_id }
    }

    /// Get a tenant-scoped collection handle.
    pub fn collection(&self, name: &str) -> TenantCollection<'_> {
        self.engine.collection_ensure(name);
        TenantCollection {
            engine: self.engine,
            collection: name.to_string(),
            tenant_id: self.tenant_id.clone(),
        }
    }
}

// ---------------------------------------------------------------------------
// TenantCollection
// ---------------------------------------------------------------------------

/// A collection handle that transparently scopes every operation to a tenant.
pub struct TenantCollection<'a> {
    engine: &'a StorageEngine,
    collection: String,
    tenant_id: String,
}

impl<'a> TenantCollection<'a> {
    // -----------------------------------------------------------------------
    // CRUD
    // -----------------------------------------------------------------------

    pub fn insert(&self, doc: Value) -> Result<String, StoreError> {
        self.engine
            .insert(&self.collection, doc, Some(self.tenant_id.clone()))
    }

    pub fn insert_many(&self, docs: Vec<Value>) -> Result<Vec<String>, StoreError> {
        self.engine
            .insert_many(&self.collection, docs, Some(self.tenant_id.clone()))
    }

    pub fn find(&self, query: Query) -> Cursor {
        self.engine
            .find(&self.collection, &query, Some(&self.tenant_id))
    }

    pub fn find_one(&self, query: Query) -> Result<Option<Document>, StoreError> {
        self.engine
            .find_one(&self.collection, &query, Some(&self.tenant_id))
    }

    pub fn find_by_id(&self, id: &str) -> Result<Option<Document>, StoreError> {
        self.engine
            .find_by_id(&self.collection, id, Some(&self.tenant_id))
    }

    pub fn update(&self, query: Query, update: Update) -> Result<UpdateResult, StoreError> {
        self.engine
            .update(&self.collection, &query, &update, Some(&self.tenant_id))
    }

    pub fn update_by_id(&self, id: &str, update: Update) -> Result<bool, StoreError> {
        self.engine
            .update_by_id(&self.collection, id, &update, Some(&self.tenant_id))
    }

    pub fn delete(&self, query: Query) -> Result<u64, StoreError> {
        self.engine
            .delete(&self.collection, &query, Some(&self.tenant_id))
    }

    pub fn delete_by_id(&self, id: &str) -> Result<bool, StoreError> {
        self.engine
            .delete_by_id(&self.collection, id, Some(&self.tenant_id))
    }

    pub fn count(&self, query: Query) -> u64 {
        self.engine
            .count(&self.collection, &query, Some(&self.tenant_id))
    }

    pub fn count_all(&self) -> u64 {
        // Note: count_all is unscoped in the engine, so we use count with All filter.
        self.engine
            .count(&self.collection, &Query::new(), Some(&self.tenant_id))
    }

    pub fn name(&self) -> &str {
        &self.collection
    }
}
