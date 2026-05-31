//! # adapto_events
//!
//! In-process, typed publish/subscribe message bus for the Adapto framework.
//!
//! - [`Event`] — a Rust message type with a stable `TOPIC` and optional
//!   `coalesce_key`.
//! - [`Subscription`] — targets one event type; `ephemeral` (at-most-once,
//!   in-memory) or `durable` (at-least-once, log + cursor + retry + dead-letter),
//!   with optional typed `filter` and throttle primitives.
//! - [`EventBus`] / [`EventBusHandle`] — build, [`EventBus::spawn`], and
//!   [`EventBusHandle::emit`] events.
//!
//! Durable delivery uses an append-only `_events` log plus a per-subscription
//! cursor, so a restart replays anything missed while down. The bus only routes;
//! heavy work (e.g. an HTTP scrape) is handed to `adapto_scheduler` by the
//! handler. See the design spec at
//! `docs/superpowers/specs/2026-05-31-event-bus-design.md`.
//!
//! ```no_run
//! use adapto_events::{Event, EventBus, Subscription};
//! use adapto_store::AdaptoStore;
//! use serde::{Deserialize, Serialize};
//! use std::time::Duration;
//!
//! #[derive(Serialize, Deserialize)]
//! struct PageViewed { path: String }
//! impl Event for PageViewed {
//!     const TOPIC: &'static str = "page.viewed";
//!     fn coalesce_key(&self) -> Option<String> { Some(self.path.clone()) }
//! }
//!
//! # async fn demo() {
//! let store = AdaptoStore::open(None).unwrap();
//! let bus = EventBus::new(store)
//!     // fire-and-forget stats counter
//!     .subscribe(
//!         Subscription::<PageViewed>::new("stats.pageviews")
//!             .ephemeral()
//!             .on(|_e| async { Ok(()) }),
//!     )
//!     // durable, throttled freshness check for weather pages only
//!     .subscribe(
//!         Subscription::<PageViewed>::new("scraper.freshness")
//!             .durable()
//!             .filter(|e| e.path.starts_with("/weather"))
//!             .coalesce_by_key(Duration::from_secs(3600))
//!             .on(|_e| async { Ok(()) }),
//!     );
//!
//! let handle = bus.spawn();
//! handle.emit(PageViewed { path: "/weather/almaty".into() });
//! handle.shutdown().await;
//! # }
//! ```

pub mod admin;
mod bus;
mod dispatch;
pub mod event;
mod state;
mod subscription;
mod throttle;

pub use admin::SubscriptionStatus;
pub use bus::{EventBus, EventBusHandle};
pub use event::{Event, EventEnvelope, EventError};
pub use subscription::{Mode, Subscription};
