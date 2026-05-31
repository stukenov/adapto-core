//! The [`EventBus`] builder and the cheap, clonable [`EventBusHandle`] used to
//! publish events. Dispatchers (ephemeral + durable) are wired into
//! [`EventBus::spawn`] by the `dispatch` module.

use crate::event::{Event, EventEnvelope};
use crate::state;
use crate::subscription::{Mode, Subscriber, Subscription};
use adapto_store::AdaptoStore;
use chrono::{DateTime, Utc};
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, Notify};

/// Builder + owner of a set of subscriptions. Call [`EventBus::spawn`] to start.
pub struct EventBus {
    pub(crate) store: AdaptoStore,
    pub(crate) broadcast_capacity: usize,
    pub(crate) gc_interval: Duration,
    pub(crate) retention: Duration,
    pub(crate) retry_backoff: Duration,
    pub(crate) shutdown_grace: Duration,
    pub(crate) subscribers: Vec<Subscriber>,
    pub(crate) now_fn: fn() -> DateTime<Utc>,
}

impl EventBus {
    /// Create a bus bound to a store. Defaults: 1024 broadcast capacity, 5m GC
    /// interval, 24h retention, 2s base retry backoff, 5s shutdown grace.
    pub fn new(store: AdaptoStore) -> Self {
        EventBus {
            store,
            broadcast_capacity: 1024,
            gc_interval: Duration::from_secs(300),
            retention: Duration::from_secs(86_400),
            retry_backoff: Duration::from_secs(2),
            shutdown_grace: Duration::from_secs(5),
            subscribers: Vec::new(),
            now_fn: Utc::now,
        }
    }

    /// Depth of the live broadcast channel (default 1024).
    pub fn broadcast_capacity(mut self, n: usize) -> Self {
        self.broadcast_capacity = n.max(1);
        self
    }

    /// How often the log-GC task runs (default 5 minutes).
    pub fn gc_interval(mut self, d: Duration) -> Self {
        self.gc_interval = d;
        self
    }

    /// Dead-letter retention floor (default 24 hours).
    pub fn retention(mut self, d: Duration) -> Self {
        self.retention = d;
        self
    }

    /// Base delay between durable handler retries; doubles per attempt, capped
    /// at 16× (default 2 seconds).
    pub fn retry_backoff(mut self, d: Duration) -> Self {
        self.retry_backoff = d;
        self
    }

    /// How long [`EventBusHandle::shutdown`] waits for in-flight durable
    /// handlers to drain (default 5 seconds).
    pub fn shutdown_grace(mut self, d: Duration) -> Self {
        self.shutdown_grace = d;
        self
    }

    /// Register a subscription.
    pub fn subscribe<E: Event>(mut self, sub: Subscription<E>) -> Self {
        self.subscribers.push(sub.build());
        self
    }

    #[doc(hidden)] // test-only clock injection
    pub fn now_fn(mut self, f: fn() -> DateTime<Utc>) -> Self {
        self.now_fn = f;
        self
    }

    /// Seed seq from the store, open the broadcast channel, and return a handle.
    /// Dispatchers are started here (added by the `dispatch` module).
    pub fn spawn(self) -> EventBusHandle {
        state::ensure_indexes(&self.store);
        let highwater = state::load_seq_highwater(&self.store);
        let durable_topics: HashSet<&'static str> = self
            .subscribers
            .iter()
            .filter(|s| s.mode == Mode::Durable)
            .map(|s| s.topic)
            .collect();
        let (tx, _rx) = broadcast::channel(self.broadcast_capacity);

        let handle = EventBusHandle {
            inner: Arc::new(HandleInner {
                store: self.store.clone(),
                tx,
                seq: AtomicU64::new(highwater),
                durable_topics,
                subscribers: self.subscribers.clone(),
                now_fn: self.now_fn,
                retry_backoff: self.retry_backoff,
                shutdown_grace: self.shutdown_grace,
                closed: AtomicBool::new(false),
                shutdown: Notify::new(),
                inflight: AtomicUsize::new(0),
            }),
        };
        crate::dispatch::start(&handle, &self.subscribers, self.gc_interval, self.retention);
        handle
    }
}

/// Cheap, clonable handle to a running bus.
#[derive(Clone)]
pub struct EventBusHandle {
    pub(crate) inner: Arc<HandleInner>,
}

pub(crate) struct HandleInner {
    pub(crate) store: AdaptoStore,
    pub(crate) tx: broadcast::Sender<EventEnvelope>,
    pub(crate) seq: AtomicU64,
    pub(crate) durable_topics: HashSet<&'static str>,
    pub(crate) subscribers: Vec<Subscriber>,
    pub(crate) now_fn: fn() -> DateTime<Utc>,
    pub(crate) retry_backoff: Duration,
    pub(crate) shutdown_grace: Duration,
    pub(crate) closed: AtomicBool,
    pub(crate) shutdown: Notify,
    pub(crate) inflight: AtomicUsize,
}

impl EventBusHandle {
    /// Publish an event: assign a seq, persist to the log if the topic has any
    /// durable subscriber, and broadcast to live subscribers. Synchronous core;
    /// only a single store insert in the durable case.
    pub fn emit<E: Event>(&self, e: E) {
        let payload = match serde_json::to_value(&e) {
            Ok(v) => v,
            Err(err) => {
                tracing::error!("events: serialize '{}' failed: {err}", E::TOPIC);
                return;
            }
        };
        let seq = self.inner.seq.fetch_add(1, Ordering::SeqCst) + 1;
        let ts = (self.inner.now_fn)();
        if self.inner.durable_topics.contains(E::TOPIC) {
            state::append_event(
                &self.inner.store,
                &state::EventRow { seq, topic: E::TOPIC.to_string(), payload: payload.clone(), ts },
            );
            state::save_seq_highwater(&self.inner.store, seq);
        }
        // Err only means "no live receivers" — durable subs recover from the log.
        let _ = self.inner.tx.send(EventEnvelope { seq, topic: E::TOPIC, payload, ts });
    }

    /// Async convenience wrapper around [`EventBusHandle::emit`].
    pub async fn publish<E: Event>(&self, e: E) {
        self.emit(e);
    }

    /// The current seq high-water mark (last assigned seq).
    pub(crate) fn current_seq(&self) -> u64 {
        self.inner.seq.load(Ordering::SeqCst)
    }

    /// Stop dispatchers and wait for in-flight durable handlers to drain (bounded
    /// by the configured shutdown grace).
    pub async fn shutdown(&self) {
        self.inner.closed.store(true, Ordering::SeqCst);
        self.inner.shutdown.notify_waiters();
        let deadline = tokio::time::Instant::now() + self.inner.shutdown_grace;
        while self.inner.inflight.load(Ordering::SeqCst) > 0
            && tokio::time::Instant::now() < deadline
        {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::Event;
    use crate::state;
    use crate::subscription::Subscription;
    use adapto_store::{AdaptoStore, Query};
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    struct PageViewed {
        path: String,
    }
    impl Event for PageViewed {
        const TOPIC: &'static str = "page.viewed";
        fn coalesce_key(&self) -> Option<String> {
            Some(self.path.clone())
        }
    }

    fn seqs(store: &AdaptoStore) -> Vec<u64> {
        let mut s: Vec<u64> = store
            .collection(state::EVENTS)
            .find(Query::new())
            .into_iter()
            .filter_map(|d| d.data.get("seq").and_then(|v| v.as_u64()))
            .collect();
        s.sort();
        s
    }

    #[tokio::test]
    async fn publish_durable_topic_persists() {
        let store = AdaptoStore::open(None).unwrap();
        let h = EventBus::new(store.clone())
            .subscribe(Subscription::<PageViewed>::new("d").durable().on(|_| async { Ok(()) }))
            .spawn();
        h.publish(PageViewed { path: "/a".into() }).await;
        assert_eq!(store.collection(state::EVENTS).count_all(), 1);
    }

    #[tokio::test]
    async fn publish_ephemeral_only_topic_no_log() {
        let store = AdaptoStore::open(None).unwrap();
        let h = EventBus::new(store.clone())
            .subscribe(Subscription::<PageViewed>::new("e").ephemeral().on(|_| async { Ok(()) }))
            .spawn();
        h.publish(PageViewed { path: "/a".into() }).await;
        assert_eq!(store.collection(state::EVENTS).count_all(), 0);
    }

    #[tokio::test]
    async fn seq_monotonic_across_publishes() {
        let store = AdaptoStore::open(None).unwrap();
        let h = EventBus::new(store.clone())
            .subscribe(Subscription::<PageViewed>::new("d").durable().on(|_| async { Ok(()) }))
            .spawn();
        h.publish(PageViewed { path: "/a".into() }).await;
        h.publish(PageViewed { path: "/b".into() }).await;
        assert_eq!(seqs(&store), vec![1, 2]);
    }

    #[tokio::test]
    async fn seq_resumes_from_highwater() {
        let store = AdaptoStore::open(None).unwrap();
        state::ensure_indexes(&store);
        state::append_event(
            &store,
            &state::EventRow {
                seq: 5,
                topic: "page.viewed".into(),
                payload: serde_json::json!({}),
                ts: chrono::Utc::now(),
            },
        );
        let h = EventBus::new(store.clone())
            .subscribe(Subscription::<PageViewed>::new("d").durable().on(|_| async { Ok(()) }))
            .spawn();
        h.publish(PageViewed { path: "/a".into() }).await;
        assert_eq!(seqs(&store).into_iter().max().unwrap(), 6);
    }
}
