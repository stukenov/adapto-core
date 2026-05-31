//! adapto_events demo: an event published from a page request fans out to an
//! ephemeral stats counter and a durable, throttled "freshness scraper" that
//! hands work to a queue collection (the bus → scheduler handoff pattern).
//!
//! Run: `cargo run -p example-events-demo` then visit:
//!   GET /p/weather/almaty   — publish a PageViewed, fan out to subscribers
//!   GET /stats              — live ephemeral page-view counter
//!   GET /admin/events       — durable subscription cursors / lag / dead-letter
//!   GET /queue              — tasks the durable scraper enqueued

use adapto_app::{App, PageResponse};
use adapto_events::{Event, EventBus, Subscription};
use adapto_store::{AdaptoStore, Query};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[derive(Serialize, Deserialize)]
struct PageViewed {
    path: String,
}
impl Event for PageViewed {
    const TOPIC: &'static str = "page.viewed";
    fn coalesce_key(&self) -> Option<String> {
        Some(self.path.clone())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let store = AdaptoStore::open(None)?;
    let views = Arc::new(AtomicU64::new(0));

    // Ephemeral: fire-and-forget page-view counter.
    let views_sub = views.clone();
    let stats = Subscription::<PageViewed>::new("stats.pageviews").ephemeral().on(move |_e| {
        let v = views_sub.clone();
        async move {
            v.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    });

    // Durable: only weather pages, at most one re-check per page per hour;
    // hands the heavy work off by enqueueing a task (here, a store collection).
    let store_scraper = store.clone();
    let scraper = Subscription::<PageViewed>::new("scraper.freshness")
        .durable()
        .filter(|e| e.path.starts_with("/p/weather"))
        .coalesce_by_key(Duration::from_secs(3600))
        .on(move |e| {
            let store = store_scraper.clone();
            async move {
                store
                    .collection("scrape_tasks")
                    .insert(serde_json::json!({ "url": e.path, "reason": "freshness" }))
                    .map_err(adapto_events::EventError::from_err)?;
                Ok(())
            }
        });

    let bus = EventBus::new(store.clone()).subscribe(stats).subscribe(scraper);

    let views_page = views.clone();
    let store_queue = store.clone();

    App::new("Events Demo")
        .port(8099)
        .store(store)
        .events(bus)
        .events_admin("/admin/events", |_ctx| true) // demo: always authorized
        .page("/p/*path", |ctx| {
            let path = ctx.path().to_string();
            if let Some(h) = ctx.events() {
                h.emit(PageViewed { path: path.clone() });
            }
            PageResponse::Ok(format!("<h1>{path}</h1><p>view recorded</p>"))
        })
        .page("/stats", move |_ctx| {
            PageResponse::Ok(format!(
                "<h1>Page views: {}</h1>",
                views_page.load(Ordering::SeqCst)
            ))
        })
        .page("/queue", move |_ctx| {
            let n = store_queue.collection("scrape_tasks").count_all();
            let rows: Vec<String> = store_queue
                .collection("scrape_tasks")
                .find(Query::new())
                .filter_map(|d| d.data.get("url").and_then(|v| v.as_str()).map(String::from))
                .map(|u| format!("<li>{u}</li>"))
                .collect();
            PageResponse::Ok(format!("<h1>Scrape queue ({n})</h1><ul>{}</ul>", rows.join("")))
        })
        .run()
        .await
}
