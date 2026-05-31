use chrono::{DateTime, Utc};
use serde::{de::DeserializeOwned, Serialize};

/// A message type that can be published on the bus.
///
/// Each event is a Rust type carrying a stable [`Event::TOPIC`] string. The
/// topic is the event's durable identity in the `_events` log and the routing
/// key; the Rust type gives compile-time safety at the subscription site.
pub trait Event: Serialize + DeserializeOwned + Send + Sync + 'static {
    /// Stable topic name used for the log, routing, and admin. e.g. `"page.viewed"`.
    const TOPIC: &'static str;

    /// Optional key used by the `coalesce`/`sample` delivery primitives.
    /// `None` (the default) disables those primitives for this event.
    fn coalesce_key(&self) -> Option<String> {
        None
    }
}

/// Error returned by an event handler.
#[derive(Debug, thiserror::Error)]
pub enum EventError {
    #[error("{0}")]
    Message(String),
}

impl EventError {
    /// Construct from any message.
    pub fn msg(s: impl Into<String>) -> Self {
        EventError::Message(s.into())
    }
    /// Wrap any error's `Display` into an `EventError`
    /// (use in handlers: `.map_err(EventError::from_err)`).
    pub fn from_err<E: std::fmt::Display>(e: E) -> Self {
        EventError::Message(e.to_string())
    }
}

/// The in-memory envelope broadcast to live subscribers on publish.
///
/// Ephemeral subscribers use `payload` directly; durable subscribers treat the
/// envelope as a wake signal and re-read the event from the `_events` log.
#[derive(Clone, Debug)]
pub struct EventEnvelope {
    pub seq: u64,
    pub topic: &'static str,
    pub payload: serde_json::Value,
    pub ts: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct PageViewed {
        path: String,
    }
    impl Event for PageViewed {
        const TOPIC: &'static str = "page.viewed";
        fn coalesce_key(&self) -> Option<String> {
            Some(self.path.clone())
        }
    }

    #[test]
    fn topic_and_coalesce_key() {
        let e = PageViewed { path: "/x".into() };
        assert_eq!(PageViewed::TOPIC, "page.viewed");
        assert_eq!(e.coalesce_key(), Some("/x".into()));
    }

    #[test]
    fn default_coalesce_key_is_none() {
        #[derive(Serialize, Deserialize)]
        struct Bare;
        impl Event for Bare {
            const TOPIC: &'static str = "bare";
        }
        assert_eq!(Bare.coalesce_key(), None);
    }

    #[test]
    fn error_constructors() {
        let e = EventError::msg("boom");
        assert_eq!(e.to_string(), "boom");
        let e2 = EventError::from_err(std::io::Error::other("io"));
        assert!(e2.to_string().contains("io"));
    }
}
