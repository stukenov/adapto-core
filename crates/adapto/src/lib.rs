pub use adapto_store;
pub use adapto_ui;
pub use adapto_forms;
pub use adapto_client_protocol;
pub use adapto_ssr;
pub use adapto_live;
pub use adapto_runtime;
pub use adapto_auth;
pub use adapto_audit;
pub use adapto_macros;
pub use adapto_app;

pub mod prelude {
    // Store
    pub use adapto_store::{AdaptoStore, Collection, Cursor, Document, Query, Filter, SortDir, Update, UpdateResult, StoreError, IndexInfo};
    pub use adapto_store::tenant::{TenantScope, TenantCollection};

    // UI
    pub use adapto_ui::{bundle_css, style_tag, html_escape};
    pub use adapto_ui::components::*;

    // Forms
    pub use adapto_forms::schema::{FormSchema, FieldSchema, FieldType};
    pub use adapto_forms::validation::ValidationResult;

    // Protocol
    pub use adapto_client_protocol::patch::{PatchMessage, PatchOp, ServerMessage, ServerPayload};

    // App builder
    pub use adapto_app::{App, ResourceMeta, ActionContext, ActionResult, LayoutConfig, LIVE_JS};

    // Macros
    pub use adapto_macros::Resource;

    // Re-exports of common deps
    pub use serde::{Serialize, Deserialize};
    pub use serde_json::{json, Value};
}
