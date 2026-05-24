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
