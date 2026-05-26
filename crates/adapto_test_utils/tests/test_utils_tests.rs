use adapto_client_protocol::event::*;
use adapto_client_protocol::patch::*;
use adapto_test_utils::assertions::*;
use adapto_test_utils::builders::*;
use adapto_test_utils::fixtures::*;
use adapto_test_utils::mock::*;
use chrono::{TimeZone, Utc};
use serde_json::json;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

#[test]
fn test_ctx_has_correct_fields() {
    let ctx = test_ctx();

    assert_eq!(
        ctx.user_id.unwrap(),
        adapto_runtime::types::UserId(
            Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap()
        )
    );
    assert_eq!(
        ctx.tenant_id.unwrap(),
        adapto_runtime::types::TenantId(
            Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap()
        )
    );
    assert_eq!(
        ctx.request_id,
        adapto_runtime::types::RequestId(
            Uuid::parse_str("00000000-0000-0000-0000-000000000003").unwrap()
        )
    );
    assert_eq!(ctx.route.0, "/test");
    assert_eq!(ctx.session_id.0, "test-session-001");
}

#[test]
fn test_ctx_with_permissions_adds_perms() {
    let ctx = test_ctx_with_permissions(&["admin", "read", "write"]);

    assert!(ctx.permissions.has("admin"));
    assert!(ctx.permissions.has("read"));
    assert!(ctx.permissions.has("write"));
    assert!(!ctx.permissions.has("delete"));
}

#[test]
fn test_ctx_anonymous_has_no_user() {
    let ctx = test_ctx_anonymous();

    assert!(ctx.user_id.is_none());
    assert!(ctx.tenant_id.is_none());
}

#[test]
fn test_ctx_no_tenant_has_user_but_no_tenant() {
    let ctx = test_ctx_no_tenant();

    assert!(ctx.user_id.is_some());
    assert!(ctx.tenant_id.is_none());
}

// ---------------------------------------------------------------------------
// EventBuilder
// ---------------------------------------------------------------------------

#[test]
fn event_builder_click() {
    let msg = EventBuilder::click("on_save").build();

    assert_eq!(msg.v, PROTOCOL_VERSION);
    match &msg.payload {
        ClientPayload::Event(event) => {
            assert_eq!(event.event, "click");
            assert_eq!(event.handler, "on_save");
            assert_eq!(event.session, "test-session-001");
            assert_eq!(event.component, "test-component");
            assert_eq!(event.seq, 1);
            assert!(event.payload.is_empty());
        }
        other => panic!("Expected Event payload, got {:?}", other),
    }
}

#[test]
fn event_builder_input() {
    let msg = EventBuilder::input("on_change", "hello").build();

    match &msg.payload {
        ClientPayload::Event(event) => {
            assert_eq!(event.event, "input");
            assert_eq!(event.handler, "on_change");
            assert_eq!(
                event.payload.get("value"),
                Some(&serde_json::Value::String("hello".to_string()))
            );
        }
        other => panic!("Expected Event payload, got {:?}", other),
    }
}

#[test]
fn event_builder_with_custom_payload() {
    let msg = EventBuilder::click("on_select")
        .session("custom-session")
        .component("sidebar")
        .seq(42)
        .payload_field("index", json!(3))
        .payload_field("label", json!("Option C"))
        .build();

    match &msg.payload {
        ClientPayload::Event(event) => {
            assert_eq!(event.session, "custom-session");
            assert_eq!(event.component, "sidebar");
            assert_eq!(event.seq, 42);
            assert_eq!(event.payload.get("index"), Some(&json!(3)));
            assert_eq!(event.payload.get("label"), Some(&json!("Option C")));
        }
        other => panic!("Expected Event payload, got {:?}", other),
    }
}

// ---------------------------------------------------------------------------
// FormBuilder
// ---------------------------------------------------------------------------

#[test]
fn form_builder_simple() {
    let msg = FormBuilder::new("on_submit")
        .field("name", "Alice")
        .build();

    assert_eq!(msg.v, PROTOCOL_VERSION);
    match &msg.payload {
        ClientPayload::FormSubmit(form) => {
            assert_eq!(form.handler, "on_submit");
            assert_eq!(form.form.get("name"), Some(&json!("Alice")));
            assert_eq!(form.session, "test-session-001");
        }
        other => panic!("Expected FormSubmit payload, got {:?}", other),
    }
}

#[test]
fn form_builder_with_multiple_fields() {
    let msg = FormBuilder::new("register")
        .field("email", "alice@example.com")
        .field("age", 30)
        .field("agree", true)
        .session("sess-99")
        .component("register-form")
        .seq(7)
        .build();

    match &msg.payload {
        ClientPayload::FormSubmit(form) => {
            assert_eq!(form.handler, "register");
            assert_eq!(form.session, "sess-99");
            assert_eq!(form.component, "register-form");
            assert_eq!(form.seq, 7);
            assert_eq!(form.form.get("email"), Some(&json!("alice@example.com")));
            assert_eq!(form.form.get("age"), Some(&json!(30)));
            assert_eq!(form.form.get("agree"), Some(&json!(true)));
        }
        other => panic!("Expected FormSubmit payload, got {:?}", other),
    }
}

// ---------------------------------------------------------------------------
// PatchBuilder
// ---------------------------------------------------------------------------

#[test]
fn patch_builder_single_op() {
    let msg = PatchBuilder::new(1)
        .replace_text("#title", "Hello")
        .build();

    assert_eq!(msg.v, PROTOCOL_VERSION);
    match &msg.payload {
        ServerPayload::Patch(patch) => {
            assert_eq!(patch.seq, 1);
            assert_eq!(patch.ops.len(), 1);
            match &patch.ops[0] {
                PatchOp::ReplaceText { target, value } => {
                    assert_eq!(target, "#title");
                    assert_eq!(value, "Hello");
                }
                other => panic!("Expected ReplaceText, got {:?}", other),
            }
        }
        other => panic!("Expected Patch payload, got {:?}", other),
    }
}

#[test]
fn patch_builder_multiple_ops() {
    let msg = PatchBuilder::new(5)
        .replace_text("#count", "42")
        .replace_html("#list", "<li>Item</li>")
        .set_attr("#btn", "disabled", "true")
        .remove_attr("#btn", "disabled")
        .add_class("#card", "active")
        .remove_class("#card", "hidden")
        .flash(FlashLevel::Success, "Saved!")
        .redirect("/dashboard")
        .build();

    match &msg.payload {
        ServerPayload::Patch(patch) => {
            assert_eq!(patch.seq, 5);
            assert_eq!(patch.ops.len(), 8);
        }
        other => panic!("Expected Patch payload, got {:?}", other),
    }
}

// ---------------------------------------------------------------------------
// StateBuilder
// ---------------------------------------------------------------------------

#[test]
fn state_builder_set_and_build() {
    let store = StateBuilder::new()
        .set("count", json!(10))
        .set("name", json!("Alice"))
        .build();

    assert_eq!(store.get("count"), Some(&json!(10)));
    assert_eq!(store.get("name"), Some(&json!("Alice")));
    assert!(store.is_dirty("count"));
    assert!(store.is_dirty("name"));
}

// ---------------------------------------------------------------------------
// Assertions
// ---------------------------------------------------------------------------

#[test]
fn assertion_state_eq_passes() {
    let store = StateBuilder::new()
        .set("score", json!(100))
        .build();

    assert_state_eq(&store, "score", &json!(100));
}

#[test]
fn assertion_state_dirty() {
    let store = StateBuilder::new()
        .set("flag", json!(true))
        .build();

    assert_state_dirty(&store, "flag");
}

#[test]
fn assertion_patch_contains_text() {
    let msg = PatchBuilder::new(1)
        .replace_text("#name", "Bob")
        .replace_text("#age", "30")
        .build();

    assert_patch_contains_text(&msg, "#name", "Bob");
    assert_patch_contains_text(&msg, "#age", "30");
}

// ---------------------------------------------------------------------------
// MockClock
// ---------------------------------------------------------------------------

#[test]
fn mock_clock_advance_time() {
    let start = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
    let clock = MockClock::new(start);

    assert_eq!(clock.now(), start);

    clock.advance(chrono::Duration::hours(2));
    let expected = Utc.with_ymd_and_hms(2025, 1, 1, 2, 0, 0).unwrap();
    assert_eq!(clock.now(), expected);
}

#[test]
fn mock_clock_set_time() {
    let start = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
    let clock = MockClock::new(start);

    let new_time = Utc.with_ymd_and_hms(2025, 6, 15, 12, 30, 0).unwrap();
    clock.set(new_time);
    assert_eq!(clock.now(), new_time);
}

// ---------------------------------------------------------------------------
// MockSecretProvider
// ---------------------------------------------------------------------------

#[test]
fn mock_secret_provider_returns_consistent_secret() {
    let provider = MockSecretProvider::new();
    let secret_a = provider.secret().to_vec();
    let secret_b = provider.secret().to_vec();

    assert_eq!(secret_a, secret_b);
    assert!(!secret_a.is_empty());
    assert_eq!(secret_a, b"test-secret-key-for-tests");
}

// ---------------------------------------------------------------------------
// Additional EventBuilder tests
// ---------------------------------------------------------------------------

#[test]
fn event_builder_submit() {
    let msg = EventBuilder::submit("on_submit").build();

    match &msg.payload {
        ClientPayload::Event(event) => {
            assert_eq!(event.event, "submit");
            assert_eq!(event.handler, "on_submit");
            assert!(event.payload.is_empty());
        }
        other => panic!("Expected Event payload, got {:?}", other),
    }
}

#[test]
fn event_builder_defaults() {
    let msg = EventBuilder::click("h").build();
    match &msg.payload {
        ClientPayload::Event(event) => {
            assert_eq!(event.session, "test-session-001");
            assert_eq!(event.component, "test-component");
            assert_eq!(event.seq, 1);
        }
        other => panic!("Expected Event payload, got {:?}", other),
    }
}

// ---------------------------------------------------------------------------
// Additional FormBuilder tests
// ---------------------------------------------------------------------------

#[test]
fn form_builder_empty_form() {
    let msg = FormBuilder::new("save").build();
    match &msg.payload {
        ClientPayload::FormSubmit(form) => {
            assert!(form.form.is_empty());
            assert_eq!(form.handler, "save");
        }
        other => panic!("Expected FormSubmit payload, got {:?}", other),
    }
}

#[test]
fn form_builder_defaults() {
    let msg = FormBuilder::new("h").build();
    match &msg.payload {
        ClientPayload::FormSubmit(form) => {
            assert_eq!(form.session, "test-session-001");
            assert_eq!(form.component, "test-component");
            assert_eq!(form.seq, 1);
        }
        other => panic!("Expected FormSubmit payload, got {:?}", other),
    }
}

// ---------------------------------------------------------------------------
// Additional PatchBuilder tests
// ---------------------------------------------------------------------------

#[test]
fn patch_builder_empty_ops() {
    let msg = PatchBuilder::new(0).build();
    match &msg.payload {
        ServerPayload::Patch(patch) => {
            assert_eq!(patch.seq, 0);
            assert!(patch.ops.is_empty());
        }
        other => panic!("Expected Patch payload, got {:?}", other),
    }
}

#[test]
fn patch_builder_replace_html_op() {
    let msg = PatchBuilder::new(1)
        .replace_html("#content", "<b>hi</b>")
        .build();
    match &msg.payload {
        ServerPayload::Patch(patch) => {
            assert_eq!(patch.ops.len(), 1);
            match &patch.ops[0] {
                PatchOp::ReplaceHtml { target, html } => {
                    assert_eq!(target, "#content");
                    assert_eq!(html, "<b>hi</b>");
                }
                other => panic!("Expected ReplaceHtml, got {:?}", other),
            }
        }
        other => panic!("Expected Patch payload, got {:?}", other),
    }
}

#[test]
fn patch_builder_set_and_remove_attr() {
    let msg = PatchBuilder::new(2)
        .set_attr("#el", "disabled", "true")
        .remove_attr("#el", "disabled")
        .build();
    match &msg.payload {
        ServerPayload::Patch(patch) => {
            assert_eq!(patch.ops.len(), 2);
            assert!(matches!(&patch.ops[0], PatchOp::SetAttr { .. }));
            assert!(matches!(&patch.ops[1], PatchOp::RemoveAttr { .. }));
        }
        other => panic!("Expected Patch payload, got {:?}", other),
    }
}

#[test]
fn patch_builder_add_and_remove_class() {
    let msg = PatchBuilder::new(3)
        .add_class("#box", "active")
        .remove_class("#box", "hidden")
        .build();
    match &msg.payload {
        ServerPayload::Patch(patch) => {
            assert_eq!(patch.ops.len(), 2);
            match &patch.ops[0] {
                PatchOp::AddClass { target, class } => {
                    assert_eq!(target, "#box");
                    assert_eq!(class, "active");
                }
                other => panic!("Expected AddClass, got {:?}", other),
            }
            match &patch.ops[1] {
                PatchOp::RemoveClass { target, class } => {
                    assert_eq!(target, "#box");
                    assert_eq!(class, "hidden");
                }
                other => panic!("Expected RemoveClass, got {:?}", other),
            }
        }
        other => panic!("Expected Patch payload, got {:?}", other),
    }
}

#[test]
fn patch_builder_flash_op() {
    let msg = PatchBuilder::new(1)
        .flash(FlashLevel::Danger, "oops")
        .build();
    match &msg.payload {
        ServerPayload::Patch(patch) => {
            assert_eq!(patch.ops.len(), 1);
            match &patch.ops[0] {
                PatchOp::Flash { level, message } => {
                    assert!(matches!(level, FlashLevel::Danger));
                    assert_eq!(message, "oops");
                }
                other => panic!("Expected Flash, got {:?}", other),
            }
        }
        other => panic!("Expected Patch payload, got {:?}", other),
    }
}

#[test]
fn patch_builder_redirect_op() {
    let msg = PatchBuilder::new(1)
        .redirect("/login")
        .build();
    match &msg.payload {
        ServerPayload::Patch(patch) => {
            assert_eq!(patch.ops.len(), 1);
            match &patch.ops[0] {
                PatchOp::Redirect { url } => assert_eq!(url, "/login"),
                other => panic!("Expected Redirect, got {:?}", other),
            }
        }
        other => panic!("Expected Patch payload, got {:?}", other),
    }
}

// ---------------------------------------------------------------------------
// Additional StateBuilder tests
// ---------------------------------------------------------------------------

#[test]
fn state_builder_empty() {
    let store = StateBuilder::new().build();
    assert!(store.get("anything").is_none());
    assert!(store.get_dirty().is_empty());
}

#[test]
fn state_builder_overwrite_key() {
    let store = StateBuilder::new()
        .set("x", json!(1))
        .set("x", json!(2))
        .build();
    assert_eq!(store.get("x"), Some(&json!(2)));
}

// ---------------------------------------------------------------------------
// Additional fixture tests
// ---------------------------------------------------------------------------

#[test]
fn test_tenant_id_deterministic() {
    let a = test_tenant_id();
    let b = test_tenant_id();
    assert_eq!(a, b);
}

#[test]
fn test_user_id_deterministic() {
    let a = test_user_id();
    let b = test_user_id();
    assert_eq!(a, b);
}

#[test]
fn test_session_id_deterministic() {
    let a = test_session_id();
    let b = test_session_id();
    assert_eq!(a, b);
    assert_eq!(a.0, "test-session-001");
}

#[test]
fn test_request_id_deterministic() {
    let a = test_request_id();
    let b = test_request_id();
    assert_eq!(a, b);
}

// ---------------------------------------------------------------------------
// Additional MockAuditSink tests
// ---------------------------------------------------------------------------

#[test]
fn mock_audit_sink_write_and_read() {
    use adapto_audit::event::AuditEvent;

    let sink = MockAuditSink::new();
    assert!(sink.is_empty());
    assert_eq!(sink.len(), 0);

    let ctx = test_ctx();
    let event = AuditEvent::new("user.login", &ctx, "login");
    sink.write(event);

    assert_eq!(sink.len(), 1);
    assert!(!sink.is_empty());

    let events = sink.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event, "user.login");
    assert_eq!(events[0].action, "login");
}

#[test]
fn mock_audit_sink_clear() {
    use adapto_audit::event::AuditEvent;

    let sink = MockAuditSink::new();
    let ctx = test_ctx();
    sink.write(AuditEvent::new("a", &ctx, "x"));
    sink.write(AuditEvent::new("b", &ctx, "y"));
    assert_eq!(sink.len(), 2);

    sink.clear();
    assert_eq!(sink.len(), 0);
    assert!(sink.is_empty());
}

#[test]
fn mock_audit_sink_multiple_events() {
    use adapto_audit::event::AuditEvent;

    let sink = MockAuditSink::new();
    let ctx = test_ctx();
    for i in 0..5 {
        sink.write(AuditEvent::new(&format!("ev_{i}"), &ctx, "act"));
    }
    assert_eq!(sink.len(), 5);
    let events = sink.events();
    assert_eq!(events[0].event, "ev_0");
    assert_eq!(events[4].event, "ev_4");
}

// ---------------------------------------------------------------------------
// Additional MockClock tests
// ---------------------------------------------------------------------------

#[test]
fn mock_clock_multiple_advances() {
    let start = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
    let clock = MockClock::new(start);

    clock.advance(chrono::Duration::minutes(30));
    clock.advance(chrono::Duration::minutes(30));

    let expected = Utc.with_ymd_and_hms(2025, 1, 1, 1, 0, 0).unwrap();
    assert_eq!(clock.now(), expected);
}

#[test]
fn mock_clock_set_overrides_advances() {
    let start = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
    let clock = MockClock::new(start);

    clock.advance(chrono::Duration::hours(5));
    let target = Utc.with_ymd_and_hms(2025, 3, 15, 12, 0, 0).unwrap();
    clock.set(target);
    assert_eq!(clock.now(), target);
}

// ---------------------------------------------------------------------------
// Additional assertion tests
// ---------------------------------------------------------------------------

#[test]
fn assertion_patch_op_count() {
    let msg = PatchBuilder::new(1)
        .replace_text("#a", "1")
        .replace_text("#b", "2")
        .replace_html("#c", "<p>3</p>")
        .build();
    assert_patch_op_count(&msg, 3);
}

#[test]
fn assertion_patch_contains_html() {
    let msg = PatchBuilder::new(1)
        .replace_html("#list", "<li>item</li>")
        .build();
    assert_patch_contains_html(&msg, "#list");
}

#[test]
fn assertion_state_clean() {
    let mut store = StateBuilder::new()
        .set("a", json!(1))
        .build();
    store.clear_dirty();
    assert_state_clean(&store, "a");
}
