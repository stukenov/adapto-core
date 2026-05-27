pub use adapto_store;

#[cfg(feature = "app")]
pub use adapto_app;
#[cfg(feature = "ui")]
pub use adapto_ui;
#[cfg(feature = "forms")]
pub use adapto_forms;
#[cfg(feature = "auth")]
pub use adapto_auth;
#[cfg(feature = "audit")]
pub use adapto_audit;
#[cfg(feature = "macros")]
pub use adapto_macros;
#[cfg(feature = "live")]
pub use adapto_client_protocol;
#[cfg(feature = "live")]
pub use adapto_ssr;
#[cfg(feature = "live")]
pub use adapto_live;
#[cfg(feature = "live")]
pub use adapto_runtime;
#[cfg(feature = "ai")]
pub use adapto_ai;
#[cfg(feature = "db")]
pub use adapto_db;
#[cfg(feature = "parser")]
pub use adapto_parser;
#[cfg(feature = "parser")]
pub use adapto_compiler;

pub mod prelude {
    // Store (always available)
    pub use adapto_store::{
        AdaptoStore, Collection, Cursor, Document, Query, Filter, SortDir,
        Update, UpdateResult, StoreError, IndexInfo,
    };
    pub use adapto_store::tenant::{TenantScope, TenantCollection};

    // App
    #[cfg(feature = "app")]
    pub use adapto_app::{
        App, ResourceMeta, ActionContext, ActionResult, LayoutConfig, LIVE_JS,
        RequestContext, PageResponse, FallbackResponse, TestClient,
    };

    // UI
    #[cfg(feature = "ui")]
    pub use adapto_ui::{bundle_css, style_tag, html_escape};
    #[cfg(feature = "ui")]
    pub use adapto_ui::components::*;

    // Forms
    #[cfg(feature = "forms")]
    pub use adapto_forms::schema::{FormSchema, FieldSchema, FieldType};
    #[cfg(feature = "forms")]
    pub use adapto_forms::validation::ValidationResult;

    // Auth
    #[cfg(feature = "auth")]
    pub use adapto_auth::jwt::{Claims, encode as jwt_encode, decode as jwt_decode};
    #[cfg(feature = "auth")]
    pub use adapto_auth::password::{hash_password, verify_password};
    #[cfg(feature = "auth")]
    pub use adapto_auth::session_store::{SessionStore, InMemorySessionStore};
    #[cfg(feature = "auth")]
    pub use adapto_auth::middleware::AuthConfig;

    // Audit
    #[cfg(feature = "audit")]
    pub use adapto_audit::event::AuditEvent;
    #[cfg(feature = "audit")]
    pub use adapto_audit::sink::{AuditSink, InMemoryAuditSink};

    // Protocol
    #[cfg(feature = "live")]
    pub use adapto_client_protocol::patch::{PatchMessage, PatchOp, ServerMessage, ServerPayload};

    // Macros
    #[cfg(feature = "macros")]
    pub use adapto_macros::Resource;

    // AI
    #[cfg(feature = "ai")]
    pub use adapto_ai::client::{LlmClient, CompletionRequest, CompletionResponse, MockLlmClient};
    #[cfg(feature = "ai")]
    pub use adapto_ai::prompt::PromptTemplate;
    #[cfg(feature = "ai")]
    pub use adapto_ai::cache::ResponseCache;

    // DB
    #[cfg(feature = "db")]
    pub use adapto_db::pool::DatabasePool;
    #[cfg(feature = "db")]
    pub use adapto_db::runner::MigrationRunner;

    // Common deps
    pub use serde::{Serialize, Deserialize};
    pub use serde_json::{json, Value};
}
