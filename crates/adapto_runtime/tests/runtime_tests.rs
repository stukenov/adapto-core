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
