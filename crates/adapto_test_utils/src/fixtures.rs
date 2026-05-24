use adapto_runtime::context::{Ctx, PermissionSet};
use adapto_runtime::types::*;
use uuid::Uuid;

pub fn test_tenant_id() -> TenantId {
    TenantId(Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap())
}

pub fn test_user_id() -> UserId {
    UserId(Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap())
}

pub fn test_session_id() -> SessionId {
    SessionId("test-session-001".to_string())
}

pub fn test_request_id() -> RequestId {
    RequestId(Uuid::parse_str("00000000-0000-0000-0000-000000000003").unwrap())
}

pub fn test_ctx() -> Ctx {
    Ctx {
        user_id: Some(test_user_id()),
        tenant_id: Some(test_tenant_id()),
        request_id: test_request_id(),
        permissions: PermissionSet::new(),
        route: RouteId("/test".to_string()),
        session_id: test_session_id(),
    }
}

pub fn test_ctx_with_permissions(perms: &[&str]) -> Ctx {
    let mut ctx = test_ctx();
    for p in perms {
        ctx.permissions.add(p);
    }
    ctx
}

pub fn test_ctx_anonymous() -> Ctx {
    Ctx {
        user_id: None,
        tenant_id: None,
        request_id: test_request_id(),
        permissions: PermissionSet::new(),
        route: RouteId("/test".to_string()),
        session_id: test_session_id(),
    }
}

pub fn test_ctx_no_tenant() -> Ctx {
    Ctx {
        user_id: Some(test_user_id()),
        tenant_id: None,
        request_id: test_request_id(),
        permissions: PermissionSet::new(),
        route: RouteId("/test".to_string()),
        session_id: test_session_id(),
    }
}
