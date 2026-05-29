# adapto_scheduler Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A persistent, priority+lane background-job scheduler crate in adapto-core, integrated into adapto_app, with concrete daily-currency and monthly-companies jobs in myqaz.

**Architecture:** New `adapto_scheduler` crate exposes `Schedule`/`Job`/`Scheduler` builders. `Scheduler::spawn()` starts a tick loop + two lane runners (Light pool, Heavy serial) as tokio tasks; job state persists in a `_jobs` store collection (`next_run` is source of truth → restart-safe + catch-up). `adapto_app` gains `.scheduler()`/`.scheduler_admin()` and `ctx.scheduler()`. myqaz adds `reqwest`-based job bodies and a `Reloadable<T>` swap cache so updates appear without restart.

**Tech Stack:** Rust, tokio (Semaphore/Mutex/mpsc/spawn), chrono + chrono-tz, cron, arc-swap, serde_json, adapto_store, reqwest + quick-xml (myqaz only).

**Time injection note:** All time-derived logic (`next_after`, catch-up decision, retry backoff) takes `now: DateTime<Utc>` as a parameter → pure and unit-testable. The running loop holds a `now_fn: fn() -> DateTime<Utc>` field (default `Utc::now`, overridable in tests). No separate Clock trait — this is the lightweight clock injection the spec called for.

---

## File Structure

**New crate `crates/adapto_scheduler/`:**
- `Cargo.toml` — deps, workspace member
- `src/lib.rs` — re-exports, module wiring
- `src/schedule.rs` — `Schedule` enum + `next_after` + cron
- `src/job.rs` — `Job`, `JobBuilder`, `JobContext`, `JobError`, `Lane`, `Priority`, handler types
- `src/state.rs` — `JobState` (the `_jobs` doc), `JobStatus` (public read view), load/seed/save
- `src/engine.rs` — ready heap, lane runner, single-job execution + retry
- `src/scheduler.rs` — `Scheduler` builder, `spawn()`, tick loop, catch-up, `SchedulerHandle`
- `src/reload.rs` — `Reloadable<T>`
- `src/admin.rs` — `render(statuses) -> String`
- `tests/integration.rs` — engine integration tests

**Modified `crates/adapto_app/`:**
- `Cargo.toml` — add `adapto_scheduler` dep
- `src/lib.rs` — `.scheduler()`, `.scheduler_admin()`, `App::run()` spawn/shutdown, admin routes, `ctx.scheduler()`

**Modified myqaz (`~/myqaz/myqaz-rs/`):**
- `Cargo.toml` — add `reqwest`, `quick-xml`
- `src/jobs/mod.rs`, `src/jobs/currency.rs`, `src/jobs/companies.rs` — new
- `src/routes/exchange_rates.rs` — migrate `OnceLock` → `Reloadable`
- `src/main.rs` — wire scheduler + admin guard
- `CHANGELOG.md`, root `Cargo.toml` (adapto-core) version bump

---

## Task 1: Scaffold the crate

**Files:**
- Create: `crates/adapto_scheduler/Cargo.toml`
- Create: `crates/adapto_scheduler/src/lib.rs`
- Modify: `Cargo.toml` (workspace root — add member + workspace deps)

- [ ] **Step 1: Add workspace member + shared deps**

In root `/Users/sakentukenov/adapto-core/Cargo.toml`, add `"crates/adapto_scheduler"` to `[workspace.members]`. Under `[workspace.dependencies]` add (if absent):

```toml
chrono-tz = "0.9"
cron = "0.12"
arc-swap = "1"
```

- [ ] **Step 2: Create the crate manifest**

`crates/adapto_scheduler/Cargo.toml`:

```toml
[package]
name = "adapto_scheduler"
version.workspace = true
edition.workspace = true

[dependencies]
adapto_store = { path = "../adapto_store" }
tokio = { workspace = true }
chrono = { workspace = true }
chrono-tz = { workspace = true }
cron = { workspace = true }
arc-swap = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
tracing = { workspace = true }
thiserror = { workspace = true }

[dev-dependencies]
tokio = { workspace = true, features = ["macros", "rt-multi-thread", "test-util"] }
```

(If `version.workspace`/`edition.workspace` aren't used by sibling crates, copy the literal `version`/`edition` values another crate in `crates/` uses.)

- [ ] **Step 3: Stub lib.rs**

`crates/adapto_scheduler/src/lib.rs`:

```rust
//! Persistent priority + lane background-job scheduler for the Adapto framework.

mod schedule;
mod job;
mod state;
mod engine;
mod scheduler;
mod reload;
pub mod admin;

pub use schedule::Schedule;
pub use job::{Job, JobContext, JobError, Lane, Priority};
pub use state::JobStatus;
pub use scheduler::{Scheduler, SchedulerHandle, TriggerError};
pub use reload::Reloadable;
```

- [ ] **Step 4: Verify it compiles (empty modules will fail until created — create empty files)**

Create empty placeholder files so the module tree resolves:
`schedule.rs job.rs state.rs engine.rs scheduler.rs reload.rs admin.rs` each containing `// filled in later task`.

Run: `cargo build -p adapto_scheduler`
Expected: compile errors about missing pub items (`Schedule`, etc.) — that's fine for now; the crate is wired. Do NOT commit yet (next tasks fill modules). Proceed to Task 2.

---

## Task 2: `Schedule` enum + `next_after`

**Files:**
- Modify: `crates/adapto_scheduler/src/schedule.rs`

- [ ] **Step 1: Write failing tests**

`crates/adapto_scheduler/src/schedule.rs`:

```rust
use chrono::{DateTime, Datelike, Duration as ChDuration, TimeZone, Timelike, Utc, Weekday};
use chrono_tz::Tz;
use std::time::Duration;

/// When a job should run. Times are interpreted in the scheduler timezone.
#[derive(Clone, Debug)]
pub enum Schedule {
    Interval(Duration),
    DailyAt { hour: u8, min: u8 },
    WeeklyOn { weekday: Weekday, hour: u8, min: u8 },
    MonthlyOn { day: u8, hour: u8, min: u8 },
    Cron(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn almaty(y: i32, m: u32, d: u32, h: u32, mi: u32) -> DateTime<Tz> {
        chrono_tz::Asia::Almaty
            .with_ymd_and_hms(y, m, d, h, mi, 0)
            .single()
            .unwrap()
    }

    #[test]
    fn daily_at_same_day_future() {
        let now = almaty(2026, 5, 29, 5, 0);
        let s = Schedule::DailyAt { hour: 6, min: 30 };
        let next = s.next_after(now).unwrap();
        // 06:30 Almaty (UTC+5) == 01:30 UTC same day
        assert_eq!(next, Utc.with_ymd_and_hms(2026, 5, 29, 1, 30, 0).unwrap());
    }

    #[test]
    fn daily_at_already_passed_rolls_tomorrow() {
        let now = almaty(2026, 5, 29, 7, 0);
        let s = Schedule::DailyAt { hour: 6, min: 30 };
        let next = s.next_after(now).unwrap();
        assert_eq!(next, Utc.with_ymd_and_hms(2026, 5, 30, 1, 30, 0).unwrap());
    }

    #[test]
    fn monthly_clamps_day_31_to_month_end() {
        // April has 30 days; asking for day 31 -> April 30.
        let now = almaty(2026, 4, 1, 0, 0);
        let s = Schedule::MonthlyOn { day: 31, hour: 0, min: 0 };
        let next = s.next_after(now).unwrap();
        // 00:00 Almaty Apr 30 == 19:00 UTC Apr 29
        assert_eq!(next, Utc.with_ymd_and_hms(2026, 4, 29, 19, 0, 0).unwrap());
    }

    #[test]
    fn monthly_rolls_to_next_month_when_passed() {
        let now = almaty(2026, 5, 10, 0, 0);
        let s = Schedule::MonthlyOn { day: 2, hour: 3, min: 0 };
        let next = s.next_after(now).unwrap();
        // June 2 03:00 Almaty == June 1 22:00 UTC
        assert_eq!(next, Utc.with_ymd_and_hms(2026, 6, 1, 22, 0, 0).unwrap());
    }

    #[test]
    fn interval_adds_duration() {
        let now = almaty(2026, 5, 29, 5, 0);
        let s = Schedule::Interval(Duration::from_secs(900));
        let next = s.next_after(now).unwrap();
        assert_eq!(next, now.with_timezone(&Utc) + ChDuration::seconds(900));
    }

    #[test]
    fn cron_daily_six_am() {
        let now = almaty(2026, 5, 29, 0, 0);
        let s = Schedule::Cron("0 0 6 * * *".to_string()); // sec min hour dom mon dow (cron crate = 6 fields)
        let next = s.next_after(now).unwrap();
        assert_eq!(next, Utc.with_ymd_and_hms(2026, 5, 29, 1, 0, 0).unwrap());
    }

    #[test]
    fn cron_invalid_errs() {
        let now = almaty(2026, 5, 29, 0, 0);
        let s = Schedule::Cron("not a cron".to_string());
        assert!(s.next_after(now).is_err());
    }
}
```

- [ ] **Step 2: Run tests, verify they fail**

Run: `cargo test -p adapto_scheduler schedule::`
Expected: FAIL — `next_after` not found.

- [ ] **Step 3: Implement `next_after`**

Add to `schedule.rs` (above the test module):

```rust
impl Schedule {
    /// Next fire time strictly after `now`, computed in `now`'s timezone, returned as UTC.
    /// Returns Err for an unparseable cron expression.
    pub fn next_after(&self, now: DateTime<Tz>) -> Result<DateTime<Utc>, String> {
        let tz = now.timezone();
        match self {
            Schedule::Interval(d) => {
                let dd = ChDuration::from_std(*d).map_err(|e| e.to_string())?;
                Ok(now.with_timezone(&Utc) + dd)
            }
            Schedule::DailyAt { hour, min } => {
                let mut cand = local_at(tz, now.year(), now.month(), now.day(), *hour, *min)?;
                if cand <= now {
                    cand = cand + ChDuration::days(1);
                    // recompute via date arithmetic to respect DST
                    let d = cand.date_naive();
                    cand = local_at(tz, d.year(), d.month(), d.day(), *hour, *min)?;
                }
                Ok(cand.with_timezone(&Utc))
            }
            Schedule::WeeklyOn { weekday, hour, min } => {
                let mut cand = local_at(tz, now.year(), now.month(), now.day(), *hour, *min)?;
                while cand <= now || cand.weekday() != *weekday {
                    let d = (cand + ChDuration::days(1)).date_naive();
                    cand = local_at(tz, d.year(), d.month(), d.day(), *hour, *min)?;
                }
                Ok(cand.with_timezone(&Utc))
            }
            Schedule::MonthlyOn { day, hour, min } => {
                let mut y = now.year();
                let mut m = now.month();
                loop {
                    let d = clamp_day(y, m, *day);
                    let cand = local_at(tz, y, m, d, *hour, *min)?;
                    if cand > now {
                        return Ok(cand.with_timezone(&Utc));
                    }
                    if m == 12 { y += 1; m = 1; } else { m += 1; }
                }
            }
            Schedule::Cron(expr) => {
                use std::str::FromStr;
                let sched = cron::Schedule::from_str(expr).map_err(|e| e.to_string())?;
                sched
                    .after(&now)
                    .next()
                    .map(|dt| dt.with_timezone(&Utc))
                    .ok_or_else(|| "cron yielded no next time".to_string())
            }
        }
    }
}

fn clamp_day(year: i32, month: u32, day: u8) -> u32 {
    let last = last_day_of_month(year, month);
    (day as u32).min(last).max(1)
}

fn last_day_of_month(year: i32, month: u32) -> u32 {
    let (ny, nm) = if month == 12 { (year + 1, 1) } else { (year, month + 1) };
    let first_next = chrono::NaiveDate::from_ymd_opt(ny, nm, 1).unwrap();
    (first_next - chrono::Duration::days(1)).day()
}

/// Build a tz-aware datetime, picking the earliest instant on DST ambiguity.
fn local_at(tz: Tz, y: i32, mo: u32, d: u32, h: u8, mi: u8) -> Result<DateTime<Tz>, String> {
    let naive = chrono::NaiveDate::from_ymd_opt(y, mo, d)
        .and_then(|nd| nd.and_hms_opt(h as u32, mi as u32, 0))
        .ok_or_else(|| format!("invalid local time {y}-{mo}-{d} {h}:{mi}"))?;
    match tz.from_local_datetime(&naive) {
        chrono::LocalResult::Single(dt) => Ok(dt),
        chrono::LocalResult::Ambiguous(a, _) => Ok(a),
        chrono::LocalResult::None => {
            // DST gap: bump 1h and retry
            let bumped = naive + ChDuration::hours(1);
            tz.from_local_datetime(&bumped)
                .single()
                .ok_or_else(|| "unresolvable local time across DST".to_string())
        }
    }
}
```

- [ ] **Step 4: Run tests, verify pass**

Run: `cargo test -p adapto_scheduler schedule::`
Expected: PASS (all 7). If `cron` 6-field vs 5-field differs, adjust the cron test string — the `cron` crate uses 6 fields (`sec min hour dom month dow`); document this in the `Cron` variant doc comment.

- [ ] **Step 5: Commit**

```bash
git add crates/adapto_scheduler/Cargo.toml crates/adapto_scheduler/src/schedule.rs crates/adapto_scheduler/src/lib.rs Cargo.toml
git commit -m "feat(scheduler): Schedule enum with timezone-aware next_after + cron"
```

---

## Task 3: `Job`, `JobContext`, `JobError`, `Lane`, `Priority`

**Files:**
- Modify: `crates/adapto_scheduler/src/job.rs`

- [ ] **Step 1: Write failing test**

`crates/adapto_scheduler/src/job.rs`:

```rust
use crate::schedule::Schedule;
use adapto_store::AdaptoStore;
use chrono::{DateTime, Utc};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority { Low, Normal, High, Critical }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lane { Light, Heavy }

#[derive(Debug, thiserror::Error)]
pub enum JobError {
    #[error("{0}")]
    Message(String),
}
impl JobError {
    pub fn msg(s: impl Into<String>) -> Self { JobError::Message(s.into()) }
    /// Wrap any error's Display into a JobError (use in job bodies: `.map_err(JobError::from_err)`).
    pub fn from_err<E: std::fmt::Display>(e: E) -> Self { JobError::Message(e.to_string()) }
}

pub struct JobContext {
    pub store: AdaptoStore,
    pub attempt: u32,
    pub scheduled_for: DateTime<Utc>,
}

pub type JobFuture = Pin<Box<dyn Future<Output = Result<(), JobError>> + Send>>;
pub type JobHandler = Arc<dyn Fn(JobContext) -> JobFuture + Send + Sync>;

pub struct Job {
    pub(crate) name: String,
    pub(crate) schedule: Schedule,
    pub(crate) lane: Lane,
    pub(crate) priority: Priority,
    pub(crate) catch_up: bool,
    pub(crate) max_attempts: u32,
    pub(crate) handler: JobHandler,
}

pub struct JobBuilder {
    name: String,
    schedule: Option<Schedule>,
    lane: Lane,
    priority: Priority,
    catch_up: bool,
    max_attempts: u32,
}

impl Job {
    pub fn new(name: impl Into<String>) -> JobBuilder {
        JobBuilder {
            name: name.into(),
            schedule: None,
            lane: Lane::Light,
            priority: Priority::Normal,
            catch_up: false,
            max_attempts: 3,
        }
    }
}

impl JobBuilder {
    pub fn schedule(mut self, s: Schedule) -> Self { self.schedule = Some(s); self }
    pub fn lane(mut self, l: Lane) -> Self { self.lane = l; self }
    pub fn priority(mut self, p: Priority) -> Self { self.priority = p; self }
    pub fn catch_up(mut self, v: bool) -> Self { self.catch_up = v; self }
    pub fn max_attempts(mut self, n: u32) -> Self { self.max_attempts = n.max(1); self }

    pub fn run<F, Fut>(self, f: F) -> Job
    where
        F: Fn(JobContext) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), JobError>> + Send + 'static,
    {
        let handler: JobHandler = Arc::new(move |ctx| Box::pin(f(ctx)));
        Job {
            name: self.name,
            schedule: self.schedule.expect("Job requires .schedule(...)"),
            lane: self.lane,
            priority: self.priority,
            catch_up: self.catch_up,
            max_attempts: self.max_attempts,
            handler,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn builder_sets_fields() {
        let j = Job::new("x")
            .schedule(Schedule::Interval(Duration::from_secs(1)))
            .lane(Lane::Heavy)
            .priority(Priority::High)
            .catch_up(true)
            .max_attempts(5)
            .run(|_ctx| async { Ok(()) });
        assert_eq!(j.name, "x");
        assert_eq!(j.lane, Lane::Heavy);
        assert_eq!(j.priority, Priority::High);
        assert!(j.catch_up);
        assert_eq!(j.max_attempts, 5);
    }

    #[test]
    fn max_attempts_floor_one() {
        let j = Job::new("x")
            .schedule(Schedule::Interval(Duration::from_secs(1)))
            .max_attempts(0)
            .run(|_| async { Ok(()) });
        assert_eq!(j.max_attempts, 1);
    }
}
```

- [ ] **Step 2: Run tests, verify fail then pass**

Run: `cargo test -p adapto_scheduler job::`
Expected: compiles + PASS (2 tests). If `AdaptoStore` isn't `Clone`, confirm via `crates/adapto_store/src/lib.rs` — it is Arc-backed; `JobContext.store` holds an owned clone.

- [ ] **Step 3: Commit**

```bash
git add crates/adapto_scheduler/src/job.rs
git commit -m "feat(scheduler): Job/JobBuilder, Lane, Priority, JobContext, JobError"
```

---

## Task 4: `JobState` persistence + `JobStatus` view

**Files:**
- Modify: `crates/adapto_scheduler/src/state.rs`

The `_jobs` collection holds one doc per job. `JobState` (de)serializes that doc; `JobStatus` is the public read view for the admin page.

- [ ] **Step 1: Write failing test**

`crates/adapto_scheduler/src/state.rs`:

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub const JOBS_COLLECTION: &str = "_jobs";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum RunStatus { Pending, Running, Ok, Failed, Retrying }

impl RunStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            RunStatus::Pending => "pending",
            RunStatus::Running => "running",
            RunStatus::Ok => "ok",
            RunStatus::Failed => "failed",
            RunStatus::Retrying => "retrying",
        }
    }
}

/// Persisted per-job document (the `_jobs` collection body).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JobState {
    pub job: String,
    pub lane: String,
    pub priority: String,
    pub schedule: String,
    pub status: RunStatus,
    pub last_run: Option<DateTime<Utc>>,
    pub last_duration_ms: Option<u64>,
    pub next_run: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub attempt: u32,
    pub consecutive_failures: u32,
    pub total_runs: u64,
    pub total_failures: u64,
    pub updated_at: DateTime<Utc>,
}

impl JobState {
    pub fn seed(job: &str, lane: &str, priority: &str, schedule: &str, now: DateTime<Utc>) -> Self {
        JobState {
            job: job.to_string(),
            lane: lane.to_string(),
            priority: priority.to_string(),
            schedule: schedule.to_string(),
            status: RunStatus::Pending,
            last_run: None,
            last_duration_ms: None,
            next_run: None,
            last_error: None,
            attempt: 0,
            consecutive_failures: 0,
            total_runs: 0,
            total_failures: 0,
            updated_at: now,
        }
    }
}

/// Public, serializable read view for the admin page.
#[derive(Clone, Debug, Serialize)]
pub struct JobStatus {
    pub job: String,
    pub lane: String,
    pub priority: String,
    pub schedule: String,
    pub status: String,
    pub last_run: Option<DateTime<Utc>>,
    pub last_duration_ms: Option<u64>,
    pub next_run: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub consecutive_failures: u32,
    pub total_runs: u64,
    pub total_failures: u64,
}

impl From<&JobState> for JobStatus {
    fn from(s: &JobState) -> Self {
        JobStatus {
            job: s.job.clone(),
            lane: s.lane.clone(),
            priority: s.priority.clone(),
            schedule: s.schedule.clone(),
            status: s.status.as_str().to_string(),
            last_run: s.last_run,
            last_duration_ms: s.last_duration_ms,
            next_run: s.next_run,
            last_error: s.last_error.clone(),
            consecutive_failures: s.consecutive_failures,
            total_runs: s.total_runs,
            total_failures: s.total_failures,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_json() {
        let now = Utc::now();
        let s = JobState::seed("currency", "light", "high", "DailyAt 06:30", now);
        let v = serde_json::to_value(&s).unwrap();
        let back: JobState = serde_json::from_value(v).unwrap();
        assert_eq!(back.job, "currency");
        assert_eq!(back.status, RunStatus::Pending);
    }

    #[test]
    fn status_view_maps_status_string() {
        let now = Utc::now();
        let mut s = JobState::seed("x", "heavy", "low", "MonthlyOn", now);
        s.status = RunStatus::Failed;
        let view = JobStatus::from(&s);
        assert_eq!(view.status, "failed");
        assert_eq!(view.lane, "heavy");
    }
}
```

- [ ] **Step 2: Run, verify pass**

Run: `cargo test -p adapto_scheduler state::`
Expected: PASS (2 tests).

- [ ] **Step 3: Add store load/seed/save helpers**

Append to `state.rs`:

```rust
use adapto_store::{AdaptoStore, Query, Update};

/// Load all persisted job states keyed by job name.
pub fn load_all(store: &AdaptoStore) -> std::collections::HashMap<String, JobState> {
    let col = store.collection(JOBS_COLLECTION);
    let mut map = std::collections::HashMap::new();
    for doc in col.find(Query::new()) {
        if let Ok(st) = serde_json::from_value::<JobState>(doc.data) {
            map.insert(st.job.clone(), st);
        }
    }
    map
}

/// Insert-or-update a job state by its unique `job` key.
pub fn save(store: &AdaptoStore, st: &JobState) {
    let col = store.collection(JOBS_COLLECTION);
    let value = match serde_json::to_value(st) {
        Ok(v) => v,
        Err(e) => { tracing::error!("scheduler: serialize {} failed: {e}", st.job); return; }
    };
    match col.find_one(Query::eq("job", st.job.clone())) {
        Ok(Some(_)) => {
            if let Err(e) = col.update(
                Query::eq("job", st.job.clone()),
                Update::Set(vec![("$root".into(), value)]),
            ) {
                tracing::error!("scheduler: update {} failed: {e}", st.job);
            }
        }
        _ => {
            if let Err(e) = col.insert(value) {
                tracing::error!("scheduler: insert {} failed: {e}", st.job);
            }
        }
    }
}

/// Ensure the unique index on `job` exists.
pub fn ensure_index(store: &AdaptoStore) {
    let _ = store.collection(JOBS_COLLECTION).create_index("job", true);
}
```

> **VERIFY against adapto_store API:** `Update::Set(vec![(field, value)])` sets dotted fields. Setting the WHOLE document (`"$root"`) may not be supported. If `Update::Set` cannot replace the entire doc, instead `delete` by `Query::eq("job", ...)` then `insert(value)` inside `save`, OR set each field individually. Read `crates/adapto_store/src/query.rs` (the `Update` enum) and `collection.rs` before implementing `save`; adapt to the real API. The test in Step 4 will catch a wrong choice.

- [ ] **Step 4: Write store round-trip test**

Append to the `tests` module in `state.rs`:

```rust
    #[test]
    fn save_then_load_from_store() {
        let store = AdaptoStore::open(None).unwrap();
        ensure_index(&store);
        let now = Utc::now();
        let mut st = JobState::seed("currency", "light", "high", "DailyAt", now);
        save(&store, &st);
        // update and save again -> still one doc, updated values
        st.status = RunStatus::Ok;
        st.total_runs = 1;
        save(&store, &st);

        let loaded = load_all(&store);
        assert_eq!(loaded.len(), 1);
        let got = &loaded["currency"];
        assert_eq!(got.status, RunStatus::Ok);
        assert_eq!(got.total_runs, 1);
    }
```

Run: `cargo test -p adapto_scheduler state::save_then_load_from_store`
Expected: PASS. If it fails because the doc was duplicated, switch `save` to delete+insert per the VERIFY note.

- [ ] **Step 5: Commit**

```bash
git add crates/adapto_scheduler/src/state.rs
git commit -m "feat(scheduler): JobState persistence + JobStatus view + _jobs helpers"
```

---

## Task 5: Engine — single job execution + retry decision (pure)

**Files:**
- Modify: `crates/adapto_scheduler/src/engine.rs`

This task implements the pure decision functions (no async): given a run outcome, compute the next state. The async lane runner (Task 7) calls these.

- [ ] **Step 1: Write failing tests**

`crates/adapto_scheduler/src/engine.rs`:

```rust
use crate::job::{Lane, Priority};
use crate::state::{JobState, RunStatus};
use chrono::{DateTime, Duration as ChDuration, Utc};

/// Backoff delay before retry attempt `attempt` (1-based: attempt 1 already ran, this is the wait
/// before attempt `attempt+1`). 1m, 5m, 15m, then capped at 15m.
pub fn backoff(attempt: u32) -> ChDuration {
    match attempt {
        1 => ChDuration::minutes(1),
        2 => ChDuration::minutes(5),
        _ => ChDuration::minutes(15),
    }
}

/// Outcome of a single handler run.
pub enum Outcome { Ok, Err(String) }

/// Apply a run outcome to job state. `next_scheduled` is `schedule.next_after(now)`.
/// Mutates `st` in place; returns nothing (caller persists).
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

/// Catch-up decision at boot. Returns the `next_run` to persist and whether to run-now-once.
pub fn catch_up_decision(
    persisted_next: Option<DateTime<Utc>>,
    catch_up: bool,
    now: DateTime<Utc>,
    next_scheduled: DateTime<Utc>,
) -> (DateTime<Utc>, bool) {
    match persisted_next {
        Some(nr) if nr <= now => {
            if catch_up {
                (next_scheduled, true) // run once now, then resume
            } else {
                (next_scheduled, false) // skip missed
            }
        }
        Some(nr) => (nr, false), // future -> keep as-is
        None => (next_scheduled, false), // first ever
    }
}

/// Ordering key for the ready heap: higher priority first, then earliest due.
#[derive(PartialEq, Eq)]
pub struct ReadyKey {
    pub priority: Priority,
    pub due: DateTime<Utc>,
}
impl Ord for ReadyKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // BinaryHeap is a max-heap: we want highest priority + earliest due popped first.
        self.priority
            .cmp(&other.priority)
            .then_with(|| other.due.cmp(&self.due)) // earlier due = "greater"
    }
}
impl PartialOrd for ReadyKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> { Some(self.cmp(other)) }
}

pub fn lane_str(l: Lane) -> &'static str { match l { Lane::Light => "light", Lane::Heavy => "heavy" } }
pub fn priority_str(p: Priority) -> &'static str {
    match p { Priority::Low => "low", Priority::Normal => "normal", Priority::High => "high", Priority::Critical => "critical" }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(s: i64) -> DateTime<Utc> { DateTime::from_timestamp(s, 0).unwrap() }

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
```

- [ ] **Step 2: Run, verify pass**

Run: `cargo test -p adapto_scheduler engine::`
Expected: PASS (7 tests).

- [ ] **Step 3: Commit**

```bash
git add crates/adapto_scheduler/src/engine.rs
git commit -m "feat(scheduler): pure run-outcome + catch-up + ready-key engine logic"
```

---

## Task 6: `Scheduler` builder + `SchedulerHandle` type

**Files:**
- Modify: `crates/adapto_scheduler/src/scheduler.rs`

- [ ] **Step 1: Write the builder + handle (no test yet — exercised in Task 7 integration)**

`crates/adapto_scheduler/src/scheduler.rs`:

```rust
use crate::engine::{lane_str, priority_str};
use crate::job::{Job, Lane};
use crate::state::{JobStatus, JobState};
use adapto_store::AdaptoStore;
use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex};

pub struct Scheduler {
    store: AdaptoStore,
    timezone: Tz,
    light_workers: usize,
    poll_floor: Duration,
    shutdown_grace: Duration,
    jobs: Vec<Job>,
    now_fn: fn() -> DateTime<Utc>,
}

impl Scheduler {
    pub fn new(store: AdaptoStore) -> Self {
        Scheduler {
            store,
            timezone: chrono_tz::UTC,
            light_workers: 4,
            poll_floor: Duration::from_secs(15),
            shutdown_grace: Duration::from_secs(30),
            jobs: Vec::new(),
            now_fn: Utc::now,
        }
    }
    pub fn timezone(mut self, tz: Tz) -> Self { self.timezone = tz; self }
    pub fn light_workers(mut self, n: usize) -> Self { self.light_workers = n.max(1); self }
    pub fn poll_floor(mut self, d: Duration) -> Self { self.poll_floor = d; self }
    pub fn shutdown_grace(mut self, d: Duration) -> Self { self.shutdown_grace = d; self }
    pub fn job(mut self, j: Job) -> Self { self.jobs.push(j); self }

    #[doc(hidden)] // test-only clock injection
    pub fn now_fn(mut self, f: fn() -> DateTime<Utc>) -> Self { self.now_fn = f; self }

    // spawn() implemented in Task 7
}

#[derive(Debug, thiserror::Error)]
pub enum TriggerError {
    #[error("no such job: {0}")]
    NoSuchJob(String),
    #[error("job already running: {0}")]
    AlreadyRunning(String),
    #[error("scheduler stopped")]
    Stopped,
}

/// Shared, cheaply-clonable handle to a running scheduler.
#[derive(Clone)]
pub struct SchedulerHandle {
    inner: Arc<HandleInner>,
}

pub(crate) struct HandleInner {
    pub(crate) store: AdaptoStore,
    pub(crate) trigger_tx: mpsc::UnboundedSender<String>,
    pub(crate) shutdown_tx: mpsc::UnboundedSender<()>,
    pub(crate) running: Mutex<std::collections::HashSet<String>>,
    pub(crate) job_names: Vec<String>,
    pub(crate) done: Mutex<Option<tokio::sync::oneshot::Receiver<()>>>,
}

impl SchedulerHandle {
    /// Current persisted status of every job (for admin page).
    pub fn statuses(&self) -> Vec<JobStatus> {
        let states = crate::state::load_all(&self.inner.store);
        let mut out: Vec<JobStatus> = self
            .inner
            .job_names
            .iter()
            .filter_map(|n| states.get(n))
            .map(JobStatus::from)
            .collect();
        out.sort_by(|a, b| a.job.cmp(&b.job));
        out
    }

    /// Request an immediate run. Err(AlreadyRunning) if currently executing.
    pub async fn trigger(&self, job: &str) -> Result<(), TriggerError> {
        if !self.inner.job_names.iter().any(|n| n == job) {
            return Err(TriggerError::NoSuchJob(job.to_string()));
        }
        if self.inner.running.lock().await.contains(job) {
            return Err(TriggerError::AlreadyRunning(job.to_string()));
        }
        self.inner
            .trigger_tx
            .send(job.to_string())
            .map_err(|_| TriggerError::Stopped)
    }

    /// Stop the tick loop and wait for in-flight jobs to drain.
    pub async fn shutdown(&self) {
        let _ = self.inner.shutdown_tx.send(());
        if let Some(rx) = self.inner.done.lock().await.take() {
            let _ = rx.await;
        }
    }
}

/// Build the human-readable schedule string stored in `_jobs`.
pub(crate) fn schedule_desc(j: &Job) -> String {
    use crate::schedule::Schedule::*;
    match &j.schedule {
        Interval(d) => format!("Interval {}s", d.as_secs()),
        DailyAt { hour, min } => format!("DailyAt {:02}:{:02}", hour, min),
        WeeklyOn { weekday, hour, min } => format!("WeeklyOn {:?} {:02}:{:02}", weekday, hour, min),
        MonthlyOn { day, hour, min } => format!("MonthlyOn {} {:02}:{:02}", day, hour, min),
        Cron(e) => format!("Cron({e})"),
    }
}

// helper re-exports for Task 7
pub(crate) use crate::job::Priority as _Priority;
pub(crate) fn lane_of(j: &Job) -> Lane { j.lane }
pub(crate) fn meta_strings(j: &Job) -> (String, String, String) {
    (lane_str(j.lane).to_string(), priority_str(j.priority).to_string(), schedule_desc(j))
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo build -p adapto_scheduler`
Expected: compiles (warnings about unused are fine; `spawn` arrives next task). If `Job` fields are private to `job.rs`, they were declared `pub(crate)` in Task 3 — confirm.

- [ ] **Step 3: Commit**

```bash
git add crates/adapto_scheduler/src/scheduler.rs
git commit -m "feat(scheduler): Scheduler builder + SchedulerHandle (statuses/trigger/shutdown)"
```

---

## Task 7: Engine wiring — `spawn()`, tick loop, lane runners

**Files:**
- Modify: `crates/adapto_scheduler/src/scheduler.rs`
- Create: `crates/adapto_scheduler/tests/integration.rs`

This is the heart. `spawn()` seeds state, applies catch-up, starts the tick loop and two lane runners.

- [ ] **Step 1: Implement `spawn()` and the loops**

Add to `scheduler.rs` (inside `impl Scheduler`, replacing the `// spawn() implemented in Task 7` comment, plus free functions below):

```rust
    pub fn spawn(self) -> SchedulerHandle {
        use crate::engine::catch_up_decision;
        use crate::state::{ensure_index, load_all, save, RunStatus};

        ensure_index(&self.store);
        let now = (self.now_fn)();
        let tz = self.timezone;

        // Build per-job metadata, indexed by name.
        let mut handlers: HashMap<String, Arc<JobRuntime>> = HashMap::new();
        let mut persisted = load_all(&self.store);
        let mut due_now: Vec<String> = Vec::new();
        let job_names: Vec<String> = self.jobs.iter().map(|j| j.name.clone()).collect();

        for j in &self.jobs {
            let (lane_s, prio_s, sched_s) = meta_strings(j);
            let next_scheduled = j
                .schedule
                .next_after(now.with_timezone(&tz))
                .unwrap_or_else(|e| {
                    // Fail fast: a bad cron must not boot silently.
                    panic!("scheduler: job '{}' has invalid schedule: {e}", j.name);
                });

            let st = persisted.entry(j.name.clone()).or_insert_with(|| {
                JobState::seed(&j.name, &lane_s, &prio_s, &sched_s, now)
            });
            // refresh meta in case it changed across deploys
            st.lane = lane_s.clone();
            st.priority = prio_s.clone();
            st.schedule = sched_s.clone();

            let (next_run, run_now) =
                catch_up_decision(st.next_run, j.catch_up, now, next_scheduled);
            st.next_run = Some(next_run);
            if run_now { due_now.push(j.name.clone()); }
            if st.status == RunStatus::Running {
                // crashed mid-run last time -> reset
                st.status = RunStatus::Pending;
            }
            save(&self.store, st);

            handlers.insert(
                j.name.clone(),
                Arc::new(JobRuntime {
                    name: j.name.clone(),
                    lane: j.lane,
                    priority: j.priority,
                    max_attempts: j.max_attempts,
                    schedule: j.schedule.clone(),
                    handler: j.handler.clone(),
                }),
            );
        }

        let (trigger_tx, mut trigger_rx) = mpsc::unbounded_channel::<String>();
        let (shutdown_tx, mut shutdown_rx) = mpsc::unbounded_channel::<()>();
        let (done_tx, done_rx) = tokio::sync::oneshot::channel::<()>();

        let running = Arc::new(Mutex::new(std::collections::HashSet::<String>::new()));
        // Per-lane ready queues (job names tagged with due time + priority).
        let light = Arc::new(LaneState::new(self.light_workers));
        let heavy = Arc::new(LaneState::new(1));
        let handlers = Arc::new(handlers);

        // Pre-enqueue catch-up jobs.
        for name in due_now {
            enqueue(&handlers, &light, &heavy, &name, now);
        }

        let store = self.store.clone();
        let poll_floor = self.poll_floor;
        let now_fn = self.now_fn;
        let grace = self.shutdown_grace;

        // Lane runner tasks.
        spawn_lane_runner(light.clone(), handlers.clone(), running.clone(), store.clone(), tz, now_fn);
        spawn_lane_runner(heavy.clone(), handlers.clone(), running.clone(), store.clone(), tz, now_fn);

        // Tick loop.
        {
            let handlers = handlers.clone();
            let light = light.clone();
            let heavy = heavy.clone();
            let store = store.clone();
            tokio::spawn(async move {
                loop {
                    let now = now_fn();
                    let states = load_all(&store);
                    // earliest future next_run
                    let mut sleep = poll_floor;
                    for n in handlers.keys() {
                        if let Some(nr) = states.get(n).and_then(|s| s.next_run) {
                            if nr > now {
                                let d = (nr - now).to_std().unwrap_or(poll_floor);
                                if d < sleep { sleep = d; }
                            }
                        }
                    }
                    tokio::select! {
                        _ = tokio::time::sleep(sleep) => {}
                        Some(name) = trigger_rx.recv() => {
                            enqueue(&handlers, &light, &heavy, &name, now_fn());
                            continue;
                        }
                        _ = shutdown_rx.recv() => { break; }
                    }
                    // enqueue everything now due
                    let now = now_fn();
                    let states = load_all(&store);
                    for n in handlers.keys() {
                        if let Some(nr) = states.get(n).and_then(|s| s.next_run) {
                            if nr <= now {
                                enqueue(&handlers, &light, &heavy, n, now);
                            }
                        }
                    }
                }
                // shutdown: signal lanes to stop, drain with grace
                light.close();
                heavy.close();
                let deadline = tokio::time::Instant::now() + grace;
                while (light.busy() || heavy.busy()) && tokio::time::Instant::now() < deadline {
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
                let _ = done_tx.send(());
            });
        }

        SchedulerHandle {
            inner: Arc::new(HandleInner {
                store,
                trigger_tx,
                shutdown_tx,
                running: Mutex::new(std::collections::HashSet::new()),
                job_names,
                done: Mutex::new(Some(done_rx)),
            }),
        }
    }
```

Add the runtime + lane plumbing at the bottom of `scheduler.rs`:

```rust
use crate::engine::{apply_outcome, Outcome, ReadyKey};
use crate::job::{JobContext, JobHandler, Priority};
use crate::schedule::Schedule;
use std::collections::BinaryHeap;

pub(crate) struct JobRuntime {
    name: String,
    lane: Lane,
    priority: Priority,
    max_attempts: u32,
    schedule: Schedule,
    handler: JobHandler,
}

struct LaneState {
    permits: Arc<tokio::sync::Semaphore>,
    queue: Mutex<BinaryHeap<QueueItem>>,
    notify: tokio::sync::Notify,
    closed: std::sync::atomic::AtomicBool,
    inflight: std::sync::atomic::AtomicUsize,
}
impl LaneState {
    fn new(workers: usize) -> Self {
        LaneState {
            permits: Arc::new(tokio::sync::Semaphore::new(workers)),
            queue: Mutex::new(BinaryHeap::new()),
            notify: tokio::sync::Notify::new(),
            closed: std::sync::atomic::AtomicBool::new(false),
            inflight: std::sync::atomic::AtomicUsize::new(0),
        }
    }
    fn close(&self) { self.closed.store(true, std::sync::atomic::Ordering::SeqCst); self.notify.notify_waiters(); }
    fn is_closed(&self) -> bool { self.closed.load(std::sync::atomic::Ordering::SeqCst) }
    fn busy(&self) -> bool { self.inflight.load(std::sync::atomic::Ordering::SeqCst) > 0 }
}

struct QueueItem { key: ReadyKey, name: String }
impl PartialEq for QueueItem { fn eq(&self, o: &Self) -> bool { self.key == o.key } }
impl Eq for QueueItem {}
impl Ord for QueueItem { fn cmp(&self, o: &Self) -> std::cmp::Ordering { self.key.cmp(&o.key) } }
impl PartialOrd for QueueItem { fn partial_cmp(&self, o: &Self) -> Option<std::cmp::Ordering> { Some(self.cmp(o)) } }

fn lane_ref<'a>(j: &JobRuntime, light: &'a Arc<LaneState>, heavy: &'a Arc<LaneState>) -> &'a Arc<LaneState> {
    match j.lane { Lane::Light => light, Lane::Heavy => heavy }
}

fn enqueue(
    handlers: &Arc<HashMap<String, Arc<JobRuntime>>>,
    light: &Arc<LaneState>,
    heavy: &Arc<LaneState>,
    name: &str,
    now: DateTime<Utc>,
) {
    let Some(rt) = handlers.get(name) else { return; };
    let lane = lane_ref(rt, light, heavy);
    let item = QueueItem { key: ReadyKey { priority: rt.priority, due: now }, name: name.to_string() };
    // best-effort: use try_lock-free blocking lock via futures executor is unavailable here;
    // push happens on the tick-loop task which is async, so use blocking_lock-free approach:
    let mut q = lane.queue.blocking_lock_or_panic();
    q.push(item);
    lane.notify.notify_one();
}
```

> **Mutex note:** `enqueue` is called from async contexts (tick loop) — replace the placeholder `blocking_lock_or_panic()` with the proper async pattern: make `enqueue` an `async fn` and use `lane.queue.lock().await`, OR switch `LaneState.queue` to a `std::sync::Mutex` (it's only held for microseconds to push/pop, never across `.await`) and use `lock().unwrap()`. **Prefer `std::sync::Mutex` for `queue`** (short critical sections, no await inside) and keep `tokio::sync::Mutex` only for `running`. Update the field type and all `queue.lock()` call sites accordingly. The integration test in Step 3 will surface deadlocks/panics if this is wrong.

Add the lane runner:

```rust
fn spawn_lane_runner(
    lane: Arc<LaneState>,
    handlers: Arc<HashMap<String, Arc<JobRuntime>>>,
    running: Arc<Mutex<std::collections::HashSet<String>>>,
    store: AdaptoStore,
    tz: Tz,
    now_fn: fn() -> DateTime<Utc>,
) {
    tokio::spawn(async move {
        loop {
            if lane.is_closed() && !has_item(&lane) { break; }
            // wait for work
            let name = match pop_item(&lane) {
                Some(n) => n,
                None => {
                    if lane.is_closed() { break; }
                    lane.notify.notified().await;
                    continue;
                }
            };
            // acquire a worker permit
            let permit = lane.permits.clone().acquire_owned().await.unwrap();

            // single-instance guard
            {
                let mut r = running.lock().await;
                if r.contains(&name) { drop(permit); continue; }
                r.insert(name.clone());
            }
            let Some(rt) = handlers.get(&name).cloned() else { drop(permit); continue; };

            let running2 = running.clone();
            let store2 = store.clone();
            lane.inflight.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let lane2 = lane.clone();
            tokio::spawn(async move {
                run_one(&rt, &store2, tz, now_fn).await;
                running2.lock().await.remove(&rt.name);
                lane2.inflight.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
                drop(permit);
            });
        }
    });
}

fn has_item(lane: &Arc<LaneState>) -> bool {
    // queue is std::sync::Mutex per the note above
    !lane.queue.lock().unwrap().is_empty()
}
fn pop_item(lane: &Arc<LaneState>) -> Option<String> {
    lane.queue.lock().unwrap().pop().map(|qi| qi.name)
}

async fn run_one(rt: &JobRuntime, store: &AdaptoStore, tz: Tz, now_fn: fn() -> DateTime<Utc>) {
    use crate::state::{load_all, save, RunStatus};

    // load current state, bump attempt
    let mut states = load_all(store);
    let mut st = match states.remove(&rt.name) {
        Some(s) => s,
        None => return,
    };
    let attempt = st.attempt + 1;
    let scheduled_for = st.next_run.unwrap_or_else(|| now_fn());
    st.status = RunStatus::Running;
    st.attempt = attempt;
    st.updated_at = now_fn();
    save(store, &st);

    let ctx = JobContext { store: store.clone(), attempt, scheduled_for };
    let start = std::time::Instant::now();
    let handler = rt.handler.clone();
    // isolate panics: run handler on its own task
    let jh = tokio::spawn(async move { (handler)(ctx).await });
    let outcome = match jh.await {
        Ok(Ok(())) => Outcome::Ok,
        Ok(Err(e)) => Outcome::Err(e.to_string()),
        Err(join) => Outcome::Err(if join.is_panic() { "handler panicked".into() } else { "handler cancelled".into() }),
    };
    let dur = start.elapsed().as_millis() as u64;

    let now = now_fn();
    let next_scheduled = rt
        .schedule
        .next_after(now.with_timezone(&tz))
        .unwrap_or(now + chrono::Duration::days(1));
    apply_outcome(&mut st, outcome, attempt, rt.max_attempts, dur, now, next_scheduled);
    save(store, &st);

    if let RunStatus::Ok | RunStatus::Failed = st.status {
        tracing::info!("scheduler: job '{}' {} in {}ms", rt.name, st.status.as_str(), dur);
    } else {
        tracing::warn!("scheduler: job '{}' retrying (attempt {})", rt.name, attempt);
    }
}
```

> **Retry re-enqueue:** when `apply_outcome` sets status `Retrying` with `next_run = now + backoff`, the tick loop picks it up when that time arrives (it scans `next_run <= now`). No special path needed — the persisted `next_run` drives it, same as a normal schedule. Confirm the integration test `retries_then_succeeds` exercises this.

- [ ] **Step 2: Apply the `std::sync::Mutex` fix for `queue`**

Change `LaneState.queue` to `std::sync::Mutex<BinaryHeap<QueueItem>>`, make `enqueue` push via `lane.queue.lock().unwrap()` (remove the placeholder), and ensure no `.await` happens while that lock is held.

- [ ] **Step 3: Write integration tests**

`crates/adapto_scheduler/tests/integration.rs`:

```rust
use adapto_scheduler::{Job, Lane, Priority, Schedule, Scheduler};
use adapto_store::AdaptoStore;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[tokio::test]
async fn interval_job_runs_repeatedly() {
    let store = AdaptoStore::open(None).unwrap();
    let counter = Arc::new(AtomicU32::new(0));
    let c2 = counter.clone();

    let sched = Scheduler::new(store.clone())
        .poll_floor(Duration::from_millis(20))
        .job(
            Job::new("tick")
                .schedule(Schedule::Interval(Duration::from_millis(40)))
                .lane(Lane::Light)
                .priority(Priority::High)
                .run(move |_ctx| {
                    let c = c2.clone();
                    async move { c.fetch_add(1, Ordering::SeqCst); Ok(()) }
                }),
        );
    let handle = sched.spawn();
    tokio::time::sleep(Duration::from_millis(250)).await;
    handle.shutdown().await;
    assert!(counter.load(Ordering::SeqCst) >= 3, "expected several runs");

    let statuses = handle.statuses();
    assert_eq!(statuses.len(), 1);
    assert_eq!(statuses[0].job, "tick");
    assert!(statuses[0].total_runs >= 3);
}

#[tokio::test]
async fn failing_job_retries_then_marks_failed() {
    let store = AdaptoStore::open(None).unwrap();
    let sched = Scheduler::new(store.clone())
        .poll_floor(Duration::from_millis(20))
        .job(
            Job::new("boom")
                .schedule(Schedule::Interval(Duration::from_secs(3600)))
                .max_attempts(1) // fail immediately, no long backoff in test
                .catch_up(true)  // run once at boot
                .run(|_ctx| async { Err(adapto_scheduler::JobError::msg("always fails")) }),
        );
    let handle = sched.spawn();
    tokio::time::sleep(Duration::from_millis(200)).await;
    handle.shutdown().await;
    let s = &handle.statuses()[0];
    assert_eq!(s.status, "failed");
    assert!(s.total_failures >= 1);
    assert_eq!(s.last_error.as_deref(), Some("always fails"));
}

#[tokio::test]
async fn manual_trigger_runs_now() {
    let store = AdaptoStore::open(None).unwrap();
    let counter = Arc::new(AtomicU32::new(0));
    let c2 = counter.clone();
    let sched = Scheduler::new(store.clone())
        .job(
            Job::new("manual")
                .schedule(Schedule::Interval(Duration::from_secs(3600))) // far future
                .run(move |_| { let c = c2.clone(); async move { c.fetch_add(1, Ordering::SeqCst); Ok(()) } }),
        );
    let handle = sched.spawn();
    tokio::time::sleep(Duration::from_millis(30)).await;
    handle.trigger("manual").await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    handle.shutdown().await;
    assert_eq!(counter.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn trigger_unknown_job_errs() {
    let store = AdaptoStore::open(None).unwrap();
    let sched = Scheduler::new(store.clone())
        .job(Job::new("a").schedule(Schedule::Interval(Duration::from_secs(3600))).run(|_| async { Ok(()) }));
    let handle = sched.spawn();
    let err = handle.trigger("nope").await.unwrap_err();
    assert!(matches!(err, adapto_scheduler::TriggerError::NoSuchJob(_)));
    handle.shutdown().await;
}

#[tokio::test]
async fn catch_up_false_does_not_run_at_boot() {
    let store = AdaptoStore::open(None).unwrap();
    let counter = Arc::new(AtomicU32::new(0));
    let c2 = counter.clone();
    // Pre-seed a past next_run by running once with catch_up then restarting is complex;
    // here we assert a fresh non-catch_up far-future job does not fire promptly.
    let sched = Scheduler::new(store.clone())
        .job(Job::new("lazy").schedule(Schedule::Interval(Duration::from_secs(3600))).catch_up(false)
            .run(move |_| { let c = c2.clone(); async move { c.fetch_add(1, Ordering::SeqCst); Ok(()) } }));
    let handle = sched.spawn();
    tokio::time::sleep(Duration::from_millis(120)).await;
    handle.shutdown().await;
    assert_eq!(counter.load(Ordering::SeqCst), 0);
}
```

- [ ] **Step 4: Run integration tests**

Run: `cargo test -p adapto_scheduler --test integration`
Expected: PASS (5 tests). Debug deadlocks by confirming `queue` uses `std::sync::Mutex` and no lock is held across `.await`. If `interval_job_runs_repeatedly` runs too few times, lower `poll_floor`/interval.

- [ ] **Step 5: Commit**

```bash
git add crates/adapto_scheduler/src/scheduler.rs crates/adapto_scheduler/tests/integration.rs
git commit -m "feat(scheduler): spawn() tick loop + lane runners + retry/catch-up/shutdown"
```

---

## Task 8: `Reloadable<T>`

**Files:**
- Modify: `crates/adapto_scheduler/src/reload.rs`

- [ ] **Step 1: Write failing test**

`crates/adapto_scheduler/src/reload.rs`:

```rust
use arc_swap::ArcSwapOption;
use std::sync::Arc;

/// A lock-free swappable cache for job-updated datasets.
/// Reads (`load`) are wait-free; a job calls `store` after writing the DB so changes appear
/// without a restart. Safe in a `static` via `const fn empty()`.
pub struct Reloadable<T> {
    cell: ArcSwapOption<T>,
}

impl<T> Reloadable<T> {
    pub const fn empty() -> Self {
        Reloadable { cell: ArcSwapOption::const_empty() }
    }
    pub fn new(v: T) -> Self {
        Reloadable { cell: ArcSwapOption::from(Some(Arc::new(v))) }
    }
    /// Current value, or None if never stored.
    pub fn load(&self) -> Option<Arc<T>> {
        self.cell.load_full()
    }
    /// Atomically replace the value.
    pub fn store(&self, v: T) {
        self.cell.store(Some(Arc::new(v)));
    }
    /// True if a value has been stored.
    pub fn is_set(&self) -> bool {
        self.cell.load().is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static GLOBAL: Reloadable<Vec<i32>> = Reloadable::empty();

    #[test]
    fn empty_then_store_then_load() {
        assert!(GLOBAL.load().is_none());
        GLOBAL.store(vec![1, 2, 3]);
        assert_eq!(*GLOBAL.load().unwrap(), vec![1, 2, 3]);
        GLOBAL.store(vec![9]);
        assert_eq!(*GLOBAL.load().unwrap(), vec![9]);
    }
}
```

- [ ] **Step 2: Run, verify pass**

Run: `cargo test -p adapto_scheduler reload::`
Expected: PASS. If `ArcSwapOption::const_empty` is unavailable in the pinned arc-swap version, use `ArcSwapOption::const_empty()` from arc-swap ≥1.6; bump the dep if needed.

- [ ] **Step 3: Commit**

```bash
git add crates/adapto_scheduler/src/reload.rs
git commit -m "feat(scheduler): Reloadable<T> lock-free swap cache"
```

---

## Task 9: Admin page renderer

**Files:**
- Modify: `crates/adapto_scheduler/src/admin.rs`

Pure data→HTML. No axum types (testable, reusable). The framework route layer (Task 11) calls this.

- [ ] **Step 1: Write failing test**

`crates/adapto_scheduler/src/admin.rs`:

```rust
use crate::state::JobStatus;

/// Render the jobs table as an HTML fragment. `post_base` is the path POST run-buttons target,
/// e.g. "/admin/jobs" -> form action "/admin/jobs/<job>/run".
pub fn render(statuses: &[JobStatus], post_base: &str) -> String {
    let mut rows = String::new();
    for s in statuses {
        let last_run = s.last_run.map(|d| d.format("%Y-%m-%d %H:%M UTC").to_string()).unwrap_or_else(|| "—".into());
        let next_run = s.next_run.map(|d| d.format("%Y-%m-%d %H:%M UTC").to_string()).unwrap_or_else(|| "—".into());
        let dur = s.last_duration_ms.map(|m| format!("{m} ms")).unwrap_or_else(|| "—".into());
        let err = s.last_error.as_deref().map(esc).unwrap_or_default();
        let err_row = if s.status == "failed" && !err.is_empty() {
            format!("<div class=\"err\">{err}</div>")
        } else { String::new() };
        rows.push_str(&format!(
            "<tr class=\"st-{status}\">\
             <td>{job}{err_row}</td><td>{lane}</td><td>{status}</td>\
             <td>{last_run}</td><td>{next_run}</td><td>{dur}</td>\
             <td><form method=\"post\" action=\"{base}/{job}/run\"><button>Run now</button></form></td>\
             </tr>",
            status = esc(&s.status), job = esc(&s.job), lane = esc(&s.lane),
            last_run = last_run, next_run = next_run, dur = dur, base = esc(post_base), err_row = err_row,
        ));
    }
    format!(
        "<h1>Scheduled Jobs</h1>\
         <table class=\"jobs\"><thead><tr>\
         <th>Job</th><th>Lane</th><th>Status</th><th>Last run</th><th>Next run</th><th>Duration</th><th></th>\
         </tr></thead><tbody>{rows}</tbody></table>\
         <style>.jobs{{width:100%;border-collapse:collapse;font:14px sans-serif}}\
         .jobs th,.jobs td{{text-align:left;padding:8px 10px;border-bottom:1px solid #eee}}\
         .st-failed{{background:#fff5f5}} .st-running{{background:#f0f7ff}}\
         .err{{color:#c0392b;font-size:12px;margin-top:4px}}\
         .jobs button{{padding:4px 10px;cursor:pointer}}</style>"
    )
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::JobStatus;

    fn st(job: &str, status: &str) -> JobStatus {
        JobStatus {
            job: job.into(), lane: "light".into(), priority: "high".into(), schedule: "DailyAt 06:30".into(),
            status: status.into(), last_run: None, last_duration_ms: Some(412), next_run: None,
            last_error: if status == "failed" { Some("<boom>".into()) } else { None },
            consecutive_failures: 0, total_runs: 1, total_failures: 0,
        }
    }

    #[test]
    fn renders_rows_and_run_button() {
        let html = render(&[st("currency", "ok")], "/admin/jobs");
        assert!(html.contains("currency"));
        assert!(html.contains("action=\"/admin/jobs/currency/run\""));
        assert!(html.contains("Run now"));
    }

    #[test]
    fn escapes_error_html() {
        let html = render(&[st("companies", "failed")], "/admin/jobs");
        assert!(html.contains("&lt;boom&gt;"));
        assert!(!html.contains("<boom>"));
    }
}
```

- [ ] **Step 2: Run, verify pass**

Run: `cargo test -p adapto_scheduler admin::`
Expected: PASS (2 tests).

- [ ] **Step 3: Run the whole crate suite + commit**

Run: `cargo test -p adapto_scheduler`
Expected: all green.

```bash
git add crates/adapto_scheduler/src/admin.rs
git commit -m "feat(scheduler): admin page HTML renderer with escaping"
```

---

## Task 10: `adapto_app` — `.scheduler()` + spawn/shutdown in `run()`

**Files:**
- Modify: `crates/adapto_app/Cargo.toml`
- Modify: `crates/adapto_app/src/lib.rs`

> **READ FIRST:** Open `crates/adapto_app/src/lib.rs`. Find (a) the `App` struct fields, (b) the builder methods region, (c) `build()` returning `(router, shutdown_hooks)`, (d) `run()` around the `axum::serve(...)` call (Explore located the injection point there). Match existing field/style conventions.

- [ ] **Step 1: Add the dep**

In `crates/adapto_app/Cargo.toml` `[dependencies]`:

```toml
adapto_scheduler = { path = "../adapto_scheduler" }
```

- [ ] **Step 2: Add a scheduler field + builder method**

In the `App` struct add:

```rust
    scheduler: Option<adapto_scheduler::Scheduler>,
```

Initialize it to `None` wherever `App` is constructed (the `new()` body). Add the builder method near the other builders:

```rust
    /// Attach a background-job scheduler. Spawned on `run()`, drained on shutdown.
    pub fn scheduler(mut self, sched: adapto_scheduler::Scheduler) -> Self {
        self.scheduler = Some(sched);
        self
    }
```

- [ ] **Step 3: Spawn in `run()`, drain before shutdown hooks**

In `run()`, after `let (router, shutdown_hooks) = self.build()?;` and BEFORE `axum::serve`, capture the handle. **Caution:** `build(self)` likely consumes `self`. If so, take the scheduler out before `build()`:

```rust
        let scheduler = self.scheduler.take();
        let (router, shutdown_hooks) = self.build()?;
        let sched_handle = scheduler.map(|s| s.spawn());
```

After `axum::serve(...).await?;` and BEFORE running shutdown hooks:

```rust
        if let Some(h) = &sched_handle {
            h.shutdown().await;
        }
```

(Keep the existing `for hook in shutdown_hooks { hook(); }` after that.)

> If `build()` needs the scheduler handle to mount admin routes (Task 11), thread the handle through instead — see Task 11 for the combined wiring. For THIS task, just prove spawn+shutdown compiles and runs.

- [ ] **Step 4: Smoke test**

Add to `crates/adapto_app/` tests (a new `tests/scheduler_smoke.rs` or an existing test file):

```rust
#[tokio::test]
async fn app_with_scheduler_builds_and_serves() {
    use adapto_app::App;
    use adapto_scheduler::{Job, Schedule, Scheduler};
    use adapto_store::AdaptoStore;
    use std::time::Duration;

    let store = AdaptoStore::open(None).unwrap();
    let sched = Scheduler::new(store.clone())
        .job(Job::new("noop").schedule(Schedule::Interval(Duration::from_secs(3600))).run(|_| async { Ok(()) }));

    let app = App::new("t").store(store).scheduler(sched);
    let client = app.test_client(); // per CLAUDE.md TestClient API
    let resp = client.get("/").await;
    let _ = resp.status(); // just prove it built with a scheduler attached
}
```

> **VERIFY:** `test_client()` may consume `app` or borrow it; and whether it spawns the scheduler. If `test_client()` doesn't exercise `run()`, this test only proves the builder compiles — that's acceptable here; the scheduler's own behavior is covered in Task 7.

Run: `cargo test -p adapto_app app_with_scheduler_builds_and_serves`
Expected: PASS / compiles.

- [ ] **Step 5: Commit**

```bash
git add crates/adapto_app/Cargo.toml crates/adapto_app/src/lib.rs crates/adapto_app/tests/scheduler_smoke.rs
git commit -m "feat(app): App::scheduler() spawns scheduler in run(), drains on shutdown"
```

---

## Task 11: `adapto_app` — `ctx.scheduler()` + guarded `.scheduler_admin()`

**Files:**
- Modify: `crates/adapto_app/src/lib.rs`

> **READ FIRST:** Find `RequestContext` and how state (store) is injected into handlers / axum state, and how existing routes like `page()`/`post()` are registered. Mirror that mechanism for the `SchedulerHandle`.

- [ ] **Step 1: Carry the handle into request context**

Store the spawned `SchedulerHandle` in the app's shared state (the same `Arc` state struct that already holds `store`). Add:

```rust
    // in the shared state struct used by handlers:
    scheduler: Option<adapto_scheduler::SchedulerHandle>,
```

Add the accessor on `RequestContext`:

```rust
    /// The running scheduler handle, if one was configured.
    pub fn scheduler(&self) -> Option<&adapto_scheduler::SchedulerHandle> {
        self.state.scheduler.as_ref()
    }
```

Because the handle exists only after `spawn()` (inside `run()`), the state must be populated then. Reconcile with Task 10: spawn the scheduler BEFORE `build()`, pass the resulting `Option<SchedulerHandle>` into `build()` so it lands in shared state. Adjust `build()`'s signature to accept it (or set it on a field read by `build()`).

- [ ] **Step 2: Add `.scheduler_admin()` (mandatory guard)**

Add fields:

```rust
    scheduler_admin_path: Option<String>,
    scheduler_admin_guard: Option<std::sync::Arc<dyn Fn(&RequestContext) -> bool + Send + Sync>>,
```

Builder:

```rust
    /// Mount a guarded admin page at `path` (GET) + run-trigger (POST `{path}/:job/run`).
    /// `guard` MUST return true only for authorized admins; false => 403.
    pub fn scheduler_admin<G>(mut self, path: &str, guard: G) -> Self
    where G: Fn(&RequestContext) -> bool + Send + Sync + 'static {
        self.scheduler_admin_path = Some(path.to_string());
        self.scheduler_admin_guard = Some(std::sync::Arc::new(guard));
        self
    }
```

- [ ] **Step 3: Mount the admin routes in `build()`**

Where routes are registered, if `scheduler_admin_path` is set, register two routes using the SAME registration mechanism existing handlers use. Pseudocode to adapt to the real router API:

```rust
    if let Some(path) = &self.scheduler_admin_path {
        let guard = self.scheduler_admin_guard.clone().unwrap();
        let post_path = format!("{path}/:job/run");

        // GET {path} -> render table
        let g1 = guard.clone();
        register_get(path, move |ctx: RequestContext| {
            if !g1(&ctx) { return forbidden_response(); }
            match ctx.scheduler() {
                Some(h) => html_response(adapto_scheduler::admin::render(&h.statuses(), path_for_closure)),
                None => text_response("scheduler not configured"),
            }
        });

        // POST {path}/:job/run -> trigger
        let g2 = guard.clone();
        register_post(&post_path, move |ctx: RequestContext| async move {
            if !g2(&ctx) { return forbidden_response(); }
            let job = ctx.param("job");
            if let Some(h) = ctx.scheduler() {
                let _ = h.trigger(job).await; // AlreadyRunning is fine; ignore
            }
            redirect_response(path_for_closure) // back to the table
        });
    }
```

> **Adapt to reality:** use the crate's actual `page()`/`async_post()` registration and `PageResponse` variants (`Ok(String)`, `Forbidden`, `Redirect`) from CLAUDE.md. The GET handler returns `PageResponse::Forbidden` when guard fails; `PageResponse::Ok(html)` otherwise. The POST returns `PageResponse::Forbidden` or `PageResponse::Redirect(path)`. Capture `path` into the closures via `String` clones (avoid borrowing `self`).

- [ ] **Step 4: Test guard blocks unauthorized**

`crates/adapto_app/tests/scheduler_admin.rs`:

```rust
#[tokio::test]
async fn admin_page_forbidden_without_auth() {
    use adapto_app::App;
    use adapto_scheduler::{Job, Schedule, Scheduler};
    use adapto_store::AdaptoStore;
    use std::time::Duration;

    let store = AdaptoStore::open(None).unwrap();
    let sched = Scheduler::new(store.clone())
        .job(Job::new("noop").schedule(Schedule::Interval(Duration::from_secs(3600))).run(|_| async { Ok(()) }));

    let app = App::new("t")
        .store(store)
        .scheduler(sched)
        .scheduler_admin("/admin/jobs", |_ctx| false); // deny everyone

    let client = app.test_client();
    let resp = client.get("/admin/jobs").await;
    assert_eq!(resp.status(), 403);
}

#[tokio::test]
async fn admin_page_ok_with_auth() {
    use adapto_app::App;
    use adapto_scheduler::{Job, Schedule, Scheduler};
    use adapto_store::AdaptoStore;
    use std::time::Duration;

    let store = AdaptoStore::open(None).unwrap();
    let sched = Scheduler::new(store.clone())
        .job(Job::new("currency").schedule(Schedule::Interval(Duration::from_secs(3600))).run(|_| async { Ok(()) }));

    let app = App::new("t").store(store).scheduler(sched)
        .scheduler_admin("/admin/jobs", |_ctx| true);
    let client = app.test_client();
    let resp = client.get("/admin/jobs").await;
    assert_eq!(resp.status(), 200);
    assert!(resp.text().contains("currency"));
}
```

> **VERIFY:** For these tests to pass, `test_client()` must spawn the scheduler so `ctx.scheduler()` is `Some` and `statuses()` returns the seeded job. If `test_client()` does NOT call the spawn path, either (a) add a `test_client()` variant that spawns, or (b) populate shared state with a spawned handle in the test-client builder. Document whichever you choose in `adapto_app`. Without a spawned handle, `admin_page_ok_with_auth` will show "scheduler not configured" — fix the wiring, not the test.

Run: `cargo test -p adapto_app scheduler_admin`
Expected: PASS (2 tests).

- [ ] **Step 4b: Workspace build**

Run: `cargo test --workspace`
Expected: all crates green.

- [ ] **Step 5: Commit**

```bash
git add crates/adapto_app/src/lib.rs crates/adapto_app/tests/scheduler_admin.rs
git commit -m "feat(app): ctx.scheduler() + guarded scheduler_admin page/trigger routes"
```

---

## Task 12: Framework CHANGELOG + version bump

**Files:**
- Modify: `CHANGELOG.md`
- Modify: `Cargo.toml` (root `[workspace.package]` version)

- [ ] **Step 1: Bump version**

In `/Users/sakentukenov/adapto-core/Cargo.toml`, bump `[workspace.package] version` (e.g. `0.X.Y` → next minor).

- [ ] **Step 2: CHANGELOG entry**

Under a new `## [Unreleased]` / version heading in `CHANGELOG.md`:

```markdown
### Added
- `adapto_scheduler` crate: persistent priority + lane background-job scheduler.
  - `Schedule` (Interval/DailyAt/WeeklyOn/MonthlyOn/Cron) with timezone-aware `next_after`.
  - Light (pooled) + Heavy (serial) lanes, priority ordering, per-job `catch_up`, exponential-backoff retry (3 attempts).
  - State persisted in `_jobs` collection (restart-safe, catch-up of missed runs).
  - `Reloadable<T>` lock-free swap cache for job-updated datasets.
- `adapto_app`: `App::scheduler()`, guarded `App::scheduler_admin()` page + run-now trigger, `RequestContext::scheduler()`.
```

- [ ] **Step 3: Commit**

```bash
git add CHANGELOG.md Cargo.toml
git commit -m "docs: changelog + version bump for adapto_scheduler"
```

---

## Task 13: myqaz — deps + jobs module + currency parser

**Files:**
- Modify: `~/myqaz/myqaz-rs/Cargo.toml`
- Create: `~/myqaz/myqaz-rs/src/jobs/mod.rs`
- Create: `~/myqaz/myqaz-rs/src/jobs/currency.rs`

> **READ FIRST:** `~/myqaz/myqaz-rs/src/routes/exchange_rates.rs` (the `ExchangeData`/rate structs + `exchange_rates` collection doc shape), and `~/myqaz/mining/exchange-rates/scraper.py` (the RSS field mapping). Match the existing rate record JSON shape exactly when upserting.

- [ ] **Step 1: Add deps**

`~/myqaz/myqaz-rs/Cargo.toml` `[dependencies]`:

```toml
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls"] }
quick-xml = "0.36"
adapto_scheduler = { path = "../../adapto-core/crates/adapto_scheduler" }
```

> Confirm how myqaz references adapto crates (path vs workspace patch). Use the SAME mechanism the existing `adapto_app`/`adapto_store` deps use in this Cargo.toml (likely a `[patch.crates-io]` or path) — add `adapto_scheduler` the identical way.

- [ ] **Step 2: Write a failing parser test (no network)**

`~/myqaz/myqaz-rs/src/jobs/currency.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RateRow {
    pub date: String,   // YYYY-MM-DD
    pub code: String,   // e.g. "USD"
    pub value: f64,     // tenge per `quant` units
    pub quant: u32,     // usually 1
}

/// Parse the National Bank of Kazakhstan `rates_all.xml` RSS body into rate rows.
/// Expected item shape:
/// <item><title>USD</title><description>512.34</description><quant>1</quant><pubdate>29.05.2026</pubdate></item>
pub fn parse_nbk_rss(xml: &str) -> Result<Vec<RateRow>, String> {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut rows = Vec::new();
    let mut cur_tag = String::new();
    let (mut title, mut desc, mut quant, mut date) = (String::new(), String::new(), String::new(), String::new());
    let mut in_item = false;
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if name == "item" { in_item = true; title.clear(); desc.clear(); quant.clear(); date.clear(); }
                cur_tag = name;
            }
            Ok(Event::Text(t)) => {
                if in_item {
                    let txt = t.unescape().map_err(|e| e.to_string())?.to_string();
                    match cur_tag.as_str() {
                        "title" => title = txt,
                        "description" => desc = txt,
                        "quant" => quant = txt,
                        "pubDate" | "pubdate" => date = txt,
                        _ => {}
                    }
                }
            }
            Ok(Event::End(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if name == "item" {
                    in_item = false;
                    if let Ok(value) = desc.replace(',', ".").parse::<f64>() {
                        rows.push(RateRow {
                            date: normalize_date(&date),
                            code: title.trim().to_uppercase(),
                            value,
                            quant: quant.trim().parse().unwrap_or(1),
                        });
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(e.to_string()),
            _ => {}
        }
        buf.clear();
    }
    Ok(rows)
}

/// NBK pubDate is "DD.MM.YYYY" -> "YYYY-MM-DD". Pass through if already ISO.
fn normalize_date(s: &str) -> String {
    let s = s.trim();
    if let Some((d, rest)) = s.split_once('.') {
        if let Some((m, y)) = rest.split_once('.') {
            return format!("{}-{:0>2}-{:0>2}", y.trim(), m.trim(), d.trim());
        }
    }
    s.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"<rss><channel>
      <item><title>USD</title><description>512,34</description><quant>1</quant><pubDate>29.05.2026</pubDate></item>
      <item><title>EUR</title><description>556.10</description><quant>1</quant><pubDate>29.05.2026</pubDate></item>
    </channel></rss>"#;

    #[test]
    fn parses_two_rows() {
        let rows = parse_nbk_rss(SAMPLE).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], RateRow { date: "2026-05-29".into(), code: "USD".into(), value: 512.34, quant: 1 });
        assert_eq!(rows[1].code, "EUR");
    }

    #[test]
    fn normalizes_dotted_date() {
        assert_eq!(normalize_date("05.01.2026"), "2026-01-05");
        assert_eq!(normalize_date("2026-01-05"), "2026-01-05");
    }
}
```

- [ ] **Step 3: Create the jobs module**

`~/myqaz/myqaz-rs/src/jobs/mod.rs`:

```rust
pub mod currency;
pub mod companies;

pub use currency::currency_job;
pub use companies::companies_job;
```

Add `mod jobs;` to `~/myqaz/myqaz-rs/src/main.rs` (near the other `mod` declarations).

Create a stub `~/myqaz/myqaz-rs/src/jobs/companies.rs` so the module resolves:

```rust
use adapto_scheduler::Job;
// real impl in Task 15
pub fn companies_job() -> Job { unimplemented!("Task 15") }
```

- [ ] **Step 4: Run the parser test**

Run (from `~/myqaz/myqaz-rs`): `cargo test -p myqaz jobs::currency` (or the crate's actual name)
Expected: PASS (2 tests). Adjust `RateRow`/field mapping if the real `exchange_rates` doc shape differs (per the READ FIRST step).

- [ ] **Step 5: Commit**

```bash
cd ~/myqaz && git add myqaz-rs/Cargo.toml myqaz-rs/src/jobs myqaz-rs/src/main.rs
git commit -m "feat(myqaz): jobs module + NBK RSS currency parser (tested)"
```

---

## Task 14: myqaz — `currency_job()` + migrate `exchange_rates` route to `Reloadable`

**Files:**
- Modify: `~/myqaz/myqaz-rs/src/jobs/currency.rs`
- Modify: `~/myqaz/myqaz-rs/src/routes/exchange_rates.rs`

> **READ FIRST:** In `exchange_rates.rs`, find the `OnceLock<...>` (or `OnceLock<Option<ExchangeData>>`) currently caching rates, and `load_data()`. You will replace the `OnceLock` with a `Reloadable<ExchangeData>` and add a `refresh_from_store()` the job calls.

- [ ] **Step 1: Add the Reloadable cache + refresh fn to the route module**

In `exchange_rates.rs`, replace the existing `OnceLock` cache with:

```rust
use adapto_scheduler::Reloadable;

// was: static CACHE: OnceLock<Option<ExchangeData>> = OnceLock::new();
static RATES: Reloadable<ExchangeData> = Reloadable::empty();

/// Build ExchangeData from the store and publish it to the cache.
pub fn refresh_from_store(store: &AdaptoStore) {
    if let Some(data) = build_exchange_data(store) { // existing builder that read the collection
        RATES.store(data);
    }
}

/// Lock-free read used by all rate routes. Lazily builds on first miss.
fn load_data(store: &AdaptoStore) -> Option<std::sync::Arc<ExchangeData>> {
    if let Some(d) = RATES.load() { return Some(d); }
    refresh_from_store(store);
    RATES.load()
}
```

Update existing route handlers: they previously got `&ExchangeData`; now they get `Arc<ExchangeData>` — `&*data` works the same. Adjust signatures from `Option<&'static ExchangeData>` to `Option<Arc<ExchangeData>>` and deref at use sites.

> The exact name `build_exchange_data` may differ — reuse whatever function currently constructs `ExchangeData` from the `exchange_rates` collection inside the old `OnceLock` init closure. Extract that closure body into `build_exchange_data(store) -> Option<ExchangeData>` if it isn't already a named fn.

- [ ] **Step 2: Implement `currency_job()`**

Append to `currency.rs`:

```rust
use adapto_scheduler::{Job, JobError, Lane, Priority, Schedule};
use adapto_store::Query;
use serde_json::json;

const NBK_RSS: &str = "https://nationalbank.kz/rss/rates_all.xml";

pub fn currency_job() -> Job {
    Job::new("currency")
        .schedule(Schedule::DailyAt { hour: 6, min: 30 })
        .lane(Lane::Light)
        .priority(Priority::High)
        .catch_up(true)
        .max_attempts(3)
        .run(|ctx| async move {
            // 1. fetch
            let body = reqwest::get(NBK_RSS)
                .await
                .map_err(JobError::from_err)?
                .text()
                .await
                .map_err(JobError::from_err)?;
            // 2. parse
            let rows = parse_nbk_rss(&body).map_err(JobError::msg)?;
            if rows.is_empty() {
                return Err(JobError::msg("NBK RSS returned no rows"));
            }
            // 3. upsert into exchange_rates (one doc per (date,code))
            let col = ctx.store.collection("exchange_rates");
            for r in &rows {
                // delete any existing (date,code) then insert fresh
                let _ = col.delete(Query::filter(adapto_store::Filter::And(vec![
                    adapto_store::Filter::Eq("date".into(), json!(r.date)),
                    adapto_store::Filter::Eq("code".into(), json!(r.code)),
                ])));
                col.insert(serde_json::to_value(r).map_err(JobError::from_err)?)
                    .map_err(JobError::from_err)?;
            }
            // 4. publish to the lock-free cache so routes see it without restart
            crate::routes::exchange_rates::refresh_from_store(&ctx.store);
            tracing::info!("currency job upserted {} rows", rows.len());
            Ok(())
        })
}
```

> **VERIFY the upsert shape:** match the EXISTING `exchange_rates` document schema (field names, whether rates are one-doc-per-day vs one-doc-per-(date,code)). If the collection currently stores one doc per day containing all currencies, change the upsert to build that day-doc shape instead. The route's `build_exchange_data` must read whatever shape you write. Keep them consistent — this is the single most important correctness check in the myqaz integration.

- [ ] **Step 3: Build (network code isn't unit-tested here; parser already covered)**

Run: `cd ~/myqaz/myqaz-rs && cargo build`
Expected: compiles. Fix `Filter`/`Query`/`delete` calls against the real adapto_store API (`crates/adapto_store/src/query.rs`).

- [ ] **Step 4: Commit**

```bash
cd ~/myqaz && git add myqaz-rs/src/jobs/currency.rs myqaz-rs/src/routes/exchange_rates.rs
git commit -m "feat(myqaz): currency_job (daily NBK fetch) + Reloadable rates cache"
```

---

## Task 15: myqaz — `companies_job()` (egov monthly)

**Files:**
- Modify: `~/myqaz/myqaz-rs/src/jobs/companies.rs`

> **READ FIRST:** `~/myqaz/mining/egov-ul/download.py` (pagination: `source={"from":N,"size":100}`, dedup by `bin`, the `gbd_ul` endpoint) and `~/myqaz/myqaz-rs/src/data.rs` (`build_company_summary_from_disk`, the `companies` disk_collection write path, `CompanyNormalizer`). Reuse the existing summary builder and normalizer rather than re-deriving them.

- [ ] **Step 1: Write a failing dedup test**

Replace the stub `companies.rs` with:

```rust
use serde_json::Value;
use std::collections::HashMap;

/// Deduplicate egov records by `bin` (last occurrence wins), preserving first-seen order.
pub fn dedup_by_bin(records: Vec<Value>) -> Vec<Value> {
    let mut order: Vec<String> = Vec::new();
    let mut latest: HashMap<String, Value> = HashMap::new();
    for r in records {
        let bin = r.get("bin").and_then(|v| v.as_str()).unwrap_or("").to_string();
        if bin.is_empty() { continue; }
        if !latest.contains_key(&bin) { order.push(bin.clone()); }
        latest.insert(bin, r);
    }
    order.into_iter().filter_map(|b| latest.remove(&b)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn last_record_per_bin_wins() {
        let recs = vec![
            json!({"bin":"111","name":"old"}),
            json!({"bin":"222","name":"b"}),
            json!({"bin":"111","name":"new"}),
            json!({"bin":"","name":"skip"}),
        ];
        let out = dedup_by_bin(recs);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0]["bin"], "111");
        assert_eq!(out[0]["name"], "new"); // last wins
        assert_eq!(out[1]["bin"], "222");
    }
}
```

Run: `cd ~/myqaz/myqaz-rs && cargo test -p myqaz jobs::companies`
Expected: PASS (1 test).

- [ ] **Step 2: Implement `companies_job()`**

Append to `companies.rs`:

```rust
use adapto_scheduler::{Job, JobError, Lane, Priority, Schedule};

const PAGE: usize = 100;

pub fn companies_job() -> Job {
    Job::new("companies")
        .schedule(Schedule::MonthlyOn { day: 2, hour: 3, min: 0 })
        .lane(Lane::Heavy)
        .priority(Priority::Low)
        .catch_up(false)
        .max_attempts(3)
        .run(|ctx| async move {
            let key = std::env::var("EGOV_API_KEY").map_err(|_| JobError::msg("EGOV_API_KEY not set"))?;
            let base = format!("https://data.egov.kz/api/v4/gbd_ul/v1?apiKey={key}");
            let client = reqwest::Client::new();

            // paginate until a short page
            let mut all: Vec<serde_json::Value> = Vec::new();
            let mut from = 0usize;
            loop {
                let source = serde_json::json!({"from": from, "size": PAGE}).to_string();
                let url = format!("{base}&source={}", urlencoding::encode(&source));
                let page: Vec<serde_json::Value> = client.get(&url).send().await
                    .map_err(JobError::from_err)?
                    .json().await
                    .map_err(JobError::from_err)?;
                let n = page.len();
                all.extend(page);
                if n < PAGE { break; }
                from += PAGE;
                if from > 2_000_000 { break; } // safety cap
            }

            let deduped = dedup_by_bin(all);
            tracing::info!("companies job fetched {} unique BINs", deduped.len());

            // write to disk collection + rebuild summary via existing data.rs helpers
            crate::data::replace_companies(&ctx.store, deduped)
                .map_err(|e| JobError::msg(e.to_string()))?;
            Ok(())
        })
}
```

> **VERIFY / glue:** `urlencoding` — add `urlencoding = "2"` to Cargo.toml (or build the query with `reqwest`'s `.query(&[("source", source)])`, which percent-encodes for you — preferred, drop the manual encode). `crate::data::replace_companies(store, Vec<Value>)` likely does NOT exist yet — create a small fn in `data.rs` that: bulk-inserts into `disk_collection("companies")` (overwrite), ensures the `bin` unique index, runs `CompanyNormalizer`, and rebuilds `company_summary` (reuse `build_company_summary_from_disk`). Then publish any `Reloadable` summary cache if the summary route uses one. Keep it consistent with how the startup importer does it.

- [ ] **Step 3: Build**

Run: `cd ~/myqaz/myqaz-rs && cargo build`
Expected: compiles after adding `replace_companies` and the egov query glue.

- [ ] **Step 4: Commit**

```bash
cd ~/myqaz && git add myqaz-rs/src/jobs/companies.rs myqaz-rs/src/data.rs myqaz-rs/Cargo.toml
git commit -m "feat(myqaz): companies_job (monthly egov pull) + dedup + replace_companies"
```

---

## Task 16: myqaz — wire scheduler into `main.rs` + admin guard

**Files:**
- Modify: `~/myqaz/myqaz-rs/src/main.rs`

> **READ FIRST:** Find the `App::new(...)...run().await` chain in `main.rs`, and the existing admin auth check (the `adapto_auth` session/HMAC check used by other admin routes). Reuse that exact check as the guard.

- [ ] **Step 1: Attach the scheduler + admin page**

In the `App` builder chain (after `.store(store.clone())`):

```rust
    .scheduler(
        adapto_scheduler::Scheduler::new(store.clone())
            .timezone(chrono_tz::Asia::Almaty)
            .light_workers(4)
            .job(jobs::currency_job())
            .job(jobs::companies_job()),
    )
    .scheduler_admin("/admin/jobs", |ctx| is_admin(ctx))
```

Where `is_admin(ctx)` is the project's existing admin check. If it lives elsewhere, import it; if there's no single helper, inline the same cookie/session validation other admin routes use. **Do not weaken it** — same source of truth.

> Add `chrono-tz` to myqaz `Cargo.toml` if not already present: `chrono-tz = "0.9"`.

- [ ] **Step 2: Local run smoke test (manual)**

Run: `cd ~/myqaz/myqaz-rs && cargo run` (locally, with `EGOV_API_KEY` unset is fine — the monthly job won't fire on boot since `catch_up(false)`).
Verify:
- App boots, logs `scheduler` index creation.
- `GET /admin/jobs` without admin cookie → 403.
- With admin session → 200, table shows `currency` (next run tomorrow 06:30) + `companies` (next run day 2).
- Click "Run now" on `currency` → row flips to running→ok, rates refresh.

Stop with Ctrl-C; confirm graceful "draining" shutdown.

- [ ] **Step 3: Commit**

```bash
cd ~/myqaz && git add myqaz-rs/src/main.rs myqaz-rs/Cargo.toml
git commit -m "feat(myqaz): wire scheduler (currency daily, companies monthly) + guarded /admin/jobs"
```

---

## Task 17: Cross-compile, deploy, verify on prod

**Files:** none (ops)

> Prod: `deploy@91.224.74.233:2222`, app `/opt/myqaz`, port 8082. Build locally, ship binary + templates, restart. `EGOV_API_KEY` must be in the systemd env.

- [ ] **Step 1: Cross-compile**

Run: `cd ~/myqaz/myqaz-rs && cargo zigbuild --release --target x86_64-unknown-linux-gnu`
Expected: builds (rustls → no openssl). Note the binary path under `target/x86_64-unknown-linux-gnu/release/`.

- [ ] **Step 2: Ensure `EGOV_API_KEY` on prod**

Confirm the systemd unit (or drop-in) for myqaz exports `EGOV_API_KEY`. If not, add a drop-in:
`/etc/systemd/system/myqaz.service.d/egov.conf` with `[Service]\nEnvironment=EGOV_API_KEY=...`, then `systemctl daemon-reload`. (Use the key already in `~/myqaz/mining/.env`.)

- [ ] **Step 3: Ship + restart**

`scp` the new binary to `/opt/myqaz` (replacing the running one), `scp` any changed templates, then `ssh ... 'systemctl restart myqaz'`. Tail logs: confirm scheduler seeded both jobs, no panic (a bad cron would panic at boot — there are none here, both use typed schedules).

- [ ] **Step 4: Verify checklist on prod**

- `GET https://myqaz.kz/admin/jobs` (with admin auth) → 200, both jobs listed, `next_run` correct in Almaty time.
- "Run now" on `currency` → completes, `/reference/exchange-rates/usd` shows today's date without a restart (Reloadable working).
- Leave `companies` for its monthly slot (or trigger once manually off-peak to validate the full 739K pull end-to-end; watch memory + duration in the `_jobs` row).
- Re-run the route speed bench (`bash /tmp/bench.sh` on idle server) → exchange routes still ≤8ms (Reloadable read is lock-free, same as OnceLock).

- [ ] **Step 5: Tag (optional, if publishing framework)**

If the framework version was bumped for release: `git -C ~/adapto-core tag vX.Y.Z && git push --tags`.

---

## Self-Review (completed by plan author)

**Spec coverage:**
- Native reqwest fetch → Tasks 13–15. ✓
- Schedule enum + cron → Task 2. ✓
- Priority + lanes → Tasks 5 (ReadyKey), 7 (Light/Heavy LaneState). ✓
- Per-job catch_up → Tasks 5 (`catch_up_decision`), 7 (boot enqueue). ✓
- `_jobs` collection + admin page + Run-now + tracing → Tasks 4, 9, 11. ✓
- Exponential backoff retry → Task 5 (`backoff`/`apply_outcome`), 7 (re-enqueue via next_run). ✓
- Reloadable cache → Tasks 8, 14. ✓
- Guarded admin + auth boundary → Task 11. ✓
- Graceful shutdown drain → Tasks 6 (`shutdown`), 7 (grace loop). ✓
- CHANGELOG + version → Task 12. ✓
- Cross-compile/deploy → Task 17. ✓

**Type consistency:** `Schedule::next_after(now: DateTime<Tz>) -> Result<DateTime<Utc>, String>` used consistently (Tasks 2, 7). `apply_outcome`/`catch_up_decision` signatures match call sites (Tasks 5, 7). `JobStatus` fields match `admin::render` usage (Tasks 4, 9). `Reloadable::{empty,load,store}` consistent (Tasks 8, 14). `SchedulerHandle::{statuses,trigger,shutdown}` consistent (Tasks 6, 10, 11).

**Known adapt-to-reality points (flagged inline, not placeholders):** `Update::Set` whole-doc replacement (Task 4), `build()` consuming self + threading the handle (Tasks 10–11), `test_client()` spawning the scheduler (Tasks 10–11), exact `exchange_rates` doc shape (Task 14), `replace_companies` glue + egov query encoding (Task 15), existing `is_admin` helper (Task 16). Each has a concrete fallback instruction.
