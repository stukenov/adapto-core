//! The typed [`Subscription`] builder and the type-erased [`Subscriber`] the
//! bus stores. Erasure happens in [`Subscription::build`]: predicate, coalesce
//! key, and handler each deserialize the JSON payload to `E` and call the
//! user's typed closure, so the dispatcher works only with `serde_json::Value`.

use crate::event::{Event, EventError};
use serde_json::Value;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

/// Delivery guarantee of a subscription.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// At-most-once, in-memory, no persistence.
    Ephemeral,
    /// At-least-once, log + cursor, retry, dead-letter.
    Durable,
}

/// Resolved delivery-control configuration for a subscription.
#[derive(Clone, Debug)]
pub struct ThrottleCfg {
    pub coalesce: Option<Duration>,
    pub rate: Option<(u32, Duration)>,
    pub sample: f64,
}

impl Default for ThrottleCfg {
    fn default() -> Self {
        ThrottleCfg { coalesce: None, rate: None, sample: 1.0 }
    }
}

type BoxFut = Pin<Box<dyn Future<Output = Result<(), EventError>> + Send>>;
/// Erased predicate: deserialize payload to `E`, apply the typed user filter.
type PredicateFn = Arc<dyn Fn(&Value) -> bool + Send + Sync>;
/// Erased coalesce-key extractor.
type KeyFn = Arc<dyn Fn(&Value) -> Option<String> + Send + Sync>;
/// Erased handler: deserialize payload to `E`, run the user closure.
type HandlerFn = Arc<dyn Fn(Value) -> BoxFut + Send + Sync>;

/// Type-erased subscriber stored and dispatched by the bus.
#[derive(Clone)]
pub struct Subscriber {
    pub id: String,
    pub topic: &'static str,
    pub mode: Mode,
    pub throttle: ThrottleCfg,
    pub max_attempts: u32,
    /// Deserializes the payload to `E` then applies the typed user filter.
    /// Returns `false` if the payload does not deserialize to `E`.
    pub predicate: PredicateFn,
    /// Deserializes the payload to `E` then calls `E::coalesce_key`.
    pub coalesce_key: KeyFn,
    /// Deserializes the payload to `E` then runs the user handler.
    pub handler: HandlerFn,
}

/// A typed subscription builder. Targets one event type `E`.
pub struct Subscription<E: Event> {
    id: String,
    mode: Mode,
    throttle: ThrottleCfg,
    max_attempts: u32,
    filter: Arc<dyn Fn(&E) -> bool + Send + Sync>,
    handler: Option<HandlerFn>,
    _marker: PhantomData<fn() -> E>,
}

impl<E: Event> Subscription<E> {
    /// Start a subscription with a unique id (the cursor / dead-letter key).
    pub fn new(id: impl Into<String>) -> Self {
        Subscription {
            id: id.into(),
            mode: Mode::Ephemeral,
            throttle: ThrottleCfg::default(),
            max_attempts: 3,
            filter: Arc::new(|_| true),
            handler: None,
            _marker: PhantomData,
        }
    }

    /// At-most-once delivery (the default).
    pub fn ephemeral(mut self) -> Self {
        self.mode = Mode::Ephemeral;
        self
    }

    /// At-least-once delivery (log + cursor + retry + dead-letter).
    pub fn durable(mut self) -> Self {
        self.mode = Mode::Durable;
        self
    }

    /// Typed predicate on the event. Only matching events are delivered.
    pub fn filter<F>(mut self, f: F) -> Self
    where
        F: Fn(&E) -> bool + Send + Sync + 'static,
    {
        self.filter = Arc::new(f);
        self
    }

    /// Suppress repeats of the same `coalesce_key` within `window`.
    pub fn coalesce_by_key(mut self, window: Duration) -> Self {
        self.throttle.coalesce = Some(window);
        self
    }

    /// Limit handler invocations to `n` per `per`.
    pub fn rate_limit(mut self, n: u32, per: Duration) -> Self {
        self.throttle.rate = Some((n, per));
        self
    }

    /// Deterministically sample a `fraction` of distinct keys.
    pub fn sample(mut self, fraction: f64) -> Self {
        self.throttle.sample = fraction;
        self
    }

    /// Retry budget before a durable event is dead-lettered. Floored at 1.
    pub fn max_attempts(mut self, n: u32) -> Self {
        self.max_attempts = n.max(1);
        self
    }

    /// Set the async handler and finalize the configuration.
    pub fn on<F, Fut>(mut self, f: F) -> Self
    where
        F: Fn(E) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), EventError>> + Send + 'static,
    {
        let handler: HandlerFn =
            Arc::new(move |v: Value| match serde_json::from_value::<E>(v) {
                Ok(e) => Box::pin(f(e)) as BoxFut,
                Err(err) => Box::pin(async move { Err(EventError::from_err(err)) }) as BoxFut,
            });
        self.handler = Some(handler);
        self
    }

    /// Erase the typed subscription into a [`Subscriber`].
    ///
    /// # Panics
    /// Panics if [`Subscription::on`] was never called.
    pub fn build(self) -> Subscriber {
        let filter = self.filter.clone();
        let predicate: PredicateFn =
            Arc::new(move |v: &Value| match serde_json::from_value::<E>(v.clone()) {
                Ok(e) => filter(&e),
                Err(_) => false,
            });
        let coalesce_key: KeyFn = Arc::new(move |v: &Value| {
            serde_json::from_value::<E>(v.clone()).ok().and_then(|e| e.coalesce_key())
        });
        Subscriber {
            id: self.id,
            topic: E::TOPIC,
            mode: self.mode,
            throttle: self.throttle,
            max_attempts: self.max_attempts,
            predicate,
            coalesce_key,
            handler: self.handler.expect("Subscription requires .on(...)"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::Event;
    use serde::{Deserialize, Serialize};
    use serde_json::json;
    use std::sync::atomic::{AtomicBool, Ordering};
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

    fn sub() -> Subscription<PageViewed> {
        Subscription::<PageViewed>::new("scraper.freshness")
            .durable()
            .filter(|e| e.path.starts_with("/w"))
            .coalesce_by_key(Duration::from_secs(3600))
            .rate_limit(10, Duration::from_secs(60))
            .max_attempts(3)
            .on(|_e| async { Ok(()) })
    }

    #[test]
    fn builder_sets_fields() {
        let s = sub().build();
        assert_eq!(s.id, "scraper.freshness");
        assert_eq!(s.topic, "page.viewed");
        assert_eq!(s.mode, Mode::Durable);
        assert_eq!(s.max_attempts, 3);
        assert!(s.throttle.coalesce.is_some());
        assert_eq!(s.throttle.rate, Some((10, Duration::from_secs(60))));
    }

    #[test]
    fn erased_predicate_filters() {
        let s = sub().build();
        assert!((s.predicate)(&json!({ "path": "/weather" })));
        assert!(!(s.predicate)(&json!({ "path": "/x" })));
    }

    #[test]
    fn erased_coalesce_key() {
        let s = sub().build();
        assert_eq!((s.coalesce_key)(&json!({ "path": "/weather" })), Some("/weather".into()));
    }

    #[tokio::test]
    async fn erased_handler_runs() {
        let flag = Arc::new(AtomicBool::new(false));
        let f2 = flag.clone();
        let s = Subscription::<PageViewed>::new("x")
            .on(move |_e| {
                let f = f2.clone();
                async move {
                    f.store(true, Ordering::SeqCst);
                    Ok(())
                }
            })
            .build();
        (s.handler)(json!({ "path": "/a" })).await.unwrap();
        assert!(flag.load(Ordering::SeqCst));
    }

    #[test]
    fn max_attempts_floor_one() {
        let s = Subscription::<PageViewed>::new("x").max_attempts(0).on(|_| async { Ok(()) }).build();
        assert_eq!(s.max_attempts, 1);
    }

    #[test]
    fn default_mode_ephemeral() {
        let s = Subscription::<PageViewed>::new("x").on(|_| async { Ok(()) }).build();
        assert_eq!(s.mode, Mode::Ephemeral);
    }
}
