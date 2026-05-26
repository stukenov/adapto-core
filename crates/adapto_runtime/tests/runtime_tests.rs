use adapto_runtime::config::AdaptoConfig;
use adapto_runtime::context::{Ctx, PermissionSet};
use adapto_runtime::error::RuntimeError;
use adapto_runtime::state::StateStore;
use adapto_runtime::types::*;
use serde_json::json;
use std::collections::HashMap;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// StateStore
// ---------------------------------------------------------------------------

#[test]
fn state_store_set_and_get() {
    let mut store = StateStore::new();
    store.set("count", json!(42));

    assert_eq!(store.get("count"), Some(&json!(42)));
    assert_eq!(store.get("missing"), None);
}

#[test]
fn state_store_dirty_tracking() {
    let mut store = StateStore::new();
    store.set("a", json!(1));
    store.set("b", json!(2));

    assert!(store.is_dirty("a"));
    assert!(store.is_dirty("b"));
    assert!(!store.is_dirty("c"));
    assert_eq!(store.get_dirty().len(), 2);
}

#[test]
fn state_store_clear_dirty() {
    let mut store = StateStore::new();
    store.set("x", json!("hello"));
    assert!(store.is_dirty("x"));

    store.clear_dirty();
    assert!(!store.is_dirty("x"));
    assert!(store.get_dirty().is_empty());
    // Value should still be present after clearing dirty flags.
    assert_eq!(store.get("x"), Some(&json!("hello")));
}

#[test]
fn state_store_merge() {
    let mut store = StateStore::new();
    store.set("existing", json!(1));
    store.clear_dirty();

    let mut other = HashMap::new();
    other.insert("new_key".to_string(), json!(99));
    other.insert("existing".to_string(), json!(2));
    store.merge(other);

    assert_eq!(store.get("new_key"), Some(&json!(99)));
    assert_eq!(store.get("existing"), Some(&json!(2)));
    assert!(store.is_dirty("new_key"));
    assert!(store.is_dirty("existing"));
}

// ---------------------------------------------------------------------------
// PermissionSet
// ---------------------------------------------------------------------------

#[test]
fn permission_set_add_and_has() {
    let mut perms = PermissionSet::new();
    perms.add("read");
    perms.add("write");

    assert!(perms.has("read"));
    assert!(perms.has("write"));
    assert!(!perms.has("delete"));
}

#[test]
fn permission_set_has_any() {
    let mut perms = PermissionSet::new();
    perms.add("read");

    assert!(perms.has_any(&["read", "write"]));
    assert!(!perms.has_any(&["delete", "admin"]));
}

#[test]
fn permission_set_has_all() {
    let mut perms = PermissionSet::new();
    perms.add("read");
    perms.add("write");

    assert!(perms.has_all(&["read", "write"]));
    assert!(!perms.has_all(&["read", "write", "admin"]));
}

// ---------------------------------------------------------------------------
// Ctx
// ---------------------------------------------------------------------------

fn make_ctx(
    user_id: Option<UserId>,
    tenant_id: Option<TenantId>,
    permissions: PermissionSet,
) -> Ctx {
    Ctx {
        user_id,
        tenant_id,
        request_id: RequestId::default(),
        permissions,
        route: RouteId::from("/test"),
        session_id: SessionId::from("sess_123"),
    }
}

#[test]
fn ctx_require_permission_success() {
    let mut perms = PermissionSet::new();
    perms.add("users:read");
    let ctx = make_ctx(Some(UserId::default()), None, perms);

    assert!(ctx.require("users:read").is_ok());
}

#[test]
fn ctx_require_permission_denied() {
    let perms = PermissionSet::new();
    let ctx = make_ctx(Some(UserId::default()), None, perms);

    let err = ctx.require("admin").unwrap_err();
    assert!(matches!(err, RuntimeError::PermissionDenied { .. }));
    assert!(err.to_string().contains("admin"));
}

#[test]
fn ctx_require_auth_success() {
    let uid = UserId::default();
    let ctx = make_ctx(Some(uid.clone()), None, PermissionSet::new());

    let result = ctx.require_auth().unwrap();
    assert_eq!(result, &uid);
}

#[test]
fn ctx_require_auth_failure() {
    let ctx = make_ctx(None, None, PermissionSet::new());

    let err = ctx.require_auth().unwrap_err();
    assert!(matches!(err, RuntimeError::Unauthenticated));
}

#[test]
fn ctx_require_tenant_success() {
    let tid = TenantId::default();
    let ctx = make_ctx(None, Some(tid.clone()), PermissionSet::new());

    let result = ctx.require_tenant().unwrap();
    assert_eq!(result, &tid);
}

#[test]
fn ctx_require_tenant_failure() {
    let ctx = make_ctx(None, None, PermissionSet::new());

    let err = ctx.require_tenant().unwrap_err();
    assert!(matches!(err, RuntimeError::TenantRequired));
}

// ---------------------------------------------------------------------------
// Config defaults
// ---------------------------------------------------------------------------

#[test]
fn config_default_values() {
    let config = AdaptoConfig::default();

    assert_eq!(config.app.name, "adapto_app");
    assert_eq!(config.app.env, "development");
    assert_eq!(config.server.host, "0.0.0.0");
    assert_eq!(config.server.port, 3000);
    assert!(config.security.csrf);
    assert!(config.security.secure_cookies);
    assert_eq!(config.security.content_security_policy, "strict");
    assert_eq!(config.live.websocket_path, "/_adapto/live");
    assert_eq!(config.live.max_sessions_per_user, 10);
    assert_eq!(config.live.event_rate_limit_per_second, 20);
    assert_eq!(config.tenant.mode, "required");
    assert_eq!(config.tenant.strategy, "subdomain");
    assert!(config.ai.default_model.is_none());
    assert!(config.ai.fallback_model.is_none());
}

// ---------------------------------------------------------------------------
// Display implementations
// ---------------------------------------------------------------------------

#[test]
fn session_id_display() {
    let sid = SessionId::from("sess_abc");
    assert_eq!(sid.to_string(), "sess_abc");
}

#[test]
fn user_id_display() {
    let uuid = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
    let uid = UserId(uuid);
    assert_eq!(uid.to_string(), "550e8400-e29b-41d4-a716-446655440000");
}

#[test]
fn tenant_id_display() {
    let uuid = Uuid::parse_str("6ba7b810-9dad-11d1-80b4-00c04fd430c8").unwrap();
    let tid = TenantId(uuid);
    assert_eq!(tid.to_string(), "6ba7b810-9dad-11d1-80b4-00c04fd430c8");
}

#[test]
fn request_id_default_generates_new_uuid() {
    let r1 = RequestId::default();
    let r2 = RequestId::default();
    assert_ne!(r1, r2, "Each default RequestId must be unique");
}

// ---------------------------------------------------------------------------
// Type conversions
// ---------------------------------------------------------------------------

#[test]
fn session_id_from_string() {
    let sid = SessionId::from("test".to_string());
    assert_eq!(sid.0, "test");
}

#[test]
fn route_id_from_str_and_string() {
    let r1 = RouteId::from("/users");
    let r2 = RouteId::from("/users".to_string());
    assert_eq!(r1, r2);
}

#[test]
fn component_id_from_str_and_string() {
    let c1 = ComponentId::from("Counter");
    let c2 = ComponentId::from("Counter".to_string());
    assert_eq!(c1, c2);
    assert_eq!(c1.to_string(), "Counter");
}

#[test]
fn user_id_from_uuid() {
    let uuid = Uuid::new_v4();
    let uid = UserId::from(uuid);
    assert_eq!(uid.0, uuid);
}

#[test]
fn tenant_id_from_uuid() {
    let uuid = Uuid::new_v4();
    let tid = TenantId::from(uuid);
    assert_eq!(tid.0, uuid);
}

// ---------------------------------------------------------------------------
// Type equality and hashing
// ---------------------------------------------------------------------------

#[test]
fn session_id_equality() {
    let a = SessionId::from("s1");
    let b = SessionId::from("s1");
    let c = SessionId::from("s2");
    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn type_ids_usable_as_hash_keys() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    set.insert(SessionId::from("a"));
    set.insert(SessionId::from("b"));
    set.insert(SessionId::from("a"));
    assert_eq!(set.len(), 2);
}

// ---------------------------------------------------------------------------
// StateStore edge cases
// ---------------------------------------------------------------------------

#[test]
fn state_store_overwrite_marks_dirty() {
    let mut store = StateStore::new();
    store.set("x", json!(1));
    store.clear_dirty();

    store.set("x", json!(2));
    assert!(store.is_dirty("x"));
    assert_eq!(store.get("x"), Some(&json!(2)));
}

#[test]
fn state_store_keys() {
    let mut store = StateStore::new();
    store.set("a", json!(1));
    store.set("b", json!(2));
    let mut keys: Vec<&String> = store.keys().collect();
    keys.sort();
    assert_eq!(keys.len(), 2);
}

#[test]
fn state_store_to_map() {
    let mut store = StateStore::new();
    store.set("k", json!("v"));
    let map = store.to_map();
    assert_eq!(map.get("k"), Some(&json!("v")));
}

#[test]
fn state_store_merge_empty() {
    let mut store = StateStore::new();
    store.set("existing", json!(1));
    store.clear_dirty();

    store.merge(HashMap::new());
    assert!(store.get_dirty().is_empty());
    assert_eq!(store.get("existing"), Some(&json!(1)));
}

#[test]
fn state_store_complex_json() {
    let mut store = StateStore::new();
    store.set("user", json!({
        "name": "Alice",
        "roles": ["admin", "user"],
        "meta": { "active": true }
    }));
    let val = store.get("user").unwrap();
    assert_eq!(val["name"], "Alice");
    assert_eq!(val["roles"][0], "admin");
    assert_eq!(val["meta"]["active"], true);
}

// ---------------------------------------------------------------------------
// PermissionSet edge cases
// ---------------------------------------------------------------------------

#[test]
fn permission_set_empty() {
    let perms = PermissionSet::new();
    assert!(!perms.has("anything"));
    assert!(!perms.has_any(&["a", "b"]));
    assert!(perms.has_all(&[]));
}

#[test]
fn permission_set_has_all_empty_list() {
    let mut perms = PermissionSet::new();
    perms.add("x");
    assert!(perms.has_all(&[]));
}

#[test]
fn permission_set_has_any_empty_list() {
    let mut perms = PermissionSet::new();
    perms.add("x");
    assert!(!perms.has_any(&[]));
}

#[test]
fn permission_set_duplicate_add() {
    let mut perms = PermissionSet::new();
    perms.add("read");
    perms.add("read");
    assert!(perms.has("read"));
}

// ---------------------------------------------------------------------------
// Ctx combinations
// ---------------------------------------------------------------------------

#[test]
fn ctx_permission_denied_contains_user_id() {
    let uid = UserId::default();
    let ctx = make_ctx(Some(uid.clone()), None, PermissionSet::new());
    if let Err(RuntimeError::PermissionDenied { user_id, permission }) = ctx.require("admin") {
        assert_eq!(permission, "admin");
        assert_eq!(user_id, Some(uid));
    } else {
        panic!("Expected PermissionDenied");
    }
}

#[test]
fn ctx_permission_denied_without_user() {
    let ctx = make_ctx(None, None, PermissionSet::new());
    if let Err(RuntimeError::PermissionDenied { user_id, .. }) = ctx.require("admin") {
        assert!(user_id.is_none());
    } else {
        panic!("Expected PermissionDenied");
    }
}

#[test]
fn ctx_full_context() {
    let uid = UserId::default();
    let tid = TenantId::default();
    let mut perms = PermissionSet::new();
    perms.add("customers.read");
    perms.add("customers.write");

    let ctx = make_ctx(Some(uid), Some(tid), perms);

    assert!(ctx.require_auth().is_ok());
    assert!(ctx.require_tenant().is_ok());
    assert!(ctx.require("customers.read").is_ok());
    assert!(ctx.require("customers.write").is_ok());
    assert!(ctx.require("customers.delete").is_err());
}

// ---------------------------------------------------------------------------
// Config serialization roundtrip
// ---------------------------------------------------------------------------

#[test]
fn config_serde_roundtrip() {
    let config = AdaptoConfig::default();
    let json = serde_json::to_string(&config).unwrap();
    let parsed: AdaptoConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.server.port, 3000);
    assert_eq!(parsed.app.name, "adapto_app");
}

#[test]
fn config_partial_json() {
    let json = r#"{"app":{"name":"myapp"},"server":{"port":8080}}"#;
    let config: AdaptoConfig = serde_json::from_str(json).unwrap();
    assert_eq!(config.app.name, "myapp");
    assert_eq!(config.server.port, 8080);
    assert_eq!(config.server.host, "0.0.0.0");
    assert!(config.security.csrf);
}

#[test]
fn config_empty_json() {
    let config: AdaptoConfig = serde_json::from_str("{}").unwrap();
    assert_eq!(config.server.port, 3000);
    assert_eq!(config.tenant.mode, "required");
}

#[test]
fn config_ai_with_models() {
    let json = r#"{"ai":{"default_model":"gpt-4","fallback_model":"claude"}}"#;
    let config: AdaptoConfig = serde_json::from_str(json).unwrap();
    assert_eq!(config.ai.default_model.as_deref(), Some("gpt-4"));
    assert_eq!(config.ai.fallback_model.as_deref(), Some("claude"));
}

// ---------------------------------------------------------------------------
// RuntimeError Display
// ---------------------------------------------------------------------------

#[test]
fn error_display_messages() {
    assert_eq!(RuntimeError::Unauthenticated.to_string(), "Unauthenticated");
    assert_eq!(RuntimeError::TenantRequired.to_string(), "Tenant required");
    assert_eq!(RuntimeError::RateLimitExceeded.to_string(), "Rate limit exceeded");
    assert_eq!(
        RuntimeError::SessionNotFound("s1".into()).to_string(),
        "Session not found: s1"
    );
    assert_eq!(
        RuntimeError::UnknownHandler("foo".into()).to_string(),
        "Unknown handler: foo"
    );
    assert_eq!(
        RuntimeError::ValidationError("bad".into()).to_string(),
        "Validation error: bad"
    );
    assert_eq!(
        RuntimeError::Internal("oops".into()).to_string(),
        "Internal error: oops"
    );
    let err = RuntimeError::PermissionDenied {
        permission: "admin".into(),
        user_id: None,
    };
    assert_eq!(err.to_string(), "Permission denied: admin");
}
