use crate::engine::{apply_outcome, catch_up_decision, Outcome, ReadyKey};
use crate::job::{Job, JobContext, JobHandler, Lane, Priority};
use crate::schedule::Schedule;
use crate::state::{ensure_index, load_all, save, JobState, JobStatus, RunStatus};
use adapto_store::AdaptoStore;
use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::{mpsc, oneshot, Notify, Semaphore};

// ---------------------------------------------------------------------------
// Builder
// ---------------------------------------------------------------------------

/// Builder + owner of a set of jobs. Call [`Scheduler::spawn`] to start it.
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
    /// Create a scheduler bound to a store. Defaults: UTC timezone, 4 light
    /// workers, 15s poll floor, 30s shutdown grace.
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
    /// Timezone used to interpret `DailyAt`/`WeeklyOn`/`MonthlyOn` schedules.
    pub fn timezone(mut self, tz: Tz) -> Self {
        self.timezone = tz;
        self
    }
    /// Number of concurrent workers in the Light lane (Heavy is always 1).
    pub fn light_workers(mut self, n: usize) -> Self {
        self.light_workers = n.max(1);
        self
    }
    /// Maximum time the tick loop sleeps before re-checking (also bounds
    /// manual-trigger latency).
    pub fn poll_floor(mut self, d: Duration) -> Self {
        self.poll_floor = d;
        self
    }
    /// How long to wait for in-flight jobs to finish on shutdown.
    pub fn shutdown_grace(mut self, d: Duration) -> Self {
        self.shutdown_grace = d;
        self
    }
    /// Register a job.
    pub fn job(mut self, j: Job) -> Self {
        self.jobs.push(j);
        self
    }

    #[doc(hidden)] // test-only clock injection
    pub fn now_fn(mut self, f: fn() -> DateTime<Utc>) -> Self {
        self.now_fn = f;
        self
    }

    /// Seed state, apply catch-up, and start the tick loop + lane runners as
    /// tokio tasks. Must be called from within a tokio runtime.
    ///
    /// # Panics
    /// Panics if any job has an unparseable `Cron` schedule (fail fast at boot).
    pub fn spawn(self) -> SchedulerHandle {
        ensure_index(&self.store);
        let now = (self.now_fn)();
        let tz = self.timezone;
        let now_fn = self.now_fn;

        let mut persisted = load_all(&self.store);
        let job_names: Vec<String> = self.jobs.iter().map(|j| j.name.clone()).collect();
        let mut handlers: HashMap<String, Arc<JobRuntime>> = HashMap::new();
        let mut due_now: Vec<String> = Vec::new();

        for j in &self.jobs {
            let lane_s = lane_str(j.lane).to_string();
            let prio_s = priority_str(j.priority).to_string();
            let sched_s = schedule_desc(j);
            let next_scheduled = j
                .schedule
                .next_after(now.with_timezone(&tz))
                .unwrap_or_else(|e| panic!("scheduler: job '{}' invalid schedule: {e}", j.name));

            let st = persisted
                .entry(j.name.clone())
                .or_insert_with(|| JobState::seed(&j.name, &lane_s, &prio_s, &sched_s, now));
            st.lane = lane_s;
            st.priority = prio_s;
            st.schedule = sched_s;
            if st.status == RunStatus::Running {
                // crashed mid-run last boot -> reset
                st.status = RunStatus::Pending;
            }

            let (next_run, run_now) = catch_up_decision(st.next_run, j.catch_up, now, next_scheduled);
            st.next_run = Some(next_run);
            if run_now {
                due_now.push(j.name.clone());
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

        let handlers = Arc::new(handlers);
        let active = Arc::new(Mutex::new(HashSet::<String>::new()));
        let light = Arc::new(LaneState::new(self.light_workers));
        let heavy = Arc::new(LaneState::new(1));

        let (trigger_tx, mut trigger_rx) = mpsc::unbounded_channel::<String>();
        let (shutdown_tx, mut shutdown_rx) = mpsc::unbounded_channel::<()>();
        let (done_tx, done_rx) = oneshot::channel::<()>();

        // Pre-enqueue catch-up jobs.
        for name in &due_now {
            enqueue(&handlers, &active, &light, &heavy, name, now);
        }

        // Lane runner tasks.
        spawn_lane_runner(light.clone(), handlers.clone(), active.clone(), self.store.clone(), tz, now_fn);
        spawn_lane_runner(heavy.clone(), handlers.clone(), active.clone(), self.store.clone(), tz, now_fn);

        // Tick loop.
        {
            let store = self.store.clone();
            let handlers = handlers.clone();
            let active = active.clone();
            let light = light.clone();
            let heavy = heavy.clone();
            let poll_floor = self.poll_floor;
            let grace = self.shutdown_grace;
            tokio::spawn(async move {
                loop {
                    let now = now_fn();
                    let states = load_all(&store);
                    let mut sleep = poll_floor;
                    for n in handlers.keys() {
                        if let Some(nr) = states.get(n).and_then(|s| s.next_run) {
                            if nr > now {
                                if let Ok(d) = (nr - now).to_std() {
                                    if d < sleep {
                                        sleep = d;
                                    }
                                }
                            }
                        }
                    }
                    tokio::select! {
                        _ = tokio::time::sleep(sleep) => {}
                        Some(name) = trigger_rx.recv() => {
                            enqueue(&handlers, &active, &light, &heavy, &name, now_fn());
                            continue;
                        }
                        _ = shutdown_rx.recv() => { break; }
                    }
                    let now = now_fn();
                    let states = load_all(&store);
                    for n in handlers.keys() {
                        if let Some(nr) = states.get(n).and_then(|s| s.next_run) {
                            if nr <= now {
                                enqueue(&handlers, &active, &light, &heavy, n, now);
                            }
                        }
                    }
                }
                // Shutdown: stop accepting, drain in-flight within grace.
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
                store: self.store,
                trigger_tx,
                shutdown_tx,
                active,
                job_names,
                done: Mutex::new(Some(done_rx)),
            }),
        }
    }
}

// ---------------------------------------------------------------------------
// Handle
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum TriggerError {
    #[error("no such job: {0}")]
    NoSuchJob(String),
    #[error("job already running: {0}")]
    AlreadyRunning(String),
    #[error("scheduler stopped")]
    Stopped,
}

/// Cheap, clonable handle to a running scheduler.
#[derive(Clone)]
pub struct SchedulerHandle {
    inner: Arc<HandleInner>,
}

struct HandleInner {
    store: AdaptoStore,
    trigger_tx: mpsc::UnboundedSender<String>,
    shutdown_tx: mpsc::UnboundedSender<()>,
    active: Arc<Mutex<HashSet<String>>>,
    job_names: Vec<String>,
    done: Mutex<Option<oneshot::Receiver<()>>>,
}

impl SchedulerHandle {
    /// Current persisted status of every registered job (for an admin page),
    /// sorted by job name.
    pub fn statuses(&self) -> Vec<JobStatus> {
        let states = load_all(&self.inner.store);
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

    /// Request an immediate run of `job`.
    pub async fn trigger(&self, job: &str) -> Result<(), TriggerError> {
        if !self.inner.job_names.iter().any(|n| n == job) {
            return Err(TriggerError::NoSuchJob(job.to_string()));
        }
        if self.inner.active.lock().unwrap().contains(job) {
            return Err(TriggerError::AlreadyRunning(job.to_string()));
        }
        self.inner
            .trigger_tx
            .send(job.to_string())
            .map_err(|_| TriggerError::Stopped)
    }

    /// Stop the tick loop and wait for in-flight jobs to drain (bounded by the
    /// configured shutdown grace).
    pub async fn shutdown(&self) {
        let _ = self.inner.shutdown_tx.send(());
        let rx = self.inner.done.lock().unwrap().take();
        if let Some(rx) = rx {
            let _ = rx.await;
        }
    }
}

// ---------------------------------------------------------------------------
// Internals
// ---------------------------------------------------------------------------

struct JobRuntime {
    name: String,
    lane: Lane,
    priority: Priority,
    max_attempts: u32,
    schedule: Schedule,
    handler: JobHandler,
}

struct QueueItem {
    key: ReadyKey,
    name: String,
}
impl PartialEq for QueueItem {
    fn eq(&self, o: &Self) -> bool {
        self.key == o.key
    }
}
impl Eq for QueueItem {}
impl Ord for QueueItem {
    fn cmp(&self, o: &Self) -> std::cmp::Ordering {
        self.key.cmp(&o.key)
    }
}
impl PartialOrd for QueueItem {
    fn partial_cmp(&self, o: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(o))
    }
}

struct LaneState {
    permits: Arc<Semaphore>,
    queue: Mutex<BinaryHeap<QueueItem>>,
    notify: Notify,
    closed: AtomicBool,
    inflight: AtomicUsize,
}
impl LaneState {
    fn new(workers: usize) -> Self {
        LaneState {
            permits: Arc::new(Semaphore::new(workers)),
            queue: Mutex::new(BinaryHeap::new()),
            notify: Notify::new(),
            closed: AtomicBool::new(false),
            inflight: AtomicUsize::new(0),
        }
    }
    fn close(&self) {
        self.closed.store(true, Ordering::SeqCst);
        self.notify.notify_waiters();
    }
    fn is_closed(&self) -> bool {
        self.closed.load(Ordering::SeqCst)
    }
    fn busy(&self) -> bool {
        self.inflight.load(Ordering::SeqCst) > 0
    }
    fn push(&self, item: QueueItem) {
        self.queue.lock().unwrap().push(item);
        self.notify.notify_one();
    }
    fn pop(&self) -> Option<String> {
        self.queue.lock().unwrap().pop().map(|qi| qi.name)
    }
}

fn lane_ref<'a>(lane: Lane, light: &'a Arc<LaneState>, heavy: &'a Arc<LaneState>) -> &'a Arc<LaneState> {
    match lane {
        Lane::Light => light,
        Lane::Heavy => heavy,
    }
}

/// Add a job to its lane's ready queue, unless it is already active
/// (enqueued or running). Marks the job `pending` for visibility.
fn enqueue(
    handlers: &Arc<HashMap<String, Arc<JobRuntime>>>,
    active: &Arc<Mutex<HashSet<String>>>,
    light: &Arc<LaneState>,
    heavy: &Arc<LaneState>,
    name: &str,
    now: DateTime<Utc>,
) {
    let Some(rt) = handlers.get(name) else {
        return;
    };
    {
        let mut a = active.lock().unwrap();
        if a.contains(name) {
            return;
        }
        a.insert(name.to_string());
    }
    let lane = lane_ref(rt.lane, light, heavy);
    lane.push(QueueItem {
        key: ReadyKey {
            priority: rt.priority,
            due: now,
        },
        name: name.to_string(),
    });
}

fn spawn_lane_runner(
    lane: Arc<LaneState>,
    handlers: Arc<HashMap<String, Arc<JobRuntime>>>,
    active: Arc<Mutex<HashSet<String>>>,
    store: AdaptoStore,
    tz: Tz,
    now_fn: fn() -> DateTime<Utc>,
) {
    tokio::spawn(async move {
        loop {
            // Wait for the next item (poll fallback avoids lost-wakeups).
            let name = loop {
                if let Some(n) = lane.pop() {
                    break n;
                }
                if lane.is_closed() {
                    return;
                }
                tokio::select! {
                    _ = lane.notify.notified() => {}
                    _ = tokio::time::sleep(Duration::from_millis(100)) => {}
                }
            };

            let permit = match lane.permits.clone().acquire_owned().await {
                Ok(p) => p,
                Err(_) => return,
            };
            let Some(rt) = handlers.get(&name).cloned() else {
                active.lock().unwrap().remove(&name);
                drop(permit);
                continue;
            };

            lane.inflight.fetch_add(1, Ordering::SeqCst);
            let lane2 = lane.clone();
            let active2 = active.clone();
            let store2 = store.clone();
            tokio::spawn(async move {
                run_one(&rt, &store2, tz, now_fn).await;
                active2.lock().unwrap().remove(&rt.name);
                lane2.inflight.fetch_sub(1, Ordering::SeqCst);
                drop(permit);
            });
        }
    });
}

async fn run_one(rt: &JobRuntime, store: &AdaptoStore, tz: Tz, now_fn: fn() -> DateTime<Utc>) {
    let mut states = load_all(store);
    let mut st = match states.remove(&rt.name) {
        Some(s) => s,
        None => return,
    };
    let attempt = st.attempt + 1;
    let scheduled_for = st.next_run.unwrap_or_else(now_fn);
    st.status = RunStatus::Running;
    st.attempt = attempt;
    st.updated_at = now_fn();
    save(store, &st);

    let ctx = JobContext {
        store: store.clone(),
        attempt,
        scheduled_for,
    };
    let start = std::time::Instant::now();
    let handler = rt.handler.clone();
    // Isolate panics: run the handler on its own task.
    let jh = tokio::spawn(async move { (handler)(ctx).await });
    let outcome = match jh.await {
        Ok(Ok(())) => Outcome::Ok,
        Ok(Err(e)) => Outcome::Err(e.to_string()),
        Err(join) => Outcome::Err(if join.is_panic() {
            "handler panicked".into()
        } else {
            "handler cancelled".into()
        }),
    };
    let dur = start.elapsed().as_millis() as u64;

    let now = now_fn();
    let next_scheduled = rt
        .schedule
        .next_after(now.with_timezone(&tz))
        .unwrap_or(now + chrono::Duration::days(1));
    apply_outcome(&mut st, outcome, attempt, rt.max_attempts, dur, now, next_scheduled);
    save(store, &st);

    match st.status {
        RunStatus::Ok => tracing::info!("scheduler: job '{}' ok in {}ms", rt.name, dur),
        RunStatus::Failed => tracing::error!(
            "scheduler: job '{}' failed after {} attempts: {}",
            rt.name,
            attempt,
            st.last_error.as_deref().unwrap_or("")
        ),
        _ => tracing::warn!("scheduler: job '{}' retrying (attempt {})", rt.name, attempt),
    }
}

fn lane_str(l: Lane) -> &'static str {
    match l {
        Lane::Light => "light",
        Lane::Heavy => "heavy",
    }
}
fn priority_str(p: Priority) -> &'static str {
    match p {
        Priority::Low => "low",
        Priority::Normal => "normal",
        Priority::High => "high",
        Priority::Critical => "critical",
    }
}
fn schedule_desc(j: &Job) -> String {
    match &j.schedule {
        Schedule::Interval(d) => format!("Interval {}s", d.as_secs()),
        Schedule::DailyAt { hour, min } => format!("DailyAt {:02}:{:02}", hour, min),
        Schedule::WeeklyOn { weekday, hour, min } => {
            format!("WeeklyOn {:?} {:02}:{:02}", weekday, hour, min)
        }
        Schedule::MonthlyOn { day, hour, min } => format!("MonthlyOn {} {:02}:{:02}", day, hour, min),
        Schedule::Cron(e) => format!("Cron({e})"),
    }
}
