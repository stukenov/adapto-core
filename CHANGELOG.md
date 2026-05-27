# Changelog

All notable changes to this project will be documented in this file.

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
Versioning: [Semantic Versioning](https://semver.org/spec/v2.0.0.html)

## [0.2.3] - 2026-05-27

### Added

#### Phase 7: CLI dev & build Commands
- **`cmd_dev` real implementation** — async, compiles project via `ProjectLoader`, registers dependency graphs, constructs `AdaptoServer`, starts HTTP+WS server. Reports file/route/resource counts. (`commands.rs`)
- **`cmd_build` real implementation** — compiles project, validates all files, reports manifest with routes listing (method, path, auth). (`commands.rs`)
- **`compile_project()` helper** — wraps `ProjectLoader::load_project()` with CLI error mapping. (`commands.rs`)
- **Async main** — `#[tokio::main]` for dev server startup. (`main.rs`)
- **CLI deps** — added `adapto_ssr`, `adapto_store`, `adapto_live`, `tokio`. (`Cargo.toml`)
- **Async test migration** — 6 tests converted from `#[test]` to `#[tokio::test]` for async `run()`. (`cli_tests.rs`)

#### Phase 6: WS Session Lifecycle
- **`init_state_from_defaults()`** — LiveSession method parses `StateFieldIR.default` values as JSON and populates StateStore, clearing dirty flags after init. (`session.rs`)
- **Auto-session creation** — `process_ws_message()` auto-creates LiveSession from pending session map when session not found on WS connect. Uses PageRenderer component/graph lookup. (`server.rs`)
- **Pending session store** — `AppState.pending_sessions` (RwLock HashMap) maps session_id→route_id, populated during SSR page render, consumed on WS connect. (`server.rs`)
- **`render_page()` returns session_id** — now returns `(html, session_id)` tuple for session tracking. (`renderer.rs`, `page.rs`)
- **PageRenderer route/component lookup** — added `get_component()`, `get_dependency_graph()`, `match_route()`, `register_dependency_graph()`. (`page.rs`)
- **Background cleanup task** — `serve()` spawns tokio task: expired session cleanup every 60s (5min timeout), pending session overflow cap. (`server.rs`)
- **11 Phase 6 tests** — state init from defaults/partial, event→patches after init, multiple increments, expiry, touch prevents expiry, manager add+dispatch, cleanup removes expired, heartbeat ack, event→patch dispatch, seq monotonicity. (`live_tests.rs`)

#### Phase 5: JS Client Runtime
- **adapto-client.js** — full client runtime (~400 LOC) replacing 22-line inline script. All 15 PatchOp handlers, WebSocket with exponential backoff reconnection + jitter, heartbeat keepalive, event delegation (click/input/change/submit/keydown/keyup/focus/blur), form serialization (text/checkbox/radio/select/multi-select), modifier support (prevent/stop/debounce/throttle), flash notification system with ARIA live region, modal dialog with Escape key + backdrop dismiss + focus trap, post-redirect flash via sessionStorage, public API via `window.__adapto`. (`static/adapto-client.js`)
- **server.rs** — `handle_client_js()` now uses `include_str!` for static file instead of inline JS. (`server.rs`)
- **renderer.rs** — `websocket_url` simplified from `/ws/{session_id}` to `/ws` — session ID sent as first WS message. (`renderer.rs`)
- **17 Phase 5 tests** — JS content verification (all 15 PatchOps, event delegation, form serialization, modifiers, reconnection, heartbeat, flash, modal, public API), bootstrap websocket_url format, protocol JSON shape tests (event, patch, form_submit, navigate, error, redirect, all 15 PatchOp serialization). (`ssr_tests.rs`)

#### Phase 4: Project Loader + Auto-Discovery
- **ProjectLoader** — `load_project(path, secret)` scans directory tree for `.adapto` files, parses and compiles all, registers components/routes/resources/layouts into a `CompiledProject` struct. Skips `.`-prefixed dirs, `target/`, `node_modules/`. (`project.rs`)
- **CompiledProject** — holds `PageRenderer`, `LayoutManager`, `RouteManifest`, `ResourceManager` map, `DependencyGraph` map, `ComponentIR` map, file count. (`project.rs`)
- **compiler→runtime ResourceIR conversion** — `compiler_to_runtime_resource()` bridges compiler and runtime ResourceIR types without cyclic dependency. (`project.rs`)
- **template→HTML extraction** — `template_to_raw_html()` / `node_to_html()` recursively convert parser AST nodes to raw HTML for layout registration. (`project.rs`)
- **5 project loader tests** — discover files, register routes, register resources, register components, empty dir error. (`project.rs`)
- **adapto_store** added as dependency for `adapto_ssr`. (`Cargo.toml`)

#### Phase 3: Resource → CRUD Pipeline
- **ResourceIR** — compiled resource definition with fields, indexes, permissions, tenant scoping. Defined in both `adapto_compiler::ir` and `adapto_runtime::resource` (structurally identical, serde-compatible). (`ir.rs`, `resource.rs`)
- **Compiler resource compilation** — `compile_resource()` transforms parser `ResourceBlock` → `ResourceIR` with field constraints, unique/searchable indexes, permission map. Added `resource_ir: Option<ResourceIR>` to `CompileOutput`. (`compiler.rs`)
- **ResourceManager** — CRUD operations against `AdaptoStore` with automatic tenant scoping via `TenantScope`/`TenantCollection`, field validation (required, min/max length, readonly), default value application, permission checking. (`resource.rs`)
- **16 resource tests** — create/get, tenant isolation, missing required field, field too long, update, readonly field rejection, delete, permission denied (read/create), tenant required, count, default values, non-tenant resource, full CRUD lifecycle, list with limit/skip. (`resource.rs`)

#### Phase 2: Full Action Interpreter
- **Interpreter module** — lexer, parser, evaluator for action body DSL. Supports assignments (`=`, `+=`, `-=`, `*=`, `/=`), binary operators (`+`, `-`, `*`, `/`, `%`, `==`, `!=`, `<`, `<=`, `>`, `>=`, `&&`, `||`), unary (`!`, `-`), `let` bindings, `if/else`, `for..in` loops, dot-path access/assignment, index access, method calls, array/object literals, comments, string concatenation. (`interpreter.rs`)
- **Built-in methods** — `len()`, `push()`, `contains()`, `is_empty()`, `to_lowercase()`, `to_uppercase()`, `trim()`, `starts_with()`, `ends_with()`, `split()`, `replace()`, `keys()`, `values()`, `abs()`. (`interpreter.rs`)
- **LiveSession interpreter fallback** — `handle_event()` now falls back to interpreting `ActionIR.body` when no manual handler is registered, with permission checking. (`session.rs`)
- **36 interpreter tests** — simple/compound assignment, dot-path mutation, arithmetic, comparison, string concat, if/else, for loop, let binding, string/array methods, args access, logical ops, multi-statement, array/object literals, index access, dirty tracking, counter/toggle/form scenarios, edge cases (div/0, empty body, comments, nested if, coercion, modulo, parentheses, negative numbers). (`interpreter.rs`)

#### Phase 1: SSR Conditional & Loop Rendering
- **SegmentBody / LoopBody IR** — `DynamicSegment` now holds nested `then_body`, `else_body`, `else_if_bodies`, `loop_body`, `permission_body` instead of flattening children into parent arrays. (`ir.rs`)
- **Compiler nested compilation** — `compile_body()` method compiles if/each/can branch children into isolated `SegmentBody` structs on the `DynamicSegment`. (`compiler.rs`)
- **SSR conditional rendering** — `render_conditional()` evaluates condition via `eval_expr()`, renders matching branch body. (`renderer.rs`)
- **SSR loop rendering** — `render_loop()` evaluates iterable via `eval_expr_raw()`, iterates items with scoped `StateStore` clone per iteration. (`renderer.rs`)
- **Recursive body rendering** — `render_body()` recursively renders nested `SegmentBody` segments. (`renderer.rs`)
- **13 new Phase 1 SSR tests** — if true/false, if-else branches, truthy values, dynamic content inside if, each with 0/1/3/5 items, each with index, each with objects, nested if-inside-each, e2e customer page if+each. (`ssr_tests.rs`)

### Fixed
- **eval_expr dot-path with state. prefix** — `eval_expr()` and `eval_expr_raw()` now try dot-path resolution on `state.`-stripped expression before original, fixing `state.user.name` lookups in loop scopes. (`renderer.rs`)

## [0.2.2] - 2026-05-27

### Added

#### Tests — 525 new tests across 13 WIP crates (843 total workspace)
- **adapto_runtime** — 42 tests: StateStore CRUD/dirty/merge, PermissionSet logic, Ctx auth/tenant/permissions, Config serde roundtrip, RuntimeError display, type conversions/equality/hashing. (`runtime_tests.rs`)
- **adapto_auth** — 49 tests: CSRF generate/validate/expired/tampered, session tokens sign/verify/tampered, RBAC define/assign/revoke/multi-role, rate limiter within/exceed/independent keys. (`auth_tests.rs`)
- **adapto_audit** — 15 tests: AuditEvent creation/metadata/status, InMemory/Channel/Log sinks, serialization. (`audit_tests.rs`)
- **adapto_forms** — 37 tests: schema builder, all field types (String/Email/Integer/Decimal/Boolean/UUID/DateTime/Enum), constraints (min/max length, required, pattern), edge cases. (`form_tests.rs`)
- **adapto_db** — 38 tests: Query builder eq/ne/gt/lt/like/in/null/AND/OR with parameterized SQL, InMemoryRepository CRUD/tenant isolation/search/count, Migration create_table SQL. (`db_tests.rs`)
- **adapto_ai** — 41 tests: PiiRedactor email/phone/SSN/CC/custom patterns, BudgetTracker set/check/record/exceed/reset, ModelRouter add/resolve/default/fallback/cost, TraceCollector, TokenUsage Add trait. (`ai_tests.rs`)
- **adapto_parser** — 74 tests: all DSL blocks (route/script/template/style/resource), error recovery, edge cases, multi-block files, AI action parsing. (`parse_tests.rs`)
- **adapto_compiler** — 59 tests: IR generation, codegen output, DependencyGraph construction/lookup, RouteManifest/ComponentManifest, CompileError display, full pipeline with if/each/can. (`compiler_tests.rs`)
- **adapto_ssr** — 39 tests: Renderer static/dynamic/events, page wrapping with bootstrap/styles, Router exact/dynamic/nested matching, PageRenderer auth/tenant/permissions, Layout register/compose, 5 end-to-end integration tests (parse→compile→render). (`ssr_tests.rs`)
- **adapto_live** — 38 tests: SessionManager add/count/has/remove/cleanup_expired/max sessions, PatchGenerator, event validation. (`live_tests.rs`)
- **adapto_cli** — 23 tests: clap parsing, all command variants (new/dev/build/check/generate), CliError display. (`cli_tests.rs`)
- **adapto_macros** — 8 tests: derive Resource expansion, field names, collection name, route prefix. (`macros_tests.rs`)
- **adapto_test_utils** — 42 tests: all builders (Event/Form/Patch/State), fixture functions, MockAuditSink, MockClock. (`test_utils_tests.rs`)

#### Benchmarks — 4 benchmark suites with custom harness
- **adapto_store** — insert (100–100K), find_by_id, query eq/range/complex, update, delete, indexed vs scan (365x speedup), bulk_insert, sort, WAL persistence/replay/compact, concurrent writes same/different collections, concurrent read+write mix. (`benches/benchmark.rs`)
- **adapto_parser** — parse minimal/counter/full-page/resource, throughput measurement (~75 MB/s). (`benches/benchmark.rs`)
- **adapto_compiler** — compile minimal/counter/full-page, dependency graph lookup, codegen size analysis, throughput (~106 MB/s). (`benches/benchmark.rs`)
- **adapto_ssr** — render_component minimal/counter/full-page, render_page with layout, render throughput (~224 MB/s), full pipeline parse→compile→render (~64K ops/sec). (`benches/benchmark.rs`)

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
