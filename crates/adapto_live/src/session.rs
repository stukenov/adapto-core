use adapto_runtime::types::*;
use adapto_runtime::state::StateStore;
use adapto_runtime::context::{Ctx, PermissionSet};
use adapto_compiler::ir::ComponentIR;
use adapto_compiler::dependency::DependencyGraph;
use adapto_client_protocol::event::*;
use adapto_client_protocol::patch::*;
use crate::error::LiveError;
use crate::patch::PatchGenerator;
use std::collections::HashMap;

pub struct LiveSession {
    pub id: SessionId,
    pub user_id: Option<UserId>,
    pub tenant_id: Option<TenantId>,
    pub route: RouteId,
    pub component_ir: ComponentIR,
    pub dependency_graph: DependencyGraph,
    pub state: StateStore,
    pub permissions: PermissionSet,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_event_at: chrono::DateTime<chrono::Utc>,
    pub seq: u64,
    action_handlers: HashMap<String, ActionHandler>,
}

/// An action handler is a function that mutates state.
pub type ActionHandler =
    Box<dyn Fn(&mut StateStore, &Ctx, serde_json::Value) -> Result<(), LiveError> + Send + Sync>;

impl LiveSession {
    pub fn new(
        id: SessionId,
        user_id: Option<UserId>,
        tenant_id: Option<TenantId>,
        route: RouteId,
        component_ir: ComponentIR,
        dependency_graph: DependencyGraph,
        permissions: PermissionSet,
    ) -> Self {
        let now = chrono::Utc::now();
        Self {
            id,
            user_id,
            tenant_id,
            route,
            component_ir,
            dependency_graph,
            state: StateStore::new(),
            permissions,
            created_at: now,
            last_event_at: now,
            seq: 0,
            action_handlers: HashMap::new(),
        }
    }

    /// Register an action handler by name.
    pub fn register_handler(&mut self, name: &str, handler: ActionHandler) {
        self.action_handlers.insert(name.to_string(), handler);
    }

    /// Process an incoming client event, dispatching to the registered handler
    /// or falling back to the interpreter for the action body from IR.
    pub fn handle_event(&mut self, event: &ClientEvent) -> Result<PatchMessage, LiveError> {
        self.touch();

        let args = serde_json::Value::Object(
            event
                .payload
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        );

        if let Some(handler) = self.action_handlers.get(&event.handler) {
            let ctx = self.ctx();
            handler(&mut self.state, &ctx, args)?;
        } else if let Some(action_ir) = self.component_ir.actions.iter().find(|a| a.name == event.handler) {
            if let Some(ref perm) = action_ir.permission {
                let ctx = self.ctx();
                ctx.require(perm).map_err(|e| LiveError::PermissionDenied(e.to_string()))?;
            }
            adapto_runtime::interpreter::Interpreter::execute(
                &action_ir.body,
                &mut self.state,
                &args,
            ).map_err(|e| LiveError::Internal(e.to_string()))?;
        } else {
            return Err(LiveError::UnknownHandler(event.handler.clone()));
        }

        Ok(self.generate_patches())
    }

    /// Process a form submission event.
    pub fn handle_form_submit(
        &mut self,
        event: &FormSubmitEvent,
    ) -> Result<PatchMessage, LiveError> {
        self.touch();

        let handler = self
            .action_handlers
            .get(&event.handler)
            .ok_or_else(|| LiveError::UnknownHandler(event.handler.clone()))?;

        let ctx = self.ctx();
        let args = serde_json::Value::Object(
            event
                .form
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        );

        handler(&mut self.state, &ctx, args)?;

        Ok(self.generate_patches())
    }

    /// Generate patches for all dirty state fields, then clear the dirty set.
    pub fn generate_patches(&mut self) -> PatchMessage {
        let seq = self.next_seq();
        let ops = PatchGenerator::generate(
            &self.state,
            &self.dependency_graph,
            &self.component_ir.dynamic_segments,
        );
        self.state.clear_dirty();
        PatchMessage { seq, ops }
    }

    /// Build the per-request context from the current session state.
    pub fn ctx(&self) -> Ctx {
        Ctx {
            user_id: self.user_id.clone(),
            tenant_id: self.tenant_id.clone(),
            request_id: RequestId::default(),
            permissions: self.permissions.clone(),
            route: self.route.clone(),
            session_id: self.id.clone(),
        }
    }

    /// Returns `true` if no event has been received within `timeout`.
    pub fn is_expired(&self, timeout: std::time::Duration) -> bool {
        let elapsed = chrono::Utc::now()
            .signed_duration_since(self.last_event_at)
            .to_std()
            .unwrap_or(std::time::Duration::MAX);
        elapsed > timeout
    }

    /// Stamp the session with the current time.
    fn touch(&mut self) {
        self.last_event_at = chrono::Utc::now();
    }

    /// Increment and return the next sequence number.
    fn next_seq(&mut self) -> u64 {
        self.seq += 1;
        self.seq
    }
}
