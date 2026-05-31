# adapto_events — Message Bus Design

**Date:** 2026-05-31
**Status:** Approved (brainstorming complete)
**Crate:** `adapto_events` (new, WIP)

## Problem

Adapto needs an in-process publish/subscribe message bus so that parts of the
system can react to things that happen elsewhere without tight coupling. Two
concrete consumers motivate it:

1. **Stats collection** — fire-and-forget counters off events like a page view.
2. **Background freshness scraping** — after a page is served, *some* pages
   should be re-verified against their source API to confirm the rendered data
   is still current. The system must **not** scrape everything; only a throttled,
   targeted subset. The scraper subscribes to page-view events, decides what is
   worth re-checking, and hands the heavy work to the existing scheduler.

This spec covers **only the bus** (`adapto_events`). The stats consumer and the
freshness-scraper consumer are separate specs built on top of it.

## Decisions (from brainstorming)

| # | Decision | Choice |
|---|----------|--------|
| 1 | Scope | Build the generic bus first; consumers are later specs. |
| 2 | Delivery guarantees | **Hybrid, per-subscription**: each subscription is `durable` (at-least-once) or `ephemeral` (at-most-once). |
| 3 | Message matching | **Typed Rust events**: each event is a Rust type implementing `Event` (serde + a stable `TOPIC`); a subscription targets one type with an optional typed predicate filter. |
| 4 | Throttling | **Both layers**: the bus offers generic primitives (coalesce-by-key, rate-limit, sample); domain logic (page stale/popular) lives in the consumer's handler. |
| 5 | Heavy-work execution | **Bus routes, scheduler executes**: light handlers run inline in the bus; heavy work is handed off to `adapto_scheduler`. The bus does **not** depend on the scheduler crate. |
| 6 | Durability implementation | **Model C — log + cursors**: live delivery via `tokio::broadcast`; durability via an append-only `_events` log (a store collection, reusing adapto_store's own WAL) plus a per-subscriber cursor (`last_seq`). |
| 7 | Poison handling | **Skip-after-N → dead-letter**: after `max_attempts` with backoff, the event is written to `_event_deadletter`, the cursor advances, and the failure is visible in the admin view. |

## Architecture Overview

```
publish(e: E)                         ┌─────────────────────────────┐
   │  seq = atomic++                  │  per-subscription dispatcher │
   ├─ if topic has durable subs:      │  (one tokio task each)       │
   │     insert _events {seq,topic,…} │                              │
   └─ broadcast {seq,topic,payload} ──┤  ephemeral: payload from     │
                                      │    broadcast → filter →      │
   _events (append-only log)          │    throttle → handler        │
   _event_cursors (last_seq/sub)      │  durable: broadcast = "wake, │
   _event_deadletter (poison)         │    drain log from cursor" →  │
   _event_seq (high-water)            │    filter → throttle →       │
                                      │    handler(retry) → advance  │
                                      │    cursor | dead-letter      │
                                      └─────────────────────────────┘
```

The broadcast channel carries only a small envelope `{seq, topic, payload}`.
Ephemeral subscribers use the payload directly. Durable subscribers treat the
broadcast purely as a **wake signal** and drain the `_events` log from their
cursor — the log is the single source of truth, so the live path and the
restart catch-up path are the *same* code.

## Components

### `Event` trait

```rust
pub trait Event: Serialize + DeserializeOwned + Send + Sync + 'static {
    /// Stable topic name used for the log, routing, and admin. e.g. "page.viewed".
    const TOPIC: &'static str;
    /// Optional key used by coalesce/sample primitives. None disables them.
    fn coalesce_key(&self) -> Option<String> { None }
}
```

Events are defined in any crate (including the consumer, e.g. myqaz). The topic
string is the durable identity in the log; the Rust type provides compile-time
safety at the subscription site.

### `Subscription<E>`

A declarative, typed builder:

```rust
Subscription::<PageViewed>::new("scraper.freshness")  // unique subscription id
    .durable()                                          // or .ephemeral()
    .filter(|e: &PageViewed| e.path.starts_with("/weather"))
    .coalesce_by_key(Duration::from_secs(3600))         // ≤1 per key per hour
    .rate_limit(10, Duration::from_secs(60))            // ≤10 invocations / minute
    .sample(1.0)                                         // fraction of keys (default 1.0)
    .max_attempts(3)                                     // retry budget before dead-letter
    .on(|e: PageViewed| async move { /* handler */ Ok(()) })
```

- `id` is unique across the bus; it is the cursor/dead-letter key.
- Delivery mode defaults to `ephemeral` (cheapest); `durable()` opts in.
- `filter` is a typed predicate on the deserialized event.
- Throttle primitives are optional and compose (see §Delivery Controls).
- `max_attempts` defaults to 3 (floored at 1), like the scheduler.
- `on` takes `Fn(E) -> impl Future<Output = Result<(), EventError>>`.

Internally each subscription is type-erased into a `Subscriber` that knows its
topic, mode, throttle config, and a handler `Arc<dyn Fn(serde_json::Value) ->
Future>` that deserializes the payload to `E` and runs the user closure.

### `EventBus` builder + `EventBusHandle`

```rust
let bus = EventBus::new(store.clone())
    .broadcast_capacity(1024)        // live channel depth (default 1024)
    .gc_interval(Duration::from_secs(300))
    .retention(Duration::from_secs(86_400))   // dead-letter / log retention floor
    .subscribe(sub_a)
    .subscribe(sub_b);

let handle: EventBusHandle = bus.spawn();   // starts dispatchers; must be in a tokio runtime
handle.publish(PageViewed { path, status }).await;
```

`EventBusHandle` (cheap, clonable, like `SchedulerHandle`):

- `publish<E: Event>(&self, e: E)` — assigns seq, persists if the topic has
  durable subscribers, broadcasts. Non-blocking beyond one store insert.
- `statuses() -> Vec<SubscriptionStatus>` — per-subscription cursor, lag,
  processed/failed counts, dead-letter count (for the admin page).
- `shutdown(&self)` — stop dispatchers, drain in-flight within a grace window.

## Data Model

System collections (underscore-prefixed, like `_jobs`):

### `_events` — append-only log
```jsonc
{ "seq": 42, "topic": "page.viewed", "payload": { ... }, "ts": "2026-05-31T..." }
```
- Indexes: unique `seq`; non-unique `topic`.
- **Written only when the topic has ≥1 durable subscriber.** Pure ephemeral
  topics (e.g. high-frequency stats) never touch the log → no bloat.

### `_event_cursors` — per durable subscription
```jsonc
{ "subscription": "scraper.freshness", "topic": "page.viewed",
  "last_seq": 41, "processed": 1234, "failed": 3, "updated_at": "..." }
```
- Index: unique `subscription`.

### `_event_deadletter` — poison events
```jsonc
{ "subscription": "scraper.freshness", "seq": 7, "topic": "page.viewed",
  "payload": { ... }, "attempts": 3, "last_error": "...", "failed_at": "..." }
```
- Index: non-unique `subscription`.

### `_event_seq` — monotonic high-water counter
```jsonc
{ "name": "global", "value": 42 }
```
- The live seq source is an in-process `AtomicU64` in the handle, seeded at
  startup from `max(_events.seq, _event_seq.value) + 1`, persisted opportunistically
  on each durable insert. This makes seq monotonic even after log GC trims rows.
- **Limitation:** single-process publishers only. Multiple processes publishing
  to the same store would need a store-level atomic counter — out of scope; documented.

## Delivery

### Publish path
1. `seq = counter.fetch_add(1)`.
2. If the topic has ≥1 durable subscriber: insert `_events` row; bump `_event_seq`.
3. Broadcast `EventEnvelope { seq, topic, payload, ts }` to live subscribers. If
   the broadcast channel is full, durable subscribers still recover from the log;
   ephemeral subscribers drop (accepted at-most-once).

### Per-subscription dispatcher (one tokio task each)

**Ephemeral:**
- Receive envelope → if `topic == E::TOPIC` → deserialize → predicate → throttle
  → run handler (fire-and-forget; errors logged, not retried). Never touches the DB.
- On broadcast lag (`RecvError::Lagged`): skipped events are simply lost (at-most-once).

**Durable:**
- Broadcast envelope (or a periodic poll fallback, or startup) is a **wake signal**.
- Drain the log: query `_events` where `topic == E::TOPIC` and `seq > last_seq`,
  ordered by `seq` ascending. For each event, in order:
  - predicate fails → **ack** (advance cursor), do not invoke.
  - throttle suppresses → **ack**, do not invoke.
  - else invoke handler with retry + exponential backoff up to `max_attempts`:
    - success → advance cursor, bump `processed`.
    - attempts exhausted → write `_event_deadletter`, advance cursor, bump `failed`
      (skip-after-N).
  - cursor is persisted to `_event_cursors` after each advance.
- **Restart catch-up = the same drain loop.** Events missed during downtime are
  replayed from the persisted cursor automatically.

### Ordering & at-least-once
Durable delivery is **ordered per subscription** and **at-least-once** (a crash
between handler success and cursor persist re-delivers the event — handlers
should be idempotent; documented). Ephemeral is unordered best-effort.

### Log GC
A periodic task (`gc_interval`) deletes `_events` rows where
`seq <= min(last_seq over all durable subscriptions of that topic)` — i.e. every
durable consumer has passed them (Kafka-style retention by consumer offset). A
topic with no durable subscriptions never logs, so nothing to GC. Dead-letter
rows older than `retention` are also pruned.

## Delivery Controls (bus primitives)

Applied in the dispatcher *before* invoking the handler:

- **`coalesce_by_key(window)`** — uses `Event::coalesce_key()`. An in-memory map
  `key → last_processed Instant`; within `window` of a processed key, later events
  with the same key are acked-without-invoke. (Map is in-memory; resets on
  restart → at worst one extra invocation per key after a restart.) This is the
  primary "don't scrape the same page more than once an hour" control.
- **`rate_limit(n, per)`** — token bucket per subscription. **Durable**: no token →
  the cursor *waits* (sleep until a token frees) so work is paced, not dropped.
  **Ephemeral**: no token → drop.
- **`sample(fraction)`** — deterministic by `hash(coalesce_key) % 10_000 <
  fraction*10_000`; same key is consistently in or out (no RNG). For stat
  sampling. Events with no `coalesce_key` are always invoked.

Domain throttling (is this page stale? popular enough to re-check?) lives in the
consumer's handler, which reads the store itself. The bus primitives are generic.

## adapto_app Integration

Mirrors the existing scheduler wiring (`crates/adapto_app/src/lib.rs`):

- `App::events(EventBus)` — store the builder; spawn it on `run()`/`build()`.
- `ctx.events() -> Option<&EventBusHandle>` — publish from request handlers:
  `ctx.events().unwrap().publish(PageViewed { .. }).await;`
- `App::events_admin(path, guard)` — guarded GET page rendering
  `handle.statuses()` (cursors, lag, processed/failed, dead-letter counts), same
  guard pattern as `scheduler_admin`.
- The events bus and scheduler are spawned together; shutdown drains both.
- Auto-publishing `PageViewed` on every request is a **consumer** concern (myqaz
  wires a middleware/wrapper); the crate only exposes `publish`.

## Bus → Scheduler Contract

`adapto_events` has **no dependency** on `adapto_scheduler`. A durable
subscription's handler closure captures whatever it needs. Recommended pattern
for heavy work (documented in the crate docs, implemented in the scraper spec):

1. Handler writes a task doc (e.g. the URL to re-scrape) to a work-queue
   collection.
2. Handler calls `scheduler.trigger("scrape_drain")`.
3. A scheduler job `scrape_drain` (Heavy lane, retries, persisted) drains the
   queue and performs the HTTP scrape + comparison.

This keeps routing (bus) and heavy execution (scheduler) cleanly separated and
the crates decoupled.

## Error Handling

- `EventError` (thiserror) — handler error type, `msg`/`from_err` constructors,
  mirroring `JobError`.
- Handler panics are isolated on their own task (like the scheduler's `run_one`)
  and counted as a failed attempt.
- Serialization failures on publish are logged and drop the event (cannot persist
  what won't serialize); the call does not panic.
- A full broadcast channel never blocks publish — durable recovers from log,
  ephemeral drops.

## Testing

**Unit:**
- `Event` topic + serde roundtrip.
- typed predicate filter include/exclude.
- `coalesce_by_key` suppresses same key within window, allows after.
- `rate_limit` paces (token bucket math) with injected clock.
- `sample` deterministic for a given key; fraction boundaries.
- cursor advance/persist; skip-after-N writes dead-letter and advances.

**Integration (in-memory `AdaptoStore::open(None)`):**
- publish → ephemeral handler runs.
- publish → durable handler runs, cursor persisted.
- restart (rebuild bus on the same store) replays events with `seq > last_seq`.
- poison event → dead-letter after `max_attempts`, cursor advances, later events
  still delivered.
- log GC trims fully-consumed seqs; un-consumed seqs retained.
- ephemeral-only topic writes nothing to `_events`.

**Determinism:** inject `now_fn` (and a clock for windows/backoff) exactly like
`Scheduler::now_fn`, so time-based tests are deterministic.

## Out of Scope (YAGNI)

- Multi-process / cross-machine publishing (single-process seq counter only).
- External (webhook/HTTP) subscribers — handlers are in-process Rust closures.
- Dynamic runtime subscribe/unsubscribe — subscriptions are registered at build
  time, like scheduler jobs.
- The stats consumer and the freshness-scraper consumer (separate specs).

## File Plan

```
crates/adapto_events/
  Cargo.toml
  src/
    lib.rs          # crate docs + re-exports
    event.rs        # Event trait, EventError, EventEnvelope
    subscription.rs # Subscription<E> builder, type-erased Subscriber
    throttle.rs     # coalesce / rate_limit / sample primitives
    bus.rs          # EventBus builder, EventBusHandle, publish
    dispatch.rs     # per-subscription dispatcher loops (ephemeral + durable)
    state.rs        # _events / _event_cursors / _event_deadletter / _event_seq persistence
    admin.rs        # SubscriptionStatus + render() for the admin page
```

Integration touches `crates/adapto_app/src/lib.rs` (App::events, ctx.events,
events_admin) and the workspace `Cargo.toml` members list. CHANGELOG.md updated
per project rules.
