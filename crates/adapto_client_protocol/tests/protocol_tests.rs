use std::collections::HashMap;

use adapto_client_protocol::error::ProtocolError;
use adapto_client_protocol::event::*;
use adapto_client_protocol::message::*;
use adapto_client_protocol::patch::*;
use adapto_client_protocol::session::*;

// ---------------------------------------------------------------------------
// 1. Serialize/deserialize ClientEvent
// ---------------------------------------------------------------------------

#[test]
fn test_client_event_serde() {
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
    let parsed: ClientMessage = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.v, 1);
    match &parsed.payload {
        ClientPayload::Event(e) => {
            assert_eq!(e.session, "sess-001");
            assert_eq!(e.component, "counter");
            assert_eq!(e.event, "click");
            assert_eq!(e.handler, "increment");
            assert_eq!(e.payload.get("value").unwrap(), &serde_json::json!("hello"));
            assert_eq!(e.seq, 1);
        }
        _ => panic!("Expected Event payload"),
    }
}

// ---------------------------------------------------------------------------
// 2. Serialize/deserialize FormSubmitEvent
// ---------------------------------------------------------------------------

#[test]
fn test_form_submit_event_serde() {
    let mut form = HashMap::new();
    form.insert("name".into(), serde_json::json!("Alice"));
    form.insert("email".into(), serde_json::json!("alice@example.com"));

    let msg = ClientMessage {
        v: 1,
        payload: ClientPayload::FormSubmit(FormSubmitEvent {
            session: "sess-002".into(),
            component: "signup_form".into(),
            handler: "submit_signup".into(),
            form,
            seq: 5,
        }),
    };

    let json = serde_json::to_string(&msg).unwrap();
    let parsed: ClientMessage = serde_json::from_str(&json).unwrap();

    match &parsed.payload {
        ClientPayload::FormSubmit(f) => {
            assert_eq!(f.session, "sess-002");
            assert_eq!(f.component, "signup_form");
            assert_eq!(f.handler, "submit_signup");
            assert_eq!(f.form.get("name").unwrap(), &serde_json::json!("Alice"));
            assert_eq!(
                f.form.get("email").unwrap(),
                &serde_json::json!("alice@example.com")
            );
            assert_eq!(f.seq, 5);
        }
        _ => panic!("Expected FormSubmit payload"),
    }
}

// ---------------------------------------------------------------------------
// 3. Serialize/deserialize NavigateEvent
// ---------------------------------------------------------------------------

#[test]
fn test_navigate_event_serde() {
    let msg = ClientMessage {
        v: 1,
        payload: ClientPayload::Navigate(NavigateEvent {
            session: "sess-003".into(),
            path: "/dashboard/settings".into(),
            seq: 10,
        }),
    };

    let json = serde_json::to_string(&msg).unwrap();
    let parsed: ClientMessage = serde_json::from_str(&json).unwrap();

    match &parsed.payload {
        ClientPayload::Navigate(n) => {
            assert_eq!(n.session, "sess-003");
            assert_eq!(n.path, "/dashboard/settings");
            assert_eq!(n.seq, 10);
        }
        _ => panic!("Expected Navigate payload"),
    }
}

// ---------------------------------------------------------------------------
// 4. Serialize/deserialize HeartbeatEvent
// ---------------------------------------------------------------------------

#[test]
fn test_heartbeat_event_serde() {
    let msg = ClientMessage {
        v: 1,
        payload: ClientPayload::Heartbeat(HeartbeatEvent {
            session: "sess-004".into(),
            seq: 42,
        }),
    };

    let json = serde_json::to_string(&msg).unwrap();
    let parsed: ClientMessage = serde_json::from_str(&json).unwrap();

    match &parsed.payload {
        ClientPayload::Heartbeat(h) => {
            assert_eq!(h.session, "sess-004");
            assert_eq!(h.seq, 42);
        }
        _ => panic!("Expected Heartbeat payload"),
    }
}

// ---------------------------------------------------------------------------
// 5. Serialize/deserialize PatchMessage with ReplaceText
// ---------------------------------------------------------------------------

#[test]
fn test_patch_replace_text_serde() {
    let msg = ServerMessage::new(ServerPayload::Patch(PatchMessage {
        seq: 1,
        ops: vec![PatchOp::ReplaceText {
            target: "c:counter#count".into(),
            value: "42".into(),
        }],
    }));

    let json = serde_json::to_string(&msg).unwrap();
    let parsed: ServerMessage = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.v, 1);
    match &parsed.payload {
        ServerPayload::Patch(p) => {
            assert_eq!(p.seq, 1);
            assert_eq!(p.ops.len(), 1);
            match &p.ops[0] {
                PatchOp::ReplaceText { target, value } => {
                    assert_eq!(target, "c:counter#count");
                    assert_eq!(value, "42");
                }
                _ => panic!("Expected ReplaceText op"),
            }
        }
        _ => panic!("Expected Patch payload"),
    }
}

// ---------------------------------------------------------------------------
// 6. Serialize/deserialize PatchMessage with ReplaceHtml
// ---------------------------------------------------------------------------

#[test]
fn test_patch_replace_html_serde() {
    let msg = ServerMessage::new(ServerPayload::Patch(PatchMessage {
        seq: 2,
        ops: vec![PatchOp::ReplaceHtml {
            target: "c:list#items".into(),
            html: "<li>Item 1</li><li>Item 2</li>".into(),
        }],
    }));

    let json = serde_json::to_string(&msg).unwrap();
    let parsed: ServerMessage = serde_json::from_str(&json).unwrap();

    match &parsed.payload {
        ServerPayload::Patch(p) => {
            assert_eq!(p.ops.len(), 1);
            match &p.ops[0] {
                PatchOp::ReplaceHtml { target, html } => {
                    assert_eq!(target, "c:list#items");
                    assert_eq!(html, "<li>Item 1</li><li>Item 2</li>");
                }
                _ => panic!("Expected ReplaceHtml op"),
            }
        }
        _ => panic!("Expected Patch payload"),
    }
}

// ---------------------------------------------------------------------------
// 7. Serialize/deserialize PatchMessage with multiple ops
// ---------------------------------------------------------------------------

#[test]
fn test_patch_multiple_ops_serde() {
    let msg = ServerMessage::new(ServerPayload::Patch(PatchMessage {
        seq: 3,
        ops: vec![
            PatchOp::ReplaceText {
                target: "#title".into(),
                value: "Updated Title".into(),
            },
            PatchOp::AddClass {
                target: "#container".into(),
                class: "active".into(),
            },
            PatchOp::RemoveNode {
                target: "#old-banner".into(),
            },
        ],
    }));

    let json = serde_json::to_string(&msg).unwrap();
    let parsed: ServerMessage = serde_json::from_str(&json).unwrap();

    match &parsed.payload {
        ServerPayload::Patch(p) => {
            assert_eq!(p.seq, 3);
            assert_eq!(p.ops.len(), 3);

            assert!(matches!(&p.ops[0], PatchOp::ReplaceText { .. }));
            assert!(matches!(&p.ops[1], PatchOp::AddClass { .. }));
            assert!(matches!(&p.ops[2], PatchOp::RemoveNode { .. }));
        }
        _ => panic!("Expected Patch payload"),
    }
}

// ---------------------------------------------------------------------------
// 8. Serialize/deserialize all PatchOp variants
// ---------------------------------------------------------------------------

#[test]
fn test_all_patch_op_variants_serde() {
    let all_ops = vec![
        PatchOp::ReplaceText {
            target: "#a".into(),
            value: "text".into(),
        },
        PatchOp::ReplaceHtml {
            target: "#b".into(),
            html: "<p>html</p>".into(),
        },
        PatchOp::SetAttr {
            target: "#c".into(),
            name: "disabled".into(),
            value: "true".into(),
        },
        PatchOp::RemoveAttr {
            target: "#d".into(),
            name: "hidden".into(),
        },
        PatchOp::AddClass {
            target: "#e".into(),
            class: "highlight".into(),
        },
        PatchOp::RemoveClass {
            target: "#f".into(),
            class: "dim".into(),
        },
        PatchOp::InsertBefore {
            target: "#g".into(),
            html: "<div>before</div>".into(),
        },
        PatchOp::InsertAfter {
            target: "#h".into(),
            html: "<div>after</div>".into(),
        },
        PatchOp::RemoveNode {
            target: "#i".into(),
        },
        PatchOp::Focus {
            target: "#j".into(),
        },
        PatchOp::ScrollTo {
            target: "#k".into(),
        },
        PatchOp::Redirect {
            url: "/new-page".into(),
        },
        PatchOp::Flash {
            level: FlashLevel::Success,
            message: "Saved!".into(),
        },
        PatchOp::ModalOpen {
            id: "confirm-dialog".into(),
            html: "<div>Are you sure?</div>".into(),
        },
        PatchOp::ModalClose {
            id: "confirm-dialog".into(),
        },
    ];

    let msg = ServerMessage::new(ServerPayload::Patch(PatchMessage { seq: 99, ops: all_ops }));

    let json = serde_json::to_string(&msg).unwrap();
    let parsed: ServerMessage = serde_json::from_str(&json).unwrap();

    match &parsed.payload {
        ServerPayload::Patch(p) => {
            assert_eq!(p.ops.len(), 15);
        }
        _ => panic!("Expected Patch payload"),
    }
}

// ---------------------------------------------------------------------------
// 9. Serialize/deserialize ErrorMessage
// ---------------------------------------------------------------------------

#[test]
fn test_error_message_serde() {
    let msg = ServerMessage::new(ServerPayload::Error(ErrorMessage {
        seq: Some(7),
        code: "INVALID_HANDLER".into(),
        message: "Handler 'foo' not found on component 'bar'".into(),
    }));

    let json = serde_json::to_string(&msg).unwrap();
    let parsed: ServerMessage = serde_json::from_str(&json).unwrap();

    match &parsed.payload {
        ServerPayload::Error(e) => {
            assert_eq!(e.seq, Some(7));
            assert_eq!(e.code, "INVALID_HANDLER");
            assert!(e.message.contains("foo"));
        }
        _ => panic!("Expected Error payload"),
    }

    // Also test with seq = None (session-level error)
    let session_err = ServerMessage::new(ServerPayload::Error(ErrorMessage {
        seq: None,
        code: "SESSION_EXPIRED".into(),
        message: "Session has expired".into(),
    }));

    let json2 = serde_json::to_string(&session_err).unwrap();
    let parsed2: ServerMessage = serde_json::from_str(&json2).unwrap();

    match &parsed2.payload {
        ServerPayload::Error(e) => {
            assert_eq!(e.seq, None);
            assert_eq!(e.code, "SESSION_EXPIRED");
        }
        _ => panic!("Expected Error payload"),
    }
}

// ---------------------------------------------------------------------------
// 10. Serialize/deserialize RedirectMessage
// ---------------------------------------------------------------------------

#[test]
fn test_redirect_message_serde() {
    let msg = ServerMessage::new(ServerPayload::Redirect(RedirectMessage {
        url: "/login".into(),
        flash: Some((FlashLevel::Warning, "Please sign in".into())),
    }));

    let json = serde_json::to_string(&msg).unwrap();
    let parsed: ServerMessage = serde_json::from_str(&json).unwrap();

    match &parsed.payload {
        ServerPayload::Redirect(r) => {
            assert_eq!(r.url, "/login");
            let (level, text) = r.flash.as_ref().unwrap();
            assert_eq!(*level, FlashLevel::Warning);
            assert_eq!(text, "Please sign in");
        }
        _ => panic!("Expected Redirect payload"),
    }

    // Without flash
    let msg_no_flash = ServerMessage::new(ServerPayload::Redirect(RedirectMessage {
        url: "/home".into(),
        flash: None,
    }));

    let json2 = serde_json::to_string(&msg_no_flash).unwrap();
    let parsed2: ServerMessage = serde_json::from_str(&json2).unwrap();

    match &parsed2.payload {
        ServerPayload::Redirect(r) => {
            assert_eq!(r.url, "/home");
            assert!(r.flash.is_none());
        }
        _ => panic!("Expected Redirect payload"),
    }
}

// ---------------------------------------------------------------------------
// 11. Serialize/deserialize BootstrapPayload
// ---------------------------------------------------------------------------

#[test]
fn test_bootstrap_payload_serde() {
    let bootstrap = BootstrapPayload {
        session_id: "sess-abc-123".into(),
        websocket_url: "ws://localhost:3000/live/ws".into(),
        csrf_token: "tok_xyz".into(),
        initial_state_hash: "sha256:abcdef".into(),
        component_tree: vec![
            ComponentMeta {
                id: "c:0".into(),
                name: "App".into(),
                dynamic_targets: vec![DynamicTarget {
                    id: "title".into(),
                    deps: vec!["page_title".into()],
                }],
            },
            ComponentMeta {
                id: "c:1".into(),
                name: "Counter".into(),
                dynamic_targets: vec![
                    DynamicTarget {
                        id: "count".into(),
                        deps: vec!["count".into()],
                    },
                    DynamicTarget {
                        id: "label".into(),
                        deps: vec!["count".into(), "label_text".into()],
                    },
                ],
            },
        ],
    };

    let json = serde_json::to_string(&bootstrap).unwrap();
    let parsed: BootstrapPayload = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.session_id, "sess-abc-123");
    assert_eq!(parsed.websocket_url, "ws://localhost:3000/live/ws");
    assert_eq!(parsed.csrf_token, "tok_xyz");
    assert_eq!(parsed.initial_state_hash, "sha256:abcdef");
    assert_eq!(parsed.component_tree.len(), 2);
    assert_eq!(parsed.component_tree[0].name, "App");
    assert_eq!(parsed.component_tree[1].dynamic_targets.len(), 2);
    assert_eq!(parsed.component_tree[1].dynamic_targets[1].deps.len(), 2);
}

// ---------------------------------------------------------------------------
// 12. Validate protocol version
// ---------------------------------------------------------------------------

#[test]
fn test_validate_protocol_version() {
    let msg = ClientMessage {
        v: 99,
        payload: ClientPayload::Heartbeat(HeartbeatEvent {
            session: "sess".into(),
            seq: 1,
        }),
    };

    let result = msg.validate();
    assert!(result.is_err());

    match result.unwrap_err() {
        ProtocolError::InvalidVersion(v) => assert_eq!(v, 99),
        other => panic!("Expected InvalidVersion, got: {:?}", other),
    }

    // Valid version should pass
    let valid_msg = ClientMessage {
        v: 1,
        payload: ClientPayload::Heartbeat(HeartbeatEvent {
            session: "sess".into(),
            seq: 1,
        }),
    };
    assert!(valid_msg.validate().is_ok());
}

// ---------------------------------------------------------------------------
// 13. Test decode_client_message with valid JSON
// ---------------------------------------------------------------------------

#[test]
fn test_decode_client_message_valid() {
    let json = r#"{"v":1,"type":"heartbeat","session":"sess-100","seq":7}"#;
    let msg = decode_client_message(json).unwrap();

    assert_eq!(msg.v, 1);
    match &msg.payload {
        ClientPayload::Heartbeat(h) => {
            assert_eq!(h.session, "sess-100");
            assert_eq!(h.seq, 7);
        }
        _ => panic!("Expected Heartbeat"),
    }
}

// ---------------------------------------------------------------------------
// 14. Test decode_client_message with invalid JSON
// ---------------------------------------------------------------------------

#[test]
fn test_decode_client_message_invalid() {
    let bad_json = r#"{"this is not valid json"#;
    let result = decode_client_message(bad_json);
    assert!(result.is_err());

    match result.unwrap_err() {
        ProtocolError::Serialization(msg) => {
            assert!(!msg.is_empty());
        }
        other => panic!("Expected Serialization error, got: {:?}", other),
    }
}

// ---------------------------------------------------------------------------
// 15. Test encode_server_message
// ---------------------------------------------------------------------------

#[test]
fn test_encode_server_message() {
    let msg = ServerMessage::new(ServerPayload::HeartbeatAck(HeartbeatAck { seq: 42 }));

    let json = encode_server_message(&msg).unwrap();

    assert!(json.contains("\"v\":1"));
    assert!(json.contains("\"type\":\"heartbeat_ack\""));
    assert!(json.contains("\"seq\":42"));
}

// ---------------------------------------------------------------------------
// 16. Test default SessionConfig values
// ---------------------------------------------------------------------------

#[test]
fn test_session_config_defaults() {
    let config = SessionConfig::default();

    assert_eq!(config.heartbeat_interval_ms, 30_000);
    assert_eq!(config.reconnect_max_retries, 10);
    assert_eq!(config.reconnect_backoff_ms, 1_000);
    assert_eq!(config.event_rate_limit, 20);
}

// ---------------------------------------------------------------------------
// 17. Roundtrip: encode then decode client message
// ---------------------------------------------------------------------------

#[test]
fn test_client_message_roundtrip() {
    let mut payload = HashMap::new();
    payload.insert("x".into(), serde_json::json!(100));
    payload.insert("y".into(), serde_json::json!(200));

    let original = ClientMessage {
        v: 1,
        payload: ClientPayload::Event(ClientEvent {
            session: "sess-rt".into(),
            component: "canvas".into(),
            event: "click".into(),
            handler: "handle_click".into(),
            payload,
            seq: 55,
        }),
    };

    let encoded = encode_client_message(&original).unwrap();
    let decoded = decode_client_message(&encoded).unwrap();

    assert_eq!(decoded.v, original.v);
    match (&original.payload, &decoded.payload) {
        (ClientPayload::Event(a), ClientPayload::Event(b)) => {
            assert_eq!(a.session, b.session);
            assert_eq!(a.component, b.component);
            assert_eq!(a.event, b.event);
            assert_eq!(a.handler, b.handler);
            assert_eq!(a.seq, b.seq);
            assert_eq!(a.payload.len(), b.payload.len());
        }
        _ => panic!("Payload type mismatch after roundtrip"),
    }
}

// ---------------------------------------------------------------------------
// 18. Roundtrip: encode then decode server message
// ---------------------------------------------------------------------------

#[test]
fn test_server_message_roundtrip() {
    let original = ServerMessage::new(ServerPayload::Patch(PatchMessage {
        seq: 10,
        ops: vec![
            PatchOp::ReplaceText {
                target: "#count".into(),
                value: "5".into(),
            },
            PatchOp::Flash {
                level: FlashLevel::Info,
                message: "Count updated".into(),
            },
        ],
    }));

    let encoded = encode_server_message(&original).unwrap();
    let decoded = decode_server_message(&encoded).unwrap();

    assert_eq!(decoded.v, original.v);
    match (&original.payload, &decoded.payload) {
        (ServerPayload::Patch(a), ServerPayload::Patch(b)) => {
            assert_eq!(a.seq, b.seq);
            assert_eq!(a.ops.len(), b.ops.len());
        }
        _ => panic!("Payload type mismatch after roundtrip"),
    }
}

// ---------------------------------------------------------------------------
// 19. Test Flash levels serialize correctly
// ---------------------------------------------------------------------------

#[test]
fn test_flash_levels_json_values() {
    let levels = vec![
        (FlashLevel::Success, "success"),
        (FlashLevel::Info, "info"),
        (FlashLevel::Warning, "warning"),
        (FlashLevel::Danger, "danger"),
    ];

    for (level, expected_str) in levels {
        let json = serde_json::to_string(&level).unwrap();
        assert_eq!(json, format!("\"{}\"", expected_str));

        let parsed: FlashLevel = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, level);
    }
}

// ---------------------------------------------------------------------------
// 20. Test all PatchOp tag names are correct in JSON
// ---------------------------------------------------------------------------

#[test]
fn test_patch_op_tag_names() {
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
                html: "h".into(),
            },
            "replace_html",
        ),
        (
            PatchOp::SetAttr {
                target: "t".into(),
                name: "n".into(),
                value: "v".into(),
            },
            "set_attr",
        ),
        (
            PatchOp::RemoveAttr {
                target: "t".into(),
                name: "n".into(),
            },
            "remove_attr",
        ),
        (
            PatchOp::AddClass {
                target: "t".into(),
                class: "c".into(),
            },
            "add_class",
        ),
        (
            PatchOp::RemoveClass {
                target: "t".into(),
                class: "c".into(),
            },
            "remove_class",
        ),
        (
            PatchOp::InsertBefore {
                target: "t".into(),
                html: "h".into(),
            },
            "insert_before",
        ),
        (
            PatchOp::InsertAfter {
                target: "t".into(),
                html: "h".into(),
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
                url: "u".into(),
            },
            "redirect",
        ),
        (
            PatchOp::Flash {
                level: FlashLevel::Success,
                message: "m".into(),
            },
            "flash",
        ),
        (
            PatchOp::ModalOpen {
                id: "i".into(),
                html: "h".into(),
            },
            "modal_open",
        ),
        (PatchOp::ModalClose { id: "i".into() }, "modal_close"),
    ];

    for (op, expected_tag) in test_cases {
        let json = serde_json::to_string(&op).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        let actual_tag = parsed["op"].as_str().unwrap();
        assert_eq!(
            actual_tag, expected_tag,
            "PatchOp tag mismatch: expected '{}', got '{}'",
            expected_tag, actual_tag
        );
    }
}

// ---------------------------------------------------------------------------
// Additional validation tests
// ---------------------------------------------------------------------------

#[test]
fn test_validate_empty_session() {
    let msg = ClientMessage {
        v: 1,
        payload: ClientPayload::Heartbeat(HeartbeatEvent {
            session: "".into(),
            seq: 1,
        }),
    };

    match msg.validate().unwrap_err() {
        ProtocolError::InvalidSession => {}
        other => panic!("Expected InvalidSession, got: {:?}", other),
    }
}

#[test]
fn test_validate_empty_handler() {
    let msg = ClientMessage {
        v: 1,
        payload: ClientPayload::Event(ClientEvent {
            session: "sess".into(),
            component: "comp".into(),
            event: "click".into(),
            handler: "".into(),
            payload: HashMap::new(),
            seq: 1,
        }),
    };

    match msg.validate().unwrap_err() {
        ProtocolError::MissingField(field) => assert_eq!(field, "handler"),
        other => panic!("Expected MissingField(handler), got: {:?}", other),
    }
}

#[test]
fn test_validate_navigate_path_no_leading_slash() {
    let msg = ClientMessage {
        v: 1,
        payload: ClientPayload::Navigate(NavigateEvent {
            session: "sess".into(),
            path: "dashboard".into(),
            seq: 1,
        }),
    };

    match msg.validate().unwrap_err() {
        ProtocolError::InvalidEventType(detail) => {
            assert!(detail.contains("must start with '/'"));
        }
        other => panic!("Expected InvalidEventType, got: {:?}", other),
    }
}

#[test]
fn test_validate_form_submit_empty_component() {
    let msg = ClientMessage {
        v: 1,
        payload: ClientPayload::FormSubmit(FormSubmitEvent {
            session: "sess".into(),
            component: "".into(),
            handler: "submit".into(),
            form: HashMap::new(),
            seq: 1,
        }),
    };

    match msg.validate().unwrap_err() {
        ProtocolError::MissingField(field) => assert_eq!(field, "component"),
        other => panic!("Expected MissingField(component), got: {:?}", other),
    }
}
