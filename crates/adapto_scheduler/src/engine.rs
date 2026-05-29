use crate::job::Priority;
use crate::state::{JobState, RunStatus};
use chrono::{DateTime, Duration as ChDuration, Utc};

/// Backoff delay before the next attempt, given the attempt that just failed
/// (1-based). 1m, 5m, then 15m (capped).
pub fn backoff(attempt: u32) -> ChDuration {
    match attempt {
        1 => ChDuration::minutes(1),
        2 => ChDuration::minutes(5),
        _ => ChDuration::minutes(15),
    }
}

/// Outcome of a single handler run.
pub enum Outcome {
    Ok,
    Err(String),
}

/// Apply a run outcome to job state in place. `next_scheduled` is
/// `schedule.next_after(now)` (the normal cadence target).
#[allow(clippy::too_many_arguments)]
pub fn apply_outcome(
    st: &mut JobState,
    outcome: Outcome,
    attempt: u32,
    max_attempts: u32,
    duration_ms: u64,
    now: DateTime<Utc>,
    next_scheduled: DateTime<Utc>,
) {
    st.last_run = Some(now);
    st.last_duration_ms = Some(duration_ms);
    st.updated_at = now;
    match outcome {
        Outcome::Ok => {
            st.status = RunStatus::Ok;
            st.last_error = None;
            st.attempt = 0;
            st.consecutive_failures = 0;
            st.total_runs += 1;
            st.next_run = Some(next_scheduled);
        }
        Outcome::Err(msg) => {
            st.total_failures += 1;
            if attempt < max_attempts {
                st.status = RunStatus::Retrying;
                st.attempt = attempt;
                st.last_error = Some(msg);
                st.next_run = Some(now + backoff(attempt));
            } else {
                st.status = RunStatus::Failed;
                st.attempt = 0;
                st.consecutive_failures += 1;
                st.last_error = Some(msg);
                st.next_run = Some(next_scheduled);
            }
        }
    }
}

/// Catch-up decision at boot. Returns `(next_run_to_persist, run_now_once)`.
pub fn catch_up_decision(
    persisted_next: Option<DateTime<Utc>>,
    catch_up: bool,
    now: DateTime<Utc>,
    next_scheduled: DateTime<Utc>,
) -> (DateTime<Utc>, bool) {
    match persisted_next {
        Some(nr) if nr <= now => {
            // missed: run once now if catch_up, then resume normal cadence
            (next_scheduled, catch_up)
        }
        Some(nr) => (nr, false), // still in the future -> keep
        None => (next_scheduled, false), // first ever
    }
}

/// Ordering key for the per-lane ready heap: highest priority first, then earliest due.
#[derive(PartialEq, Eq)]
pub struct ReadyKey {
    pub priority: Priority,
    pub due: DateTime<Utc>,
}

impl Ord for ReadyKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // BinaryHeap is a max-heap: highest priority + earliest due pop first.
        self.priority
            .cmp(&other.priority)
            .then_with(|| other.due.cmp(&self.due)) // earlier due = "greater"
    }
}
impl PartialOrd for ReadyKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(s: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(s, 0).unwrap()
    }

    #[test]
    fn ok_resets_failures_and_sets_next() {
        let mut st = JobState::seed("x", "light", "high", "d", t(0));
        st.consecutive_failures = 2;
        apply_outcome(&mut st, Outcome::Ok, 1, 3, 100, t(10), t(1000));
        assert_eq!(st.status, RunStatus::Ok);
        assert_eq!(st.consecutive_failures, 0);
        assert_eq!(st.total_runs, 1);
        assert_eq!(st.next_run, Some(t(1000)));
    }

    #[test]
    fn err_with_attempts_left_retries_with_backoff() {
        let mut st = JobState::seed("x", "light", "high", "d", t(0));
        apply_outcome(&mut st, Outcome::Err("boom".into()), 1, 3, 50, t(100), t(99999));
        assert_eq!(st.status, RunStatus::Retrying);
        assert_eq!(st.next_run, Some(t(100) + ChDuration::minutes(1)));
        assert_eq!(st.total_failures, 1);
    }

    #[test]
    fn err_final_attempt_fails_and_resumes_schedule() {
        let mut st = JobState::seed("x", "light", "high", "d", t(0));
        apply_outcome(&mut st, Outcome::Err("boom".into()), 3, 3, 50, t(100), t(99999));
        assert_eq!(st.status, RunStatus::Failed);
        assert_eq!(st.next_run, Some(t(99999)));
        assert_eq!(st.consecutive_failures, 1);
    }

    #[test]
    fn catch_up_true_runs_once() {
        let (next, run_now) = catch_up_decision(Some(t(10)), true, t(100), t(5000));
        assert_eq!(next, t(5000));
        assert!(run_now);
    }

    #[test]
    fn catch_up_false_skips() {
        let (next, run_now) = catch_up_decision(Some(t(10)), false, t(100), t(5000));
        assert_eq!(next, t(5000));
        assert!(!run_now);
    }

    #[test]
    fn future_next_run_preserved() {
        let (next, run_now) = catch_up_decision(Some(t(9999)), true, t(100), t(5000));
        assert_eq!(next, t(9999));
        assert!(!run_now);
    }

    #[test]
    fn ready_key_orders_priority_then_due() {
        use std::collections::BinaryHeap;
        let mut h = BinaryHeap::new();
        h.push(ReadyKey { priority: Priority::Low, due: t(1) });
        h.push(ReadyKey { priority: Priority::High, due: t(5) });
        h.push(ReadyKey { priority: Priority::High, due: t(2) });
        let first = h.pop().unwrap();
        assert_eq!(first.priority, Priority::High);
        assert_eq!(first.due, t(2)); // earlier due among High
    }
}
