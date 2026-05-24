use crate::error::RuntimeError;
use crate::types::*;
use std::collections::HashSet;

/// Per-request context carrying identity, tenant, permissions, and routing
/// information. Threaded through every handler invocation.
#[derive(Debug, Clone)]
pub struct Ctx {
    pub user_id: Option<UserId>,
    pub tenant_id: Option<TenantId>,
    pub request_id: RequestId,
    pub permissions: PermissionSet,
    pub route: RouteId,
    pub session_id: SessionId,
}

impl Ctx {
    /// Require a specific permission. Returns `Ok(())` if the permission is
    /// present, or a `PermissionDenied` error otherwise.
    pub fn require(&self, permission: &str) -> Result<(), RuntimeError> {
        if self.permissions.has(permission) {
            Ok(())
        } else {
            Err(RuntimeError::PermissionDenied {
                permission: permission.to_string(),
                user_id: self.user_id.clone(),
            })
        }
    }

    /// Require an authenticated user. Returns a reference to the `UserId` or
    /// an `Unauthenticated` error.
    pub fn require_auth(&self) -> Result<&UserId, RuntimeError> {
        self.user_id.as_ref().ok_or(RuntimeError::Unauthenticated)
    }

    /// Require a tenant context. Returns a reference to the `TenantId` or a
    /// `TenantRequired` error.
    pub fn require_tenant(&self) -> Result<&TenantId, RuntimeError> {
        self.tenant_id.as_ref().ok_or(RuntimeError::TenantRequired)
    }
}

// ---------------------------------------------------------------------------
// PermissionSet
// ---------------------------------------------------------------------------

/// An unordered collection of permission strings. Provides efficient
/// membership tests for single permissions and bulk checks.
#[derive(Debug, Clone, Default)]
pub struct PermissionSet {
    permissions: HashSet<String>,
}

impl PermissionSet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, permission: &str) {
        self.permissions.insert(permission.to_string());
    }

    pub fn has(&self, permission: &str) -> bool {
        self.permissions.contains(permission)
    }

    /// Returns `true` if the set contains **at least one** of the given
    /// permissions.
    pub fn has_any(&self, permissions: &[&str]) -> bool {
        permissions.iter().any(|p| self.permissions.contains(*p))
    }

    /// Returns `true` only if **every** given permission is present in the
    /// set.
    pub fn has_all(&self, permissions: &[&str]) -> bool {
        permissions.iter().all(|p| self.permissions.contains(*p))
    }
}
