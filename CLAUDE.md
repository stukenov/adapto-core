# Adapto Core — Development Rules

## Project Overview

Adapto is a Rust web framework for building data-driven sites and apps. Workspace of 18 crates.

**Source**: `~/adapto-core/`
**Consumer project**: `~/myqaz/myqaz-rs/` (myqaz.kz — SEO portal built on Adapto)

## Crate Map

| Crate | Purpose | Status |
|-------|---------|--------|
| `adapto_store` | Embedded document DB (JSON docs, BTree indexes, WAL) | Production |
| `adapto_app` | HTTP app builder (axum-based, routes, WebSocket, fallbacks) | Production |
| `adapto_ui` | HTML component library, `html_escape()` | Production |
| `adapto_macros` | Derive macros for `#[derive(Resource)]` | WIP |
| `adapto` | Umbrella re-export crate | WIP |
| `adapto_parser` | Template/DSL parser (pest) | WIP |
| `adapto_compiler` | Template compiler | WIP |
| `adapto_ssr` | Server-side rendering | WIP |
| `adapto_live` | Live reload / hot updates | WIP |
| `adapto_runtime` | Client runtime (JS) | WIP |
| `adapto_forms` | Form validation (garde) | WIP |
| `adapto_auth` | Authentication (HMAC/JWT) | WIP |
| `adapto_audit` | Audit logging | WIP |
| `adapto_ai` | AI integration | WIP |
| `adapto_db` | PostgreSQL layer (sqlx) | WIP |
| `adapto_cli` | CLI tooling | WIP |
| `adapto_client_protocol` | Client-server protocol | WIP |
| `adapto_test_utils` | Test helpers | WIP |

## adapto_store — Embedded Document DB

### Core Concepts

- **Store**: top-level entry point. `AdaptoStore::open(Some("./data"))` for persistent, `open(None)` for in-memory
- **Collection**: named group of JSON documents. `store.collection("users")`
- **DiskCollection**: mmap-backed, indexes in memory, data on disk. For 100K+ documents. `store.disk_collection("companies")`
- **Document**: `{ id: String, data: serde_json::Value, created_at, updated_at }`
- **Query**: filter + sort + limit + skip + projection. `Query::eq("slug", "my-item")`
- **Index**: BTree-based. `col.create_index("slug", true)` for unique

### API Reference

```rust
use adapto_store::{AdaptoStore, Query, Document};

// Open store
let store = AdaptoStore::open(Some("./data"))?;

// Collections
let users = store.collection("users");

// CRUD
let id = users.insert(json!({"name": "Alice", "age": 30}))?;
let ids = users.insert_many(vec![json!({...}), json!({...})])?;
let doc = users.find_by_id(&id)?;            // Option<Document>
let doc = users.find_one(Query::eq("name", "Alice"))?; // Option<Document>
let cursor = users.find(Query::new());        // Cursor (iterator)
users.update(Query::eq("name", "Alice"), Update::Set(vec![("age".into(), json!(31))]))?;
users.delete(Query::eq("name", "Alice"))?;
users.delete_by_id(&id)?;
let count = users.count(Query::new())?;
let total = users.count_all();

// Indexes
users.create_index("email", true)?;           // unique
users.create_compound_index(&["company", "role"], false)?;

// Document field access
doc.get("address.city");                       // dot-notation
doc.get_str("name");                           // -> Option<&str>
doc.get_i64("age");                            // -> Option<i64>

// Query builder
Query::new()                                   // match all
Query::eq("field", value)                      // equality
Query::filter(Filter::And(vec![...]))          // complex
    .sort("name", SortDir::Asc)
    .limit(10)
    .skip(20)
    .project(&["name", "email"])

// Filters: Eq, Ne, Gt, Gte, Lt, Lte, In, Nin, Exists, Regex, Contains, And, Or, Not, All

// DiskCollection (for large datasets)
let companies = store.disk_collection("companies")?;
companies.bulk_insert(docs)?;                  // overwrites existing
companies.find(Query::eq("bin", "123456789012"));
companies.find_one(Query::eq("bin", "123456789012"));
companies.create_index("bin", true)?;
companies.index_keys("bin");                   // Vec<String> — keys without loading docs
companies.count_all();

// Store management
store.compact()?;                              // WAL compaction
store.stats();                                 // StoreStats
store.collections();                           // Vec<String>
store.drop_collection("temp")?;
```

### Data Import Rules (CRITICAL)

- **NEVER** store an entire JSON file as a single document (blob). Each searchable entity MUST be a separate document.
- For arrays of items (drugs, companies, etc.): iterate and `insert_many()` — each item becomes its own document.
- For multi-file collections (laws, codes): each file = one document.
- For nested single-document data (constitution, namaz, quran): acceptable as single doc, but MUST use `OnceLock` in renderer to deserialize once.
- **ALWAYS** create indexes on lookup fields after import: `col.create_index("slug", true)`.
- Use `Query::eq("field", value)` for single-entity lookups — O(1) via BTree index.
- Use `Query::new()` only for listing/index pages that need all documents.

## adapto_app — HTTP App Builder

### Core Concepts

- **App**: builder pattern. Configure title, port, store, routes, then `.run().await`
- **page()**: register a GET route. Handler receives `RequestContext`
- **post()/put()/delete()/patch()**: register routes for other HTTP methods
- **async_page()/async_post()/..**.: async variants for handlers that need `.await`
- **fallback_fn()**: catch-all for unmatched routes
- **PageResponse**: `Ok(String)`, `NotFound`, `Redirect`, `Json`, `BadRequest`, `Forbidden`, `InternalError`, `Custom`
- **TestClient**: `app.test_client()` for HTTP testing without TCP binding

### API Reference

```rust
use adapto_app::{App, PageResponse, RequestContext};
use adapto_store::AdaptoStore;

let store = AdaptoStore::open(None)?;

App::new("My App")
    .port(8080)
    .bind("0.0.0.0")
    .store(store)
    .from_env()                              // reads PORT, BIND_ADDR, STORE_PATH
    .health_check("/health")                 // 200 "ok"
    .static_dir("/static", "./public")       // serve static files
    // GET routes
    .page("/", |ctx| {
        render_home(ctx.store())
    })
    .page("/users/:id", |ctx| {
        let id = ctx.param("id");
        render_user(ctx.store(), id)
    })
    // POST/PUT/DELETE routes
    .post("/api/users", |ctx| {
        let body: NewUser = ctx.body_json().unwrap();
        PageResponse::Json(json!({"id": "123"}))
    })
    .put("/api/users/:id", |ctx| {
        PageResponse::Json(json!({"updated": true}))
    })
    .delete("/api/users/:id", |ctx| {
        PageResponse::Json(json!({"deleted": true}))
    })
    // Async handlers
    .async_page("/external", |ctx| async move {
        let data = fetch_api().await;
        PageResponse::Ok(render(data))
    })
    // Middleware
    .with_middleware(|router| {
        router.layer(tower_http::cors::CorsLayer::permissive())
    })
    // Graceful shutdown
    .on_shutdown(|| { println!("bye"); })
    // Fallback
    .fallback_fn(|path| {
        if path == "/robots.txt" {
            return FallbackResponse::Raw {
                body: "User-agent: *\nAllow: /".into(),
                content_type: "text/plain; charset=utf-8",
            };
        }
        FallbackResponse::NotFound
    })
    .run()
    .await?;
```

### RequestContext

```rust
ctx.store()                 // &AdaptoStore
ctx.param("name")           // &str — path parameter value
ctx.path()                  // &str — full request path
ctx.method()                // &Method — HTTP method
ctx.header("X-Custom")      // Option<&str> — request header
ctx.headers()               // &HeaderMap — all headers
ctx.body_json::<T>()        // Result<T> — parse body as JSON
ctx.body_bytes()            // &[u8] — raw body
ctx.body_str()              // Result<&str> — body as string
ctx.cookie("session")       // Option<&str> — cookie value
ctx.remote_addr()           // Option<SocketAddr> — client IP
ctx.query_param("page")     // Option<String> — single query param
ctx.query_pairs()           // Vec<(String, String)> — all query params
ctx.lang_code()             // &str — language code (localized routes)
ctx.lang_prefix()           // &str — language URL prefix
```

### TestClient

```rust
let app = App::new("Test").page("/hello", |_| "hi");
let client = app.test_client();

let resp = client.get("/hello").await;
assert_eq!(resp.status(), 200);
assert!(resp.text().contains("hi"));

let resp = client.post("/api", r#"{"key":"val"}"#).await;
let json: Value = resp.json();
```

### Route Registration Order

Routes are matched in registration order. Register more specific paths first:
```rust
.page("/reference/drugs/type/:slug", handler)  // before generic :slug
.page("/reference/drugs/:slug/:section", handler)
.page("/reference/drugs/:slug", handler)
```

### ResourceMeta Trait

For CRUD resources with auto-generated routes:
```rust
impl ResourceMeta for Customer {
    fn collection_name() -> &'static str { "customers" }
    fn field_names() -> &'static [&'static str] { &["name", "email"] }
    fn resource_label() -> &'static str { "Customer" }
    fn resource_label_plural() -> &'static str { "Customers" }
    fn route_prefix() -> &'static str { "/customers" }
    fn ensure_indexes(store: &AdaptoStore) {
        store.collection("customers").create_index("email", true).ok();
    }
}
```

## Renderer Rules

- Single-doc collections: use `OnceLock<Option<T>>` to deserialize from store once, return `.clone()`.
- Multi-doc collections: use `find_one(Query::eq(...))` for detail pages, `OnceLock<Vec<T>>` for index pages.
- **NEVER** add page-level caching (HashMap/RwLock) inside adapto_app. Caching belongs in renderers via OnceLock or in AdaptoStore indexes.
- **NEVER** add workarounds (OnceLock in app framework, static caches) when the real fix is proper data import and indexing.

## Architecture

- adapto_store is a document-oriented embedded DB (like MongoDB). Treat it as such.
- Documents are `serde_json::Value` objects with auto-generated IDs.
- Indexes are BTree-based, support `find_eq`, `find_range`.
- Collections support `insert`, `insert_many`, `find`, `find_one`, `create_index`.

## Building & Testing

```bash
cargo test -p adapto_store          # store tests (54 unit + integration)
cargo test -p adapto_app            # app builder tests
cargo test --workspace              # all crates
```

## Cross-Compilation (for Linux deployment)

```bash
cargo zigbuild --release --target x86_64-unknown-linux-gnu
```

Requires `cargo-zigbuild`: `cargo install cargo-zigbuild`

## Changelog (MANDATORY)

**Every change to the framework MUST be documented in `CHANGELOG.md`.**

- Use [Keep a Changelog](https://keepachangelog.com/) format.
- Group by: Added, Changed, Fixed, Removed.
- Include file:line references for significant fixes.
- Bump version in root `Cargo.toml` `[workspace.package]` before publishing.
- After publishing to crates.io, tag the commit: `git tag v0.X.Y && git push --tags`.
