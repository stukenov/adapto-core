use adapto_store::{AdaptoStore, Query, Update};
use crate::context::Ctx;
use crate::error::RuntimeError;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value, Map};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceIR {
    pub name: String,
    pub collection_name: String,
    pub tenant_scoped: bool,
    pub primary_key: String,
    pub fields: Vec<ResourceFieldIR>,
    pub indexes: Vec<ResourceIndexIR>,
    pub permissions: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceFieldIR {
    pub name: String,
    pub ty: String,
    pub required: bool,
    pub unique: bool,
    pub searchable: bool,
    pub readonly: bool,
    pub default: Option<String>,
    pub min: Option<usize>,
    pub max: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceIndexIR {
    pub field: String,
    pub unique: bool,
}

pub struct ResourceManager {
    ir: ResourceIR,
}

impl ResourceManager {
    pub fn new(ir: ResourceIR) -> Self {
        Self { ir }
    }

    pub fn resource_ir(&self) -> &ResourceIR {
        &self.ir
    }

    pub fn collection_name(&self) -> &str {
        &self.ir.collection_name
    }

    pub fn ensure_indexes(&self, store: &AdaptoStore) -> Result<(), RuntimeError> {
        let col = store.collection(&self.ir.collection_name);
        for idx in &self.ir.indexes {
            col.create_index(&idx.field, idx.unique)
                .map_err(|e| RuntimeError::Internal(format!("index error: {}", e)))?;
        }
        Ok(())
    }

    pub fn list(
        &self,
        store: &AdaptoStore,
        ctx: &Ctx,
        limit: Option<usize>,
        skip: Option<usize>,
    ) -> Result<Vec<Value>, RuntimeError> {
        self.check_permission("read", ctx)?;
        let mut query = Query::new();
        if let Some(l) = limit { query = query.limit(l); }
        if let Some(s) = skip { query = query.skip(s); }

        if self.ir.tenant_scoped {
            let tid = ctx.require_tenant()?;
            let scope = store.tenant(&tid.to_string());
            let tcol = scope.collection(&self.ir.collection_name);
            Ok(tcol.find(query).map(|d| doc_to_value(&d)).collect())
        } else {
            let col = store.collection(&self.ir.collection_name);
            Ok(col.find(query).map(|d| doc_to_value(&d)).collect())
        }
    }

    pub fn get(
        &self,
        store: &AdaptoStore,
        ctx: &Ctx,
        id: &str,
    ) -> Result<Option<Value>, RuntimeError> {
        self.check_permission("read", ctx)?;

        if self.ir.tenant_scoped {
            let tid = ctx.require_tenant()?;
            let scope = store.tenant(&tid.to_string());
            let tcol = scope.collection(&self.ir.collection_name);
            let doc = tcol.find_by_id(id).map_err(|e| RuntimeError::Internal(e.to_string()))?;
            Ok(doc.map(|d| doc_to_value(&d)))
        } else {
            let col = store.collection(&self.ir.collection_name);
            let doc = col.find_by_id(id).map_err(|e| RuntimeError::Internal(e.to_string()))?;
            Ok(doc.map(|d| doc_to_value(&d)))
        }
    }

    pub fn create(
        &self,
        store: &AdaptoStore,
        ctx: &Ctx,
        mut data: Map<String, Value>,
    ) -> Result<Value, RuntimeError> {
        self.check_permission("create", ctx)?;
        self.validate_fields(&data, false)?;

        for field in &self.ir.fields {
            if !data.contains_key(&field.name) {
                if let Some(ref default) = field.default {
                    data.insert(field.name.clone(), parse_default(default, &field.ty));
                }
            }
        }

        let id = if self.ir.tenant_scoped {
            let tid = ctx.require_tenant()?;
            let scope = store.tenant(&tid.to_string());
            let tcol = scope.collection(&self.ir.collection_name);
            tcol.insert(Value::Object(data.clone()))
                .map_err(|e| RuntimeError::Internal(e.to_string()))?
        } else {
            let col = store.collection(&self.ir.collection_name);
            col.insert(Value::Object(data.clone()))
                .map_err(|e| RuntimeError::Internal(e.to_string()))?
        };

        data.insert("id".to_string(), json!(id));
        Ok(Value::Object(data))
    }

    pub fn update(
        &self,
        store: &AdaptoStore,
        ctx: &Ctx,
        id: &str,
        data: Map<String, Value>,
    ) -> Result<bool, RuntimeError> {
        self.check_permission("update", ctx)?;

        for (key, _) in &data {
            if let Some(field) = self.ir.fields.iter().find(|f| f.name == *key) {
                if field.readonly {
                    return Err(RuntimeError::ValidationError(
                        format!("field '{}' is readonly", key),
                    ));
                }
            }
        }

        self.validate_fields(&data, true)?;

        let set_pairs: Vec<(String, Value)> = data.into_iter().collect();
        if set_pairs.is_empty() {
            return Ok(false);
        }

        let upd = Update::Set(set_pairs);

        if self.ir.tenant_scoped {
            let tid = ctx.require_tenant()?;
            let scope = store.tenant(&tid.to_string());
            let tcol = scope.collection(&self.ir.collection_name);
            let modified = tcol.update_by_id(id, upd)
                .map_err(|e| RuntimeError::Internal(e.to_string()))?;
            Ok(modified)
        } else {
            let col = store.collection(&self.ir.collection_name);
            let modified = col.update_by_id(id, upd)
                .map_err(|e| RuntimeError::Internal(e.to_string()))?;
            Ok(modified)
        }
    }

    pub fn delete(
        &self,
        store: &AdaptoStore,
        ctx: &Ctx,
        id: &str,
    ) -> Result<bool, RuntimeError> {
        self.check_permission("delete", ctx)?;

        if self.ir.tenant_scoped {
            let tid = ctx.require_tenant()?;
            let scope = store.tenant(&tid.to_string());
            let tcol = scope.collection(&self.ir.collection_name);
            tcol.delete_by_id(id)
                .map_err(|e| RuntimeError::Internal(e.to_string()))
        } else {
            let col = store.collection(&self.ir.collection_name);
            col.delete_by_id(id)
                .map_err(|e| RuntimeError::Internal(e.to_string()))
        }
    }

    pub fn count(
        &self,
        store: &AdaptoStore,
        ctx: &Ctx,
    ) -> Result<u64, RuntimeError> {
        self.check_permission("read", ctx)?;

        if self.ir.tenant_scoped {
            let tid = ctx.require_tenant()?;
            let scope = store.tenant(&tid.to_string());
            let tcol = scope.collection(&self.ir.collection_name);
            Ok(tcol.count_all())
        } else {
            let col = store.collection(&self.ir.collection_name);
            col.count(Query::new()).map_err(|e| RuntimeError::Internal(e.to_string()))
        }
    }

    fn check_permission(&self, action: &str, ctx: &Ctx) -> Result<(), RuntimeError> {
        if let Some(perm) = self.ir.permissions.get(action) {
            ctx.require(perm)?;
        }
        Ok(())
    }

    fn validate_fields(&self, data: &Map<String, Value>, is_update: bool) -> Result<(), RuntimeError> {
        for field in &self.ir.fields {
            if field.readonly { continue; }
            let value = data.get(&field.name);

            if !is_update && field.required && value.is_none() {
                return Err(RuntimeError::ValidationError(
                    format!("field '{}' is required", field.name),
                ));
            }

            if let Some(val) = value {
                if let Some(s) = val.as_str() {
                    if let Some(min) = field.min {
                        if s.len() < min {
                            return Err(RuntimeError::ValidationError(
                                format!("field '{}' must be at least {} characters", field.name, min),
                            ));
                        }
                    }
                    if let Some(max) = field.max {
                        if s.len() > max {
                            return Err(RuntimeError::ValidationError(
                                format!("field '{}' must be at most {} characters", field.name, max),
                            ));
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

fn doc_to_value(doc: &adapto_store::Document) -> Value {
    json!({
        "id": doc.id,
        "data": doc.data,
        "created_at": doc.created_at.to_rfc3339(),
        "updated_at": doc.updated_at.to_rfc3339(),
    })
}

fn parse_default(default: &str, ty: &str) -> Value {
    match ty.to_lowercase().as_str() {
        "bool" | "boolean" => json!(default == "true"),
        "i32" | "i64" | "integer" | "int" => {
            default.parse::<i64>().map(|n| json!(n)).unwrap_or(json!(0))
        }
        "f32" | "f64" | "float" | "decimal" => {
            default.parse::<f64>().map(|n| json!(n)).unwrap_or(json!(0.0))
        }
        _ => json!(default),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use adapto_store::AdaptoStore;
    use crate::context::{Ctx, PermissionSet};
    use crate::types::*;

    fn test_resource_ir() -> ResourceIR {
        ResourceIR {
            name: "Customer".into(),
            collection_name: "customers".into(),
            tenant_scoped: true,
            primary_key: "id".into(),
            fields: vec![
                ResourceFieldIR {
                    name: "id".into(), ty: "Uuid".into(),
                    required: false, unique: false, searchable: false,
                    readonly: true, default: None, min: None, max: None,
                },
                ResourceFieldIR {
                    name: "name".into(), ty: "String".into(),
                    required: true, unique: false, searchable: true,
                    readonly: false, default: None, min: Some(1), max: Some(120),
                },
                ResourceFieldIR {
                    name: "email".into(), ty: "Email".into(),
                    required: true, unique: true, searchable: false,
                    readonly: false, default: None, min: None, max: None,
                },
                ResourceFieldIR {
                    name: "status".into(), ty: "Enum".into(),
                    required: false, unique: false, searchable: false,
                    readonly: false, default: Some("active".into()), min: None, max: None,
                },
            ],
            indexes: vec![
                ResourceIndexIR { field: "email".into(), unique: true },
                ResourceIndexIR { field: "name".into(), unique: false },
            ],
            permissions: {
                let mut m = HashMap::new();
                m.insert("read".into(), "customers.read".into());
                m.insert("create".into(), "customers.create".into());
                m.insert("update".into(), "customers.update".into());
                m.insert("delete".into(), "customers.delete".into());
                m
            },
        }
    }

    fn test_ctx(perms: &[&str]) -> Ctx {
        let mut ps = PermissionSet::new();
        for p in perms { ps.add(p); }
        Ctx {
            user_id: Some(UserId::default()),
            tenant_id: Some(TenantId::default()),
            request_id: RequestId::default(),
            permissions: ps,
            route: RouteId::from("/customers"),
            session_id: SessionId::from("s1"),
        }
    }

    fn test_ctx_no_tenant(perms: &[&str]) -> Ctx {
        let mut ps = PermissionSet::new();
        for p in perms { ps.add(p); }
        Ctx {
            user_id: Some(UserId::default()),
            tenant_id: None,
            request_id: RequestId::default(),
            permissions: ps,
            route: RouteId::from("/customers"),
            session_id: SessionId::from("s1"),
        }
    }

    #[test]
    fn ensure_indexes_creates_indexes() {
        let store = AdaptoStore::open(None).unwrap();
        let mgr = ResourceManager::new(test_resource_ir());
        mgr.ensure_indexes(&store).unwrap();
    }

    #[test]
    fn create_and_get() {
        let store = AdaptoStore::open(None).unwrap();
        let mgr = ResourceManager::new(test_resource_ir());
        mgr.ensure_indexes(&store).unwrap();
        let ctx = test_ctx(&["customers.read", "customers.create"]);

        let mut data = Map::new();
        data.insert("name".into(), json!("Alice Corp"));
        data.insert("email".into(), json!("alice@test.com"));

        let result = mgr.create(&store, &ctx, data).unwrap();
        let id = result.get("id").unwrap().as_str().unwrap();
        assert!(!id.is_empty());
        assert_eq!(result.get("status"), Some(&json!("active")));

        let fetched = mgr.get(&store, &ctx, id).unwrap();
        assert!(fetched.is_some());
        let doc = fetched.unwrap();
        assert_eq!(doc.get("data").and_then(|d| d.get("name")), Some(&json!("Alice Corp")));
    }

    #[test]
    fn list_with_tenant_isolation() {
        let store = AdaptoStore::open(None).unwrap();
        let mgr = ResourceManager::new(test_resource_ir());
        mgr.ensure_indexes(&store).unwrap();

        let ctx1 = test_ctx(&["customers.read", "customers.create"]);

        let mut data = Map::new();
        data.insert("name".into(), json!("Tenant1 Co"));
        data.insert("email".into(), json!("t1@test.com"));
        mgr.create(&store, &ctx1, data).unwrap();

        let items = mgr.list(&store, &ctx1, None, None).unwrap();
        assert_eq!(items.len(), 1);

        // Different tenant sees 0
        let mut ctx2 = test_ctx(&["customers.read"]);
        ctx2.tenant_id = Some(TenantId::default()); // different random UUID
        let items2 = mgr.list(&store, &ctx2, None, None).unwrap();
        assert_eq!(items2.len(), 0);
    }

    #[test]
    fn create_missing_required_field() {
        let store = AdaptoStore::open(None).unwrap();
        let mgr = ResourceManager::new(test_resource_ir());
        let ctx = test_ctx(&["customers.create"]);

        let mut data = Map::new();
        data.insert("name".into(), json!("Test"));
        let result = mgr.create(&store, &ctx, data);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("email"));
    }

    #[test]
    fn create_field_too_long() {
        let store = AdaptoStore::open(None).unwrap();
        let mgr = ResourceManager::new(test_resource_ir());
        let ctx = test_ctx(&["customers.create"]);

        let mut data = Map::new();
        data.insert("name".into(), json!("x".repeat(200)));
        data.insert("email".into(), json!("test@test.com"));
        let result = mgr.create(&store, &ctx, data);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("at most 120"));
    }

    #[test]
    fn update_document() {
        let store = AdaptoStore::open(None).unwrap();
        let mgr = ResourceManager::new(test_resource_ir());
        mgr.ensure_indexes(&store).unwrap();
        let ctx = test_ctx(&["customers.read", "customers.create", "customers.update"]);

        let mut data = Map::new();
        data.insert("name".into(), json!("Old Name"));
        data.insert("email".into(), json!("old@test.com"));
        let result = mgr.create(&store, &ctx, data).unwrap();
        let id = result.get("id").unwrap().as_str().unwrap().to_string();

        let mut update_data = Map::new();
        update_data.insert("name".into(), json!("New Name"));
        let updated = mgr.update(&store, &ctx, &id, update_data).unwrap();
        assert!(updated);
    }

    #[test]
    fn update_readonly_field_fails() {
        let store = AdaptoStore::open(None).unwrap();
        let mgr = ResourceManager::new(test_resource_ir());
        let ctx = test_ctx(&["customers.update"]);

        let mut data = Map::new();
        data.insert("id".into(), json!("new-id"));
        let result = mgr.update(&store, &ctx, "some-id", data);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("readonly"));
    }

    #[test]
    fn delete_document() {
        let store = AdaptoStore::open(None).unwrap();
        let mgr = ResourceManager::new(test_resource_ir());
        mgr.ensure_indexes(&store).unwrap();
        let ctx = test_ctx(&["customers.read", "customers.create", "customers.delete"]);

        let mut data = Map::new();
        data.insert("name".into(), json!("To Delete"));
        data.insert("email".into(), json!("del@test.com"));
        let result = mgr.create(&store, &ctx, data).unwrap();
        let id = result.get("id").unwrap().as_str().unwrap().to_string();

        let deleted = mgr.delete(&store, &ctx, &id).unwrap();
        assert!(deleted);

        let fetched = mgr.get(&store, &ctx, &id).unwrap();
        assert!(fetched.is_none());
    }

    #[test]
    fn permission_denied_on_read() {
        let store = AdaptoStore::open(None).unwrap();
        let mgr = ResourceManager::new(test_resource_ir());
        let ctx = test_ctx(&[]);

        let result = mgr.list(&store, &ctx, None, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Permission denied"));
    }

    #[test]
    fn permission_denied_on_create() {
        let store = AdaptoStore::open(None).unwrap();
        let mgr = ResourceManager::new(test_resource_ir());
        let ctx = test_ctx(&["customers.read"]);

        let mut data = Map::new();
        data.insert("name".into(), json!("Test"));
        data.insert("email".into(), json!("test@test.com"));
        let result = mgr.create(&store, &ctx, data);
        assert!(result.is_err());
    }

    #[test]
    fn tenant_required_error() {
        let store = AdaptoStore::open(None).unwrap();
        let mgr = ResourceManager::new(test_resource_ir());
        let ctx = test_ctx_no_tenant(&["customers.read"]);

        let result = mgr.list(&store, &ctx, None, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Tenant"));
    }

    #[test]
    fn count_documents() {
        let store = AdaptoStore::open(None).unwrap();
        let mgr = ResourceManager::new(test_resource_ir());
        mgr.ensure_indexes(&store).unwrap();
        let ctx = test_ctx(&["customers.read", "customers.create"]);

        for i in 0..3 {
            let mut data = Map::new();
            data.insert("name".into(), json!(format!("Co {}", i)));
            data.insert("email".into(), json!(format!("co{}@test.com", i)));
            mgr.create(&store, &ctx, data).unwrap();
        }

        let count = mgr.count(&store, &ctx).unwrap();
        assert_eq!(count, 3);
    }

    #[test]
    fn default_value_applied() {
        let store = AdaptoStore::open(None).unwrap();
        let mgr = ResourceManager::new(test_resource_ir());
        mgr.ensure_indexes(&store).unwrap();
        let ctx = test_ctx(&["customers.read", "customers.create"]);

        let mut data = Map::new();
        data.insert("name".into(), json!("Test"));
        data.insert("email".into(), json!("def@test.com"));
        let result = mgr.create(&store, &ctx, data).unwrap();
        assert_eq!(result.get("status"), Some(&json!("active")));
    }

    #[test]
    fn non_tenant_scoped_resource() {
        let mut ir = test_resource_ir();
        ir.tenant_scoped = false;
        ir.permissions.clear();

        let store = AdaptoStore::open(None).unwrap();
        let mgr = ResourceManager::new(ir);
        mgr.ensure_indexes(&store).unwrap();
        let ctx = test_ctx_no_tenant(&[]);

        let mut data = Map::new();
        data.insert("name".into(), json!("Public Co"));
        data.insert("email".into(), json!("pub@test.com"));
        mgr.create(&store, &ctx, data).unwrap();

        let items = mgr.list(&store, &ctx, None, None).unwrap();
        assert_eq!(items.len(), 1);
    }

    #[test]
    fn full_crud_lifecycle() {
        let store = AdaptoStore::open(None).unwrap();
        let mgr = ResourceManager::new(test_resource_ir());
        mgr.ensure_indexes(&store).unwrap();
        let ctx = test_ctx(&[
            "customers.read", "customers.create",
            "customers.update", "customers.delete",
        ]);

        let mut data = Map::new();
        data.insert("name".into(), json!("CRUD Corp"));
        data.insert("email".into(), json!("crud@test.com"));
        let created = mgr.create(&store, &ctx, data).unwrap();
        let id = created.get("id").unwrap().as_str().unwrap().to_string();
        assert_eq!(created.get("status"), Some(&json!("active")));

        let fetched = mgr.get(&store, &ctx, &id).unwrap().unwrap();
        assert_eq!(fetched.get("data").and_then(|d| d.get("name")), Some(&json!("CRUD Corp")));

        let mut upd = Map::new();
        upd.insert("name".into(), json!("Updated Corp"));
        assert!(mgr.update(&store, &ctx, &id, upd).unwrap());

        let items = mgr.list(&store, &ctx, None, None).unwrap();
        assert_eq!(items.len(), 1);

        assert_eq!(mgr.count(&store, &ctx).unwrap(), 1);

        assert!(mgr.delete(&store, &ctx, &id).unwrap());
        assert_eq!(mgr.count(&store, &ctx).unwrap(), 0);
    }

    #[test]
    fn list_with_limit_skip() {
        let store = AdaptoStore::open(None).unwrap();
        let mgr = ResourceManager::new(test_resource_ir());
        mgr.ensure_indexes(&store).unwrap();
        let ctx = test_ctx(&["customers.read", "customers.create"]);

        for i in 0..5 {
            let mut data = Map::new();
            data.insert("name".into(), json!(format!("Co {}", i)));
            data.insert("email".into(), json!(format!("co{}@test.com", i)));
            mgr.create(&store, &ctx, data).unwrap();
        }

        let page = mgr.list(&store, &ctx, Some(2), Some(1)).unwrap();
        assert_eq!(page.len(), 2);
    }
}
