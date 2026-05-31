//! Admin view: a serializable per-subscription status and a pure data → HTML
//! renderer (no web-framework types), mirroring the scheduler's admin module.

use crate::bus::EventBusHandle;
use crate::state;
use crate::subscription::Mode;
use serde::Serialize;

/// A serializable read view of one subscription's delivery state.
#[derive(Clone, Debug, Serialize)]
pub struct SubscriptionStatus {
    pub id: String,
    pub topic: String,
    pub mode: String,
    /// Last processed seq (durable only; 0 for ephemeral).
    pub last_seq: u64,
    /// Events behind the current high-water mark (durable only).
    pub lag: u64,
    pub processed: u64,
    pub failed: u64,
    pub deadletter: u64,
}

impl EventBusHandle {
    /// Per-subscription status for an admin page, sorted by subscription id.
    pub fn statuses(&self) -> Vec<SubscriptionStatus> {
        let current = self.current_seq();
        let mut out: Vec<SubscriptionStatus> = self
            .inner
            .subscribers
            .iter()
            .map(|s| match s.mode {
                Mode::Durable => {
                    let cur = state::load_cursor(&self.inner.store, &s.id);
                    let last_seq = cur.as_ref().map(|c| c.last_seq).unwrap_or(0);
                    SubscriptionStatus {
                        id: s.id.clone(),
                        topic: s.topic.to_string(),
                        mode: "durable".into(),
                        last_seq,
                        lag: current.saturating_sub(last_seq),
                        processed: cur.as_ref().map(|c| c.processed).unwrap_or(0),
                        failed: cur.as_ref().map(|c| c.failed).unwrap_or(0),
                        deadletter: state::deadletter_count(&self.inner.store, &s.id),
                    }
                }
                Mode::Ephemeral => SubscriptionStatus {
                    id: s.id.clone(),
                    topic: s.topic.to_string(),
                    mode: "ephemeral".into(),
                    last_seq: 0,
                    lag: 0,
                    processed: 0,
                    failed: 0,
                    deadletter: 0,
                },
            })
            .collect();
        out.sort_by(|a, b| a.id.cmp(&b.id));
        out
    }
}

/// Render the subscriptions table as a standalone HTML fragment.
pub fn render(statuses: &[SubscriptionStatus]) -> String {
    let mut rows = String::new();
    for s in statuses {
        let dl = if s.deadletter > 0 {
            format!("<span class=\"dl\">{}</span>", s.deadletter)
        } else {
            "0".to_string()
        };
        rows.push_str(&format!(
            "<tr class=\"m-{mode}\">\
             <td>{id}</td><td>{topic}</td>\
             <td><span class=\"badge\">{mode}</span></td>\
             <td>{last_seq}</td><td>{lag}</td><td>{processed}</td><td>{failed}</td><td>{dl}</td>\
             </tr>",
            mode = esc(&s.mode),
            id = esc(&s.id),
            topic = esc(&s.topic),
            last_seq = s.last_seq,
            lag = s.lag,
            processed = s.processed,
            failed = s.failed,
            dl = dl,
        ));
    }
    format!(
        "<h1>Event Subscriptions</h1>\
         <table class=\"subs\"><thead><tr>\
         <th>Subscription</th><th>Topic</th><th>Mode</th><th>Cursor</th>\
         <th>Lag</th><th>Processed</th><th>Failed</th><th>Dead-letter</th>\
         </tr></thead><tbody>{rows}</tbody></table>\
         <style>.subs{{width:100%;border-collapse:collapse;font:14px -apple-system,sans-serif}}\
         .subs th,.subs td{{text-align:left;padding:8px 10px;border-bottom:1px solid #eee}}\
         .subs th{{font-size:12px;color:#888;text-transform:uppercase;letter-spacing:.3px}}\
         .badge{{font-size:12px;padding:2px 8px;border-radius:10px;background:#eee}}\
         .m-durable .badge{{background:#e8f0fe;color:#1a73e8}}\
         .m-ephemeral .badge{{background:#f1f3f4;color:#5f6368}}\
         .dl{{color:#c5221f;font-weight:600}}</style>"
    )
}

fn esc(s: &str) -> String {
    adapto_ui::html_escape(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::Event;
    use crate::subscription::Subscription;
    use crate::EventBus;
    use adapto_store::AdaptoStore;
    use serde::{Deserialize, Serialize};
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

    #[tokio::test]
    async fn statuses_lists_subscriptions_sorted() {
        let store = AdaptoStore::open(None).unwrap();
        let h = EventBus::new(store.clone())
            .retry_backoff(Duration::from_millis(1))
            .subscribe(Subscription::<PageViewed>::new("a.durable").durable().on(|_| async { Ok(()) }))
            .subscribe(Subscription::<PageViewed>::new("b.ephemeral").ephemeral().on(|_| async { Ok(()) }))
            .spawn();
        h.publish(PageViewed { path: "/a".into() }).await;
        for _ in 0..100 {
            if crate::state::load_cursor(&store, "a.durable").map(|c| c.last_seq).unwrap_or(0) >= 1 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        let st = h.statuses();
        assert_eq!(st.len(), 2);
        assert_eq!(st[0].id, "a.durable");
        assert_eq!(st[1].id, "b.ephemeral");
        assert_eq!(st[0].mode, "durable");
        assert!(st[0].last_seq >= 1);
        assert_eq!(st[0].processed, 1);
    }

    #[test]
    fn render_contains_ids_and_topic() {
        let statuses = vec![SubscriptionStatus {
            id: "scraper.freshness".into(),
            topic: "page.viewed".into(),
            mode: "durable".into(),
            last_seq: 5,
            lag: 0,
            processed: 5,
            failed: 0,
            deadletter: 0,
        }];
        let html = render(&statuses);
        assert!(html.contains("scraper.freshness"));
        assert!(html.contains("page.viewed"));
    }
}
