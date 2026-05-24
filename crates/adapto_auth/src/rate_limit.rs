use std::collections::HashMap;
use std::time::Instant;

/// Per-session token-bucket rate limiter.
///
/// Each session gets an independent bucket that refills at `default_rate`
/// tokens per second. A check consumes one token; if no tokens remain the
/// request is denied until the bucket refills.
#[derive(Debug)]
pub struct RateLimiter {
    limits: HashMap<String, TokenBucket>,
    default_rate: u32,
}

#[derive(Debug)]
struct TokenBucket {
    tokens: f64,
    max_tokens: f64,
    refill_rate: f64,
    last_refill: Instant,
}

impl TokenBucket {
    fn new(rate: u32) -> Self {
        Self {
            tokens: rate as f64,
            max_tokens: rate as f64,
            refill_rate: rate as f64,
            last_refill: Instant::now(),
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.max_tokens);
        self.last_refill = now;
    }

    fn try_consume(&mut self) -> bool {
        self.refill();
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

impl RateLimiter {
    /// Create a new rate limiter with the given default rate (events per
    /// second).
    pub fn new(default_rate: u32) -> Self {
        Self {
            limits: HashMap::new(),
            default_rate,
        }
    }

    /// Check whether the session is within its rate limit, consuming one
    /// token if so. Returns `Ok(())` on success, `Err(())` when the limit is
    /// exceeded.
    pub fn check(&mut self, session_id: &str) -> Result<(), ()> {
        let bucket = self
            .limits
            .entry(session_id.to_string())
            .or_insert_with(|| TokenBucket::new(self.default_rate));

        if bucket.try_consume() {
            Ok(())
        } else {
            Err(())
        }
    }

    /// Reset the token bucket for a session, restoring it to full capacity.
    pub fn reset(&mut self, session_id: &str) {
        if let Some(bucket) = self.limits.get_mut(session_id) {
            bucket.tokens = bucket.max_tokens;
            bucket.last_refill = Instant::now();
        }
    }

    /// Remove a session's bucket entirely (e.g. on disconnect).
    pub fn remove(&mut self, session_id: &str) {
        self.limits.remove(session_id);
    }
}
