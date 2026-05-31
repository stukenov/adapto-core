//! Persistence for the durable side of the bus: the append-only `_events` log,
//! per-subscription cursors, the dead-letter table, and the monotonic seq
//! high-water counter. All collections are system collections (underscore
//! prefix), mirroring the scheduler's `_jobs`.

use adapto_store::{AdaptoStore, Filter, Query};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Append-only event log.
pub const EVENTS: &str = "_events";
/// Per-durable-subscription delivery cursors.
pub const CURSORS: &str = "_event_cursors";
/// Poison events that exhausted their retry budget.
pub const DEADLETTER: &str = "_event_deadletter";
/// Monotonic seq high-water counter (single doc, `name = "global"`).
pub const SEQ: &str = "_event_seq";

/// One logged event (the body stored in `_events`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EventRow {
    pub seq: u64,
    pub topic: String,
    pub payload: Value,
    pub ts: DateTime<Utc>,
}

/// A durable subscription's delivery cursor (the body stored in `_event_cursors`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Cursor {
    pub subscription: String,
    pub topic: String,
    pub last_seq: u64,
    pub processed: u64,
    pub failed: u64,
    pub updated_at: DateTime<Utc>,
}

/// A poison event written after the retry budget is exhausted (skip-after-N).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeadLetter {
    pub subscription: String,
    pub seq: u64,
    pub topic: String,
    pub payload: Value,
    pub attempts: u32,
    pub last_error: String,
    pub failed_at: DateTime<Utc>,
}

/// Ensure every system index exists. Idempotent.
pub fn ensure_indexes(store: &AdaptoStore) {
    let _ = store.collection(EVENTS).create_index("seq", true);
    let _ = store.collection(EVENTS).create_index("topic", false);
    let _ = store.collection(CURSORS).create_index("subscription", true);
    let _ = store.collection(DEADLETTER).create_index("subscription", false);
    let _ = store.collection(SEQ).create_index("name", true);
}

/// Append one event to the log.
pub fn append_event(store: &AdaptoStore, row: &EventRow) {
    match serde_json::to_value(row) {
        Ok(v) => {
            if let Err(e) = store.collection(EVENTS).insert(v) {
                tracing::error!("events: append seq {} failed: {e}", row.seq);
            }
        }
        Err(e) => tracing::error!("events: serialize event seq {} failed: {e}", row.seq),
    }
}

/// Load all events for `topic` with `seq > after_seq`, ordered by `seq` ascending.
pub fn load_events_after(store: &AdaptoStore, topic: &str, after_seq: u64) -> Vec<EventRow> {
    let q = Query::filter(Filter::And(vec![
        Filter::Eq("topic".into(), Value::from(topic)),
        Filter::Gt("seq".into(), serde_json::json!(after_seq)),
    ]));
    let mut rows: Vec<EventRow> = store
        .collection(EVENTS)
        .find(q)
        .into_iter()
        .filter_map(|d| serde_json::from_value(d.data).ok())
        .collect();
    rows.sort_by_key(|r: &EventRow| r.seq);
    rows
}

/// Load a subscription's cursor, if any.
pub fn load_cursor(store: &AdaptoStore, subscription: &str) -> Option<Cursor> {
    store
        .collection(CURSORS)
        .find_one(Query::eq("subscription", subscription))
        .ok()
        .flatten()
        .and_then(|d| serde_json::from_value(d.data).ok())
}

/// Insert-or-replace a cursor keyed by `subscription`.
pub fn save_cursor(store: &AdaptoStore, c: &Cursor) {
    let col = store.collection(CURSORS);
    let v = match serde_json::to_value(c) {
        Ok(v) => v,
        Err(e) => {
            tracing::error!("events: serialize cursor '{}' failed: {e}", c.subscription);
            return;
        }
    };
    let _ = col.delete(Query::eq("subscription", c.subscription.clone()));
    if let Err(e) = col.insert(v) {
        tracing::error!("events: persist cursor '{}' failed: {e}", c.subscription);
    }
}

/// Write a poison event to the dead-letter table.
pub fn write_deadletter(store: &AdaptoStore, d: &DeadLetter) {
    match serde_json::to_value(d) {
        Ok(v) => {
            if let Err(e) = store.collection(DEADLETTER).insert(v) {
                tracing::error!("events: dead-letter for '{}' seq {} failed: {e}", d.subscription, d.seq);
            }
        }
        Err(e) => tracing::error!("events: serialize dead-letter seq {} failed: {e}", d.seq),
    }
}

/// The current seq high-water mark = max(persisted counter, max seq in `_events`).
///
/// Taking the max of both survives log GC: once trimmed rows are gone, the
/// persisted counter still holds the high-water mark.
pub fn load_seq_highwater(store: &AdaptoStore) -> u64 {
    let counter = store
        .collection(SEQ)
        .find_one(Query::eq("name", "global"))
        .ok()
        .flatten()
        .and_then(|d| d.data.get("value").and_then(Value::as_u64))
        .unwrap_or(0);
    let max_event = store
        .collection(EVENTS)
        .find(Query::new())
        .into_iter()
        .filter_map(|d| d.data.get("seq").and_then(Value::as_u64))
        .max()
        .unwrap_or(0);
    counter.max(max_event)
}

/// Persist the seq high-water mark.
pub fn save_seq_highwater(store: &AdaptoStore, value: u64) {
    let col = store.collection(SEQ);
    let v = serde_json::json!({ "name": "global", "value": value });
    let _ = col.delete(Query::eq("name", "global"));
    if let Err(e) = col.insert(v) {
        tracing::error!("events: persist seq high-water failed: {e}");
    }
}

/// Delete every `_events` row for `topic` with `seq <= min_seq` (consumed by all
/// durable subscribers of that topic).
pub fn gc_events(store: &AdaptoStore, topic: &str, min_seq: u64) {
    let q = Query::filter(Filter::And(vec![
        Filter::Eq("topic".into(), Value::from(topic)),
        Filter::Lte("seq".into(), serde_json::json!(min_seq)),
    ]));
    let _ = store.collection(EVENTS).delete(q);
}

/// Delete dead-letter rows older than `before`.
pub fn prune_deadletter(store: &AdaptoStore, before: DateTime<Utc>) {
    let q = Query::filter(Filter::Lt("failed_at".into(), serde_json::json!(before)));
    let _ = store.collection(DEADLETTER).delete(q);
}

/// The minimum `last_seq` across the given subscriptions (a missing cursor
/// counts as 0). `None` if `subs` is empty.
pub fn min_cursor_seq(store: &AdaptoStore, subs: &[String]) -> Option<u64> {
    let mut min: Option<u64> = None;
    for s in subs {
        let last = load_cursor(store, s).map(|c| c.last_seq).unwrap_or(0);
        min = Some(min.map_or(last, |m| m.min(last)));
    }
    min
}

#[cfg(test)]
mod tests {
    use super::*;
    use adapto_store::AdaptoStore;
    use chrono::Utc;
    use serde_json::json;

    fn row(seq: u64, topic: &str) -> EventRow {
        EventRow {
            seq,
            topic: topic.into(),
            payload: json!({ "n": seq }),
            ts: Utc::now(),
        }
    }

    #[test]
    fn append_then_load_after() {
        let store = AdaptoStore::open(None).unwrap();
        ensure_indexes(&store);
        for seq in [1u64, 2, 3] {
            append_event(&store, &row(seq, "t"));
        }
        let seqs: Vec<u64> = load_events_after(&store, "t", 1).iter().map(|r| r.seq).collect();
        assert_eq!(seqs, vec![2, 3]);
    }

    #[test]
    fn load_after_ignores_other_topics() {
        let store = AdaptoStore::open(None).unwrap();
        ensure_indexes(&store);
        append_event(&store, &row(1, "a"));
        append_event(&store, &row(2, "b"));
        let rows = load_events_after(&store, "a", 0);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].seq, 1);
    }

    #[test]
    fn cursor_roundtrip() {
        let store = AdaptoStore::open(None).unwrap();
        ensure_indexes(&store);
        let mut c = Cursor {
            subscription: "s".into(),
            topic: "t".into(),
            last_seq: 5,
            processed: 1,
            failed: 0,
            updated_at: Utc::now(),
        };
        save_cursor(&store, &c);
        assert_eq!(load_cursor(&store, "s").unwrap().last_seq, 5);
        c.last_seq = 7;
        save_cursor(&store, &c);
        assert_eq!(load_cursor(&store, "s").unwrap().last_seq, 7);
        assert_eq!(store.collection(CURSORS).count_all(), 1);
    }

    #[test]
    fn seq_highwater() {
        let store = AdaptoStore::open(None).unwrap();
        ensure_indexes(&store);
        append_event(&store, &row(9, "t"));
        assert!(load_seq_highwater(&store) >= 9);
        save_seq_highwater(&store, 20);
        assert_eq!(load_seq_highwater(&store), 20);
    }

    #[test]
    fn deadletter_write() {
        let store = AdaptoStore::open(None).unwrap();
        ensure_indexes(&store);
        write_deadletter(
            &store,
            &DeadLetter {
                subscription: "s".into(),
                seq: 1,
                topic: "t".into(),
                payload: json!({}),
                attempts: 3,
                last_error: "boom".into(),
                failed_at: Utc::now(),
            },
        );
        assert_eq!(store.collection(DEADLETTER).count_all(), 1);
    }

    #[test]
    fn gc_removes_consumed() {
        let store = AdaptoStore::open(None).unwrap();
        ensure_indexes(&store);
        for seq in 1u64..=5 {
            append_event(&store, &row(seq, "t"));
        }
        gc_events(&store, "t", 3);
        let seqs: Vec<u64> = load_events_after(&store, "t", 0).iter().map(|r| r.seq).collect();
        assert_eq!(seqs, vec![4, 5]);
    }

    #[test]
    fn prune_deadletter_removes_old_keeps_recent() {
        let store = AdaptoStore::open(None).unwrap();
        ensure_indexes(&store);
        let now = Utc::now();
        let dl = |seq: u64, at: chrono::DateTime<Utc>| DeadLetter {
            subscription: "s".into(),
            seq,
            topic: "t".into(),
            payload: json!({}),
            attempts: 3,
            last_error: "boom".into(),
            failed_at: at,
        };
        write_deadletter(&store, &dl(1, now - chrono::Duration::hours(2)));
        write_deadletter(&store, &dl(2, now));
        prune_deadletter(&store, now - chrono::Duration::hours(1));
        assert_eq!(store.collection(DEADLETTER).count_all(), 1);
    }

    #[test]
    fn min_cursor_seq_takes_minimum() {
        let store = AdaptoStore::open(None).unwrap();
        ensure_indexes(&store);
        save_cursor(
            &store,
            &Cursor { subscription: "a".into(), topic: "t".into(), last_seq: 5, processed: 0, failed: 0, updated_at: Utc::now() },
        );
        save_cursor(
            &store,
            &Cursor { subscription: "b".into(), topic: "t".into(), last_seq: 3, processed: 0, failed: 0, updated_at: Utc::now() },
        );
        assert_eq!(min_cursor_seq(&store, &["a".to_string(), "b".to_string()]), Some(3));
    }
}
