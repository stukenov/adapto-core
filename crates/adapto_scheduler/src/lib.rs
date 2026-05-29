//! # adapto_scheduler
//!
//! A persistent, priority + lane background-job scheduler for the Adapto
//! framework.
//!
//! - [`Schedule`] — when a job runs (`Interval`, `DailyAt`, `WeeklyOn`,
//!   `MonthlyOn`, `Cron`), timezone-aware.
//! - [`Job`] — a named unit of work with a lane, priority, catch-up flag, and
//!   retry budget.
//! - [`Scheduler`] — builds and [`Scheduler::spawn`]s the loops; state persists
//!   in the `_jobs` collection so missed runs catch up after a restart.
//! - [`Reloadable`] — a lock-free swap cache so job-updated data appears
//!   without restarting the process.
//!
//! ```no_run
//! use adapto_scheduler::{Job, Lane, Priority, Schedule, Scheduler};
//! use adapto_store::AdaptoStore;
//!
//! # async fn demo() {
//! let store = AdaptoStore::open(None).unwrap();
//! let handle = Scheduler::new(store)
//!     .timezone(chrono_tz::Asia::Almaty)
//!     .job(
//!         Job::new("currency")
//!             .schedule(Schedule::DailyAt { hour: 6, min: 30 })
//!             .lane(Lane::Light)
//!             .priority(Priority::High)
//!             .catch_up(true)
//!             .run(|_ctx| async { Ok(()) }),
//!     )
//!     .spawn();
//! handle.shutdown().await;
//! # }
//! ```

pub mod admin;
mod engine;
mod job;
mod reload;
mod schedule;
mod scheduler;
mod state;

pub use job::{Job, JobBuilder, JobContext, JobError, Lane, Priority};
pub use reload::Reloadable;
pub use schedule::Schedule;
pub use scheduler::{Scheduler, SchedulerHandle, TriggerError};
pub use state::JobStatus;
