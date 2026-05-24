use adapto_client_protocol::event::*;
use adapto_client_protocol::patch::*;
use adapto_runtime::state::StateStore;
use serde_json::Value;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// EventBuilder
// ---------------------------------------------------------------------------

/// Fluent builder for constructing [`ClientMessage`] payloads that wrap a
/// [`ClientEvent`]. Provides preset constructors for common DOM events
/// (click, input, submit) so tests read like specifications.
pub struct EventBuilder {
    session: String,
    component: String,
    event: String,
    handler: String,
    payload: HashMap<String, Value>,
    seq: u64,
}

impl EventBuilder {
    /// Create a click event targeting the given handler.
    pub fn click(handler: &str) -> Self {
        Self {
            session: "test-session-001".to_string(),
            component: "test-component".to_string(),
            event: "click".to_string(),
            handler: handler.to_string(),
            payload: HashMap::new(),
            seq: 1,
        }
    }

    /// Create an input event targeting the given handler, carrying the
    /// specified value in the payload.
    pub fn input(handler: &str, value: &str) -> Self {
        let mut payload = HashMap::new();
        payload.insert("value".to_string(), Value::String(value.to_string()));
        Self {
            session: "test-session-001".to_string(),
            component: "test-component".to_string(),
            event: "input".to_string(),
            handler: handler.to_string(),
            payload,
            seq: 1,
        }
    }

    /// Create a submit event targeting the given handler.
    pub fn submit(handler: &str) -> Self {
        Self {
            session: "test-session-001".to_string(),
            component: "test-component".to_string(),
            event: "submit".to_string(),
            handler: handler.to_string(),
            payload: HashMap::new(),
            seq: 1,
        }
    }

    /// Override the session identifier.
    pub fn session(mut self, session: &str) -> Self {
        self.session = session.to_string();
        self
    }

    /// Override the component identifier.
    pub fn component(mut self, component: &str) -> Self {
        self.component = component.to_string();
        self
    }

    /// Override the sequence number.
    pub fn seq(mut self, seq: u64) -> Self {
        self.seq = seq;
        self
    }

    /// Add a key-value pair to the event payload.
    pub fn payload_field(mut self, key: &str, value: Value) -> Self {
        self.payload.insert(key.to_string(), value);
        self
    }

    /// Consume the builder and produce a [`ClientMessage`].
    pub fn build(self) -> ClientMessage {
        ClientMessage {
            v: PROTOCOL_VERSION,
            payload: ClientPayload::Event(ClientEvent {
                session: self.session,
                component: self.component,
                event: self.event,
                handler: self.handler,
                payload: self.payload,
                seq: self.seq,
            }),
        }
    }
}

// ---------------------------------------------------------------------------
// FormBuilder
// ---------------------------------------------------------------------------

/// Fluent builder for constructing [`ClientMessage`] payloads that wrap a
/// [`FormSubmitEvent`]. Provides a natural `.field()` chain for populating
/// form values.
pub struct FormBuilder {
    session: String,
    component: String,
    handler: String,
    fields: HashMap<String, Value>,
    seq: u64,
}

impl FormBuilder {
    /// Create a new form submission builder for the given handler.
    pub fn new(handler: &str) -> Self {
        Self {
            session: "test-session-001".to_string(),
            component: "test-component".to_string(),
            handler: handler.to_string(),
            fields: HashMap::new(),
            seq: 1,
        }
    }

    /// Add a form field with the given name and value.
    pub fn field(mut self, name: &str, value: impl Into<Value>) -> Self {
        self.fields.insert(name.to_string(), value.into());
        self
    }

    /// Override the session identifier.
    pub fn session(mut self, session: &str) -> Self {
        self.session = session.to_string();
        self
    }

    /// Override the component identifier.
    pub fn component(mut self, component: &str) -> Self {
        self.component = component.to_string();
        self
    }

    /// Override the sequence number.
    pub fn seq(mut self, seq: u64) -> Self {
        self.seq = seq;
        self
    }

    /// Consume the builder and produce a [`ClientMessage`].
    pub fn build(self) -> ClientMessage {
        ClientMessage {
            v: PROTOCOL_VERSION,
            payload: ClientPayload::FormSubmit(FormSubmitEvent {
                session: self.session,
                component: self.component,
                handler: self.handler,
                form: self.fields,
                seq: self.seq,
            }),
        }
    }
}

// ---------------------------------------------------------------------------
// PatchBuilder
// ---------------------------------------------------------------------------

/// Fluent builder for constructing [`ServerMessage`] payloads containing
/// a batch of [`PatchOp`] operations. Mirrors the exact vocabulary of the
/// protocol so test expectations read like wire descriptions.
pub struct PatchBuilder {
    seq: u64,
    ops: Vec<PatchOp>,
}

impl PatchBuilder {
    /// Start a new patch batch for the given sequence number.
    pub fn new(seq: u64) -> Self {
        Self {
            seq,
            ops: Vec::new(),
        }
    }

    /// Append a `ReplaceText` operation.
    pub fn replace_text(mut self, target: &str, value: &str) -> Self {
        self.ops.push(PatchOp::ReplaceText {
            target: target.to_string(),
            value: value.to_string(),
        });
        self
    }

    /// Append a `ReplaceHtml` operation.
    pub fn replace_html(mut self, target: &str, html: &str) -> Self {
        self.ops.push(PatchOp::ReplaceHtml {
            target: target.to_string(),
            html: html.to_string(),
        });
        self
    }

    /// Append a `SetAttr` operation.
    pub fn set_attr(mut self, target: &str, name: &str, value: &str) -> Self {
        self.ops.push(PatchOp::SetAttr {
            target: target.to_string(),
            name: name.to_string(),
            value: value.to_string(),
        });
        self
    }

    /// Append a `RemoveAttr` operation.
    pub fn remove_attr(mut self, target: &str, name: &str) -> Self {
        self.ops.push(PatchOp::RemoveAttr {
            target: target.to_string(),
            name: name.to_string(),
        });
        self
    }

    /// Append an `AddClass` operation.
    pub fn add_class(mut self, target: &str, class: &str) -> Self {
        self.ops.push(PatchOp::AddClass {
            target: target.to_string(),
            class: class.to_string(),
        });
        self
    }

    /// Append a `RemoveClass` operation.
    pub fn remove_class(mut self, target: &str, class: &str) -> Self {
        self.ops.push(PatchOp::RemoveClass {
            target: target.to_string(),
            class: class.to_string(),
        });
        self
    }

    /// Append a `Flash` notification operation.
    pub fn flash(mut self, level: FlashLevel, message: &str) -> Self {
        self.ops.push(PatchOp::Flash {
            level,
            message: message.to_string(),
        });
        self
    }

    /// Append a `Redirect` operation.
    pub fn redirect(mut self, url: &str) -> Self {
        self.ops.push(PatchOp::Redirect {
            url: url.to_string(),
        });
        self
    }

    /// Consume the builder and produce a [`ServerMessage`].
    pub fn build(self) -> ServerMessage {
        ServerMessage::new(ServerPayload::Patch(PatchMessage {
            seq: self.seq,
            ops: self.ops,
        }))
    }
}

// ---------------------------------------------------------------------------
// StateBuilder
// ---------------------------------------------------------------------------

/// Fluent builder for constructing a pre-populated [`StateStore`].
pub struct StateBuilder {
    store: StateStore,
}

impl StateBuilder {
    /// Create an empty state builder.
    pub fn new() -> Self {
        Self {
            store: StateStore::new(),
        }
    }

    /// Set a key-value pair in the store.
    pub fn set(mut self, key: &str, value: impl Into<Value>) -> Self {
        self.store.set(key, value.into());
        self
    }

    /// Consume the builder and return the populated [`StateStore`].
    pub fn build(self) -> StateStore {
        self.store
    }
}

impl Default for StateBuilder {
    fn default() -> Self {
        Self::new()
    }
}
