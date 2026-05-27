# adapto_audit

Audit logging for Adapto apps — structured audit events, multiple sinks, query/filter API, PII redaction.

Part of the [Adapto](https://github.com/stukenov/adapto-core) web framework.

## Features

- **Structured events** — typed audit events with actor, action, resource, and metadata
- **Multiple sinks** — in-memory, file-based, composite, and retention-aware sinks
- **Query API** — filter events by actor, action, time range, resource
- **PII redaction** — automatic redaction of sensitive fields before storage
- **Retention policies** — configurable event retention with automatic cleanup

## Quick Start

```toml
[dependencies]
adapto_audit = "0.2"
```

```rust
use adapto_audit::{AuditEvent, InMemoryAuditSink, AuditSink};
use adapto_audit::redaction::PiiRedactor;

// Create sink
let sink = InMemoryAuditSink::new();

// Log event
let event = AuditEvent::new("user:login")
    .actor("user_42")
    .resource("session", "sess_abc")
    .metadata("ip", "192.168.1.1");
sink.log(event)?;

// Query events
let events = sink.query().actor("user_42").execute()?;

// PII redaction
let redactor = PiiRedactor::new(&["email", "phone"]);
let safe_event = redactor.redact(event);
```

## License

MIT — [Saken Tukenov](https://github.com/stukenov)
