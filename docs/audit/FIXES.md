# Audit Fixes Plan — 2026-05-26

## Phase 1: WAL Data Loss (C1 + C2 + C3)
- **C1** `engine.rs:219-236` — WAL update re-queries post-mutation, logs 0 docs. Fix: collect IDs before, snapshot after.
- **C2** `wal.rs:78-83` — No fsync on WAL append. Fix: add `sync_data()`.
- **C3** `engine.rs:626-634` — WAL replay uses `Utc::now()` instead of original timestamps. Fix: store timestamps in WalEntry::Insert.

## Phase 2: Open Redirect (C4)
- **C4** `adapto_app/lib.rs:605-613` — Trailing-slash redirect with `//evil.com/` is open redirect. Fix: reject `//` paths.

## Phase 3: Framework Panics (C5 + H1)
- **C5** `adapto_app/lib.rs:406` — `tracing_subscriber::fmt::init()` panics on second call. Fix: `try_init()`.
- **H1** `adapto_app/lib.rs:334-353` — `localized_page()` silently registers 0 routes if called before `.languages()`. Fix: warn or panic.

## Phase 4: WebSocket + Bind (H2 + H3)
- **H2** `handler.rs:73-118` — No Ping/Pong/Close handling. Fix: match all frame types.
- **H3** `adapto_app/lib.rs:631` — Bind 127.0.0.1 only. Fix: default 0.0.0.0, add `.bind()`.

## Phase 5: Query Correctness (H4 + H6)
- **H4** `adapto_app/lib.rs:517-521` — ctx.query lossy HashMap rebuild. Fix: use raw uri.query().
- **H6** `query.rs:127-179` — Fake regex. Fix: add `regex` crate dependency.

## Phase 6: UI XSS Surface (M4 + M5 + M6)
- **M4** `adapto_ui/components.rs:947` — Duplicate html_escape. Fix: delete, use crate::html_escape.
- **M5** `adapto_ui/components.rs:453-469` — Card body unescaped. Fix: document as raw HTML.
- **M6** `adapto_ui/components.rs:103-106` — button_type() injection. Fix: ButtonType enum.

## Phase 7: Store API Quality (M1 + M2 + M3)
- **M1** `index.rs:172-178` — Unique index allows multiple NULLs undocumented. Fix: document behavior.
- **M2** `cursor.rs:49-54` — Cursor clones every doc. Fix: use drain/swap pattern.
- **M3** `engine.rs:88-101` — TOCTOU drop_collection + get_or_create. Fix: hold write lock across both ops.

## Phase 8: live.js Fixes (M7 + M8)
- **M7** `live.js:30-31` — Double reconnect on error. Fix: reconnect guard.
- **M8** `live.js:56-63` — External link interception. Fix: check relative URL.

## Phase 9: Workspace Hygiene (L1-L5)
- **L1** Verify examples/counter and examples/school_ai exist or remove from workspace.
- **L2** Remove unused `garde` from workspace deps.
- **L3** Fix adapto_parser deps or update CLAUDE.md.
- **L4** Document umbrella crate publish=false.
- **L5** Non-deterministic index selection — document or fix.

## Phase 10: Remaining High (H5 + H7)
- **H5** `engine.rs:604-678` — DiskCollection not auto-reopened. Fix: scan disk/ dir on open.
- **H7** `resource.rs:148-152` — get_field() breaks non-String fields. Fix: use serde_json::Value.
