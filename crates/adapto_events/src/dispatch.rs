//! Per-subscription dispatcher loops. Ephemeral subscribers consume the live
//! broadcast directly; durable subscribers (added later) treat the broadcast as
//! a wake signal and drain the `_events` log from their cursor.

use crate::bus::EventBusHandle;
use crate::event::EventEnvelope;
use crate::subscription::{Mode, Subscriber, ThrottleCfg};
use crate::throttle::{self, Coalesce, RateLimiter};
use std::time::{Duration, Instant};
use tokio::sync::broadcast::error::RecvError;

/// Start dispatcher tasks for every subscriber. Called by [`crate::bus::EventBus::spawn`].
pub fn start(
    handle: &EventBusHandle,
    subscribers: &[Subscriber],
    _gc_interval: Duration,
    _retention: Duration,
) {
    for sub in subscribers {
        match sub.mode {
            Mode::Ephemeral => spawn_ephemeral(handle, sub.clone()),
            Mode::Durable => { /* added in the durable-dispatcher task */ }
        }
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
}
