use adapto_audit::event::{AuditEvent, AuditStatus};
use adapto_audit::sink::{AuditSink, ChannelAuditSink, InMemoryAuditSink, LogAuditSink};
use adapto_runtime::context::{Ctx, PermissionSet};
use adapto_runtime::types::*;
use serde_json::json;

fn make_ctx() -> Ctx {
    Ctx {
        user_id: Some(UserId::default()),
        tenant_id: Some(TenantId::default()),
        request_id: RequestId::default(),
        permissions: PermissionSet::new(),
        route: RouteId::from("/dashboard"),
        session_id: SessionId::from("sess_test"),
    }
}

// ---------------------------------------------------------------------------
// AuditEvent
// ---------------------------------------------------------------------------

#[test]
fn audit_event_creation_with_context() {
    let ctx = make_ctx();
    let event = AuditEvent::new("user.login", &ctx, "login");

    assert_eq!(event.event, "user.login");
    assert_eq!(event.action, "login");
    assert_eq!(event.route, "/dashboard");
    assert!(event.user_id.is_some());
    assert!(event.tenant_id.is_some());
    assert_eq!(event.status, AuditStatus::Success);
    assert!(event.metadata.is_empty());
}

#[test]
fn audit_event_with_metadata() {
    let ctx = make_ctx();
    let event = AuditEvent::new("record.update", &ctx, "update")
        .with_metadata("record_id", json!(42))
        .with_metadata("field", json!("name"));

    assert_eq!(event.metadata.len(), 2);
    assert_eq!(event.metadata.get("record_id"), Some(&json!(42)));
    assert_eq!(event.metadata.get("field"), Some(&json!("name")));
}

#[test]
fn audit_event_success_failure_denied_status() {
    let ctx = make_ctx();

    let success = AuditEvent::new("op", &ctx, "act").success();
    assert_eq!(success.status, AuditStatus::Success);

    let failure = AuditEvent::new("op", &ctx, "act").failure("db timeout");
    assert_eq!(failure.status, AuditStatus::Failure("db timeout".to_string()));

    let denied = AuditEvent::new("op", &ctx, "act").denied();
    assert_eq!(denied.status, AuditStatus::Denied);
}

// ---------------------------------------------------------------------------
// InMemoryAuditSink
// ---------------------------------------------------------------------------

#[test]
fn in_memory_sink_write_and_read() {
    let sink = InMemoryAuditSink::new();
    let ctx = make_ctx();
    let event = AuditEvent::new("test.event", &ctx, "test");

    sink.write(event);

    let events = sink.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event, "test.event");
}

#[test]
fn in_memory_sink_clear() {
    let sink = InMemoryAuditSink::new();
    let ctx = make_ctx();
    sink.write(AuditEvent::new("a", &ctx, "a"));
    sink.write(AuditEvent::new("b", &ctx, "b"));
    assert_eq!(sink.len(), 2);

    sink.clear();
    assert_eq!(sink.len(), 0);
}

#[test]
fn in_memory_sink_len_is_empty() {
    let sink = InMemoryAuditSink::new();
    assert!(sink.is_empty());
    assert_eq!(sink.len(), 0);

    let ctx = make_ctx();
    sink.write(AuditEvent::new("x", &ctx, "x"));
    assert!(!sink.is_empty());
    assert_eq!(sink.len(), 1);
}

// ---------------------------------------------------------------------------
// ChannelAuditSink
// ---------------------------------------------------------------------------

#[tokio::test]
async fn channel_sink_send_and_receive() {
    let (sink, mut rx) = ChannelAuditSink::new();
    let ctx = make_ctx();
    let event = AuditEvent::new("channel.test", &ctx, "send");

    sink.write(event);

    let received = rx.recv().await.expect("should receive event");
    assert_eq!(received.event, "channel.test");
    assert_eq!(received.action, "send");
}

// ---------------------------------------------------------------------------
// LogAuditSink
// ---------------------------------------------------------------------------

#[test]
fn log_sink_does_not_panic() {
    let sink = LogAuditSink;
    let ctx = make_ctx();
    let event = AuditEvent::new("log.test", &ctx, "log");

    // Should not panic even without a tracing subscriber configured.
    sink.write(event);
}

// ---------------------------------------------------------------------------
// Serialization
// ---------------------------------------------------------------------------

#[test]
fn audit_event_serialization_to_json() {
    let ctx = make_ctx();
    let event = AuditEvent::new("user.create", &ctx, "create")
        .with_metadata("email", json!("test@example.com"));

    let json_str = serde_json::to_string(&event).expect("serialization failed");
    let parsed: serde_json::Value =
        serde_json::from_str(&json_str).expect("deserialization failed");

    assert_eq!(parsed["event"], "user.create");
    assert_eq!(parsed["action"], "create");
    assert_eq!(parsed["route"], "/dashboard");
    assert_eq!(parsed["metadata"]["email"], "test@example.com");
    assert_eq!(parsed["status"], "Success");
}

// ---------------------------------------------------------------------------
// Ordering
// ---------------------------------------------------------------------------

#[test]
fn multiple_audit_events_ordering() {
    let sink = InMemoryAuditSink::new();
    let ctx = make_ctx();

    let names = ["first", "second", "third"];
    for name in &names {
        sink.write(AuditEvent::new(name, &ctx, name));
    }

    let events = sink.events();
    assert_eq!(events.len(), 3);
    assert_eq!(events[0].event, "first");
    assert_eq!(events[1].event, "second");
    assert_eq!(events[2].event, "third");
}
