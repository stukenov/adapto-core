//! # adapto_events
//!
//! In-process, typed publish/subscribe message bus for the Adapto framework.
//!
//! See the design spec at `docs/superpowers/specs/2026-05-31-event-bus-design.md`.

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
