//! Generic delivery-control primitives applied by a dispatcher before invoking
//! a handler. All are pure and clock-injected (callers pass `now: Instant`), so
//! they are deterministic in tests.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::time::{Duration, Instant};

/// Suppresses repeated keys within a time window. The first event for a key is
/// allowed; further events with the same key are suppressed until `window`
/// elapses since the last allowed one.
pub struct Coalesce {
    window: Duration,
    seen: HashMap<String, Instant>,
}

impl Coalesce {
    pub fn new(window: Duration) -> Self {
        Coalesce { window, seen: HashMap::new() }
    }

    /// Returns `true` if this `key` should be processed now (and records it).
    pub fn allow(&mut self, key: &str, now: Instant) -> bool {
        match self.seen.get(key) {
            Some(&last) if now.saturating_duration_since(last) < self.window => false,
            _ => {
                self.seen.insert(key.to_string(), now);
                true
            }
        }
    }
}

/// Token-bucket rate limiter. `n` tokens refill linearly over `per`.
pub struct RateLimiter {
    capacity: f64,
    tokens: f64,
    refill_per_sec: f64,
    last: Option<Instant>,
}

impl RateLimiter {
    pub fn new(n: u32, per: Duration) -> Self {
        let capacity = n.max(1) as f64;
        let secs = per.as_secs_f64().max(f64::MIN_POSITIVE);
        RateLimiter {
            capacity,
            tokens: capacity,
            refill_per_sec: capacity / secs,
            last: None,
        }
    }

    /// Consume one token. Returns `None` if a token was available (consumed);
    /// `Some(wait)` is how long until the next token frees.
    pub fn take(&mut self, now: Instant) -> Option<Duration> {
        if let Some(last) = self.last {
            let elapsed = now.saturating_duration_since(last).as_secs_f64();
            self.tokens = (self.tokens + elapsed * self.refill_per_sec).min(self.capacity);
        }
        self.last = Some(now);
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            None
        } else {
            let wait = (1.0 - self.tokens) / self.refill_per_sec;
            Some(Duration::from_secs_f64(wait))
        }
    }
}

/// Deterministic sampling: returns `true` for a stable `fraction` of distinct
/// keys (the same key always lands the same way — no RNG). `fraction >= 1.0`
/// always passes, `<= 0.0` never passes.
pub fn sample_in(key: &str, fraction: f64) -> bool {
    if fraction >= 1.0 {
        return true;
    }
    if fraction <= 0.0 {
        return false;
    }
    let mut h = std::collections::hash_map::DefaultHasher::new();
    key.hash(&mut h);
    let bucket = h.finish() % 10_000;
    (bucket as f64) < fraction * 10_000.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn coalesce_first_allows_then_suppresses_then_reallows() {
        let base = Instant::now();
        let mut c = Coalesce::new(Duration::from_secs(60));
        assert!(c.allow("k", base)); // first
        assert!(!c.allow("k", base + Duration::from_secs(30))); // within window
        assert!(c.allow("k", base + Duration::from_secs(61))); // after window
        assert!(c.allow("other", base + Duration::from_secs(30))); // different key
    }

    #[test]
    fn rate_limiter_allows_n_then_waits() {
        let base = Instant::now();
        let mut r = RateLimiter::new(2, Duration::from_secs(1));
        assert!(r.take(base).is_none());
        assert!(r.take(base).is_none());
        assert!(r.take(base).is_some());
    }

    #[test]
    fn rate_limiter_refills_after_per() {
        let base = Instant::now();
        let mut r = RateLimiter::new(2, Duration::from_secs(1));
        r.take(base);
        r.take(base); // drained
        assert!(r.take(base).is_some()); // empty
        assert!(r.take(base + Duration::from_secs(1)).is_none()); // refilled
    }

    #[test]
    fn sample_deterministic_and_bounds() {
        assert!(sample_in("weather/x", 1.0));
        assert!(!sample_in("weather/x", 0.0));
        let a = sample_in("weather/x", 0.5);
        let b = sample_in("weather/x", 0.5);
        assert_eq!(a, b);
    }
}
