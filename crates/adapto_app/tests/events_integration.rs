//! Integration test: an event published from a request handler reaches an
//! ephemeral subscriber attached to the app via `App::events`.

use adapto_app::{App, PageResponse};
use adapto_events::{Event, EventBus, Subscription};
use adapto_store::AdaptoStore;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[derive(Serialize, Deserialize)]
struct Ping {
    from: String,
}
impl Event for Ping {
    const TOPIC: &'static str = "ping";
}

#[tokio::test]
async fn publish_from_handler_reaches_subscriber() {
    let store = AdaptoStore::open(None).unwrap();
    let count = Arc::new(AtomicUsize::new(0));
    let c2 = count.clone();

    let bus = EventBus::new(store.clone()).subscribe(
        Subscription::<Ping>::new("counter").ephemeral().on(move |_e| {
            let c = c2.clone();
            async move {
                c.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
        }),
    );

    let app = App::new("Events Test").store(store).events(bus).page("/ping", |ctx| {
        if let Some(h) = ctx.events() {
            h.emit(Ping { from: "test".into() });
        }
        PageResponse::Ok("ok".to_string())
    });

    let client = app.test_client();
    let resp = client.get("/ping").await;
    assert_eq!(resp.status(), 200);

    // Dispatch is async; give the ephemeral subscriber a moment.
    tokio::time::sleep(Duration::from_millis(60)).await;
    assert_eq!(count.load(Ordering::SeqCst), 1);
}
