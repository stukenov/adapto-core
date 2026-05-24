use adapto_runtime::context::PermissionSet;
use adapto_runtime::types::UserId;
use std::collections::{HashMap, HashSet};

/// A named role carrying a set of permission strings.
#[derive(Debug, Clone)]
pub struct Role {
    pub name: String,
    pub permissions: HashSet<String>,
}

/// In-memory role-based access control store.
///
/// Maps roles to permission sets and users to role assignments. Designed for
/// embedding into the server process; a production deployment would back this
/// with a database.
#[derive(Debug, Clone, Default)]
pub struct RbacStore {
    roles: HashMap<String, Role>,
    user_roles: HashMap<UserId, HashSet<String>>,
}

impl RbacStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a role definition. If a role with the same name already
    /// exists, it is replaced.
    pub fn add_role(&mut self, role: Role) {
        self.roles.insert(role.name.clone(), role);
    }

    /// Assign a role to a user. No-op if the assignment already exists.
    pub fn assign_role(&mut self, user_id: &UserId, role_name: &str) {
        self.user_roles
            .entry(user_id.clone())
            .or_default()
            .insert(role_name.to_string());
    }

    /// Remove a role assignment from a user.
    pub fn revoke_role(&mut self, user_id: &UserId, role_name: &str) {
        if let Some(roles) = self.user_roles.get_mut(user_id) {
            roles.remove(role_name);
        }
    }

    /// Compute the effective permission set for a user by aggregating all
    /// permissions from all assigned roles.
    pub fn get_permissions(&self, user_id: &UserId) -> PermissionSet {
        let mut perms = PermissionSet::new();

        if let Some(role_names) = self.user_roles.get(user_id) {
            for role_name in role_names {
                if let Some(role) = self.roles.get(role_name) {
                    for permission in &role.permissions {
                        perms.add(permission);
                    }
                }
            }
        }

        perms
    }

    /// Check whether a user currently holds a specific role.
    pub fn has_role(&self, user_id: &UserId, role_name: &str) -> bool {
        self.user_roles
            .get(user_id)
            .map_or(false, |roles| roles.contains(role_name))
    }
}
