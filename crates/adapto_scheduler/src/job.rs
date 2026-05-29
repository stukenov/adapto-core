use crate::schedule::Schedule;
use adapto_store::AdaptoStore;
use chrono::{DateTime, Utc};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Relative priority of a job within its lane. Higher runs first when several are ready.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Low,
    Normal,
    High,
    Critical,
}

/// Execution lane. `Light` jobs share a worker pool; `Heavy` jobs run one at a time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lane {
    Light,
    Heavy,
}

/// Error returned by a job handler.
#[derive(Debug, thiserror::Error)]
pub enum JobError {
    #[error("{0}")]
    Message(String),
}

impl JobError {
    /// Construct from any message.
    pub fn msg(s: impl Into<String>) -> Self {
        JobError::Message(s.into())
    }
    /// Wrap any error's `Display` into a `JobError`
    /// (use in job bodies: `.map_err(JobError::from_err)`).
    pub fn from_err<E: std::fmt::Display>(e: E) -> Self {
        JobError::Message(e.to_string())
    }
}

/// Context handed to a job handler on each run.
pub struct JobContext {
    /// A clone of the application store.
    pub store: AdaptoStore,
    /// 1-based attempt number for this run (increments on retry).
    pub attempt: u32,
    /// The scheduled fire time this run is satisfying.
    pub scheduled_for: DateTime<Utc>,
}

pub type JobFuture = Pin<Box<dyn Future<Output = Result<(), JobError>> + Send>>;
pub type JobHandler = Arc<dyn Fn(JobContext) -> JobFuture + Send + Sync>;

/// A fully-built job definition.
pub struct Job {
    pub(crate) name: String,
    pub(crate) schedule: Schedule,
    pub(crate) lane: Lane,
    pub(crate) priority: Priority,
    pub(crate) catch_up: bool,
    pub(crate) max_attempts: u32,
    pub(crate) handler: JobHandler,
}

/// Builder for a [`Job`]. Created via [`Job::new`].
pub struct JobBuilder {
    name: String,
    schedule: Option<Schedule>,
    lane: Lane,
    priority: Priority,
    catch_up: bool,
    max_attempts: u32,
}

impl Job {
    /// Start building a job with the given unique name.
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
    /// The job's unique name.
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl JobBuilder {
    pub fn schedule(mut self, s: Schedule) -> Self {
        self.schedule = Some(s);
        self
    }
    pub fn lane(mut self, l: Lane) -> Self {
        self.lane = l;
        self
    }
    pub fn priority(mut self, p: Priority) -> Self {
        self.priority = p;
        self
    }
    pub fn catch_up(mut self, v: bool) -> Self {
        self.catch_up = v;
        self
    }
    pub fn max_attempts(mut self, n: u32) -> Self {
        self.max_attempts = n.max(1);
        self
    }

    /// Finish the job with an async handler.
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
