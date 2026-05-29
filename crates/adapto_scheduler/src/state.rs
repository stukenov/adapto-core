use adapto_store::{AdaptoStore, Query};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Name of the system collection holding per-job state.
pub const JOBS_COLLECTION: &str = "_jobs";

/// Lifecycle status of a job's most recent (or current) run.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RunStatus {
    Pending,
    Running,
    Ok,
    Failed,
    Retrying,
}

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

/// Persisted per-job document (the body stored in the `_jobs` collection).
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

/// Public, serializable read view of a job's state for the admin page.
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

/// Insert-or-replace a job state by its unique `job` key.
///
/// The store's `Update::Set` cannot replace a whole document, and the `_jobs`
/// collection is tiny (one doc per job), so this deletes any existing doc for
/// the job and inserts a fresh one. The `job` field stays the stable key.
pub fn save(store: &AdaptoStore, st: &JobState) {
    let col = store.collection(JOBS_COLLECTION);
    let value = match serde_json::to_value(st) {
        Ok(v) => v,
        Err(e) => {
            tracing::error!("scheduler: serialize state for '{}' failed: {e}", st.job);
            return;
        }
    };
    let _ = col.delete(Query::eq("job", st.job.clone()));
    if let Err(e) = col.insert(value) {
        tracing::error!("scheduler: persist state for '{}' failed: {e}", st.job);
    }
}

/// Ensure the unique index on `job` exists. Idempotent.
pub fn ensure_index(store: &AdaptoStore) {
    let _ = store.collection(JOBS_COLLECTION).create_index("job", true);
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

    #[test]
    fn save_then_load_from_store() {
        let store = AdaptoStore::open(None).unwrap();
        ensure_index(&store);
        let now = Utc::now();
        let mut st = JobState::seed("currency", "light", "high", "DailyAt", now);
        save(&store, &st);
        st.status = RunStatus::Ok;
        st.total_runs = 1;
        save(&store, &st);

        let loaded = load_all(&store);
        assert_eq!(loaded.len(), 1);
        let got = &loaded["currency"];
        assert_eq!(got.status, RunStatus::Ok);
        assert_eq!(got.total_runs, 1);
    }
}
