# Changelog

All notable changes to this project will be documented in this file.

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
Versioning: [Semantic Versioning](https://semver.org/spec/v2.0.0.html)

## [Unreleased]

### Added
- **adapto_scheduler** — new crate: a persistent, priority + lane background-job scheduler.
  - `Schedule` enum — `Interval`, `DailyAt`, `WeeklyOn`, `MonthlyOn`, plus a `Cron(String)` escape hatch; all timezone-aware via `next_after`.
  - Two lanes: `Light` (configurable worker pool, default 4) and `Heavy` (serial); `Priority` orders ready jobs within a lane.
  - Per-job `catch_up` (coalesced single run for missed schedules after downtime) and capped exponential-backoff retry (default 3 attempts: 1m/5m/15m).
  - State persisted in the `_jobs` collection (`next_run` is the source of truth → restart-safe); panic-isolated handlers; graceful drain on shutdown.
  - `Reloadable<T>` — lock-free `ArcSwap`-based cache so job-updated datasets appear without a restart.
  - `admin::render()` — server-side HTML jobs table with run-now buttons.
  - 22 unit + 5 integration tests.
- **adapto_app scheduler integration** — `App::scheduler(Scheduler)` spawns the scheduler in `build()`/`run()` and drains it on graceful shutdown; `App::scheduler_admin(path, guard)` mounts a **mandatorily-guarded** admin page (`GET path`) + run-now trigger (`POST path/:job/run`, 403 without the guard); `RequestContext::scheduler()` exposes the handle to any route. (`adapto_app/src/lib.rs`, `handler.rs`)
- **adapto_store vector search** — TF-IDF fuzzy text matching for data normalization, gated behind the `vector` feature flag (zero new dependencies). (`adapto_store/src/vector.rs`)
  - `VectorIndex` — TF-IDF + character-trigram index, L2-normalized sparse cosine similarity. `add()` / `build()` / `search()` / `search_one()` / `batch_search()` / `normalize()`. Word-order invariant, case/hyphen insensitive (Cyrillic-friendly).
  - `Collection::normalize_field()` — match a text field against an index, write `{target}` + `{target}_score` to each document.
  - `Collection::normalize_name_field()` — extract legal form (via index) + clean name (via quotes) from a name field in one pass.
  - `Collection::parse_address_field()` — parse comma-separated addresses into `postal_code` / `region` / `city` / `street` using region & city reference indexes.
  - Free functions: `extract_quoted()` (balanced nesting for `«»` — `«A «B»»` → `A «B»`; greedy first-open..last-close fallback for unbalanced source), `extract_prefix()`, `parse_address()`. Result structs: `NormalizeResult`, `AddressResult`, `ParsedAddress`.
  - Validated on 977K KZ company records: ОКЭД 99%+, legal form 99.8%, clean name 98.4%, address region 100% / city ~95% against the КАТО classifier.
  - 17 unit tests + 5 doctests.
- **Template Inheritance** — layouts are now full `.adapto` files with `<slot/>` and `<slot name="..."/>` placeholders
  - Pages fill named slots via `{#fill name}...{/fill}` syntax
  - Multi-level layout chains: page → child layout → parent layout
  - Named slots with fallback content
  - Cycle detection and max depth (10) enforcement
  - `adapto_parser`: `FillNode` AST type, `{#fill}` parsing, `layout:` field in `<layout>` block
  - `adapto_compiler`: `SlotPlaceholderIR`, `FillSegmentIR`, duplicate fill detection
  - `adapto_ssr`: rewritten `LayoutManager` with chain resolution and slot-based composition

## [0.2.5] - 2026-05-27

### Added

#### adapto_auth — Production authentication
- **PBKDF2-SHA256 password hashing** — `hash_password()`, `verify_password()`, `validate_password_strength()`. 100K iterations, 16-byte CSPRNG salt (getrandom), 32-byte hash, constant-time comparison. Format: `pbkdf2-sha256$iterations$base64(salt)$base64(hash)`. (`password.rs`)
- **HS256 JWT** — `encode()`, `decode()`, `decode_without_verify()`. Claims with sub/iat/exp/iss/aud/custom fields. Builder: `.with_issuer()`, `.with_audience()`, `.with_claim()`. Uses existing hmac/sha2/base64 deps. (`jwt.rs`)
- **Session store** — `SessionStore` trait (create/get/update/destroy/exists/cleanup_expired). `InMemorySessionStore` with Arc<RwLock<HashMap>>. (`session_store.rs`)
- **Auth middleware config** — `AuthConfig` builder with enable_jwt/disable_csrf/public_path. Helpers: `validate_csrf_header()`, `validate_bearer_token()`, `validate_session_cookie()`, `generate_csrf_token()`, `sign_session()`, `issue_jwt()`. (`middleware.rs`)
- **78 tests** (was 49)

#### adapto_audit — Production audit logging
- **File sink** — JSON-lines append sink for persistent audit logging. (`sink.rs`)
- **Composite sink** — fan-out to multiple sinks simultaneously. (`sink.rs`)
- **Retention sink** — bounded buffer with FIFO eviction. (`sink.rs`)
- **Audit filter** — builder with event/action/user/tenant/status/date-range/route-prefix. `matches()` method. Query and count on InMemoryAuditSink. (`filter.rs`)
- **PII redaction** — configurable field patterns (email, password, ssn, phone, credit_card, token, secret, api_key). Case-insensitive matching. UTF-8 safe char-based redaction. (`redact.rs`)
- **30 tests** (was 15)

#### adapto_forms — Cross-field validation & sanitizers
- **Cross-field rules** — `FieldsMatch`, `RequiredIf`, `RequiredUnless`, `MutuallyExclusive`, `AtLeastOneOf`, `Custom`. (`rules.rs`)
- **Sanitizer pipeline** — `Trim`, `Lowercase`, `Uppercase`, `StripHtml`, `TruncateTo(usize)`. Per-field chains. (`sanitize.rs`)
- **`validate_and_sanitize()`** — combined validation + sanitization in one pass. (`schema.rs`)
- **54 tests** (was 37)

#### adapto_db — Database abstraction layer
- **DatabasePool trait** — async execute/query_one/query_all/in_transaction/health_check. `InMemoryPool` for testing. (`pool.rs`)
- **SQL generation** — `insert_sql()`, `update_sql()`, `delete_sql()`, `select_by_id_sql()`, `count_sql()`, `upsert_sql()`, `truncate_sql()`. Parameterized $1,$2 placeholders. (`sql.rs`)
- **Migration runner** — `MigrationRunner` with pending/run_pending/rollback_last/status/mark_applied. (`runner.rs`)
- **50 tests** (was 38)

#### adapto_ai — LLM integration
- **LlmClient trait** — `complete()` and `complete_json()` with BoxFuture. (`client.rs`)
- **MockLlmClient** — sequential responses, request recording, call counting. (`client.rs`)
- **MultiProviderClient** — named provider registry. (`client.rs`)
- **CompletionRequest builder** — system/user/assistant messages, temperature, max_tokens, stop sequences. (`client.rs`)
- **Prompt templates** — `PromptTemplate` with {{var}} substitution, system+user templates. `PromptLibrary` for named registry. (`prompt.rs`)
- **Response cache** — TTL, max entries, LRU eviction, stats, cleanup_expired. (`cache.rs`)
- **64 tests** (was 41)

#### adapto_macros — Extended derive
- **`update_in()`** — update resource by doc ID via Update::Set. (`resource.rs`)
- **`find_one_by()`** — find one resource by field value. (`resource.rs`)
- **`delete_where()`** — delete all matching query, returns count. (`resource.rs`)
- **`exists()`** — check existence by field value. (`resource.rs`)
- **12 tests** (was 8)

#### adapto_test_utils — HTTP & store test helpers
- **TestRequest builder** — GET/POST/PUT/DELETE/PATCH with json_body, headers, bearer auth, query params. (`http.rs`)
- **TestResponse builder** — ok/json/not_found/redirect constructors. Status checks, body parsing. (`http.rs`)
- **HTTP assertions** — `assert_status()`, `assert_body_contains()`, `assert_json_field()`, `assert_header()`. (`http.rs`)
- **Store helpers** — `temp_store()`, `StoreSeeder` with insert/seed_n/with_index. (`store.rs`)
- **Store assertions** — `assert_doc_exists()`, `assert_doc_not_exists()`, `assert_doc_field()`, `assert_collection_count()`, `assert_query_count()`. (`store.rs`)
- **JSON snapshot** — `assert_json_eq()`, `assert_json_includes()`, `assert_json_shape()`, `assert_json_array_len()`, `json_diff()`. (`snapshot.rs`)
- **80 tests** (was 35)

#### adapto — Complete umbrella crate
- **Feature flags** — `default` (app/ui/forms/auth/audit/macros/live), `full` (+ai/db/parser). No-default-features gives store-only. (`Cargo.toml`)
- **15 crate re-exports** — all workspace crates except cli and test_utils. Conditional on feature flags. (`lib.rs`)
- **Expanded prelude** — auth (JWT, password, sessions), audit (events, sinks), AI (LlmClient, prompts, cache), DB (pool, migrations) added to prelude. (`lib.rs`)

### Changed
- **Workspace tests** — 1118 tests across all crates (was 948)

## [0.2.4] - 2026-05-27

### Added

#### Phase 8: Integration Testing + Examples
- **7 E2E integration tests** — full pipeline: write .adapto → ProjectLoader → SSR render → verify HTML. Counter, multi-page app, resource CRUD, conditional+loop rendering, interpreter action execution, PageRenderer with ProjectLoader, full session lifecycle (init→events→patches→heartbeat). (`ssr_tests.rs`)
- **Counter example modernized** — removed manual `ActionHandler` closures, now uses interpreter fallback for increment/decrement/reset. State initialized via `init_state_from_defaults()`. (`server.rs`)

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
