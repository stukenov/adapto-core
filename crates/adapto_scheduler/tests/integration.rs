use adapto_scheduler::{Job, Lane, Priority, Schedule, Scheduler, TriggerError};
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
                .catch_up(true)
                .run(move |_ctx| {
                    let c = c2.clone();
                    async move {
                        c.fetch_add(1, Ordering::SeqCst);
                        Ok(())
                    }
                }),
        );
    let handle = sched.spawn();
    tokio::time::sleep(Duration::from_millis(300)).await;
    handle.shutdown().await;
    assert!(
        counter.load(Ordering::SeqCst) >= 3,
        "expected several runs, got {}",
        counter.load(Ordering::SeqCst)
    );

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
                .schedule(Schedule::Interval(Duration::from_secs(3600))) // far future; trigger manually
                .max_attempts(1) // fail immediately, no long backoff in test
                .run(|_ctx| async { Err(adapto_scheduler::JobError::msg("always fails")) }),
        );
    let handle = sched.spawn();
    tokio::time::sleep(Duration::from_millis(50)).await;
    handle.trigger("boom").await.unwrap();
    tokio::time::sleep(Duration::from_millis(150)).await;
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
    let sched = Scheduler::new(store.clone()).job(
        Job::new("manual")
            .schedule(Schedule::Interval(Duration::from_secs(3600))) // far future
            .run(move |_| {
                let c = c2.clone();
                async move {
                    c.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                }
            }),
    );
    let handle = sched.spawn();
    tokio::time::sleep(Duration::from_millis(50)).await;
    handle.trigger("manual").await.unwrap();
    tokio::time::sleep(Duration::from_millis(150)).await;
    handle.shutdown().await;
    assert_eq!(counter.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn trigger_unknown_job_errs() {
    let store = AdaptoStore::open(None).unwrap();
    let sched = Scheduler::new(store.clone()).job(
        Job::new("a")
            .schedule(Schedule::Interval(Duration::from_secs(3600)))
            .run(|_| async { Ok(()) }),
    );
    let handle = sched.spawn();
    let err = handle.trigger("nope").await.unwrap_err();
    assert!(matches!(err, TriggerError::NoSuchJob(_)));
    handle.shutdown().await;
}

#[tokio::test]
async fn catch_up_false_does_not_run_at_boot() {
    let store = AdaptoStore::open(None).unwrap();
    let counter = Arc::new(AtomicU32::new(0));
    let c2 = counter.clone();
    let sched = Scheduler::new(store.clone()).job(
        Job::new("lazy")
            .schedule(Schedule::Interval(Duration::from_secs(3600)))
            .catch_up(false)
            .run(move |_| {
                let c = c2.clone();
                async move {
                    c.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                }
            }),
    );
    let handle = sched.spawn();
    tokio::time::sleep(Duration::from_millis(150)).await;
    handle.shutdown().await;
    assert_eq!(counter.load(Ordering::SeqCst), 0);
}
