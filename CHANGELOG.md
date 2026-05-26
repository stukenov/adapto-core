# Changelog

All notable changes to this project will be documented in this file.

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
Versioning: [Semantic Versioning](https://semver.org/spec/v2.0.0.html)

## [0.2.1] - 2026-05-27

### Added

#### adapto_ui — New components
- **Textarea** — multiline text input builder with rows, placeholder, error, required, ARIA. (`components.rs`)
- **Select** — dropdown select builder with options, placeholder, selected value, error/disabled. (`components.rs`)
- **Form** — form wrapper builder with action, method, child composition. (`components.rs`)
- **Table** — data table builder with headers, rows, caption, striped/hoverable/compact variants. Wrapped in responsive container. (`components.rs`)
- **Modal** — dialog builder with title, body, footer, close button, `role="dialog"`, `aria-modal`, `aria-labelledby`. Focus trap via `data-modal`/`data-modal-close`. (`components.rs`)
- **Pagination** — page navigator with ellipsis, prev/next, `aria-current="page"`. Configurable base URL and param name. (`components.rs`)
- **Toast** — notification builder (info/success/warning/error) with auto-dismiss duration, inline close button. (`components.rs`)
- **Skeleton** — loading placeholder: `.text(lines)`, `.card()`, `.circle()`, `.rect(w, h)`. CSS animation class. (`components.rs`)

#### adapto_ui — DX enhancements
- **`.id()`, `.class()`, `.attr()`** — all components now support custom id, extra CSS classes, and arbitrary HTML attributes. (`components.rs`)
- **Button `.action()` / `.data_id()`** — set `data-action` and `data-id` for live.js event handling. (`components.rs`)
- **Button `.href()`** — renders `<a>` instead of `<button>`, visually identical. (`components.rs`)
- **FormGroup `.input_id()`** — links `<label for>`, help `id="{id}-help"`, error `id="{id}-error"` for full accessibility. (`components.rs`)
- **Alert `.dismissible()`** — adds inline close button with `onclick` dismiss. (`components.rs`)
- **Progress `.indeterminate()`** — constructor for unknown-duration progress bars. (`components.rs`)
- **AvatarSize enum** — `.small()` / `.large()` methods replace raw CSS class string in `.size()`. Old `.size(&str)` kept for backward compat. (`components.rs`)
- **Breadcrumb `<nav>`** — wrapped in `<nav aria-label="Breadcrumb">`. (`components.rs`)

### Changed

#### adapto_ui
- **51 → 93 tests** — 42 new tests covering all new components and DX features. (`components.rs`)

## [0.2.0] - 2026-05-27

### Added

#### adapto_app — Full HTTP support
- **POST/PUT/DELETE/PATCH routes** — `.post()`, `.put()`, `.delete()`, `.patch()` methods on App builder. (`lib.rs`)
- **Async handlers** — `async_page()`, `async_post()`, `async_put()`, `async_delete()`, `async_patch()` for handlers that need `await`. (`lib.rs`)
- **Request body/headers/method** — `RequestContext` now exposes `method()`, `header()`, `headers()`, `body_json()`, `body_bytes()`, `body_str()`, `cookie()`, `remote_addr()`, `query_param()`, `query_pairs()`. (`lib.rs`)
- **PageResponse::Json** — return JSON responses with automatic `Content-Type: application/json`. (`lib.rs`)
- **PageResponse::BadRequest/Forbidden/InternalError** — proper HTTP status codes 400/403/500. (`lib.rs`)
- **PageResponse::Custom** — arbitrary status code, content type, headers. (`lib.rs`)
- **PageResponse::json()** — serialize any `Serialize` type to JSON response. (`lib.rs`)
- **PageResponse::with_status()** — custom status code with HTML body. (`lib.rs`)
- **PageResponse::raw()** — custom status, body, and content type. (`lib.rs`)
- **Middleware support** — `.with_middleware()` to add tower layers (CORS, compression, auth). (`lib.rs`)
- **Static file serving** — `.static_dir("/static", "./public")` via tower-http ServeDir. (`lib.rs`)
- **Graceful shutdown** — Ctrl+C and SIGTERM handling with connection drain. (`lib.rs`)
- **Shutdown hooks** — `.on_shutdown()` to run cleanup on graceful shutdown. (`lib.rs`)
- **Health check endpoint** — `.health_check("/health")` returns 200 "ok". (`lib.rs`)
- **Error handler** — `.error_handler()` for custom error page rendering. (`lib.rs`)
- **TestClient** — `app.test_client()` for HTTP testing without TCP binding. Supports `.get()`, `.post()`, `.put()`, `.delete()`, `.request()`. (`lib.rs`)
- **Environment config** — `.from_env()` reads `PORT`, `BIND_ADDR`, `STORE_PATH` from env vars. (`lib.rs`)
- **Localized POST** — `.localized_post()` for multilingual POST routes. (`lib.rs`)
- **URL decoding** — `query_param()` and `query_pairs()` with proper percent-decoding. (`lib.rs`)

### Changed

#### adapto_app
- **`index_page()` signature** — now takes `Fn(RequestContext) -> R` instead of `Fn(Arc<AppState>) -> String`. **Breaking change.** (`lib.rs`)
- **`get_route()` signature** — now takes `Fn(RequestContext) -> String` instead of `Fn(Arc<AppState>) -> String`. (`lib.rs`)
- **Internal handler type** — unified `BoxHandler` type (async-first) replaces `RouteHandler` enum. (`lib.rs`)
- **`App::build()`** — extracted router construction into public `build()` method for testing. (`lib.rs`)

### Dependencies
- Added `tower` and `tower-http` (CORS, static files) to adapto_app.

## [0.1.2] - 2026-05-26

### Documentation
- **Unique index NULL behavior** — documented sparse semantics (multiple NULLs allowed). (`index.rs:172`)
- **Card raw HTML** — documented that body/header/footer accept raw HTML, caller must escape. (`components.rs:408`)
- **Umbrella crate** — documented why `adapto` has `publish = false`. (`adapto/Cargo.toml`)
- **Index selection** — documented non-deterministic behavior in AND queries. (`collection.rs:450`)

### Fixed (adapto_parser)
- Added missing `pest`/`pest_derive` dependencies.
- Removed unused `uuid` dependency.

### Fixed

#### adapto_store — Critical WAL fixes
- **WAL update data loss** — `update()` now collects document IDs before mutation, then snapshots after. Previously, re-querying post-mutation returned 0 documents if the query field was changed, silently dropping WAL entries. (`engine.rs:219-236`)
- **WAL no fsync** — `append()` now calls `flush()` + `sync_data()` after every write. Previously, data sat in OS buffer and was lost on power failure. (`wal.rs:78-83`)
- **WAL replay lost timestamps** — `WalEntry::Insert` now stores `created_at`/`updated_at`. Previously, replay used `Utc::now()`, destroying original timestamps. (`engine.rs:626-634`)
- **drop_collection TOCTOU** — WAL append now runs under write lock to prevent ghost collections from concurrent `get_or_create`. (`engine.rs:111-120`)
- **update_by_id tenant leak** — WAL snapshot now uses caller's `tenant_id` instead of `None`. (`engine.rs:251`)
- **Cursor clones on iteration** — Replaced `Vec<Document>` + positional clone with `VecDeque::pop_front()`. Zero-copy iteration. (`cursor.rs`)
- **Fake regex engine** — `Filter::Regex` now uses the `regex` crate. Previously, `simple_regex_match` silently gave wrong results for character classes, alternation, quantifiers. (`query.rs`)
- **DiskCollections not auto-reopened** — Store now scans `disk/` directory on open and auto-registers all `.dat` files. No more manual re-registration after restart. (`engine.rs`)

#### adapto_app — Security and correctness
- **Open redirect** — Trailing-slash redirect now rejects paths starting with `//` (protocol-relative URLs). Previously `//evil.com/` produced `301 → //evil.com`. (`lib.rs:605`)
- **tracing panic** — Replaced `tracing_subscriber::fmt::init()` with `try_init()`. No longer panics on second call or when user sets their own subscriber. (`lib.rs:406`)
- **localized_page silent no-op** — Now panics with clear message if called before `.languages()`. Previously registered 0 routes silently. (`lib.rs:334`)
- **WebSocket Ping/Pong** — Event loop now responds to Ping frames and handles Close frames. Previously, proxied connections dropped on timeout. (`handler.rs:73`)
- **ctx.query data loss** — Now uses raw `uri.query()` instead of HashMap re-encoding. Preserves URL encoding and parameter order. (`lib.rs:517`)
- **Path extractor on parameterless routes** — Changed to `Option<Path<...>>` to avoid axum rejection on routes without `:param` segments. (`lib.rs:511`)

### Changed

#### adapto_app
- **Default bind address** — Changed from `127.0.0.1` to `0.0.0.0`. Added `.bind()` builder method. (`lib.rs:631`)
- **live.js reconnect** — Added guard to prevent duplicate WebSocket connections on error/close double-fire. (`live.js:30`)
- **live.js external links** — `__adapto_navigate` now detects external URLs and falls back to `location.href`. (`live.js:56`)

#### adapto_ui
- **ButtonType enum** — Replaced `button_type(&str)` with `button_type(ButtonType)` enum. Eliminates attribute injection. (`components.rs:103`)
- **Single html_escape** — Removed duplicate private `html_escape()` from `components.rs`. Now uses `crate::html_escape`. (`components.rs:960`)

#### adapto_macros
- **get_field works for all types** — Generated `get_field()` now uses `format!("{}", self.field)` instead of `.clone()`. Works with `i64`, `bool`, any `Display` type. (`resource.rs:148`)

#### Workspace
- Removed unused `garde` from workspace dependencies.

## [0.1.1] - 2026-05-26

### Added
- Per-crate README files for crates.io (adapto_store, adapto_app, adapto_ui, adapto_client_protocol)
- Root README with badges, production mention, comprehensive examples

## [0.1.0] - 2026-05-26

### Added
- Initial release of adapto_store, adapto_app, adapto_ui, adapto_client_protocol
- PageResponse enum (Ok/NotFound/Redirect) for proper HTTP status codes
- LangConfig trait + localized_page() for multilingual routing
- slugify() with Cyrillic transliteration (Russian + Kazakh)
- DiskCollection: mmap-backed storage for large datasets
- adapto_macros: #[derive(Resource)] proc macro
- MIT license
