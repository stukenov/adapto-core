//! Tests verifying JSON format compatibility between the Rust protocol types
//! and what the JavaScript client runtime (`adapto-client.js`) expects.
//!
//! The JS client produces/consumes specific JSON shapes. These tests ensure
//! the Rust serde output matches those shapes exactly, catching any accidental
//! renames or structural changes before they break the wire protocol.

use std::collections::HashMap;

use adapto_client_protocol::event::*;
use adapto_client_protocol::message::*;
use adapto_client_protocol::patch::*;
use adapto_client_protocol::session::*;

// ---------------------------------------------------------------------------
// 1. Client event JSON matches JS send format
// ---------------------------------------------------------------------------

/// The JS client sends events as:
/// ```json
/// {
///   "v": 1,
///   "type": "event",
///   "session": "...",
///   "component": "...",
///   "event": "click",
///   "handler": "increment",
///   "payload": { "value": "hello" },
///   "seq": 1
/// }
/// ```
/// Verify the Rust struct serializes to this exact shape.
#[test]
fn js_compat_client_event_json_shape() {
    let mut payload = HashMap::new();
    payload.insert("value".into(), serde_json::json!("hello"));

    let msg = ClientMessage {
        v: 1,
        payload: ClientPayload::Event(ClientEvent {
            session: "sess-001".into(),
            component: "counter".into(),
            event: "click".into(),
            handler: "increment".into(),
            payload,
            seq: 1,
        }),
    };

    let json = serde_json::to_string(&msg).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    // JS reads msg.type to dispatch
    assert_eq!(parsed["type"], "event", "JS dispatches on msg.type === 'event'");

    // JS reads these exact field names
    assert_eq!(parsed["v"], 1, "Protocol version field");
    assert_eq!(parsed["session"], "sess-001", "JS reads msg.session");
    assert_eq!(parsed["component"], "counter", "JS reads msg.component");
    assert_eq!(parsed["event"], "click", "JS reads msg.event");
    assert_eq!(parsed["handler"], "increment", "JS reads msg.handler");
    assert_eq!(parsed["seq"], 1, "JS reads msg.seq");
    assert_eq!(
        parsed["payload"]["value"], "hello",
        "JS reads msg.payload.value"
    );
}

// ---------------------------------------------------------------------------
// 2. Form submit JSON matches JS form format
// ---------------------------------------------------------------------------

/// The JS client sends form submissions as:
/// ```json
/// {
///   "v": 1,
///   "type": "form_submit",
///   "session": "...",
///   "component": "...",
///   "handler": "...",
///   "form": { "name": "Alice", "agree": true },
///   "seq": 5
/// }
/// ```
#[test]
fn js_compat_form_submit_json_shape() {
    let mut form = HashMap::new();
    form.insert("name".into(), serde_json::json!("Alice"));
    form.insert("agree".into(), serde_json::json!(true));
    form.insert("age".into(), serde_json::json!(25));

    let msg = ClientMessage {
        v: 1,
        payload: ClientPayload::FormSubmit(FormSubmitEvent {
            session: "sess-002".into(),
            component: "signup".into(),
            handler: "submit_signup".into(),
            form,
            seq: 5,
        }),
    };

    let json = serde_json::to_string(&msg).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed["type"], "form_submit", "JS dispatches on type === 'form_submit'");
    assert_eq!(parsed["v"], 1);
    assert_eq!(parsed["session"], "sess-002");
    assert_eq!(parsed["component"], "signup");
    assert_eq!(parsed["handler"], "submit_signup");
    assert_eq!(parsed["seq"], 5);

    // JS serializeForm produces mixed types in the form object
    assert_eq!(parsed["form"]["name"], "Alice", "String field");
    assert_eq!(parsed["form"]["agree"], true, "Boolean field (checkbox)");
    assert_eq!(parsed["form"]["age"], 25, "Number field");
}

// ---------------------------------------------------------------------------
// 3. Server patch JSON has correct op tags
// ---------------------------------------------------------------------------

/// The JS client switches on `op.op` to dispatch patch operations.
/// Verify all 15 PatchOp variants produce the correct `"op"` tag value.
#[test]
fn js_compat_all_patch_ops_have_correct_tags() {
    let test_cases: Vec<(PatchOp, &str)> = vec![
        (
            PatchOp::ReplaceText {
                target: "t".into(),
                value: "v".into(),
            },
            "replace_text",
        ),
        (
            PatchOp::ReplaceHtml {
                target: "t".into(),
                html: "<p>h</p>".into(),
            },
            "replace_html",
        ),
        (
            PatchOp::SetAttr {
                target: "t".into(),
                name: "disabled".into(),
                value: "true".into(),
            },
            "set_attr",
        ),
        (
            PatchOp::RemoveAttr {
                target: "t".into(),
                name: "hidden".into(),
            },
            "remove_attr",
        ),
        (
            PatchOp::AddClass {
                target: "t".into(),
                class: "active".into(),
            },
            "add_class",
        ),
        (
            PatchOp::RemoveClass {
                target: "t".into(),
                class: "dim".into(),
            },
            "remove_class",
        ),
        (
            PatchOp::InsertBefore {
                target: "t".into(),
                html: "<div>before</div>".into(),
            },
            "insert_before",
        ),
        (
            PatchOp::InsertAfter {
                target: "t".into(),
                html: "<div>after</div>".into(),
            },
            "insert_after",
        ),
        (
            PatchOp::RemoveNode {
                target: "t".into(),
            },
            "remove_node",
        ),
        (PatchOp::Focus { target: "t".into() }, "focus"),
        (PatchOp::ScrollTo { target: "t".into() }, "scroll_to"),
        (
            PatchOp::Redirect {
                url: "/page".into(),
            },
            "redirect",
        ),
        (
            PatchOp::Flash {
                level: FlashLevel::Success,
                message: "Done".into(),
            },
            "flash",
        ),
        (
            PatchOp::ModalOpen {
                id: "dlg".into(),
                html: "<p>content</p>".into(),
            },
            "modal_open",
        ),
        (PatchOp::ModalClose { id: "dlg".into() }, "modal_close"),
    ];

    for (op, expected_tag) in &test_cases {
        let json = serde_json::to_string(op).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        let actual_tag = parsed["op"]
            .as_str()
            .unwrap_or_else(|| panic!("Missing 'op' field in PatchOp JSON: {}", json));
        assert_eq!(
            actual_tag, *expected_tag,
            "PatchOp tag mismatch for variant: expected '{}', got '{}'\nJSON: {}",
            expected_tag, actual_tag, json
        );
    }

    // Ensure we tested all 15 variants
    assert_eq!(test_cases.len(), 15, "Must test all 15 PatchOp variants");
}

// ---------------------------------------------------------------------------
// 4. All PatchOp variants produce JSON the JS client handles
// ---------------------------------------------------------------------------

/// Verify each PatchOp variant includes exactly the fields the JS client
/// reads when applying that patch. This catches field name mismatches
/// (e.g., Rust uses `class` but JS reads `cls`).
#[test]
fn js_compat_patch_op_field_names() {
    // replace_text: JS reads op.target, op.value
    let json = serde_json::to_value(PatchOp::ReplaceText {
        target: "t1".into(),
        value: "v1".into(),
    })
    .unwrap();
    assert!(json["target"].is_string(), "replace_text must have 'target'");
    assert!(json["value"].is_string(), "replace_text must have 'value'");

    // replace_html: JS reads op.target, op.html
    let json = serde_json::to_value(PatchOp::ReplaceHtml {
        target: "t2".into(),
        html: "<b>h</b>".into(),
    })
    .unwrap();
    assert!(json["target"].is_string(), "replace_html must have 'target'");
    assert!(json["html"].is_string(), "replace_html must have 'html'");

    // set_attr: JS reads op.target, op.name, op.value
    let json = serde_json::to_value(PatchOp::SetAttr {
        target: "t3".into(),
        name: "href".into(),
        value: "/foo".into(),
    })
    .unwrap();
    assert!(json["target"].is_string(), "set_attr must have 'target'");
    assert!(json["name"].is_string(), "set_attr must have 'name'");
    assert!(json["value"].is_string(), "set_attr must have 'value'");

    // remove_attr: JS reads op.target, op.name
    let json = serde_json::to_value(PatchOp::RemoveAttr {
        target: "t4".into(),
        name: "disabled".into(),
    })
    .unwrap();
    assert!(json["target"].is_string(), "remove_attr must have 'target'");
    assert!(json["name"].is_string(), "remove_attr must have 'name'");

    // add_class: JS reads op.target, op["class"]
    let json = serde_json::to_value(PatchOp::AddClass {
        target: "t5".into(),
        class: "active".into(),
    })
    .unwrap();
    assert!(json["target"].is_string(), "add_class must have 'target'");
    assert!(
        json["class"].is_string(),
        "add_class must have 'class' (JS reads op[\"class\"])"
    );

    // remove_class: JS reads op.target, op["class"]
    let json = serde_json::to_value(PatchOp::RemoveClass {
        target: "t6".into(),
        class: "dim".into(),
    })
    .unwrap();
    assert!(json["target"].is_string(), "remove_class must have 'target'");
    assert!(
        json["class"].is_string(),
        "remove_class must have 'class'"
    );

    // insert_before: JS reads op.target, op.html
    let json = serde_json::to_value(PatchOp::InsertBefore {
        target: "t7".into(),
        html: "<li>item</li>".into(),
    })
    .unwrap();
    assert!(json["target"].is_string(), "insert_before must have 'target'");
    assert!(json["html"].is_string(), "insert_before must have 'html'");

    // insert_after: JS reads op.target, op.html
    let json = serde_json::to_value(PatchOp::InsertAfter {
        target: "t8".into(),
        html: "<li>item</li>".into(),
    })
    .unwrap();
    assert!(json["target"].is_string(), "insert_after must have 'target'");
    assert!(json["html"].is_string(), "insert_after must have 'html'");

    // remove_node: JS reads op.target
    let json = serde_json::to_value(PatchOp::RemoveNode {
        target: "t9".into(),
    })
    .unwrap();
    assert!(json["target"].is_string(), "remove_node must have 'target'");

    // focus: JS reads op.target
    let json = serde_json::to_value(PatchOp::Focus {
        target: "t10".into(),
    })
    .unwrap();
    assert!(json["target"].is_string(), "focus must have 'target'");

    // scroll_to: JS reads op.target
    let json = serde_json::to_value(PatchOp::ScrollTo {
        target: "t11".into(),
    })
    .unwrap();
    assert!(json["target"].is_string(), "scroll_to must have 'target'");

    // redirect: JS reads op.url
    let json = serde_json::to_value(PatchOp::Redirect {
        url: "/new".into(),
    })
    .unwrap();
    assert!(json["url"].is_string(), "redirect must have 'url'");

    // flash: JS reads op.level, op.message
    let json = serde_json::to_value(PatchOp::Flash {
        level: FlashLevel::Info,
        message: "msg".into(),
    })
    .unwrap();
    assert!(json["level"].is_string(), "flash must have 'level'");
    assert!(json["message"].is_string(), "flash must have 'message'");

    // modal_open: JS reads op.id, op.html
    let json = serde_json::to_value(PatchOp::ModalOpen {
        id: "dlg".into(),
        html: "<p>body</p>".into(),
    })
    .unwrap();
    assert!(json["id"].is_string(), "modal_open must have 'id'");
    assert!(json["html"].is_string(), "modal_open must have 'html'");

    // modal_close: JS reads op.id
    let json = serde_json::to_value(PatchOp::ModalClose {
        id: "dlg".into(),
    })
    .unwrap();
    assert!(json["id"].is_string(), "modal_close must have 'id'");
}

// ---------------------------------------------------------------------------
// 5. Bootstrap payload JSON has all fields JS expects
// ---------------------------------------------------------------------------

/// The JS client reads the bootstrap payload from a <script> tag and expects:
/// ```json
/// {
///   "session_id": "...",
///   "websocket_url": "ws://...",
///   "csrf_token": "...",
///   "initial_state_hash": "...",
///   "component_tree": [
///     {
///       "id": "c:0",
///       "name": "App",
///       "dynamic_targets": [{ "id": "title", "deps": ["page_title"] }]
///     }
///   ]
/// }
/// ```
#[test]
fn js_compat_bootstrap_payload_fields() {
    let bootstrap = BootstrapPayload {
        session_id: "sess-xyz".into(),
        websocket_url: "ws://localhost:3000/_adapto/live".into(),
        csrf_token: "tok_abc".into(),
        initial_state_hash: "sha256:deadbeef".into(),
        component_tree: vec![ComponentMeta {
            id: "c:0".into(),
            name: "App".into(),
            dynamic_targets: vec![DynamicTarget {
                id: "title".into(),
                deps: vec!["page_title".into()],
            }],
        }],
    };

    let json = serde_json::to_value(&bootstrap).unwrap();

    // JS reads these exact field names during init()
    assert_eq!(
        json["session_id"], "sess-xyz",
        "JS reads payload.session_id"
    );
    assert_eq!(
        json["websocket_url"], "ws://localhost:3000/_adapto/live",
        "JS reads payload.websocket_url"
    );
    assert_eq!(
        json["csrf_token"], "tok_abc",
        "JS reads payload.csrf_token"
    );
    assert_eq!(
        json["initial_state_hash"], "sha256:deadbeef",
        "JS reads payload.initial_state_hash"
    );

    // component_tree structure
    let tree = json["component_tree"].as_array().unwrap();
    assert_eq!(tree.len(), 1);
    assert_eq!(tree[0]["id"], "c:0", "JS reads component.id");
    assert_eq!(tree[0]["name"], "App", "JS reads component.name");

    let targets = tree[0]["dynamic_targets"].as_array().unwrap();
    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0]["id"], "title", "JS reads target.id");
    assert_eq!(
        targets[0]["deps"],
        serde_json::json!(["page_title"]),
        "JS reads target.deps"
    );
}

// ---------------------------------------------------------------------------
// 6. Flash message format matches JS showFlash expectations
// ---------------------------------------------------------------------------

/// The JS client receives flash via two paths:
/// 1. PatchOp::Flash -- with `level` and `message` fields
/// 2. RedirectMessage.flash -- as a [level, message] tuple stored in sessionStorage
///
/// Verify both formats serialize correctly.
#[test]
fn js_compat_flash_message_formats() {
    // Path 1: PatchOp::Flash
    let flash_op = PatchOp::Flash {
        level: FlashLevel::Success,
        message: "Item saved".into(),
    };
    let json = serde_json::to_value(&flash_op).unwrap();
    assert_eq!(json["op"], "flash");
    assert_eq!(
        json["level"], "success",
        "JS showFlash reads op.level as lowercase string"
    );
    assert_eq!(json["message"], "Item saved", "JS showFlash reads op.message");

    // Verify all four FlashLevel values serialize as lowercase strings
    for (level, expected) in [
        (FlashLevel::Success, "success"),
        (FlashLevel::Info, "info"),
        (FlashLevel::Warning, "warning"),
        (FlashLevel::Danger, "danger"),
    ] {
        let json_str = serde_json::to_string(&level).unwrap();
        assert_eq!(
            json_str,
            format!("\"{}\"", expected),
            "FlashLevel::{:?} must serialize as \"{}\"",
            level,
            expected
        );
    }

    // Path 2: RedirectMessage.flash is a tuple (FlashLevel, String)
    // JS reads it from sessionStorage as JSON: ["warning", "Please sign in"]
    let redirect = RedirectMessage {
        url: "/login".into(),
        flash: Some((FlashLevel::Warning, "Please sign in".into())),
    };
    let json = serde_json::to_value(&redirect).unwrap();
    let flash_tuple = json["flash"].as_array().unwrap();
    assert_eq!(flash_tuple.len(), 2, "Flash tuple must have exactly 2 elements");
    assert_eq!(
        flash_tuple[0], "warning",
        "Tuple[0] is the level string"
    );
    assert_eq!(
        flash_tuple[1], "Please sign in",
        "Tuple[1] is the message string"
    );

    // Also verify flash: null when None
    let redirect_no_flash = RedirectMessage {
        url: "/home".into(),
        flash: None,
    };
    let json2 = serde_json::to_value(&redirect_no_flash).unwrap();
    assert!(
        json2["flash"].is_null(),
        "JS checks `if (msg.flash)` -- null is falsy, correct"
    );
}

// ---------------------------------------------------------------------------
// 7. Heartbeat format matches
// ---------------------------------------------------------------------------

/// JS sends:
/// ```json
/// { "v": 1, "type": "heartbeat", "session": "...", "seq": 42 }
/// ```
/// Server responds:
/// ```json
/// { "v": 1, "type": "heartbeat_ack", "seq": 42 }
/// ```
#[test]
fn js_compat_heartbeat_format() {
    // Client heartbeat
    let client_hb = ClientMessage {
        v: 1,
        payload: ClientPayload::Heartbeat(HeartbeatEvent {
            session: "sess-hb".into(),
            seq: 42,
        }),
    };
    let json = serde_json::to_value(&client_hb).unwrap();
    assert_eq!(json["type"], "heartbeat", "JS sends type: 'heartbeat'");
    assert_eq!(json["v"], 1);
    assert_eq!(json["session"], "sess-hb");
    assert_eq!(json["seq"], 42);

    // Verify it can be deserialized back (server receives this)
    let json_str = serde_json::to_string(&client_hb).unwrap();
    let decoded = decode_client_message(&json_str).unwrap();
    assert!(matches!(decoded.payload, ClientPayload::Heartbeat(_)));

    // Server heartbeat_ack
    let server_ack = ServerMessage::new(ServerPayload::HeartbeatAck(HeartbeatAck { seq: 42 }));
    let json = serde_json::to_value(&server_ack).unwrap();
    assert_eq!(
        json["type"], "heartbeat_ack",
        "JS checks msg.type === 'heartbeat_ack'"
    );
    assert_eq!(json["v"], 1);
    assert_eq!(json["seq"], 42, "JS could use seq to measure RTT");
}

// ---------------------------------------------------------------------------
// 8. Redirect format matches
// ---------------------------------------------------------------------------

/// JS handles redirects as:
/// ```json
/// { "v": 1, "type": "redirect", "url": "/login", "flash": ["warning", "msg"] }
/// ```
/// or without flash:
/// ```json
/// { "v": 1, "type": "redirect", "url": "/home", "flash": null }
/// ```
#[test]
fn js_compat_redirect_format() {
    // With flash
    let msg = ServerMessage::new(ServerPayload::Redirect(RedirectMessage {
        url: "/login".into(),
        flash: Some((FlashLevel::Warning, "Session expired".into())),
    }));
    let json = serde_json::to_value(&msg).unwrap();
    assert_eq!(json["type"], "redirect", "JS dispatches on msg.type === 'redirect'");
    assert_eq!(json["v"], 1);
    assert_eq!(json["url"], "/login", "JS reads msg.url for location.href");

    let flash = json["flash"].as_array().unwrap();
    assert_eq!(flash[0], "warning");
    assert_eq!(flash[1], "Session expired");

    // Without flash
    let msg_no_flash = ServerMessage::new(ServerPayload::Redirect(RedirectMessage {
        url: "/home".into(),
        flash: None,
    }));
    let json2 = serde_json::to_value(&msg_no_flash).unwrap();
    assert_eq!(json2["type"], "redirect");
    assert_eq!(json2["url"], "/home");
    assert!(
        json2["flash"].is_null(),
        "JS checks `if (msg.flash)` -- null should be falsy"
    );
}

// ---------------------------------------------------------------------------
// 9. Navigate event JSON matches JS sendNavigate format
// ---------------------------------------------------------------------------

/// JS sends navigation events as:
/// ```json
/// { "v": 1, "type": "navigate", "session": "...", "path": "/dashboard", "seq": 10 }
/// ```
#[test]
fn js_compat_navigate_event_format() {
    let msg = ClientMessage {
        v: 1,
        payload: ClientPayload::Navigate(NavigateEvent {
            session: "sess-nav".into(),
            path: "/dashboard/settings".into(),
            seq: 10,
        }),
    };

    let json = serde_json::to_value(&msg).unwrap();
    assert_eq!(json["type"], "navigate", "JS sends type: 'navigate'");
    assert_eq!(json["v"], 1);
    assert_eq!(json["session"], "sess-nav");
    assert_eq!(json["path"], "/dashboard/settings", "JS reads msg.path");
    assert_eq!(json["seq"], 10);
}

// ---------------------------------------------------------------------------
// 10. Error message format matches JS handleError expectations
// ---------------------------------------------------------------------------

/// JS reads server errors as:
/// ```json
/// { "v": 1, "type": "error", "seq": 7, "code": "...", "message": "..." }
/// ```
/// The `seq` field can be null for session-level errors.
#[test]
fn js_compat_error_message_format() {
    // With seq
    let msg = ServerMessage::new(ServerPayload::Error(ErrorMessage {
        seq: Some(7),
        code: "INVALID_HANDLER".into(),
        message: "Handler 'foo' not found".into(),
    }));
    let json = serde_json::to_value(&msg).unwrap();
    assert_eq!(json["type"], "error", "JS dispatches on msg.type === 'error'");
    assert_eq!(json["code"], "INVALID_HANDLER", "JS reads msg.code");
    assert_eq!(json["message"], "Handler 'foo' not found", "JS reads msg.message");
    assert_eq!(json["seq"], 7, "JS reads msg.seq for correlation");

    // Session-level error (seq = null)
    let session_err = ServerMessage::new(ServerPayload::Error(ErrorMessage {
        seq: None,
        code: "SESSION_EXPIRED".into(),
        message: "Session has expired".into(),
    }));
    let json2 = serde_json::to_value(&session_err).unwrap();
    assert_eq!(json2["type"], "error");
    assert!(json2["seq"].is_null(), "Session-level errors have seq: null");
    assert_eq!(json2["code"], "SESSION_EXPIRED");
}

// ---------------------------------------------------------------------------
// 11. Server patch envelope matches JS handleServerMessage expectations
// ---------------------------------------------------------------------------

/// The JS client expects the full patch message envelope as:
/// ```json
/// { "v": 1, "type": "patch", "seq": 1, "ops": [...] }
/// ```
/// Verify the flattened serde representation matches.
#[test]
fn js_compat_patch_envelope_format() {
    let msg = ServerMessage::new(ServerPayload::Patch(PatchMessage {
        seq: 3,
        ops: vec![
            PatchOp::ReplaceText {
                target: "#count".into(),
                value: "42".into(),
            },
            PatchOp::AddClass {
                target: "#btn".into(),
                class: "pressed".into(),
            },
        ],
    }));

    let json = serde_json::to_value(&msg).unwrap();

    assert_eq!(json["v"], 1, "Protocol version in envelope");
    assert_eq!(json["type"], "patch", "JS dispatches on type === 'patch'");
    assert_eq!(json["seq"], 3, "JS reads msg.seq for applyPatches");

    let ops = json["ops"].as_array().unwrap();
    assert_eq!(ops.len(), 2, "JS iterates msg.ops array");
    assert_eq!(ops[0]["op"], "replace_text");
    assert_eq!(ops[1]["op"], "add_class");
}

// ---------------------------------------------------------------------------
// 12. Input event payload matches JS handleInputEvent format
// ---------------------------------------------------------------------------

/// When JS sends an input event, the payload contains `{ "value": "..." }`.
/// Verify the Rust side can parse this.
#[test]
fn js_compat_input_event_payload() {
    // Simulates what JS sends for an input event
    let raw = r#"{
        "v": 1,
        "type": "event",
        "session": "sess-input",
        "component": "search",
        "event": "input",
        "handler": "on_search",
        "payload": { "value": "hello world" },
        "seq": 3
    }"#;

    let msg = decode_client_message(raw).unwrap();
    match &msg.payload {
        ClientPayload::Event(e) => {
            assert_eq!(e.event, "input");
            assert_eq!(e.handler, "on_search");
            assert_eq!(
                e.payload.get("value").unwrap(),
                &serde_json::json!("hello world")
            );
        }
        _ => panic!("Expected Event payload"),
    }
}

// ---------------------------------------------------------------------------
// 13. Keyboard event payload matches JS handleKeyEvent format
// ---------------------------------------------------------------------------

/// JS sends keyboard events with key metadata:
/// ```json
/// { "key": "Enter", "code": "Enter", "shift": false, "ctrl": true, "alt": false, "meta": false }
/// ```
#[test]
fn js_compat_keyboard_event_payload() {
    let raw = r#"{
        "v": 1,
        "type": "event",
        "session": "sess-key",
        "component": "editor",
        "event": "keydown",
        "handler": "on_key",
        "payload": {
            "key": "Enter",
            "code": "Enter",
            "shift": false,
            "ctrl": true,
            "alt": false,
            "meta": false
        },
        "seq": 7
    }"#;

    let msg = decode_client_message(raw).unwrap();
    match &msg.payload {
        ClientPayload::Event(e) => {
            assert_eq!(e.event, "keydown");
            assert_eq!(e.payload["key"], "Enter");
            assert_eq!(e.payload["code"], "Enter");
            assert_eq!(e.payload["ctrl"], true);
            assert_eq!(e.payload["shift"], false);
        }
        _ => panic!("Expected Event payload"),
    }
}

// ---------------------------------------------------------------------------
// 14. Change event with boolean value (checkbox)
// ---------------------------------------------------------------------------

/// JS sends checkbox changes as `{ "value": true }` or `{ "value": false }`.
#[test]
fn js_compat_change_event_checkbox() {
    let raw = r#"{
        "v": 1,
        "type": "event",
        "session": "sess-chk",
        "component": "settings",
        "event": "change",
        "handler": "toggle_dark_mode",
        "payload": { "value": true },
        "seq": 4
    }"#;

    let msg = decode_client_message(raw).unwrap();
    match &msg.payload {
        ClientPayload::Event(e) => {
            assert_eq!(e.event, "change");
            assert_eq!(e.handler, "toggle_dark_mode");
            assert_eq!(e.payload["value"], true, "Checkbox value is boolean");
        }
        _ => panic!("Expected Event payload"),
    }
}

// ---------------------------------------------------------------------------
// 15. Form with mixed types matches JS serializeForm output
// ---------------------------------------------------------------------------

/// JS serializeForm produces:
/// - text inputs -> string
/// - checkboxes -> boolean
/// - number inputs -> number or null
/// - select-multiple -> array of strings
/// - radio -> string (selected value)
#[test]
fn js_compat_form_mixed_types() {
    let raw = r#"{
        "v": 1,
        "type": "form_submit",
        "session": "sess-form",
        "component": "profile",
        "handler": "save_profile",
        "form": {
            "username": "alice",
            "bio": "Hello world",
            "age": 25,
            "newsletter": true,
            "roles": ["admin", "editor"],
            "theme": "dark"
        },
        "seq": 8
    }"#;

    let msg = decode_client_message(raw).unwrap();
    match &msg.payload {
        ClientPayload::FormSubmit(f) => {
            assert_eq!(f.form["username"], "alice");
            assert_eq!(f.form["age"], 25);
            assert_eq!(f.form["newsletter"], true);
            assert_eq!(f.form["roles"], serde_json::json!(["admin", "editor"]));
            assert_eq!(f.form["theme"], "dark");
        }
        _ => panic!("Expected FormSubmit payload"),
    }
}

// ---------------------------------------------------------------------------
// 16. SessionConfig defaults match JS constants
// ---------------------------------------------------------------------------

/// Verify the Rust SessionConfig defaults match the constants in adapto-client.js.
#[test]
fn js_compat_session_config_matches_js_constants() {
    let config = SessionConfig::default();

    assert_eq!(
        config.heartbeat_interval_ms, 30_000,
        "Must match JS HEARTBEAT_INTERVAL = 30000"
    );
    assert_eq!(
        config.reconnect_max_retries, 10,
        "Must match JS RECONNECT_MAX_RETRIES = 10"
    );
    assert_eq!(
        config.reconnect_backoff_ms, 1_000,
        "Must match JS RECONNECT_BASE_DELAY = 1000"
    );
}
