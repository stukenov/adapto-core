use adapto_runtime::context::Ctx;
use adapto_runtime::types::*;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// The outcome recorded for an auditable action.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AuditStatus {
    Success,
    Failure(String),
    Denied,
}

/// A single audit log entry capturing who did what, when, and whether it
/// succeeded.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: Uuid,
    pub event: String,
    pub tenant_id: Option<TenantId>,
    pub user_id: Option<UserId>,
    pub route: String,
    pub action: String,
    pub timestamp: DateTime<Utc>,
    pub request_id: RequestId,
    pub metadata: HashMap<String, serde_json::Value>,
    pub status: AuditStatus,
}

impl AuditEvent {
    /// Create a new audit event from a request context.
    ///
    /// The event starts with `AuditStatus::Success` — call `.failure()` or
    /// `.denied()` to override before writing.
    pub fn new(event: &str, ctx: &Ctx, action: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            event: event.to_string(),
            tenant_id: ctx.tenant_id.clone(),
            user_id: ctx.user_id.clone(),
            route: ctx.route.0.clone(),
            action: action.to_string(),
            timestamp: Utc::now(),
            request_id: ctx.request_id.clone(),
            metadata: HashMap::new(),
            status: AuditStatus::Success,
        }
    }

    /// Attach an arbitrary key-value pair to the event metadata.
    pub fn with_metadata(mut self, key: &str, value: serde_json::Value) -> Self {
        self.metadata.insert(key.to_string(), value);
        self
    }

    /// Mark the event as successful (this is the default).
    pub fn success(mut self) -> Self {
        self.status = AuditStatus::Success;
        self
    }

    /// Mark the event as a failure with a reason.
    pub fn failure(mut self, reason: &str) -> Self {
        self.status = AuditStatus::Failure(reason.to_string());
        self
    }

    /// Mark the event as denied (permission / authorization failure).
    pub fn denied(mut self) -> Self {
        self.status = AuditStatus::Denied;
        self
    }
}
