use crate::event::AuditEvent;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

/// Trait for audit event consumers.
///
/// Implementations may write to a database, a log file, a message queue, or
/// simply buffer events in memory for testing.
pub trait AuditSink: Send + Sync {
    fn write(&self, event: AuditEvent);
}

// ---------------------------------------------------------------------------
// InMemoryAuditSink
// ---------------------------------------------------------------------------

/// A test-friendly sink that accumulates events in a `Vec` behind a mutex.
#[derive(Debug, Clone)]
pub struct InMemoryAuditSink {
    events: Arc<Mutex<Vec<AuditEvent>>>,
}

impl InMemoryAuditSink {
    pub fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn events(&self) -> Vec<AuditEvent> {
        self.events.lock().expect("lock poisoned").clone()
    }

    pub fn clear(&self) {
        self.events.lock().expect("lock poisoned").clear();
    }

    pub fn len(&self) -> usize {
        self.events.lock().expect("lock poisoned").len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for InMemoryAuditSink {
    fn default() -> Self {
        Self::new()
    }
}

impl AuditSink for InMemoryAuditSink {
    fn write(&self, event: AuditEvent) {
        self.events.lock().expect("lock poisoned").push(event);
    }
}

// ---------------------------------------------------------------------------
// ChannelAuditSink
// ---------------------------------------------------------------------------

/// An async-ready sink that forwards events through a Tokio unbounded channel.
///
/// The receiver end can be consumed by a background task that persists events
/// to a database or external service.
pub struct ChannelAuditSink {
    sender: mpsc::UnboundedSender<AuditEvent>,
}

impl ChannelAuditSink {
    /// Create a new channel sink, returning both the sink and the receiver.
    pub fn new() -> (Self, mpsc::UnboundedReceiver<AuditEvent>) {
        let (sender, receiver) = mpsc::unbounded_channel();
        (Self { sender }, receiver)
    }
}

impl AuditSink for ChannelAuditSink {
    fn write(&self, event: AuditEvent) {
        // Best-effort: if the receiver is dropped, we silently discard the
        // event rather than panicking.
        let _ = self.sender.send(event);
    }
}

// ---------------------------------------------------------------------------
// LogAuditSink
// ---------------------------------------------------------------------------

/// A sink that emits each event as a structured `tracing` log line at the
/// `info` level.
pub struct LogAuditSink;

impl AuditSink for LogAuditSink {
    fn write(&self, event: AuditEvent) {
        tracing::info!(
            audit_id = %event.id,
            event = %event.event,
            action = %event.action,
            route = %event.route,
            user_id = ?event.user_id,
            tenant_id = ?event.tenant_id,
            status = ?event.status,
            "audit event"
        );
    }
}

// ---------------------------------------------------------------------------
// FileSink — JSON-lines
// ---------------------------------------------------------------------------

pub struct FileSink {
    path: PathBuf,
    writer: Mutex<std::io::BufWriter<std::fs::File>>,
}

impl FileSink {
    pub fn new(path: impl Into<PathBuf>) -> Result<Self, crate::error::AuditError> {
        let path = path.into();
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| crate::error::AuditError::WriteError(e.to_string()))?;
        Ok(Self {
            path,
            writer: Mutex::new(std::io::BufWriter::new(file)),
        })
    }

    pub fn path(&self) -> &std::path::Path {
        &self.path
    }
}

impl AuditSink for FileSink {
    fn write(&self, event: AuditEvent) {
        if let Ok(mut writer) = self.writer.lock() {
            if let Ok(json) = serde_json::to_string(&event) {
                let _ = writeln!(writer, "{}", json);
                let _ = writer.flush();
            }
        }
    }
}

// ---------------------------------------------------------------------------
// CompositeSink — fan-out to multiple sinks
// ---------------------------------------------------------------------------

pub struct CompositeSink {
    sinks: Vec<Box<dyn AuditSink>>,
}

impl CompositeSink {
    pub fn new() -> Self {
        Self { sinks: Vec::new() }
    }

    pub fn add(mut self, sink: impl AuditSink + 'static) -> Self {
        self.sinks.push(Box::new(sink));
        self
    }

    pub fn len(&self) -> usize {
        self.sinks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sinks.is_empty()
    }
}

impl Default for CompositeSink {
    fn default() -> Self {
        Self::new()
    }
}

impl AuditSink for CompositeSink {
    fn write(&self, event: AuditEvent) {
        for sink in &self.sinks {
            sink.write(event.clone());
        }
    }
}

// ---------------------------------------------------------------------------
// RetentionSink — wraps a sink with max event count
// ---------------------------------------------------------------------------

pub struct RetentionSink {
    inner: InMemoryAuditSink,
    max_events: usize,
}

impl RetentionSink {
    pub fn new(max_events: usize) -> Self {
        Self {
            inner: InMemoryAuditSink::new(),
            max_events,
        }
    }

    pub fn events(&self) -> Vec<AuditEvent> {
        self.inner.events()
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

impl AuditSink for RetentionSink {
    fn write(&self, event: AuditEvent) {
        self.inner.write(event);
        let mut events = self.inner.events.lock().expect("lock poisoned");
        while events.len() > self.max_events {
            events.remove(0);
        }
    }
}
