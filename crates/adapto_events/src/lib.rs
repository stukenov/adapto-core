//! # adapto_events
//!
//! In-process, typed publish/subscribe message bus for the Adapto framework.
//!
//! See the design spec at `docs/superpowers/specs/2026-05-31-event-bus-design.md`.

pub mod event;
mod state;
mod throttle;

pub use event::{Event, EventEnvelope, EventError};
