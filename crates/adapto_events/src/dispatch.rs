//! Per-subscription dispatcher loops. Ephemeral subscribers consume the live
//! broadcast directly; durable subscribers (added later) treat the broadcast as
//! a wake signal and drain the `_events` log from their cursor.

use crate::bus::{EventBusHandle, HandleInner};
use crate::event::{EventEnvelope, EventError};
use crate::state::{self, Cursor, EventRow};
use crate::subscription::{Mode, Subscriber, ThrottleCfg};
use crate::throttle::{self, Coalesce, RateLimiter};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::broadcast::error::RecvError;

/// Start dispatcher tasks for every subscriber. Called by [`crate::bus::EventBus::spawn`].
pub fn start(
    handle: &EventBusHandle,
    subscribers: &[Subscriber],
    gc_interval: Duration,
    retention: Duration,
) {
    let mut topic_subs: HashMap<String, Vec<String>> = HashMap::new();
    for sub in subscribers {
        match sub.mode {
            Mode::Ephemeral => spawn_ephemeral(handle, sub.clone()),
            Mode::Durable => {
                topic_subs.entry(sub.topic.to_string()).or_default().push(sub.id.clone());
                spawn_durable(handle, sub.clone());
            }
        }
    }
    if !topic_subs.is_empty() {
        spawn_gc(handle, topic_subs, gc_interval, retention);
    }
}

/// `None` if this event should be processed now; `Some(())` if it was filtered
/// or throttled away. For ephemeral subscribers a missing rate-limit token
/// drops the event.
fn ephemeral_gate(
    sub: &Subscriber,
    env: &EventEnvelope,
    coalesce: &mut Option<Coalesce>,
    rl: &mut Option<RateLimiter>,
) -> bool {
    if env.topic != sub.topic {
        return false;
    }
    if !(sub.predicate)(&env.payload) {
        return false;
    }
    let key = (sub.coalesce_key)(&env.payload);
    if !sample_ok(&sub.throttle, key.as_deref()) {
        return false;
    }
    if let (Some(c), Some(k)) = (coalesce.as_mut(), key.as_deref()) {
        if !c.allow(k, Instant::now()) {
            return false;
        }
    }
    if let Some(r) = rl.as_mut() {
        if r.take(Instant::now()).is_some() {
            return false; // ephemeral: no token -> drop
        }
    }
    true
}

/// Sampling passes if no fraction is configured (1.0), or there is no key to
/// sample by, or the key falls inside the sampled fraction.
fn sample_ok(throttle: &ThrottleCfg, key: Option<&str>) -> bool {
    if throttle.sample >= 1.0 {
        return true;
    }
    match key {
        Some(k) => throttle::sample_in(k, throttle.sample),
        None => true,
    }
}

fn run_handler(sub: &Subscriber, payload: serde_json::Value) {
    let handler = sub.handler.clone();
    let id = sub.id.clone();
    tokio::spawn(async move {
        if let Err(e) = (handler)(payload).await {
            tracing::warn!("events: ephemeral '{id}' handler error: {e}");
        }
    });
}

fn spawn_ephemeral(handle: &EventBusHandle, sub: Subscriber) {
    let mut rx = handle.inner.tx.subscribe();
    tokio::spawn(async move {
        let mut coalesce = sub.throttle.coalesce.map(Coalesce::new);
        let mut rl = sub.throttle.rate.map(|(n, per)| RateLimiter::new(n, per));
        loop {
            match rx.recv().await {
                Ok(env) => {
                    if ephemeral_gate(&sub, &env, &mut coalesce, &mut rl) {
                        run_handler(&sub, env.payload);
                    }
                }
                Err(RecvError::Lagged(_)) => continue,
                Err(RecvError::Closed) => break,
            }
        }
    });
}

// ---------------------------------------------------------------------------
// Durable dispatcher
// ---------------------------------------------------------------------------

fn spawn_durable(handle: &EventBusHandle, sub: Subscriber) {
    let inner = handle.inner.clone();
    let mut rx = inner.tx.subscribe();
    tokio::spawn(async move {
        let mut cursor = state::load_cursor(&inner.store, &sub.id).unwrap_or_else(|| Cursor {
            subscription: sub.id.clone(),
            topic: sub.topic.to_string(),
            last_seq: 0,
            processed: 0,
            failed: 0,
            updated_at: (inner.now_fn)(),
        });
        let mut coalesce = sub.throttle.coalesce.map(Coalesce::new);
        let mut rl = sub.throttle.rate.map(|(n, per)| RateLimiter::new(n, per));

        // Initial catch-up drain (replays anything missed while down).
        drain(&inner, &sub, &mut cursor, &mut coalesce, &mut rl).await;

        loop {
            if inner.closed.load(Ordering::SeqCst) {
                break;
            }
            tokio::select! {
                _ = inner.shutdown.notified() => break,
                r = rx.recv() => match r {
                    Ok(_) | Err(RecvError::Lagged(_)) => {}
                    Err(RecvError::Closed) => break,
                },
                _ = tokio::time::sleep(Duration::from_millis(200)) => {}
            }
            drain(&inner, &sub, &mut cursor, &mut coalesce, &mut rl).await;
        }
    });
}

/// Process every logged event with `seq > cursor.last_seq`, in order.
async fn drain(
    inner: &Arc<HandleInner>,
    sub: &Subscriber,
    cursor: &mut Cursor,
    coalesce: &mut Option<Coalesce>,
    rl: &mut Option<RateLimiter>,
) {
    let rows = state::load_events_after(&inner.store, sub.topic, cursor.last_seq);
    for row in rows {
        if inner.closed.load(Ordering::SeqCst) {
            break;
        }
        process_durable(inner, sub, cursor, coalesce, rl, row).await;
    }
}

async fn process_durable(
    inner: &Arc<HandleInner>,
    sub: &Subscriber,
    cursor: &mut Cursor,
    coalesce: &mut Option<Coalesce>,
    rl: &mut Option<RateLimiter>,
    row: EventRow,
) {
    // Filter / sample / coalesce gates ack the event without invoking.
    if !(sub.predicate)(&row.payload) {
        return advance(inner, cursor, row.seq);
    }
    let key = (sub.coalesce_key)(&row.payload);
    if !sample_ok(&sub.throttle, key.as_deref()) {
        return advance(inner, cursor, row.seq);
    }
    if let (Some(c), Some(k)) = (coalesce.as_mut(), key.as_deref()) {
        if !c.allow(k, Instant::now()) {
            return advance(inner, cursor, row.seq);
        }
    }
    // Rate-limit: durable paces (waits) rather than dropping.
    if let Some(r) = rl.as_mut() {
        while let Some(wait) = r.take(Instant::now()) {
            if inner.closed.load(Ordering::SeqCst) {
                return; // leave the cursor; re-drain on next start
            }
            tokio::time::sleep(wait).await;
        }
    }

    inner.inflight.fetch_add(1, Ordering::SeqCst);
    let mut attempt: u32 = 1;
    let result = loop {
        match run_handler_isolated(sub, row.payload.clone()).await {
            Ok(()) => break Ok(()),
            Err(e) => {
                if attempt < sub.max_attempts {
                    tokio::time::sleep(backoff(inner.retry_backoff, attempt)).await;
                    attempt += 1;
                } else {
                    break Err(e);
                }
            }
        }
    };
    inner.inflight.fetch_sub(1, Ordering::SeqCst);

    match result {
        Ok(()) => {
            cursor.processed += 1;
            advance(inner, cursor, row.seq);
        }
        Err(e) => {
            tracing::error!("events: durable '{}' seq {} dead-lettered: {e}", sub.id, row.seq);
            state::write_deadletter(
                &inner.store,
                &state::DeadLetter {
                    subscription: sub.id.clone(),
                    seq: row.seq,
                    topic: row.topic.clone(),
                    payload: row.payload.clone(),
                    attempts: attempt,
                    last_error: e.to_string(),
                    failed_at: (inner.now_fn)(),
                },
            );
            cursor.failed += 1;
            advance(inner, cursor, row.seq);
        }
    }
}

/// Advance and persist the cursor to `seq`.
fn advance(inner: &Arc<HandleInner>, cursor: &mut Cursor, seq: u64) {
    cursor.last_seq = seq;
    cursor.updated_at = (inner.now_fn)();
    state::save_cursor(&inner.store, cursor);
}

/// Run the handler on its own task to isolate panics.
async fn run_handler_isolated(sub: &Subscriber, payload: Value) -> Result<(), EventError> {
    let handler = sub.handler.clone();
    let jh = tokio::spawn(async move { (handler)(payload).await });
    match jh.await {
        Ok(r) => r,
        Err(join) => Err(EventError::msg(if join.is_panic() {
            "handler panicked"
        } else {
            "handler cancelled"
        })),
    }
}

/// Exponential backoff: `base * 2^(attempt-1)`, capped at `base * 16`.
fn backoff(base: Duration, attempt: u32) -> Duration {
    let factor = 1u32.checked_shl(attempt.saturating_sub(1)).unwrap_or(u32::MAX).min(16);
    base.saturating_mul(factor)
}

// ---------------------------------------------------------------------------
// Log GC
// ---------------------------------------------------------------------------

fn spawn_gc(
    handle: &EventBusHandle,
    topic_subs: HashMap<String, Vec<String>>,
    gc_interval: Duration,
    retention: Duration,
) {
    let inner = handle.inner.clone();
    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = inner.shutdown.notified() => break,
                _ = tokio::time::sleep(gc_interval) => {}
            }
            if inner.closed.load(Ordering::SeqCst) {
                break;
            }
            for (topic, subs) in &topic_subs {
                if let Some(min) = state::min_cursor_seq(&inner.store, subs) {
                    state::gc_events(&inner.store, topic, min);
                }
            }
            if let Ok(ret) = chrono::Duration::from_std(retention) {
                state::prune_deadletter(&inner.store, (inner.now_fn)() - ret);
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use crate::event::Event;
    use crate::subscription::Subscription;
    use crate::EventBus;
    use adapto_store::AdaptoStore;
    use serde::{Deserialize, Serialize};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

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

    async fn settle() {
        tokio::time::sleep(Duration::from_millis(60)).await;
    }

    #[tokio::test]
    async fn ephemeral_handler_receives() {
        let store = AdaptoStore::open(None).unwrap();
        let n = Arc::new(AtomicUsize::new(0));
        let n2 = n.clone();
        let h = EventBus::new(store)
            .subscribe(Subscription::<PageViewed>::new("e").ephemeral().on(move |_e| {
                let n = n2.clone();
                async move {
                    n.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                }
            }))
            .spawn();
        h.publish(PageViewed { path: "/a".into() }).await;
        h.publish(PageViewed { path: "/b".into() }).await;
        settle().await;
        assert_eq!(n.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn ephemeral_predicate_filters() {
        let store = AdaptoStore::open(None).unwrap();
        let n = Arc::new(AtomicUsize::new(0));
        let n2 = n.clone();
        let h = EventBus::new(store)
            .subscribe(
                Subscription::<PageViewed>::new("e")
                    .ephemeral()
                    .filter(|e| e.path.starts_with("/w"))
                    .on(move |_e| {
                        let n = n2.clone();
                        async move {
                            n.fetch_add(1, Ordering::SeqCst);
                            Ok(())
                        }
                    }),
            )
            .spawn();
        h.publish(PageViewed { path: "/x".into() }).await;
        h.publish(PageViewed { path: "/weather".into() }).await;
        settle().await;
        assert_eq!(n.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn ephemeral_coalesce_suppresses() {
        let store = AdaptoStore::open(None).unwrap();
        let n = Arc::new(AtomicUsize::new(0));
        let n2 = n.clone();
        let h = EventBus::new(store)
            .subscribe(
                Subscription::<PageViewed>::new("e")
                    .ephemeral()
                    .coalesce_by_key(Duration::from_secs(3600))
                    .on(move |_e| {
                        let n = n2.clone();
                        async move {
                            n.fetch_add(1, Ordering::SeqCst);
                            Ok(())
                        }
                    }),
            )
            .spawn();
        h.publish(PageViewed { path: "/a".into() }).await;
        h.publish(PageViewed { path: "/a".into() }).await;
        settle().await;
        assert_eq!(n.load(Ordering::SeqCst), 1);
    }

    // ---- durable ----------------------------------------------------------

    use crate::state;

    fn seed(store: &AdaptoStore, seq: u64, path: &str) {
        state::append_event(
            store,
            &state::EventRow {
                seq,
                topic: "page.viewed".into(),
                payload: serde_json::json!({ "path": path }),
                ts: chrono::Utc::now(),
            },
        );
    }

    async fn poll_until<F: Fn() -> bool>(cond: F) {
        for _ in 0..100 {
            if cond() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    #[tokio::test]
    async fn durable_handler_processes_and_advances() {
        let store = AdaptoStore::open(None).unwrap();
        let n = Arc::new(AtomicUsize::new(0));
        let n2 = n.clone();
        let h = EventBus::new(store.clone())
            .retry_backoff(Duration::from_millis(1))
            .subscribe(Subscription::<PageViewed>::new("d").durable().on(move |_e| {
                let n = n2.clone();
                async move {
                    n.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                }
            }))
            .spawn();
        h.publish(PageViewed { path: "/a".into() }).await;
        let s = store.clone();
        poll_until(|| state::load_cursor(&s, "d").map(|c| c.last_seq).unwrap_or(0) >= 1).await;
        assert_eq!(n.load(Ordering::SeqCst), 1);
        let cur = state::load_cursor(&store, "d").unwrap();
        assert_eq!(cur.last_seq, 1);
        assert_eq!(cur.processed, 1);
    }

    #[tokio::test]
    async fn durable_restart_replays_missed() {
        let store = AdaptoStore::open(None).unwrap();
        state::ensure_indexes(&store);
        seed(&store, 1, "/a");
        seed(&store, 2, "/b");
        let n = Arc::new(AtomicUsize::new(0));
        let n2 = n.clone();
        let _h = EventBus::new(store.clone())
            .retry_backoff(Duration::from_millis(1))
            .subscribe(Subscription::<PageViewed>::new("d").durable().on(move |_e| {
                let n = n2.clone();
                async move {
                    n.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                }
            }))
            .spawn();
        let s = store.clone();
        poll_until(|| state::load_cursor(&s, "d").map(|c| c.last_seq).unwrap_or(0) >= 2).await;
        assert_eq!(n.load(Ordering::SeqCst), 2);
        assert_eq!(state::load_cursor(&store, "d").unwrap().last_seq, 2);
    }

    #[tokio::test]
    async fn durable_poison_deadletters_and_advances() {
        let store = AdaptoStore::open(None).unwrap();
        state::ensure_indexes(&store);
        seed(&store, 1, "/bad");
        seed(&store, 2, "/good");
        let ok = Arc::new(AtomicUsize::new(0));
        let ok2 = ok.clone();
        let _h = EventBus::new(store.clone())
            .retry_backoff(Duration::from_millis(1))
            .subscribe(
                Subscription::<PageViewed>::new("d").durable().max_attempts(2).on(move |e| {
                    let ok = ok2.clone();
                    async move {
                        if e.path == "/bad" {
                            Err(crate::EventError::msg("boom"))
                        } else {
                            ok.fetch_add(1, Ordering::SeqCst);
                            Ok(())
                        }
                    }
                }),
            )
            .spawn();
        let s = store.clone();
        poll_until(|| state::load_cursor(&s, "d").map(|c| c.last_seq).unwrap_or(0) >= 2).await;
        let cur = state::load_cursor(&store, "d").unwrap();
        assert_eq!(cur.last_seq, 2);
        assert_eq!(cur.failed, 1);
        assert_eq!(ok.load(Ordering::SeqCst), 1);
        assert_eq!(store.collection(state::DEADLETTER).count_all(), 1);
    }

    #[tokio::test]
    async fn gc_trims_consumed() {
        let store = AdaptoStore::open(None).unwrap();
        let h = EventBus::new(store.clone())
            .retry_backoff(Duration::from_millis(1))
            .gc_interval(Duration::from_millis(30))
            .subscribe(Subscription::<PageViewed>::new("d").durable().on(|_| async { Ok(()) }))
            .spawn();
        h.publish(PageViewed { path: "/a".into() }).await;
        h.publish(PageViewed { path: "/b".into() }).await;
        let s = store.clone();
        poll_until(|| state::load_cursor(&s, "d").map(|c| c.last_seq).unwrap_or(0) >= 2).await;
        tokio::time::sleep(Duration::from_millis(90)).await; // let a GC cycle run
        assert_eq!(store.collection(state::EVENTS).count_all(), 0);
    }
}
