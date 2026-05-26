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
- **page()**: register a GET route with path params. Handler receives `RequestContext`
- **raw_get()**: register a GET route returning raw HTML (no layout)
- **get_route()**: register a GET route with layout wrapping
- **fallback_fn()**: catch-all for unmatched routes
- **FallbackResponse**: `Html(String)`, `Raw { body, content_type }`, `NotFound`

### API Reference

```rust
use adapto_app::{App, FallbackResponse, RequestContext};
use adapto_store::AdaptoStore;

let store = AdaptoStore::open(None)?;
// import data into store...

let store_clone = store.clone();

App::new("My App")
    .port(8080)
    .store(store)
    // Static page
    .page("/", |ctx| {
        render_home(ctx.store())
    })
    // Page with path params (axum syntax)
    .page("/users/:id", |ctx| {
        let id = ctx.param("id");
        render_user(ctx.store(), id)
    })
    // Nested params
    .page("/law/codes/:code/:chapter/:article", |ctx| {
        render_article(ctx.store(), ctx.param("code"), ctx.param("chapter"), ctx.param("article"))
    })
    // Fallback for dynamic/custom routes
    .fallback_fn(move |path| {
        if path == "/robots.txt" {
            return FallbackResponse::Raw {
                body: "User-agent: *\nAllow: /".into(),
                content_type: "text/plain; charset=utf-8",
            };
        }
        if path.starts_with("/sitemap") && path.ends_with(".xml") {
            return FallbackResponse::Raw {
                body: generate_sitemap(&path),
                content_type: "application/xml; charset=utf-8",
            };
        }
        FallbackResponse::NotFound
    })
    .run()
    .await?;
```

### RequestContext

```rust
ctx.store()         // &AdaptoStore
ctx.param("name")   // &str — path parameter value
ctx.path()          // &str — full request path
ctx.query           // String — query string
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
