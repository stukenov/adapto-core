use adapto_runtime::types::TenantId;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// InMemoryRepository
// ---------------------------------------------------------------------------

/// A thread-safe, tenant-scoped in-memory data store.
///
/// Provides the same API surface that a generated database repository would
/// expose, making it suitable for tests, prototyping, and offline operation.
#[derive(Debug, Clone)]
pub struct InMemoryRepository<T: Clone> {
    store: Arc<RwLock<HashMap<TenantId, HashMap<Uuid, T>>>>,
}

impl<T: Clone + Send + Sync> InMemoryRepository<T> {
    pub fn new() -> Self {
        Self {
            store: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Return all records belonging to the given tenant.
    pub fn for_tenant(&self, tenant_id: &TenantId) -> Vec<T> {
        let store = self.store.read().unwrap();
        store
            .get(tenant_id)
            .map(|m| m.values().cloned().collect())
            .unwrap_or_default()
    }

    /// Find a single record by tenant and primary key.
    pub fn find(&self, tenant_id: &TenantId, id: &Uuid) -> Option<T> {
        let store = self.store.read().unwrap();
        store.get(tenant_id).and_then(|m| m.get(id).cloned())
    }

    /// Insert a new record. Returns the inserted item.
    pub fn create(&self, tenant_id: &TenantId, id: Uuid, item: T) -> T {
        let mut store = self.store.write().unwrap();
        let tenant_map = store.entry(tenant_id.clone()).or_default();
        tenant_map.insert(id, item.clone());
        item
    }

    /// Update an existing record. Returns `Some(updated)` if the record existed,
    /// `None` otherwise.
    pub fn update(&self, tenant_id: &TenantId, id: &Uuid, item: T) -> Option<T> {
        let mut store = self.store.write().unwrap();
        let tenant_map = store.get_mut(tenant_id)?;
        if tenant_map.contains_key(id) {
            tenant_map.insert(*id, item.clone());
            Some(item)
        } else {
            None
        }
    }

    /// Delete a record. Returns `true` if the record existed and was removed.
    pub fn delete(&self, tenant_id: &TenantId, id: &Uuid) -> bool {
        let mut store = self.store.write().unwrap();
        store
            .get_mut(tenant_id)
            .map(|m| m.remove(id).is_some())
            .unwrap_or(false)
    }

    /// Search within a tenant's records using a predicate.
    pub fn search<F>(&self, tenant_id: &TenantId, predicate: F) -> Vec<T>
    where
        F: Fn(&T) -> bool,
    {
        let store = self.store.read().unwrap();
        store
            .get(tenant_id)
            .map(|m| m.values().filter(|v| predicate(v)).cloned().collect())
            .unwrap_or_default()
    }

    /// Count records belonging to the given tenant.
    pub fn count(&self, tenant_id: &TenantId) -> usize {
        let store = self.store.read().unwrap();
        store.get(tenant_id).map(|m| m.len()).unwrap_or(0)
    }

    /// Return every record across all tenants.
    ///
    /// # Safety
    ///
    /// This bypasses tenant isolation and should only be used in administrative
    /// contexts. In tenant-required routes the compiler should prevent access
    /// to this method.
    pub fn all_unscoped(&self) -> Vec<T> {
        let store = self.store.read().unwrap();
        store.values().flat_map(|m| m.values().cloned()).collect()
    }
}

impl<T: Clone + Send + Sync> Default for InMemoryRepository<T> {
    fn default() -> Self {
        Self::new()
    }
}
