# adapto_scheduler — Design Spec

**Date:** 2026-05-29
**Status:** Approved (design), pending implementation plan
**Crate:** `adapto_scheduler` (new, adapto-core workspace)
**Consumer:** myqaz.kz (`~/myqaz/myqaz-rs`)

## Goal

A first-class, persistent background-job scheduler in the Adapto framework, then concrete
jobs in myqaz: **currency rates refresh daily**, **legal-entity (companies) list refresh
monthly**. Built framework-first (generic, reusable), site-second (concrete jobs).

## Decisions (locked)

| Topic | Decision |
|-------|----------|
| Fetch model | Native Rust (`reqwest`) — self-contained binary, no Python at runtime |
| Schedule API | Typed `Schedule` enum + `Cron(String)` escape hatch |
| Concurrency | Priority + **lanes**: Light lane = worker pool (default 4), Heavy lane = serial (1); priority orders within a lane |
| Missed runs | Per-job `catch_up: bool` (coalesced single catch-up run) |
| Ops surface | `_jobs` collection (state + history) + built-in `/admin/jobs` page + Run-now + `tracing` |
| Retry | Exponential backoff capped (default 3 attempts: 1m/5m/15m), then `failed`, resume normal cadence |

## Non-goals (YAGNI)

- Distributed/multi-node scheduling (single process).
- Custom user-defined lanes beyond Light/Heavy.
- Incremental company sync (future optimization; monthly full re-pull for now).
- Framework-owned auth scheme (delegates to consumer `guard`).

---

## Section 1 — Public API

```rust
pub enum Schedule {
    Interval(Duration),
    DailyAt   { hour: u8, min: u8 },
    WeeklyOn  { weekday: chrono::Weekday, hour: u8, min: u8 },
    MonthlyOn { day: u8, hour: u8, min: u8 },   // day 1..=28 safe; 29-31 clamps to month end
    Cron(String),                               // parsed at Scheduler::spawn(); parse error = fail fast
}
impl Schedule {
    /// Next fire time strictly after `now`, computed in the scheduler timezone, returned as UTC.
    fn next_after(&self, now: chrono::DateTime<chrono_tz::Tz>) -> chrono::DateTime<chrono::Utc>;
}

pub enum Lane { Light, Heavy }                  // Light = pool, Heavy = serial(1)
pub enum Priority { Low, Normal, High, Critical } // ordering within a lane (desc), tie-break earliest-due

pub struct JobContext {
    pub store: adapto_store::AdaptoStore,       // cheap Arc-clone
    pub attempt: u32,                           // 1-based
    pub scheduled_for: chrono::DateTime<chrono::Utc>,
}

pub enum JobError { /* boxed error + message; from anyhow/std error */ }

// Job builder
Job::new("currency")
    .schedule(Schedule::DailyAt { hour: 6, min: 30 })
    .lane(Lane::Light)
    .priority(Priority::High)
    .catch_up(true)
    .max_attempts(3)                            // default 3
    .run(|ctx| async move { /* fetch + upsert + cache swap */ Ok(()) });

// Scheduler builder
let sched = Scheduler::new(store.clone())
    .timezone(chrono_tz::Asia::Almaty)          // default UTC
    .light_workers(4)                           // Heavy fixed at 1
    .poll_floor(Duration::from_secs(15))
    .shutdown_grace(Duration::from_secs(30))
    .job(currency_job())
    .job(companies_job());

let handle: SchedulerHandle = sched.spawn();    // spawns tokio tasks
handle.statuses();                              // Vec<JobStatus> for admin
handle.trigger("currency").await?;              // Run-now (Err(AlreadyRunning) if running)
handle.shutdown().await;                        // drain in-flight, stop loop
```

- **Handler type:** `Arc<dyn Fn(JobContext) -> BoxFuture<'static, Result<(), JobError>> + Send + Sync>`.
- **Panic safety:** handler future wrapped in `catch_unwind`; panic → job `failed`, scheduler survives.
- **Clock abstraction:** internal `now()` via a `Clock` trait — `SystemClock` (prod), `MockClock` (tests) for instant, deterministic schedule/backoff/catch-up tests.
- **HTTP-agnostic:** no `reqwest` in this crate; jobs bring their own fetching.

---

## Section 2 — Engine & Persistence

### `_jobs` collection (one doc per job, unique index on `job`)

```jsonc
{
  "job": "currency",
  "lane": "light",
  "priority": "high",
  "schedule": "DailyAt 06:30",          // human-readable, for admin
  "status": "ok",                       // pending|running|ok|failed|retrying
  "last_run": "2026-05-29T06:30:01Z",
  "last_duration_ms": 412,
  "next_run": "2026-05-30T06:30:00Z",   // UTC — source of truth, drives loop
  "last_error": null,
  "attempt": 0,                         // current retry attempt (0 = idle)
  "consecutive_failures": 0,
  "total_runs": 138,
  "total_failures": 2,
  "updated_at": "2026-05-29T06:30:01Z"
}
```

`next_run` persisted → restart resumes exactly. On `spawn()`: load-or-seed each job's doc;
compute `next_run` if absent.

### Three cooperating tokio tasks

```
scheduler tick loop          light lane runner         heavy lane runner
- sleep until earliest       - Semaphore(N=4)          - Semaphore(1)
  next_run (cap poll_floor)  - pop by (prio, due)      - pop by (prio, due)
- enqueue due jobs           - run handler             - run handler
- select! shutdown + wake    - update _jobs doc        - update _jobs doc

shared: per-lane BinaryHeap<Ready> + running:HashSet<name> behind Mutex
in-memory state mirror of _jobs (avoids hitting store every tick)
```

**Tick loop:** read in-memory `next_run`s → `sleep_until(min(next_run))` capped by `poll_floor`
(15s) so triggers/new state are picked up → `select!` on shutdown + wake channels → on wake,
any job `next_run <= now` and not in `running` → push to its lane heap, mark `pending`.

**Lane runner:** acquire permit (caps concurrency) → pop highest `(Priority desc, due asc)` →
single-instance guard via `running` set → `status=running`, `attempt=n` → `catch_unwind`
handler, measure duration →
- **Ok:** `status=ok`, `last_run`, `last_duration_ms`, `consecutive_failures=0`, `total_runs+=1`,
  `next_run = schedule.next_after(now)`; remove from `running`.
- **Err/panic, attempt < max:** `status=retrying`, `next_run = now + backoff[attempt]`
  (1m/5m/15m), `total_failures+=1`; remove from `running` (re-enqueued by tick loop on backoff).
- **Err/panic, attempt == max:** `status=failed`, `last_error`, `consecutive_failures+=1`,
  `next_run = schedule.next_after(now)` (resume cadence); remove from `running`.

### Catch-up on boot (per-job)

After seeding, if `next_run <= now`:
- `catch_up == true` → enqueue **once** immediately (N missed = 1 run), then normal `next_run`.
- `catch_up == false` → skip, `next_run = schedule.next_after(now)` (no surprise heavy import on deploy).

### Manual trigger

`handle.trigger(name)` → mpsc to tick loop → effective due = now → wake → enqueue. If in
`running` → `Err(AlreadyRunning)`. Resolves once enqueued (execution is fire-and-forget).

### Graceful shutdown

`handle.shutdown()`: tick loop stops enqueuing; lane runners finish in-flight (bounded by
`shutdown_grace`, default 30s) then exit; final `_jobs` flush. App `on_shutdown` hooks run after.

### Failure isolation

- Handler panic → caught, job failed, scheduler keeps running.
- `_jobs` write error → `tracing` log, loop continues (in-memory state authoritative for tick).
- Cron parse error → caught at `spawn()`, returns `Err` → fail fast at startup.

Primitives: `tokio::sync::{Semaphore, Mutex, mpsc}` + `std::collections::BinaryHeap`.

---

## Section 3 — App Integration, Admin Page, Auth

### Dependency direction

`adapto_app` → depends on → `adapto_scheduler` (one-way, no cycle).

### Builder methods

```rust
App::new("myqaz")
    .store(store)
    .scheduler(sched)                                  // unspawned Scheduler
    .scheduler_admin("/admin/jobs", |ctx| is_admin(ctx)) // optional built-in page (guarded)
    .run().await?;
```

### Wiring inside `App::run()`

```rust
let (router, shutdown_hooks) = self.build()?;          // build() mounts admin routes if configured
let sched_handle = self.scheduler.take().map(|s| s.spawn());   // Option<SchedulerHandle>
let listener = TcpListener::bind(&addr).await?;
axum::serve(listener, router).with_graceful_shutdown(shutdown_signal()).await?;
if let Some(h) = sched_handle { h.shutdown().await; }  // drain first
for hook in shutdown_hooks { hook(); }                 // then user hooks
```

`SchedulerHandle` is `Clone` (Arc inside), injected into axum router state.

### Reaching the handle from routes

`RequestContext` gains `ctx.scheduler() -> Option<&SchedulerHandle>` so consumers can build
custom admin pages or trigger jobs from their own routes (`statuses()`, `trigger(name)`).

### Built-in admin page (`scheduler_admin`)

- `GET  {path}` → render jobs table (server-side HTML via `adapto_ui`).
- `POST {path}/:job/run` → `handle.trigger(job)` → redirect back with flash.
- Renderer: `adapto_scheduler::admin::render(statuses) -> String` (pure data→HTML, no axum types — testable, reusable).
- Table columns: Job, Lane, Status, Last run, Next run, Duration, [Run now]. Failed rows show
  `last_error` inline; live "running…" while executing.

### Auth — security boundary

- `scheduler_admin(path, guard)` takes a **mandatory** `guard: impl Fn(&RequestContext) -> bool + Send + Sync`.
  No unguarded variant exists — the page cannot be mounted without an authorization check.
- Both `GET` page and `POST .../run` are gated by the **same** `guard`; on `false` → `403 Forbidden`.
- The trigger is `POST`-only (never reachable via GET/link/crawler/prefetch), so a bare URL visit
  cannot fire a job.
- myqaz supplies its existing admin auth (`adapto_auth`) as the guard:
  `.scheduler_admin("/admin/jobs", |ctx| myqaz_auth::is_admin(ctx))`.
- The framework does NOT invent its own auth here — single source of truth for "who is admin"
  in the consumer, avoiding a second weaker auth path.

---

## Section 4 — myqaz Concrete Jobs

### New deps (myqaz-rs only)
`reqwest` (rustls-tls — clean zigbuild cross-compile, no openssl), `quick-xml` (NBK RSS parse).

### Cache-refresh (critical)

Routes cache served data in `OnceLock` (set-once). A job writing fresh data won't appear until
restart. Fix: job-updated datasets move to a swappable cache.

Framework ships `adapto_scheduler::Reloadable<T>` (thin `ArcSwap<T>`):
```rust
static RATES: Reloadable<ExchangeData> = Reloadable::empty();
RATES.load()         // &Arc<ExchangeData> — lock-free read on route path (keeps ≤8ms checklist)
RATES.store(data)    // atomic swap — job's last step after writing store
```

### `currency_job()` — daily
- **Source:** `https://nationalbank.kz/rss/rates_all.xml` (reqwest GET).
- **Parse:** quick-xml → `{ date, code, value, quant }` per `<item>`.
- **Write:** upsert today's date into `exchange_rates` (merge, preserve history).
- **Refresh:** rebuild `ExchangeData`, `RATES.store(...)`.
- **Schedule:** `DailyAt 06:30` Almaty, `Lane::Light`, `Priority::High`, `catch_up(true)`, `max_attempts(3)`.

### `companies_job()` — monthly
- **Source:** egov `gbd_ul` API, `EGOV_API_KEY` env (already recovered to `.env`).
- **Port of `download.py`:** paginated fetch (`source={from,size:100}`), dedup by `bin`, write to
  `disk_collection("companies")`, rebuild `company_summary`, swap its `Reloadable`.
- **Schedule:** `MonthlyOn { day: 2, hour: 3, min: 0 }` Almaty, `Lane::Heavy`, `Priority::Low`,
  `catch_up(false)`, `max_attempts(3)`.
- **Note:** full 739K re-pull is minutes-long → heavy lane, monthly, off-peak. Incremental
  (new BINs only) = future optimization, flagged not built.

### Wiring (myqaz `main.rs`)
```rust
.scheduler(
    Scheduler::new(store.clone())
        .timezone(chrono_tz::Asia::Almaty)
        .light_workers(4)
        .job(jobs::currency_job())
        .job(jobs::companies_job()),
)
.scheduler_admin("/admin/jobs", |ctx| myqaz_auth::is_admin(ctx))
```

---

## Section 5 — Testing, Build, Ship

### `adapto_scheduler` deps
`tokio`, `chrono`, `chrono-tz`, `serde`, `serde_json`, `adapto_store`, `tracing`, `thiserror`,
`arc-swap`, `cron`. No `reqwest`.

### Tests (framework)
- **Unit:** `Schedule::next_after` per variant — month-end clamp (day 31 → Feb), DST boundary,
  cron parse + error; backoff sequence; priority ordering; catch-up (past `next_run`, coalesced).
- **Integration (MockClock + in-memory store):** `Interval(ms)` jobs run; `_jobs` transitions
  pending→running→ok; failure→retrying→failed with attempt counts; `trigger` enqueues +
  `AlreadyRunning` guard; Light lane honors `light_workers`, Heavy serializes; `shutdown()`
  drains in-flight. MockClock = instant, deterministic.

### Tests (myqaz)
- Currency: sample RSS fixture → parsed records (no network).
- Companies: mocked paginated response → dedup-by-bin correctness.

### Build & ship
- `cargo test -p adapto_scheduler`, `cargo test --workspace`.
- `cargo zigbuild --release --target x86_64-unknown-linux-gnu` (rustls → no openssl).
- `EGOV_API_KEY` in prod systemd env / `.env`.
- **CHANGELOG.md mandatory** (Added: `adapto_scheduler` crate). Bump `[workspace.package]` version.

---

## Build order

1. `adapto_scheduler` crate: `Schedule` + `next_after` (+ unit tests) → `Job`/`Scheduler` builders
   → `Clock` → engine (tick loop, lanes, retry, catch-up, single-instance, shutdown) →
   `_jobs` persistence → `SchedulerHandle` → integration tests.
2. `Reloadable<T>` + admin renderer (`admin::render`).
3. `adapto_app` integration: `.scheduler()`, `.scheduler_admin()`, `App::run()` spawn/shutdown,
   `ctx.scheduler()`.
4. myqaz: `reqwest`/`quick-xml` deps, `src/jobs/`, `currency_job`, migrate exchange_rates route to
   `Reloadable`, `companies_job`, wire into `main.rs`, admin guard.
5. CHANGELOG + version bump. Cross-compile, deploy, verify `/admin/jobs` + first runs.
