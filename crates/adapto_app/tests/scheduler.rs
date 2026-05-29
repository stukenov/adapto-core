use adapto_app::App;
use adapto_scheduler::{Job, Schedule, Scheduler};
use adapto_store::AdaptoStore;
use std::time::Duration;

fn far_future_job(name: &str) -> Job {
    Job::new(name)
        .schedule(Schedule::Interval(Duration::from_secs(3600)))
        .run(|_| async { Ok(()) })
}

#[tokio::test]
async fn app_with_scheduler_builds_and_serves() {
    let store = AdaptoStore::open(None).unwrap();
    let sched = Scheduler::new(store.clone()).job(far_future_job("noop"));
    let app = App::new("t").store(store).scheduler(sched).page("/", |_| "hi");

    let client = app.test_client();
    let resp = client.get("/").await;
    assert_eq!(resp.status(), 200);
    assert!(resp.text().contains("hi"));
}

#[tokio::test]
async fn admin_page_forbidden_without_auth() {
    let store = AdaptoStore::open(None).unwrap();
    let sched = Scheduler::new(store.clone()).job(far_future_job("noop"));
    let app = App::new("t")
        .store(store)
        .scheduler(sched)
        .scheduler_admin("/admin/jobs", |_ctx| false); // deny everyone

    let client = app.test_client();
    let resp = client.get("/admin/jobs").await;
    assert_eq!(resp.status(), 403);
}

#[tokio::test]
async fn admin_page_ok_with_auth_lists_jobs() {
    let store = AdaptoStore::open(None).unwrap();
    let sched = Scheduler::new(store.clone()).job(far_future_job("currency"));
    let app = App::new("t")
        .store(store)
        .scheduler(sched)
        .scheduler_admin("/admin/jobs", |_ctx| true);

    let client = app.test_client();
    let resp = client.get("/admin/jobs").await;
    assert_eq!(resp.status(), 200);
    let body = resp.text();
    assert!(body.contains("currency"), "admin page should list the job: {body}");
    assert!(body.contains("Run now"));
}

#[tokio::test]
async fn admin_trigger_forbidden_without_auth() {
    let store = AdaptoStore::open(None).unwrap();
    let sched = Scheduler::new(store.clone()).job(far_future_job("currency"));
    let app = App::new("t")
        .store(store)
        .scheduler(sched)
        .scheduler_admin("/admin/jobs", |_ctx| false);

    let client = app.test_client();
    let resp = client.post("/admin/jobs/currency/run", "").await;
    assert_eq!(resp.status(), 403);
}
