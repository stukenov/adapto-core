use adapto_audit::error::AuditError;
use adapto_audit::event::{AuditEvent, AuditStatus};
use adapto_audit::filter::{AuditFilter, StatusFilter};
use adapto_audit::redact::{PiiRedactor, redact_value};
use adapto_audit::sink::{
    AuditSink, ChannelAuditSink, CompositeSink, FileSink, InMemoryAuditSink, LogAuditSink,
    RetentionSink,
};
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

// ---------------------------------------------------------------------------
// InMemoryAuditSink — Default trait
// ---------------------------------------------------------------------------

#[test]
fn in_memory_sink_default() {
    let sink = InMemoryAuditSink::default();
    assert!(sink.is_empty());
}

// ---------------------------------------------------------------------------
// ChannelAuditSink — dropped receiver
// ---------------------------------------------------------------------------

#[test]
fn channel_sink_write_after_receiver_dropped_does_not_panic() {
    let (sink, rx) = ChannelAuditSink::new();
    drop(rx);

    let ctx = make_ctx();
    // Should silently discard, not panic.
    sink.write(AuditEvent::new("dropped.rx", &ctx, "noop"));
}

// ---------------------------------------------------------------------------
// AuditError
// ---------------------------------------------------------------------------

#[test]
fn audit_error_write_error_display() {
    let err = AuditError::WriteError("disk full".to_string());
    assert_eq!(err.to_string(), "Failed to write audit event: disk full");
}

#[test]
fn audit_error_channel_closed_display() {
    let err = AuditError::ChannelClosed;
    assert_eq!(err.to_string(), "Audit channel closed");
}

// ---------------------------------------------------------------------------
// AuditStatus — Debug output
// ---------------------------------------------------------------------------

#[test]
fn audit_status_debug() {
    assert_eq!(format!("{:?}", AuditStatus::Success), "Success");
    assert_eq!(
        format!("{:?}", AuditStatus::Failure("oops".to_string())),
        "Failure(\"oops\")"
    );
    assert_eq!(format!("{:?}", AuditStatus::Denied), "Denied");
}

// ===========================================================================
// Filter
// ===========================================================================

#[test]
fn filter_by_event_name() {
    let sink = InMemoryAuditSink::new();
    let ctx = make_ctx();
    sink.write(AuditEvent::new("user.login", &ctx, "login"));
    sink.write(AuditEvent::new("user.logout", &ctx, "logout"));
    sink.write(AuditEvent::new("user.login", &ctx, "login"));

    let results = sink.query(&AuditFilter::new().event("user.login"));
    assert_eq!(results.len(), 2);
}

#[test]
fn filter_by_action() {
    let sink = InMemoryAuditSink::new();
    let ctx = make_ctx();
    sink.write(AuditEvent::new("op", &ctx, "create"));
    sink.write(AuditEvent::new("op", &ctx, "delete"));

    let results = sink.query(&AuditFilter::new().action("delete"));
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].action, "delete");
}

#[test]
fn filter_by_status() {
    let sink = InMemoryAuditSink::new();
    let ctx = make_ctx();
    sink.write(AuditEvent::new("op", &ctx, "a").success());
    sink.write(AuditEvent::new("op", &ctx, "b").failure("err"));
    sink.write(AuditEvent::new("op", &ctx, "c").denied());

    assert_eq!(
        sink.query(&AuditFilter::new().status(StatusFilter::Success))
            .len(),
        1
    );
    assert_eq!(
        sink.query(&AuditFilter::new().status(StatusFilter::Failure))
            .len(),
        1
    );
    assert_eq!(
        sink.query(&AuditFilter::new().status(StatusFilter::Denied))
            .len(),
        1
    );
}

#[test]
fn filter_by_route_prefix() {
    let sink = InMemoryAuditSink::new();
    let ctx = make_ctx();
    sink.write(AuditEvent::new("op", &ctx, "a"));

    let mut ctx2 = make_ctx();
    ctx2.route = RouteId::from("/api/users");
    sink.write(AuditEvent::new("op", &ctx2, "b"));

    let results = sink.query(&AuditFilter::new().route_prefix("/api"));
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].action, "b");
}

#[test]
fn filter_empty_matches_all() {
    let sink = InMemoryAuditSink::new();
    let ctx = make_ctx();
    sink.write(AuditEvent::new("a", &ctx, "a"));
    sink.write(AuditEvent::new("b", &ctx, "b"));

    assert_eq!(sink.query(&AuditFilter::new()).len(), 2);
}

#[test]
fn filter_count_matching() {
    let sink = InMemoryAuditSink::new();
    let ctx = make_ctx();
    for _ in 0..5 {
        sink.write(AuditEvent::new("op", &ctx, "a"));
    }
    assert_eq!(sink.count_matching(&AuditFilter::new().action("a")), 5);
    assert_eq!(sink.count_matching(&AuditFilter::new().action("b")), 0);
}

// ===========================================================================
// PII Redaction
// ===========================================================================

#[test]
fn redact_sensitive_metadata() {
    let ctx = make_ctx();
    let event = AuditEvent::new("user.create", &ctx, "create")
        .with_metadata("email", json!("user@example.com"))
        .with_metadata("name", json!("Alice"));

    let redactor = PiiRedactor::new();
    let redacted = redactor.redact_clone(&event);

    assert_eq!(redacted.metadata["email"], "[REDACTED]");
    assert_eq!(redacted.metadata["name"], "Alice");
}

#[test]
fn redact_custom_fields() {
    let ctx = make_ctx();
    let event = AuditEvent::new("op", &ctx, "op")
        .with_metadata("iin", json!("123456789012"));

    let redactor = PiiRedactor::new().add_field("iin");
    let redacted = redactor.redact_clone(&event);
    assert_eq!(redacted.metadata["iin"], "[REDACTED]");
}

#[test]
fn redact_custom_replacement() {
    let ctx = make_ctx();
    let event = AuditEvent::new("op", &ctx, "op")
        .with_metadata("email", json!("x@y.com"));

    let redactor = PiiRedactor::new().replacement("***");
    let redacted = redactor.redact_clone(&event);
    assert_eq!(redacted.metadata["email"], "***");
}

#[test]
fn redact_is_sensitive() {
    let redactor = PiiRedactor::new();
    assert!(redactor.is_sensitive("email"));
    assert!(redactor.is_sensitive("user_email"));
    assert!(redactor.is_sensitive("api_key"));
    assert!(!redactor.is_sensitive("name"));
    assert!(!redactor.is_sensitive("count"));
}

#[test]
fn redact_value_masks_strings() {
    let masked = redact_value(&json!("hello@world.com"));
    assert_eq!(masked, "he***");
    let short = redact_value(&json!("ab"));
    assert_eq!(short, "****");
}

// ===========================================================================
// FileSink
// ===========================================================================

#[test]
fn file_sink_writes_json_lines() {
    let dir = std::env::temp_dir().join(format!("adapto_audit_test_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("audit.jsonl");

    let sink = FileSink::new(&path).unwrap();
    let ctx = make_ctx();
    sink.write(AuditEvent::new("file.test", &ctx, "write"));
    sink.write(AuditEvent::new("file.test2", &ctx, "write2"));

    let content = std::fs::read_to_string(&path).unwrap();
    let lines: Vec<&str> = content.lines().collect();
    assert_eq!(lines.len(), 2);

    let parsed: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(parsed["event"], "file.test");

    std::fs::remove_dir_all(&dir).ok();
}

// ===========================================================================
// CompositeSink
// ===========================================================================

#[test]
fn composite_sink_fan_out() {
    let sink1 = InMemoryAuditSink::new();
    let sink2 = InMemoryAuditSink::new();
    let s1 = sink1.clone();
    let s2 = sink2.clone();

    let composite = CompositeSink::new().add(sink1).add(sink2);
    assert_eq!(composite.len(), 2);

    let ctx = make_ctx();
    composite.write(AuditEvent::new("fan.out", &ctx, "test"));

    assert_eq!(s1.len(), 1);
    assert_eq!(s2.len(), 1);
    assert_eq!(s1.events()[0].event, "fan.out");
}

// ===========================================================================
// RetentionSink
// ===========================================================================

#[test]
fn retention_sink_caps_events() {
    let sink = RetentionSink::new(3);
    let ctx = make_ctx();
    for i in 0..5 {
        sink.write(AuditEvent::new(&format!("ev{}", i), &ctx, "a"));
    }
    assert_eq!(sink.len(), 3);
    let events = sink.events();
    assert_eq!(events[0].event, "ev2");
    assert_eq!(events[2].event, "ev4");
}

#[test]
fn retention_sink_empty() {
    let sink = RetentionSink::new(10);
    assert!(sink.is_empty());
    assert_eq!(sink.len(), 0);
}
