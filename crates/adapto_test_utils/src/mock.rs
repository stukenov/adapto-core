use adapto_audit::event::AuditEvent;
use std::sync::{Arc, Mutex};

// ---------------------------------------------------------------------------
// MockAuditSink
// ---------------------------------------------------------------------------

/// In-memory audit event collector for tests.
///
/// Captures every audit event written through it, allowing test assertions
/// against the audit trail without any external dependencies.
#[derive(Debug, Clone, Default)]
pub struct MockAuditSink {
    events: Arc<Mutex<Vec<AuditEvent>>>,
}

impl MockAuditSink {
    pub fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Record an audit event.
    pub fn write(&self, event: AuditEvent) {
        self.events.lock().unwrap().push(event);
    }

    /// Return a snapshot of all recorded events.
    pub fn events(&self) -> Vec<AuditEvent> {
        self.events.lock().unwrap().clone()
    }

    /// Return the number of recorded events.
    pub fn len(&self) -> usize {
        self.events.lock().unwrap().len()
    }

    /// Return `true` if no events have been recorded.
    pub fn is_empty(&self) -> bool {
        self.events.lock().unwrap().is_empty()
    }

    /// Clear all recorded events.
    pub fn clear(&self) {
        self.events.lock().unwrap().clear();
    }
}

// ---------------------------------------------------------------------------
// MockClock
// ---------------------------------------------------------------------------

/// Deterministic clock for tests.
///
/// Wraps a shared, mutable instant so that tests can advance, set, and
/// query time without depending on wall-clock timing.
#[derive(Debug, Clone)]
pub struct MockClock {
    now: Arc<Mutex<chrono::DateTime<chrono::Utc>>>,
}

impl MockClock {
    /// Create a mock clock pinned to the given instant.
    pub fn new(now: chrono::DateTime<chrono::Utc>) -> Self {
        Self {
            now: Arc::new(Mutex::new(now)),
        }
    }

    /// Advance the clock by the given duration.
    pub fn advance(&self, duration: chrono::Duration) {
        let mut now = self.now.lock().unwrap();
        *now = *now + duration;
    }

    /// Read the current time.
    pub fn now(&self) -> chrono::DateTime<chrono::Utc> {
        *self.now.lock().unwrap()
    }

    /// Set the clock to an exact instant.
    pub fn set(&self, time: chrono::DateTime<chrono::Utc>) {
        let mut now = self.now.lock().unwrap();
        *now = time;
    }
}

// ---------------------------------------------------------------------------
// MockSecretProvider
// ---------------------------------------------------------------------------

/// Deterministic secret provider for tests.
///
/// Returns a fixed, well-known secret so that token generation and
/// verification produce repeatable results.
pub struct MockSecretProvider {
    secret: Vec<u8>,
}

impl MockSecretProvider {
    pub fn new() -> Self {
        Self {
            secret: b"test-secret-key-for-tests".to_vec(),
        }
    }

    pub fn secret(&self) -> &[u8] {
        &self.secret
    }
}

impl Default for MockSecretProvider {
    fn default() -> Self {
        Self::new()
    }
}
