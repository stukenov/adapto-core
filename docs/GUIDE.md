# Adapto Framework Guide

Version 0.2.5

---

## Table of Contents

1. [Overview](#1-overview)
2. [Getting Started](#2-getting-started)
3. [adapto_store -- Embedded Document DB](#3-adapto_store----embedded-document-db)
4. [adapto_app -- HTTP App Builder](#4-adapto_app----http-app-builder)
5. [adapto_ui -- Component Library](#5-adapto_ui----component-library)
6. [adapto_auth -- Authentication](#6-adapto_auth----authentication)
7. [adapto_audit -- Audit Logging](#7-adapto_audit----audit-logging)
8. [adapto_forms -- Form Validation](#8-adapto_forms----form-validation)
9. [adapto_db -- PostgreSQL Layer](#9-adapto_db----postgresql-layer)
10. [adapto_ai -- AI Integration](#10-adapto_ai----ai-integration)
11. [adapto_macros -- Derive Macros](#11-adapto_macros----derive-macros)
12. [adapto_runtime -- Shared Runtime Types](#12-adapto_runtime----shared-runtime-types)
13. [DSL Pipeline -- Parser, Compiler, SSR, Live](#13-dsl-pipeline----parser-compiler-ssr-live)
14. [Testing -- adapto_test_utils](#14-testing----adapto_test_utils)
15. [Deployment](#15-deployment)

---

## 1. Overview

Adapto is a Rust web framework for building data-driven sites and applications. It combines an embedded document database, an HTTP app builder, a CSS component library, authentication, audit logging, AI integration, and a template DSL into a cohesive workspace of 18 crates.

### Architecture

```
                        adapto (umbrella re-export)
                              |
       +----------------------+------------------------+
       |                      |                        |
  adapto_app             adapto_store             adapto_ui
  (HTTP server,          (embedded DB,            (HTML components,
   routes, WS)            WAL, indexes)            CSS bundle)
       |                      |
  adapto_auth            adapto_macros
  (passwords,            (#[derive(Resource)])
   JWT, sessions)
       |
  adapto_audit           adapto_forms
  (event logging,        (schemas, validation,
   sinks, PII)            sanitization)
       |
  adapto_ai              adapto_db
  (LLM clients,          (SQL generation,
   prompts, budgets)       migrations)
       |
  adapto_runtime         adapto_test_utils
  (Ctx, types,           (builders, mocks,
   state, config)          assertions)
       |
  adapto_parser -> adapto_compiler -> adapto_ssr -> adapto_live
  (pest DSL)     (template compile)  (server render) (hot reload)
       |
  adapto_client_protocol    adapto_cli
  (WebSocket wire format)   (CLI tooling)
```

### Crate Map

| Crate | Purpose | Status |
|---|---|---|
| `adapto_store` | Embedded document DB (JSON docs, BTree indexes, WAL) | Production |
| `adapto_app` | HTTP app builder (axum-based, routes, WebSocket, fallbacks) | Production |
| `adapto_ui` | HTML component library with CSS bundle | Production |
| `adapto_macros` | `#[derive(Resource)]` proc macro | WIP |
| `adapto` | Umbrella re-export crate | WIP |
| `adapto_parser` | Template/DSL parser (pest) | WIP |
| `adapto_compiler` | Template compiler | WIP |
| `adapto_ssr` | Server-side rendering | WIP |
| `adapto_live` | Live reload / hot updates | WIP |
| `adapto_runtime` | Shared types, context, state, config | WIP |
| `adapto_forms` | Form validation and sanitization | WIP |
| `adapto_auth` | Authentication (PBKDF2, JWT, sessions, CSRF, RBAC) | WIP |
| `adapto_audit` | Audit event logging with sinks and PII redaction | WIP |
| `adapto_ai` | AI/LLM integration with budget tracking | WIP |
| `adapto_db` | PostgreSQL layer (SQL generation, migrations) | WIP |
| `adapto_cli` | CLI tooling | WIP |
| `adapto_client_protocol` | Client-server WebSocket protocol | WIP |
| `adapto_test_utils` | Test helpers, builders, mocks, assertions | WIP |

---

## 2. Getting Started

### Installation

Add the crates you need to your `Cargo.toml`:

```toml
[dependencies]
adapto_store = "0.2"
adapto_app = "0.2"
adapto_ui = "0.2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
```

### Minimal Application

```rust
use adapto_app::{App, PageResponse, RequestContext};
use adapto_store::AdaptoStore;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let store = AdaptoStore::open(Some("./data"))?;

    // Seed some data
    let users = store.collection("users");
    users.create_index("email", true)?;
    users.insert(json!({"name": "Alice", "email": "alice@example.com"}))?;

    App::new("My App")
        .port(3000)
        .store(store)
        .page("/", |ctx: RequestContext| {
            let count = ctx.store().collection("users").count_all();
            PageResponse::Ok(format!("<h1>Users: {}</h1>", count))
        })
        .page("/users/:id", |ctx: RequestContext| {
            let id = ctx.param("id");
            match ctx.store().collection("users").find_by_id(id) {
                Ok(Some(doc)) => {
                    let name = doc.get_str("name").unwrap_or("Unknown");
                    PageResponse::Ok(format!("<h1>{}</h1>", name))
                }
                _ => PageResponse::NotFound,
            }
        })
        .run()
        .await
}
```

### Building and Testing

```bash
# Test a single crate
cargo test -p adapto_store

# Test the entire workspace
cargo test --workspace

# Build for release
cargo build --release
```

---

## 3. adapto_store -- Embedded Document DB

An embedded, document-oriented database -- SQLite for JSON documents. Thread-safe, WAL-backed persistent storage with BTree indexes.

### Opening a Store

```rust
use adapto_store::AdaptoStore;

// Persistent storage (WAL-backed)
let store = AdaptoStore::open(Some("./data"))?;

// In-memory only (fast, no disk I/O)
let store = AdaptoStore::open(None)?;
```

`AdaptoStore` is `Clone` -- cloned handles share the same underlying engine.

### Collections

A collection is a named group of JSON documents. Collections are created lazily on first access.

```rust
let users = store.collection("users");
```

### CRUD Operations

#### Insert

```rust
use serde_json::json;

// Insert one document, returns auto-generated ID
let id = users.insert(json!({"name": "Alice", "age": 30}))?;

// Insert many documents, returns all IDs
let ids = users.insert_many(vec![
    json!({"name": "Bob", "age": 25}),
    json!({"name": "Carol", "age": 28}),
])?;
```

#### Find

```rust
use adapto_store::{Query, Document};

// Find by ID
let doc: Option<Document> = users.find_by_id(&id)?;

// Find one matching document
let doc = users.find_one(Query::eq("name", "Alice"))?;

// Find all matching documents (returns a Cursor iterator)
let cursor = users.find(Query::new()); // all documents
for doc in cursor {
    println!("{}: {}", doc.id, doc.get_str("name").unwrap_or(""));
}
```

#### Update

```rust
use adapto_store::Update;

// Update all documents matching a query
let result = users.update(
    Query::eq("name", "Alice"),
    Update::Set(vec![("age".into(), json!(31))]),
)?;
// result.matched / result.modified

// Update by ID
let updated: bool = users.update_by_id(&id, Update::Set(vec![
    ("age".into(), json!(31)),
]))?;
```

#### Delete

```rust
// Delete matching documents, returns count
let count: u64 = users.delete(Query::eq("name", "Bob"))?;

// Delete by ID
let deleted: bool = users.delete_by_id(&id)?;
```

#### Count

```rust
let count = users.count(Query::eq("age", 30))?;
let total = users.count_all();
```

### Document API

Documents have these fields: `id: String`, `data: serde_json::Value`, `created_at`, `updated_at`.

```rust
// Dot-notation field access
let city = doc.get("address.city");       // Option<&Value>
let name = doc.get_str("name");           // Option<&str>
let age = doc.get_i64("age");             // Option<i64>
```

### Query Builder

```rust
use adapto_store::{Query, Filter, SortDir};

// Match all
Query::new()

// Equality
Query::eq("field", "value")

// Complex filters
Query::filter(Filter::And(vec![
    Filter::Gte("age".into(), json!(18)),
    Filter::Lt("age".into(), json!(65)),
]))
.sort("name", SortDir::Asc)
.limit(10)
.skip(20)
.project(&["name", "email"])
```

### Available Filters

| Filter | Description |
|---|---|
| `Filter::Eq(field, value)` | Equal |
| `Filter::Ne(field, value)` | Not equal |
| `Filter::Gt(field, value)` | Greater than |
| `Filter::Gte(field, value)` | Greater than or equal |
| `Filter::Lt(field, value)` | Less than |
| `Filter::Lte(field, value)` | Less than or equal |
| `Filter::In(field, values)` | Value in array |
| `Filter::Nin(field, values)` | Value not in array |
| `Filter::Exists(field)` | Field exists |
| `Filter::Regex(field, pattern)` | Regex match |
| `Filter::Contains(field, value)` | Array contains |
| `Filter::And(filters)` | Logical AND |
| `Filter::Or(filters)` | Logical OR |
| `Filter::Not(filter)` | Logical NOT |
| `Filter::All(field, values)` | Array contains all |

### Indexes

BTree-based indexes provide O(1) lookups via `find_eq`.

```rust
// Single-field index (unique = true for uniqueness constraint)
users.create_index("email", true)?;

// Compound index over multiple fields
users.create_compound_index(&["company", "role"], false)?;

// Drop an index
users.drop_index("email")?;

// List all indexes
let indexes: Vec<IndexInfo> = users.indexes();
```

### DiskCollection

For large datasets (100K+ documents). Data lives on disk with mmap reads; only indexes are held in memory.

```rust
let companies = store.disk_collection("companies")?;

// Bulk-insert (overwrites existing data)
companies.bulk_insert(docs)?;

// Query (same API as Collection)
let doc = companies.find_one(Query::eq("bin", "123456789012"))?;
let cursor = companies.find(Query::eq("city", "Almaty"));
let total = companies.count_all();

// Indexes
companies.create_index("bin", true)?;

// Get all index keys without loading full documents
let keys: Vec<String> = companies.index_keys("bin");
```

**Note:** DiskCollection requires the store to be opened with a path (`Some("./data")`).

### Tenant Scoping

Multi-tenant isolation at the document level. All operations through a `TenantScope` are automatically filtered by tenant ID.

```rust
let tenant = store.tenant("tenant-123");
let users = tenant.collection("users");

// Only inserts/queries/updates for tenant-123
let id = users.insert(json!({"name": "Alice"}))?;
let doc = users.find_one(Query::eq("name", "Alice"))?;
let count = users.count_all(); // only this tenant's documents
```

### Slugify and Slug Validation

Built-in Cyrillic-aware slugification (Russian + Kazakh).

```rust
use adapto_store::{slugify, is_valid_slug};

assert_eq!(slugify("Привет Мир"), "privet-mir");
assert_eq!(slugify("Қазақстан"), "qazaqstan");
assert_eq!(slugify("Аспирин\u{00AE} таблетки"), "aspirin-tabletki");

assert!(is_valid_slug("hello-world"));
assert!(!is_valid_slug("Hello World"));
```

### Store Management

```rust
store.compact()?;                    // WAL compaction: snapshot + truncate
let stats = store.stats();           // StoreStats
let names = store.collections();     // Vec<String>
store.drop_collection("temp")?;     // Drop a collection
```

### Data Import Rules

- **Never** store an entire JSON file as a single document. Each searchable entity must be a separate document.
- For arrays of items: iterate and `insert_many()` -- each item becomes its own document.
- **Always** create indexes on lookup fields after import.
- Use `Query::eq("field", value)` for single-entity lookups (O(1) via BTree index).
- Use `Query::new()` only for listing pages that need all documents.

---

## 4. adapto_app -- HTTP App Builder

A declarative HTTP app builder wrapping axum. Register resources, add handlers, and call `run()`.

### App Builder

```rust
use adapto_app::{App, PageResponse, RequestContext, FallbackResponse};
use adapto_store::AdaptoStore;

let store = AdaptoStore::open(None)?;

App::new("My App")
    .port(8080)                              // default: 3000
    .bind("0.0.0.0")                         // default: "0.0.0.0"
    .store(store)                            // pre-built store
    .store_path("./data")                    // OR path (creates store in run())
    .from_env()                              // reads PORT, BIND_ADDR, STORE_PATH
    .health_check("/health")                 // 200 "ok" endpoint
    .static_dir("/static", "./public")       // serve static files
    .run()
    .await?;
```

### Route Registration

#### Sync Handlers

Handlers receive `RequestContext` and return any type implementing `Into<PageResponse>` (including `String`, `&str`, and `PageResponse` directly).

```rust
// GET
.page("/", |ctx: RequestContext| {
    PageResponse::Ok("<h1>Home</h1>".to_string())
})

// GET with path parameters
.page("/users/:id", |ctx: RequestContext| {
    let id = ctx.param("id");
    PageResponse::Ok(format!("User: {}", id))
})

// POST
.post("/api/users", |ctx: RequestContext| {
    let body: serde_json::Value = ctx.body_json().unwrap();
    PageResponse::Json(json!({"created": true}))
})

// PUT
.put("/api/users/:id", |ctx: RequestContext| {
    PageResponse::Json(json!({"updated": true}))
})

// DELETE
.delete("/api/users/:id", |ctx: RequestContext| {
    PageResponse::Json(json!({"deleted": true}))
})

// PATCH
.patch("/api/users/:id", |ctx: RequestContext| {
    PageResponse::Json(json!({"patched": true}))
})
```

#### Async Handlers

For handlers that need `.await` (external API calls, async I/O):

```rust
.async_page("/external", |ctx: RequestContext| async move {
    let data = fetch_api().await;
    PageResponse::Ok(render(data))
})

.async_post("/api/async", |ctx: RequestContext| async move {
    PageResponse::Json(json!({"async": true}))
})

// Also: .async_put(), .async_delete(), .async_patch()
```

### RequestContext API

| Method | Return Type | Description |
|---|---|---|
| `ctx.store()` | `&AdaptoStore` | Access the document store |
| `ctx.param("name")` | `&str` | Path parameter value (empty string if missing) |
| `ctx.path()` | `&str` | Full request path |
| `ctx.method()` | `&Method` | HTTP method |
| `ctx.header("X-Custom")` | `Option<&str>` | Request header value |
| `ctx.headers()` | `&HeaderMap` | All request headers |
| `ctx.body_json::<T>()` | `Result<T>` | Parse body as JSON |
| `ctx.body_bytes()` | `&[u8]` | Raw body bytes |
| `ctx.body_str()` | `Result<&str>` | Body as UTF-8 string |
| `ctx.cookie("session")` | `Option<&str>` | Cookie value by name |
| `ctx.remote_addr()` | `Option<SocketAddr>` | Client IP address |
| `ctx.query_param("page")` | `Option<String>` | Single query parameter |
| `ctx.query_pairs()` | `Vec<(String, String)>` | All query parameters |
| `ctx.lang_code()` | `&str` | Language code (localized routes) |
| `ctx.lang_prefix()` | `&str` | Language URL prefix |

### PageResponse

```rust
pub enum PageResponse {
    Ok(String),              // 200 with HTML body
    NotFound,                // 404
    Redirect(String),        // 301 permanent redirect
    Json(Value),             // 200 with JSON body
    BadRequest(String),      // 400
    Forbidden(String),       // 403
    InternalError(String),   // 500
    Custom {                 // Arbitrary status/type/headers
        status: u16,
        body: String,
        content_type: String,
        headers: Vec<(String, String)>,
    },
}

// Convenience constructors
PageResponse::json(&serializable_value)       // serialize to JSON
PageResponse::with_status(418, "I'm a teapot")
PageResponse::raw(200, body, "text/plain")
```

`String` and `&str` automatically convert to `PageResponse::Ok(...)`.

### Localized Routes

Register routes for multiple languages with automatic prefix handling:

```rust
#[derive(Clone)]
enum Lang { Ru, Kz, En }

impl adapto_app::LangConfig for Lang {
    fn code(&self) -> &str {
        match self { Lang::Ru => "ru", Lang::Kz => "kk", Lang::En => "en" }
    }
    fn prefix(&self) -> &str {
        match self { Lang::Ru => "", Lang::Kz => "/kz", Lang::En => "/en" }
    }
}

App::new("Site")
    .languages(vec![Lang::Ru, Lang::Kz, Lang::En])
    .localized_page("/about", |ctx: RequestContext| {
        let lang = ctx.lang_code(); // "ru", "kk", or "en"
        format!("About page in {}", lang)
    })
    // Registers: /about, /kz/about, /en/about
```

### Fallback Handler

Custom handler for unmatched routes (trailing-slash normalization is automatic):

```rust
.fallback_fn(|path: String| {
    if path == "/robots.txt" {
        return FallbackResponse::Raw {
            body: "User-agent: *\nAllow: /".into(),
            content_type: "text/plain; charset=utf-8",
        };
    }
    FallbackResponse::NotFound
})
```

`FallbackResponse` variants: `Html(String)`, `HtmlNotFound(String)`, `Raw { body, content_type }`, `NotFound`.

### Middleware

Apply tower middleware layers:

```rust
.with_middleware(|router| {
    router.layer(tower_http::cors::CorsLayer::permissive())
})
```

### Graceful Shutdown

```rust
.on_shutdown(|| {
    println!("Server shutting down...");
})
```

### Error Handler

Custom error page renderer for HTTP error responses:

```rust
.error_handler(|status, msg| {
    format!("<h1>{} - {}</h1>", status.as_u16(), msg)
})
```

### TestClient

HTTP testing without TCP binding:

```rust
let app = App::new("Test")
    .page("/hello", |_| "Hello, World!")
    .post("/api/echo", |ctx: RequestContext| {
        let body: serde_json::Value = ctx.body_json().unwrap_or_default();
        PageResponse::Json(body)
    });

let client = app.test_client();

// GET
let resp = client.get("/hello").await;
assert_eq!(resp.status(), 200);
assert!(resp.text().contains("Hello"));

// POST with JSON body
let resp = client.post("/api/echo", r#"{"key":"val"}"#).await;
let json: serde_json::Value = resp.json();
assert_eq!(json["key"], "val");

// PUT / DELETE
let resp = client.put("/items/42", "{}").await;
let resp = client.delete("/items/42").await;

// Custom request with headers
let resp = client.request(
    Method::GET,
    "/protected",
    vec![("authorization", "Bearer token123")],
    None,
).await;
```

`TestResponse` API: `.status()`, `.text()`, `.json::<T>()`, `.header("name")`.

### Route Registration Order

Routes are matched in registration order. Register more specific paths first:

```rust
.page("/drugs/type/:slug", handler)      // before generic :slug
.page("/drugs/:slug/:section", handler)
.page("/drugs/:slug", handler)
```

### ResourceMeta Trait

For CRUD resources with auto-generated routes:

```rust
use adapto_app::ResourceMeta;

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

App::new("CRM")
    .resource::<Customer>()
    .run().await?;
```

---

## 5. adapto_ui -- Component Library

A handwritten CSS component library with typed HTML builders. All builders emit accessible HTML with ARIA attributes.

### CSS Bundle

```rust
use adapto_ui::{bundle_css, style_tag, TOKENS_CSS};

// Full CSS bundle as a string (all files in cascade order)
let css = bundle_css();

// As a <style> element
let tag = style_tag(); // <style>...all CSS...</style>

// Individual CSS files
adapto_ui::RESET_CSS       // Modern CSS reset
adapto_ui::TOKENS_CSS      // Design tokens
adapto_ui::TYPOGRAPHY_CSS   // Typographic scale
adapto_ui::BUTTON_CSS       // Button component
adapto_ui::INPUT_CSS        // Form controls
adapto_ui::CARD_CSS         // Card containers
adapto_ui::BADGE_CSS        // Badges
adapto_ui::ALERT_CSS        // Alert messages
adapto_ui::MODAL_CSS        // Modal dialogs
adapto_ui::TABLE_CSS        // Data tables
adapto_ui::NAV_CSS          // Navigation
adapto_ui::TOGGLE_CSS       // Toggle switch
adapto_ui::AVATAR_CSS       // Avatars
adapto_ui::TOOLTIP_CSS      // Tooltips
adapto_ui::BREADCRUMB_CSS   // Breadcrumbs
adapto_ui::PROGRESS_CSS     // Progress bars
adapto_ui::DROPDOWN_CSS     // Dropdowns
adapto_ui::FORM_GROUP_CSS   // Form groups
adapto_ui::LAYOUT_CSS       // Layout utilities
adapto_ui::SPACING_CSS      // Spacing utilities
adapto_ui::VISIBILITY_CSS   // Visibility utilities

// Iterate all CSS files
let files: Vec<(&str, &str)> = adapto_ui::all_css_files();
```

### HTML Escape

```rust
use adapto_ui::html_escape;

assert_eq!(html_escape("<script>alert('xss')</script>"),
           "&lt;script&gt;alert(&#x27;xss&#x27;)&lt;/script&gt;");
```

### Components

All components use a builder pattern with `.render()` returning an HTML string. Every component supports `.id()`, `.class()`, and `.attr()` for customization.

#### Button

Variants: `primary`, `secondary`, `destructive`, `ghost`, `outline`.
Sizes: `.small()`, default, `.large()`.

```rust
use adapto_ui::components::*;

// Basic buttons
Button::primary("Save").render()
Button::secondary("Cancel").disabled().render()
Button::destructive("Delete").large().render()

// With icon
Button::ghost("Edit").icon("<svg>...</svg>").render()

// As a link
Button::primary("Dashboard").href("/dashboard").render()

// Submit button
Button::primary("Submit").button_type(ButtonType::Submit).render()

// Loading state
Button::primary("Saving...").loading().render()

// With data attributes for live.js
Button::primary("Delete").action("delete_user").data_id("42").render()
```

#### Card

Variants: `elevated`, `flat`. Body/header/footer accept raw HTML.

```rust
Card::elevated("Card content here")
    .header("Card Title")
    .footer("Footer text")
    .hoverable()
    .render()
```

**Important:** Card body, header, and footer accept raw HTML. Use `html_escape()` on user-controlled content.

#### Alert

Levels: `info`, `success`, `warning`, `error`.

```rust
Alert::error("Something went wrong")
    .title("Error")
    .dismissible()
    .render()

Alert::info("Your profile was updated").render()
```

Error-level alerts use `aria-live="assertive"`, others use `"polite"`.

#### Badge

Variants: `Default`, `Info`, `Success`, `Warning`, `Error`.

```rust
Badge::new("Active", BadgeVariant::Success).render()
Badge::new("3 new", BadgeVariant::Info).render()
```

#### Input

Types: `text`, `email`, `password`, `number`, `search`, `tel`, `url`, `date`, `hidden`.

```rust
Input::text("username")
    .placeholder("Enter name")
    .value("Alice")
    .required()
    .render()

Input::email("email").error().render() // adds aria-invalid="true"
Input::password("pwd").disabled().render()
```

#### Textarea

```rust
Textarea::new("bio")
    .placeholder("Tell us about yourself")
    .rows(6)
    .value("Existing text")
    .required()
    .render()
```

#### Select

```rust
Select::new("status")
    .placeholder("Choose status")
    .option("active", "Active")
    .option("inactive", "Inactive")
    .options(&[("pending", "Pending"), ("archived", "Archived")])
    .selected("active")
    .required()
    .render()
```

#### Toggle

iOS-style toggle switch:

```rust
Toggle::new("dark_mode")
    .label("Dark Mode")
    .checked()
    .render()
```

Renders as a checkbox with `role="switch"`.

#### FormGroup

Composes a label, input, help text, and error message:

```rust
let input = Input::text("name").id("field-name").render();

FormGroup::new("Full Name", &input)
    .input_id("field-name")  // links <label for="...">
    .help("Your legal name")
    .error("Name is required")
    .required()
    .render()
```

#### Form

```rust
Form::new()
    .action("/api/users")
    .method("POST")
    .child(&input_html)
    .child(&Button::primary("Submit").button_type(ButtonType::Submit).render())
    .render()
```

#### Table

```rust
Table::new(&["Name", "Email", "Role"])
    .row(&["Alice", "alice@ex.com", "Admin"])
    .row(&["Bob", "bob@ex.com", "User"])
    .caption("Team Members")
    .striped()
    .hoverable()
    .compact()
    .render()
```

**Note:** Table cell content is raw HTML. Escape user content with `html_escape()`.

#### Modal

```rust
Modal::new("confirm-delete", "Are you sure you want to delete this item?")
    .title("Confirm Deletion")
    .footer(&Button::destructive("Delete").render())
    .render()
```

Renders with `role="dialog"`, `aria-modal="true"`, and `aria-labelledby`.

#### Pagination

```rust
Pagination::new(3, 10)   // current page 3 of 10
    .base_url("/users")
    .param_name("page")  // default: "page"
    .render()
```

Generates prev/next links and ellipsis for large page counts.

#### Toast

```rust
Toast::success("Changes saved!")
    .duration_ms(3000)
    .render()

Toast::error("Upload failed").render()
```

Levels: `info`, `success`, `warning`, `error`. Default duration: 5000ms.

#### Breadcrumb

```rust
Breadcrumb::render(&[
    ("Home", Some("/")),
    ("Products", Some("/products")),
    ("Widget", None),  // current page (no link)
])
```

#### Avatar

```rust
// With initials
Avatar::initials("JD", "John Doe")
    .status("online")
    .small()
    .render()

// With image
Avatar::image("/photo.jpg", "Jane Doe")
    .large()
    .render()
```

#### Progress

```rust
Progress::new(75).label("Upload progress").render()
Progress::indeterminate().render()
```

Renders with `role="progressbar"`, `aria-valuenow`, `aria-valuemin`, `aria-valuemax`.

#### Skeleton

Loading placeholders:

```rust
Skeleton::text(3)                      // 3 text lines
Skeleton::card()                       // card placeholder
Skeleton::circle()                     // avatar placeholder
Skeleton::rect("200px", "100px")       // custom rectangle
```

#### Spinner

```rust
Spinner::render(Some("Loading data"))
Spinner::render(None) // defaults to "Loading"
```

---

## 6. adapto_auth -- Authentication

Comprehensive authentication: password hashing, JWT tokens, session management, CSRF protection, RBAC, and rate limiting.

### Password Hashing

PBKDF2-HMAC-SHA256 with 100,000 iterations, 16-byte random salt, constant-time comparison.

```rust
use adapto_auth::password::{hash_password, verify_password, validate_password_strength};

// Hash a password
let hash = hash_password("MyP@ssw0rd!");
// Format: "pbkdf2-sha256$100000$<base64-salt>$<base64-hash>"

// Verify
verify_password("MyP@ssw0rd!", &hash)?;           // Ok(())
verify_password("wrong", &hash).unwrap_err();      // AuthError::PasswordMismatch

// Deterministic hashing (for testing)
use adapto_auth::password::hash_password_with_salt;
let hash = hash_password_with_salt("test", &[1u8; 16]);

// Password strength validation
let issues = validate_password_strength("weak");
// ["must be at least 8 characters", "must contain an uppercase letter", ...]

let issues = validate_password_strength("MyStr0ng!Pass");
assert!(issues.is_empty());
```

### JWT (JSON Web Tokens)

HMAC-SHA256 signed tokens. No external dependencies (no `jsonwebtoken` crate).

```rust
use adapto_auth::jwt::{Claims, encode, decode, decode_without_verify};

let secret = b"my-secret-key-at-least-32-bytes!";

// Create claims
let claims = Claims::new("user-123", 3600)  // subject, TTL in seconds
    .with_issuer("my-app")
    .with_audience("api")
    .with_claim("role", json!("admin"));

// Encode
let token = encode(&claims, secret);

// Decode (verifies signature and expiry)
let decoded = decode(&token, secret)?;
assert_eq!(decoded.sub, "user-123");
assert_eq!(decoded.iss.as_deref(), Some("my-app"));
assert_eq!(decoded.custom.get("role").unwrap(), "admin");

// Decode without verification (for inspection only)
let claims = decode_without_verify(&token)?;

// Check expiry
if claims.is_expired() { /* ... */ }
```

#### Claims Fields

| Field | Type | Description |
|---|---|---|
| `sub` | `String` | Subject (user ID) |
| `iat` | `u64` | Issued at (Unix timestamp) |
| `exp` | `u64` | Expiration (Unix timestamp) |
| `iss` | `Option<String>` | Issuer |
| `aud` | `Option<String>` | Audience |
| `custom` | `HashMap<String, Value>` | Custom claims |

### Session Management

```rust
use adapto_auth::session_store::{SessionStore, InMemorySessionStore, SessionData};
use std::time::Duration;

let store = InMemorySessionStore::new();

// Create a session
let mut data = SessionData::new("user-123");
data.set("theme", json!("dark"));
store.create("sess-abc", data)?;

// Retrieve (also updates last_accessed)
let session = store.get("sess-abc")?;
assert_eq!(session.user_id, "user-123");
assert_eq!(session.get("theme").unwrap(), "dark");

// Update
let mut session = store.get("sess-abc")?;
session.set("role", json!("admin"));
store.update("sess-abc", session)?;

// Check existence
assert!(store.exists("sess-abc"));

// Destroy
store.destroy("sess-abc")?;

// Cleanup expired sessions (removes sessions idle for > max_age)
let removed = store.cleanup_expired(Duration::from_secs(3600));

// Metrics
store.len();         // number of active sessions
store.is_empty();
store.session_ids(); // Vec<String>
```

The `SessionStore` trait can be implemented for custom backends (Redis, database, etc.):

```rust
pub trait SessionStore: Send + Sync {
    fn create(&self, session_id: &str, data: SessionData) -> Result<(), AuthError>;
    fn get(&self, session_id: &str) -> Result<SessionData, AuthError>;
    fn update(&self, session_id: &str, data: SessionData) -> Result<(), AuthError>;
    fn destroy(&self, session_id: &str) -> Result<(), AuthError>;
    fn exists(&self, session_id: &str) -> bool;
    fn cleanup_expired(&self, max_age: Duration) -> usize;
}
```

### CSRF Protection

HMAC-SHA256 timestamped tokens. Stateless -- the server validates the signature and checks the token age (max 1 hour).

```rust
use adapto_auth::csrf::{generate_token, validate_token};

let secret = b"csrf-secret-key-32-bytes!!!!!!!!";
let token = generate_token(secret);
validate_token(&token, secret)?; // Ok(())
```

### Session Token Signing

HMAC-SHA256 signed session IDs prevent forgery in cookies:

```rust
use adapto_auth::session_token::{sign_session_id, verify_session_id};

let secret = b"session-secret-key-32-bytes!!!!!";
let signed = sign_session_id("sess-abc", secret);
// Format: "sess-abc.<base64-hmac>"

let id = verify_session_id(&signed, secret)?;
assert_eq!(id, "sess-abc");
```

### Middleware Configuration

`AuthConfig` centralizes authentication settings:

```rust
use adapto_auth::middleware::*;

let config = AuthConfig::new(b"my-secret-key-32-bytes-long!!!!!")
    .enable_jwt()
    .disable_csrf()         // default: enabled
    .disable_sessions()     // default: enabled
    .public_path("/health")
    .public_path("/static/*");  // wildcard matching

// Check if a path is public
assert!(config.is_public("/static/main.js"));
assert!(!config.is_public("/api/private"));

// Generate a CSRF token
let csrf_token = generate_csrf_token(&config);

// Validate CSRF header
validate_csrf_header(&config, Some(&csrf_token))?;

// Validate Bearer token (Authorization header)
let claims = validate_bearer_token(&config, Some("Bearer <jwt>"))?;

// Validate session cookie
let session_id = validate_session_cookie(&config, Some(&signed_cookie))?;

// Issue JWT
let jwt = issue_jwt(&config, "user-123", 3600);

// Issue JWT with custom claims
let claims = Claims::new("user-123", 3600).with_issuer("my-app");
let jwt = issue_jwt_with_claims(&config, &claims);

// Sign a session ID for cookies
let signed = sign_session(&config, "sess-abc");
```

### RBAC (Role-Based Access Control)

```rust
use adapto_auth::rbac::{RbacStore, Role};
use adapto_runtime::types::UserId;
use std::collections::HashSet;

let mut rbac = RbacStore::new();

// Define roles with permissions
rbac.add_role(Role {
    name: "admin".into(),
    permissions: HashSet::from([
        "users.read".into(), "users.write".into(), "users.delete".into(),
    ]),
});

rbac.add_role(Role {
    name: "viewer".into(),
    permissions: HashSet::from(["users.read".into()]),
});

// Assign roles to users
let user_id = UserId::default();
rbac.assign_role(&user_id, "admin");

// Check permissions
let perms = rbac.get_permissions(&user_id);
assert!(perms.has("users.write"));
assert!(perms.has_all(&["users.read", "users.write"]));

// Check role membership
assert!(rbac.has_role(&user_id, "admin"));

// Revoke a role
rbac.revoke_role(&user_id, "admin");
```

### Rate Limiting

Token-bucket rate limiter, keyed per session:

```rust
use adapto_auth::rate_limit::RateLimiter;

let mut limiter = RateLimiter::new(10); // 10 tokens/second

// Check and consume a token
limiter.check("session-1")?;    // Ok(()) -- token consumed
limiter.check("session-1")?;    // Ok(()) -- still within limit

// After exhausting tokens:
// limiter.check("session-1")   // Err(()) -- rate limit exceeded

// Reset a session's bucket to full
limiter.reset("session-1");

// Remove a session (e.g., on disconnect)
limiter.remove("session-1");
```

### AuthError

All auth operations return `Result<_, AuthError>`:

| Variant | Description |
|---|---|
| `InvalidCsrfToken` | CSRF token signature mismatch |
| `ExpiredCsrfToken` | CSRF token older than 1 hour |
| `InvalidSessionSignature` | Session cookie signature invalid |
| `MalformedToken` | Token format unrecognizable |
| `InvalidPasswordHash` | Password hash format invalid |
| `PasswordMismatch` | Password verification failed |
| `InvalidJwt(String)` | JWT structure or signature invalid |
| `ExpiredJwt` | JWT past expiration |
| `SessionNotFound` | Session does not exist |
| `SessionExpired` | Session past max age |
| `RateLimitExceeded` | Token bucket empty |
| `Unauthorized` | No credentials provided |
| `Forbidden(String)` | Insufficient permissions |

---

## 7. adapto_audit -- Audit Logging

Structured audit event logging with pluggable sinks, filtering, and PII redaction.

### AuditEvent

```rust
use adapto_audit::event::{AuditEvent, AuditStatus};
use adapto_runtime::context::Ctx;

// Create from a request context
let event = AuditEvent::new("user.login", &ctx, "authenticate")
    .with_metadata("ip", json!("192.168.1.1"))
    .with_metadata("user_agent", json!("Mozilla/5.0"))
    .success();

// Mark as failure
let event = AuditEvent::new("user.login", &ctx, "authenticate")
    .failure("invalid credentials");

// Mark as denied
let event = AuditEvent::new("resource.delete", &ctx, "delete")
    .denied();
```

#### AuditEvent Fields

| Field | Type | Description |
|---|---|---|
| `id` | `Uuid` | Auto-generated event ID |
| `event` | `String` | Event name (e.g., "user.login") |
| `tenant_id` | `Option<TenantId>` | Tenant context |
| `user_id` | `Option<UserId>` | Acting user |
| `route` | `String` | Request route |
| `action` | `String` | Action name |
| `timestamp` | `DateTime<Utc>` | When it happened |
| `request_id` | `RequestId` | Request correlation ID |
| `metadata` | `HashMap<String, Value>` | Arbitrary key-value pairs |
| `status` | `AuditStatus` | `Success`, `Failure(reason)`, or `Denied` |

### Sinks

Sinks implement the `AuditSink` trait:

```rust
pub trait AuditSink: Send + Sync {
    fn write(&self, event: AuditEvent);
}
```

#### InMemoryAuditSink

Test-friendly, accumulates events in a Vec:

```rust
use adapto_audit::sink::InMemoryAuditSink;

let sink = InMemoryAuditSink::new();
sink.write(event);

let events = sink.events();       // Vec<AuditEvent>
let count = sink.len();
sink.clear();
```

#### FileSink

Appends JSON-lines to a file:

```rust
use adapto_audit::sink::FileSink;

let sink = FileSink::new("./audit.log")?;
sink.write(event); // appends one JSON line
```

#### LogAuditSink

Emits structured `tracing` log lines at `info` level:

```rust
use adapto_audit::sink::LogAuditSink;

let sink = LogAuditSink;
sink.write(event); // tracing::info!(...)
```

#### ChannelAuditSink

Forwards events through a Tokio unbounded channel for async processing:

```rust
use adapto_audit::sink::ChannelAuditSink;

let (sink, mut receiver) = ChannelAuditSink::new();
sink.write(event);

// In a background task:
while let Some(event) = receiver.recv().await {
    // persist to database
}
```

#### CompositeSink

Fan-out to multiple sinks:

```rust
use adapto_audit::sink::CompositeSink;

let composite = CompositeSink::new()
    .add(InMemoryAuditSink::new())
    .add(FileSink::new("./audit.log")?);

composite.write(event); // writes to both sinks
```

#### RetentionSink

Wraps InMemoryAuditSink with a max event count (FIFO eviction):

```rust
use adapto_audit::sink::RetentionSink;

let sink = RetentionSink::new(1000); // keep last 1000 events
sink.write(event);
let recent = sink.events();
```

### Filtering

Query recorded events with `AuditFilter`:

```rust
use adapto_audit::filter::{AuditFilter, StatusFilter};

let filter = AuditFilter::new()
    .event("user.login")
    .action("authenticate")
    .user("user-123")
    .tenant("tenant-456")
    .status(StatusFilter::Failure)
    .route_prefix("/api/")
    .from(start_time)
    .to(end_time);

// Query an InMemoryAuditSink
let results = memory_sink.query(&filter);
let count = memory_sink.count_matching(&filter);
```

### PII Redaction

Redact sensitive metadata fields before logging:

```rust
use adapto_audit::redact::PiiRedactor;

let redactor = PiiRedactor::new(); // built-in sensitive fields
// Default fields: email, password, ssn, phone, credit_card, token, secret, api_key

// Customize
let redactor = PiiRedactor::new()
    .with_fields(&["email", "ssn"])        // replace default list
    .add_field("national_id")              // add to list
    .replacement("[HIDDEN]");              // default: "[REDACTED]"

// Redact in-place
redactor.redact(&mut event);

// Redact returning a new copy
let safe_event = redactor.redact_clone(&event);

// Check if a key is sensitive
assert!(redactor.is_sensitive("user_email"));
```

---

## 8. adapto_forms -- Form Validation

Declarative form schema validation with type checking, constraints, cross-field rules, and sanitization.

### FormSchema

```rust
use adapto_forms::schema::*;
use adapto_forms::rules::FormRule;
use adapto_forms::sanitize::{Sanitizer, SanitizerPipeline};

let schema = FormSchema::new("registration")
    .field(
        FieldSchema::new("name", FieldType::String)
            .required()
            .min_length(2)
            .max_length(120)
            .label("Full Name"),
    )
    .field(
        FieldSchema::new("email", FieldType::Email)
            .required()
            .label("Email Address"),
    )
    .field(
        FieldSchema::new("age", FieldType::Integer)
            .required()
            .min(18)
            .max(150),
    )
    .field(
        FieldSchema::new("role", FieldType::Enum(vec![
            "admin".into(), "user".into(), "viewer".into(),
        ]))
        .required(),
    )
    .field(
        FieldSchema::new("bio", FieldType::Optional(Box::new(FieldType::String)))
            .max_length(1000),
    )
    // Cross-field rules
    .rule(FormRule::fields_match("password", "password_confirm"))
    .rule(FormRule::required_if("company", "role", json!("admin")));
```

### FieldType

| Type | JSON Type | Validation |
|---|---|---|
| `String` | string | Must be a JSON string |
| `Email` | string | Must contain `@` and valid domain |
| `Integer` | number | Must be i64/u64 |
| `Decimal` | number | Must be numeric (f64/i64/u64) |
| `Boolean` | boolean | Must be true/false |
| `Uuid` | string | Must parse as UUID |
| `DateTime` | string | Must be ISO 8601 / RFC 3339 |
| `Enum(variants)` | string | Must be one of the listed values |
| `Optional(inner)` | any/null | Null/absent is valid; non-null validated against inner |

### Constraints

```rust
FieldSchema::new("name", FieldType::String)
    .min_length(2)                   // Constraint::MinLength
    .max_length(100)                 // Constraint::MaxLength
    .min(0)                          // Constraint::Min (for numbers)
    .max(999)                        // Constraint::Max (for numbers)
    .pattern("^[a-z]+$")            // Constraint::Pattern
    .constraint(Constraint::Unique)  // Constraint::Unique (external check)
    .constraint(Constraint::Custom("phone_format".into()))
```

### Validation

```rust
use serde_json::json;

let data = json!({"name": "Al", "email": "bad", "age": 15}).as_object().unwrap().clone();
let result = schema.validate(&data);

if !result.is_valid() {
    // Per-field errors
    for err in result.field_errors("email") {
        println!("{}: {} ({})", err.field, err.message, err.code);
    }

    // All errors
    for err in result.all_errors() {
        println!("{}: {}", err.field, err.message);
    }
}
```

`ValidationResult` API:
- `is_valid()` -- true if no errors
- `add_error(field, code, message)` -- add an error programmatically
- `field_errors(field)` -- errors for a specific field
- `all_errors()` -- all errors across all fields

### Cross-Field Rules

```rust
use adapto_forms::rules::FormRule;

// Fields must match
FormRule::fields_match("password", "password_confirm")
FormRule::fields_match_with_message("password", "password_confirm", "Passwords don't match")

// Conditionally required
FormRule::required_if("company_name", "account_type", json!("business"))
FormRule::required_unless("phone", "email_verified", json!(true))

// Mutual exclusion
FormRule::mutually_exclusive(&["phone", "fax"])

// At least one required
FormRule::at_least_one_of(&["email", "phone", "address"])

// Custom rule
FormRule::Custom {
    name: "age_range".into(),
    validator: Box::new(|data| {
        // Return Some(ValidationError) on failure, None on success
        None
    }),
}
```

### Sanitizer Pipeline

Apply transformations to form data before validation:

```rust
use adapto_forms::sanitize::{Sanitizer, SanitizerPipeline};

let pipeline = SanitizerPipeline::new()
    .field("name", vec![Sanitizer::Trim, Sanitizer::StripHtml])
    .field("email", vec![Sanitizer::Trim, Sanitizer::Lowercase])
    .field("code", vec![Sanitizer::Trim, Sanitizer::Uppercase])
    .field("bio", vec![Sanitizer::Trim, Sanitizer::TruncateTo(500)]);

// Validate with sanitization (modifies data in-place, then validates)
let result = schema.validate_and_sanitize(&mut data, &pipeline);

// Or apply separately
pipeline.apply(&mut data);          // mutates a Map
pipeline.apply_value(&mut value);   // mutates a Value (if it's an Object)
```

Available sanitizers: `Trim`, `Lowercase`, `Uppercase`, `StripHtml`, `TruncateTo(max_len)`.

---

## 9. adapto_db -- PostgreSQL Layer

SQL generation, query building, migrations, and an in-memory pool for testing.

### DatabasePool Trait

```rust
use adapto_db::pool::{DatabasePool, DbResult};

pub trait DatabasePool: Send + Sync {
    fn execute(&self, sql: &str, params: &[Value]) -> BoxFuture<'_, DbResult<u64>>;
    fn query_one(&self, sql: &str, params: &[Value]) -> BoxFuture<'_, DbResult<Value>>;
    fn query_all(&self, sql: &str, params: &[Value]) -> BoxFuture<'_, DbResult<Vec<Value>>>;
    fn in_transaction(&self, ops: Vec<(String, Vec<Value>)>) -> BoxFuture<'_, DbResult<()>>;
    fn health_check(&self) -> BoxFuture<'_, DbResult<()>>;
}
```

### InMemoryPool

For testing without a database:

```rust
use adapto_db::pool::InMemoryPool;

let pool = InMemoryPool::new();
pool.create_table("users");
pool.insert_row("users", serde_json::Map::from_iter([
    ("name".into(), json!("Alice")),
]));

let rows = pool.table_rows("users");
let count = pool.table_count("users");
pool.clear_table("users");
```

### SQL Generation

Generate parameterized SQL (PostgreSQL `$1`, `$2`, ... placeholders):

```rust
use adapto_db::sql::*;
use std::collections::BTreeMap;

let mut data = BTreeMap::new();
data.insert("name".into(), json!("Alice"));
data.insert("email".into(), json!("alice@ex.com"));

// INSERT
let (sql, params) = insert_sql("users", &data);
// "INSERT INTO users (email, name) VALUES ($1, $2) RETURNING *"

// UPDATE
let (sql, params) = update_sql("users", "id", &json!("uuid-1"), &data);
// "UPDATE users SET email = $1, name = $2 WHERE id = $3 RETURNING *"

// DELETE
let (sql, params) = delete_sql("users", "id", &json!("uuid-1"));
// "DELETE FROM users WHERE id = $1"

// SELECT by ID
let (sql, params) = select_by_id_sql("users", "id", &json!(42));
// "SELECT * FROM users WHERE id = $1"

// COUNT
let sql = count_sql("users");
// "SELECT COUNT(*) as count FROM users"

// UPSERT
let (sql, params) = upsert_sql("users", "email", &data);
// "INSERT INTO ... ON CONFLICT (email) DO UPDATE SET ... RETURNING *"

// TRUNCATE
let sql = truncate_sql("users");
// "TRUNCATE TABLE users CASCADE"
```

### Query Builder

```rust
use adapto_db::query::{Query, Condition, Direction};

let (sql, params) = Query::table("users")
    .where_eq("status", json!("active"))
    .where_gt("age", json!(18))
    .where_like("name", "%alice%")
    .where_in("role", vec![json!("admin"), json!("editor")])
    .where_not_null("email")
    .order_by("created_at", Direction::Desc)
    .limit(10)
    .offset(20)
    .to_sql();
```

Available conditions: `where_eq`, `where_ne`, `where_gt`, `where_lt`, `where_gte`, `where_lte`, `where_like`, `where_in`, `where_null`, `where_not_null`, `and`, `or`.

### Migrations

```rust
use adapto_db::migration::{Migration, MigrationPlan, ColumnDef};
use adapto_db::runner::MigrationRunner;

// Define migrations
let migrations = vec![
    Migration {
        version: "001".into(),
        name: "create_users".into(),
        up: "CREATE TABLE users (id UUID PRIMARY KEY, name TEXT NOT NULL)".into(),
        down: "DROP TABLE users".into(),
    },
    Migration {
        version: "002".into(),
        name: "add_email".into(),
        up: "ALTER TABLE users ADD COLUMN email TEXT UNIQUE".into(),
        down: "ALTER TABLE users DROP COLUMN email".into(),
    },
];

// Generate CREATE TABLE from column definitions
let migration = MigrationPlan::create_table("posts", vec![
    ColumnDef::new("id", "UUID").primary_key(),
    ColumnDef::new("title", "TEXT").not_null(),
    ColumnDef::new("slug", "TEXT").not_null().unique(),
    ColumnDef::new("created_at", "TIMESTAMPTZ").not_null().default_value("NOW()"),
]);

// Run migrations
let pool = InMemoryPool::new();
let mut runner = MigrationRunner::new(&pool)
    .add_all(migrations);

// Check pending
let pending = runner.pending(); // Vec<&Migration>

// Apply all pending
let applied = runner.run_pending().await?; // Vec<String> of versions applied

// Rollback last migration
let rolled_back = runner.rollback_last().await?; // Option<String>

// View status
let status = runner.status(); // Vec<MigrationStatus { version, name, applied }>

// Mark already applied (e.g., baseline)
runner.mark_applied("001");
```

### InMemoryRepository

Tenant-scoped in-memory repository for tests and prototyping:

```rust
use adapto_db::repository::InMemoryRepository;
use adapto_runtime::types::TenantId;

let repo: InMemoryRepository<User> = InMemoryRepository::new();
let tenant = TenantId::default();

// CRUD
let user = repo.create(&tenant, uuid, User { name: "Alice".into() });
let found = repo.find(&tenant, &uuid);
let updated = repo.update(&tenant, &uuid, modified_user);
let deleted = repo.delete(&tenant, &uuid);

// Query
let all = repo.for_tenant(&tenant);
let results = repo.search(&tenant, |u| u.name.contains("Ali"));
let count = repo.count(&tenant);

// Admin (bypasses tenant isolation)
let everything = repo.all_unscoped();
```

### DbError

| Variant | Description |
|---|---|
| `NotFound` | Record not found |
| `Duplicate` | Duplicate record (unique constraint) |
| `TenantScopeRequired` | Missing tenant context |
| `QueryError(String)` | SQL query failed |
| `MigrationError(String)` | Migration execution failed |
| `ConnectionError(String)` | Database connection failed |

---

## 10. adapto_ai -- AI Integration

LLM client abstraction, prompt templating, response caching, budget tracking, PII redaction, and action orchestration.

### LlmClient Trait

```rust
use adapto_ai::client::*;

pub trait LlmClient: Send + Sync {
    fn complete(&self, request: CompletionRequest)
        -> BoxFuture<'_, Result<CompletionResponse, AiError>>;

    fn complete_json(&self, request: CompletionRequest)
        -> BoxFuture<'_, Result<Value, AiError>>;
}
```

### CompletionRequest

```rust
let request = CompletionRequest::new("gpt-4")
    .system("You are a helpful assistant")
    .user("Explain Rust lifetimes in one paragraph")
    .temperature(0.7)
    .max_tokens(200)
    .stop_sequence("\n\n");
```

### MockLlmClient

For testing without calling real APIs:

```rust
use adapto_ai::client::MockLlmClient;

let client = MockLlmClient::new()
    .with_response("Mock answer");

let resp = client.complete(request).await?;
assert_eq!(resp.content, "Mock answer");
assert_eq!(client.call_count(), 1);

// Sequential responses
let client = MockLlmClient::new().with_responses(vec![
    CompletionResponse { content: "first".into(), ... },
    CompletionResponse { content: "second".into(), ... },
]);

// JSON responses
let client = MockLlmClient::new()
    .with_json_response(json!({"answer": 42}));
let result = client.complete_json(request).await?;

// Inspect recorded requests
let requests = client.recorded_requests();
```

### MultiProviderClient

Route requests to different providers:

```rust
use adapto_ai::client::MultiProviderClient;

let multi = MultiProviderClient::new()
    .add("openai", openai_client)
    .add("anthropic", anthropic_client);

let client = multi.get("openai").unwrap();
let providers = multi.providers(); // ["openai", "anthropic"]
```

### PromptTemplate

Mustache-style variable substitution (`{{variable}}`):

```rust
use adapto_ai::prompt::{PromptTemplate, PromptLibrary};
use std::collections::HashMap;

let template = PromptTemplate::new("summarize", "Summarize this: {{text}}")
    .with_system("You are a concise summarizer");

// Check required variables
let vars = template.required_variables(); // ["text"]

// Render
let mut vars = HashMap::new();
vars.insert("text".into(), "Long article...".into());
let rendered = template.render(&vars)?;
// rendered.system = Some("You are a concise summarizer")
// rendered.user = "Summarize this: Long article..."
```

#### PromptLibrary

```rust
let library = PromptLibrary::new()
    .add(PromptTemplate::new("summarize", "Summarize: {{text}}"))
    .add(PromptTemplate::new("translate", "Translate to {{lang}}: {{text}}"));

let rendered = library.render("summarize", &vars)?;
let names = library.list(); // ["summarize", "translate"]
```

### ResponseCache

TTL-based cache with LRU eviction:

```rust
use adapto_ai::cache::ResponseCache;
use std::time::Duration;

let cache = ResponseCache::new(Duration::from_secs(300), 1000);

// Generate a deterministic cache key
let key = ResponseCache::cache_key("gpt-4", "my prompt");

// Set and get
cache.set(&key, "cached response");
let hit = cache.get(&key); // Some("cached response")

// Invalidate
cache.invalidate(&key);

// Clear all
cache.clear();

// Statistics
let stats = cache.stats();
// stats.entries, stats.total_hits, stats.max_entries, stats.ttl

// Cleanup expired entries
let removed = cache.cleanup_expired();
```

### Budget Tracking

Per-tenant token and cost budgets:

```rust
use adapto_ai::budget::{BudgetTracker, TenantBudget};
use adapto_runtime::types::TenantId;

let tracker = BudgetTracker::new();
let tenant = TenantId::default();

// Set budget
tracker.set_budget(&tenant, TenantBudget {
    total_tokens: 1_000_000,
    used_tokens: 0,
    total_cost: 50.0,
    used_cost: 0.0,
    max_tokens_per_request: Some(4096),
});

// Check before request
tracker.check_budget(&tenant, 500)?; // Ok(())

// Record usage after request
tracker.record_usage(&tenant, 500, 0.015);

// Query usage
let budget = tracker.get_usage(&tenant).unwrap();

// Reset usage counters (keeps limits)
tracker.reset(&tenant);
```

### PII Redaction

Regex-based PII detection and redaction for AI inputs:

```rust
use adapto_ai::pii::PiiRedactor;

// Pre-loaded with common patterns (email, phone, SSN, credit card)
let redactor = PiiRedactor::with_defaults();

// Redact: replace matches with tokens
let result = redactor.redact("Contact alice@example.com or 555-123-4567");
// result.output = "Contact [EMAIL] or [PHONE]"
// result.redacted_count = 2
// result.redacted_types = ["email", "phone"]

// Mask: replace with asterisks
let result = redactor.mask("SSN: 123-45-6789");
// result.output = "SSN: ***********"

// Custom patterns
let mut redactor = PiiRedactor::new();
redactor.add_pattern("iin", r"\b\d{12}\b", "[IIN]");
```

### PiiPolicy

Configuration for how PII is handled per AI action:

```rust
use adapto_ai::pii::PiiPolicy;

// PiiPolicy::None    -- no redaction
// PiiPolicy::Redact  -- replace with tokens
// PiiPolicy::Mask    -- replace with asterisks
// PiiPolicy::Hash    -- replace with hashes
```

### Model Configuration

```rust
use adapto_ai::model::{ModelConfig, ModelProvider, ModelRouter};

let mut router = ModelRouter::new();

router.add_model(ModelConfig {
    name: "gpt-4".into(),
    provider: ModelProvider::OpenAI,
    endpoint: None,
    api_key_env: Some("OPENAI_API_KEY".into()),
    max_tokens: Some(8192),
    default_temperature: 0.7,
    cost_per_1k_input_tokens: 0.03,
    cost_per_1k_output_tokens: 0.06,
});

router.set_default("gpt-4");
router.set_fallback("gpt-3.5-turbo");

// Resolve a model
let config = router.resolve("gpt-4").unwrap();
let config = router.resolve("default").unwrap(); // returns gpt-4
let config = router.resolve_with_fallback("unavailable"); // returns fallback

// Estimate cost
let cost = router.estimate_cost("gpt-4", 1000, 500); // Some(0.06)
```

`ModelProvider` variants: `OpenAI`, `Anthropic`, `Custom(String)`, `Local`.

### AI Action Executor

Orchestrates the full AI action lifecycle:

```rust
use adapto_ai::action::{AiActionExecutor, AiActionDef, AiRequest, TokenUsage};

let mut executor = AiActionExecutor::new();

executor.register_action(AiActionDef {
    name: "summarize".into(),
    model: "gpt-4".into(),
    temperature: Some(0.3),
    max_tokens: Some(500),
    audit: true,
    pii: Some(PiiPolicy::Redact),
    max_retries: 2,
    timeout_ms: Some(30_000),
    ..Default::default()
});

let response = executor.execute(AiRequest {
    action: "summarize".into(),
    input: json!({"text": "Long article..."}),
    tenant_id: Some(tenant_id),
    user_id: Some(user_id),
    request_id: RequestId::default(),
}).await?;
```

### Trace Collector

Observability for AI action executions:

```rust
use adapto_ai::trace::{TraceCollector, TraceStatus};

let collector = TraceCollector::new();

// Start a trace
let trace = collector.start_trace(&request, "gpt-4");

// Complete it
let mut trace = trace;
trace.status = TraceStatus::Completed;
trace.latency_ms = Some(250);
trace.tokens = Some(TokenUsage::new(100, 50));
collector.complete_trace(trace);

// Query traces
let all = collector.get_traces();
let tenant_traces = collector.get_traces_for_tenant(&tenant_id);
```

### AiError

| Variant | Description |
|---|---|
| `ActionNotFound(String)` | AI action not registered |
| `ModelNotFound(String)` | Model configuration missing |
| `ExecutionFailed(String)` | LLM call failed |
| `Timeout(u64)` | Action timed out |
| `OutputValidationFailed(String)` | Response schema validation failed |
| `PermissionDenied(String)` | Insufficient permissions |
| `BudgetExceeded(BudgetError)` | Token/cost budget exceeded |
| `PiiRedactionFailed(String)` | PII processing error |
| `RetriesExhausted` | All retry attempts failed |

---

## 11. adapto_macros -- Derive Macros

`#[derive(Resource)]` generates typed `adapto_store` operations from a struct definition.

### Usage

```rust
use adapto_macros::Resource;
use serde::{Serialize, Deserialize};

#[derive(Resource, Serialize, Deserialize, Clone)]
#[resource(collection = "customers")]
pub struct Customer {
    #[field(required, unique, format = "email")]
    pub email: String,

    #[field(required, max_length = 120)]
    pub name: String,

    #[field(one_of = ["active", "inactive", "pending"])]
    pub status: String,
}
```

### Container Attributes

- `#[resource(collection = "name")]` -- **required**. The store collection name.

### Field Attributes

All field attributes are optional inside `#[field(...)]`:

| Attribute | Description |
|---|---|
| `required` | Field is required in form schemas |
| `unique` | Creates a unique index on this field |
| `format = "..."` | Validation format hint (e.g., `"email"`) |
| `max_length = N` | Maximum string length |
| `default = "..."` | Default value for form schemas |
| `one_of = ["a", "b"]` | Allowed values; creates a non-unique index |

### Generated Methods

The derive macro generates these methods on your struct:

```rust
// Collection info
Customer::collection_name() -> &'static str          // "customers"
Customer::field_names() -> &'static [&'static str]   // ["email", "name", "status"]

// Conversion
Customer::from_document(doc: &Document) -> Option<Self>
customer.to_value() -> serde_json::Value

// Store access
Customer::store_collection(store) -> Collection<'_>
Customer::ensure_indexes(store)                       // creates indexes from field attrs

// CRUD
customer.insert_into(store) -> Result<String, StoreError>
Customer::find_by_id(store, id) -> Option<(String, Self)>
Customer::find_all(store, query) -> Vec<(String, Self)>
Customer::find_one_by(store, field, value) -> Option<(String, Self)>
Customer::count(store) -> u64
Customer::exists(store, field, value) -> bool
customer.update_in(store, id) -> Result<bool, StoreError>
Customer::delete(store, id) -> bool
Customer::delete_where(store, query) -> Result<u64, StoreError>

// Field access
customer.get_field("name") -> Option<String>
```

### Example

```rust
let store = AdaptoStore::open(None)?;
Customer::ensure_indexes(&store);

let customer = Customer {
    email: "alice@example.com".into(),
    name: "Alice".into(),
    status: "active".into(),
};

// Insert
let id = customer.insert_into(&store)?;

// Find
let (id, found) = Customer::find_one_by(&store, "email", "alice@example.com").unwrap();
assert_eq!(found.name, "Alice");

// Update
let mut updated = found;
updated.name = "Alice Smith".into();
updated.update_in(&store, &id)?;

// Check existence
assert!(Customer::exists(&store, "email", "alice@example.com"));

// Count
assert_eq!(Customer::count(&store), 1);

// Delete
Customer::delete(&store, &id);
```

---

## 12. adapto_runtime -- Shared Runtime Types

Core types, context, state management, and configuration shared across all Adapto crates.

### Type Aliases

All IDs are newtype wrappers for type safety. They implement `Debug`, `Clone`, `Hash`, `Eq`, `Serialize`, `Deserialize`, `Display`, and `From`.

| Type | Inner | Description |
|---|---|---|
| `SessionId` | `String` | WebSocket session identifier |
| `UserId` | `Uuid` | Authenticated user |
| `TenantId` | `Uuid` | Tenant in multi-tenant apps |
| `RouteId` | `String` | Route path |
| `ComponentId` | `String` | UI component identifier |
| `RequestId` | `Uuid` | Request correlation ID |

### Ctx (Request Context)

Per-request context threaded through every handler:

```rust
use adapto_runtime::context::{Ctx, PermissionSet};

// Fields
ctx.user_id       // Option<UserId>
ctx.tenant_id     // Option<TenantId>
ctx.request_id    // RequestId
ctx.permissions   // PermissionSet
ctx.route         // RouteId
ctx.session_id    // SessionId

// Require authentication
let user_id = ctx.require_auth()?;   // -> &UserId or RuntimeError::Unauthenticated

// Require tenant
let tenant_id = ctx.require_tenant()?; // -> &TenantId or RuntimeError::TenantRequired

// Require permission
ctx.require("users.write")?;  // -> () or RuntimeError::PermissionDenied
```

### PermissionSet

```rust
let mut perms = PermissionSet::new();
perms.add("users.read");
perms.add("users.write");

assert!(perms.has("users.read"));
assert!(perms.has_any(&["admin", "users.read"]));  // true if ANY match
assert!(perms.has_all(&["users.read", "users.write"])); // true if ALL match
```

### StateStore

Server-side key-value state for live sessions with dirty tracking:

```rust
use adapto_runtime::state::StateStore;

let mut state = StateStore::new();
state.set("counter", json!(0));
state.set("name", json!("Alice"));

let val = state.get("counter"); // Option<&Value>

// Dirty tracking for efficient diff-based updates
assert!(state.is_dirty("counter"));
let dirty = state.get_dirty(); // &HashSet<String>
state.clear_dirty();

// Merge values from another map
state.merge(new_values);

// Introspection
let keys = state.keys();
let map = state.to_map(); // &HashMap<String, Value>
```

### AdaptoConfig

Configuration parsed from `adapto.toml`:

```rust
use adapto_runtime::config::AdaptoConfig;

let config = AdaptoConfig::default();

// config.app.name             // "adapto_app"
// config.app.env              // "development"
// config.server.host          // "0.0.0.0"
// config.server.port          // 3000
// config.database.url         // "postgres://localhost/adapto_dev"
// config.security.csrf        // true
// config.security.secure_cookies  // true
// config.live.websocket_path  // "/_adapto/live"
// config.live.max_sessions_per_user  // 10
// config.live.event_rate_limit_per_second  // 20
// config.tenant.mode          // "required"
// config.tenant.strategy      // "subdomain"
// config.ai.default_model     // None
// config.ai.fallback_model    // None
```

---

## 13. DSL Pipeline -- Parser, Compiler, SSR, Live

The template DSL pipeline is work-in-progress. It will provide a full-stack reactive system.

### Pipeline Overview

```
.adapto template file
        |
   adapto_parser (pest grammar)
        |
   adapto_compiler (IR generation)
        |
   +-----+------+
   |             |
adapto_ssr    adapto_live
(initial      (WebSocket
 render)       updates)
   |             |
   +-----+------+
         |
  adapto_client_protocol
  (wire format: patches, events)
         |
  adapto_runtime (JS)
  (DOM patching, event handling)
```

### Crates

| Crate | Purpose |
|---|---|
| `adapto_parser` | Pest-based grammar for `.adapto` template files |
| `adapto_compiler` | Compiles parse trees into an intermediate representation |
| `adapto_ssr` | Server-side rendering of compiled templates |
| `adapto_live` | WebSocket-based live updates and hot reload |
| `adapto_client_protocol` | Wire format for client-server messages (events, patches) |
| `adapto_runtime` | Client-side JavaScript runtime for DOM patching |

### Client Protocol (Wire Format)

The client protocol defines the messages exchanged over WebSocket:

**Client to Server:** `ClientMessage` wrapping either a `ClientEvent` (click, input, submit) or a `FormSubmitEvent`.

**Server to Client:** `ServerMessage` wrapping `PatchOp` operations:
- `ReplaceText { target, value }`
- `ReplaceHtml { target, html }`
- `SetAttr { target, name, value }`
- `RemoveAttr { target, name }`
- `AddClass { target, class }`
- `RemoveClass { target, class }`
- `Flash { level, message }`
- `Redirect { url }`

---

## 14. Testing -- adapto_test_utils

Test helpers, builders, mocks, and assertion macros for the Adapto framework.

### Store Helpers

```rust
use adapto_test_utils::store::*;

// Create a temporary in-memory store
let store = temp_store();

// Seed data with fluent API
let seeder = StoreSeeder::new(&store, "users")
    .with_index("email", true);

let id = seeder.insert(json!({"name": "Alice", "email": "alice@ex.com"}));
let ids = seeder.seed_n(100, |i| json!({"name": format!("User {}", i)}));
let count = seeder.count();

// Assertions
assert_doc_exists(&store, "users", &id);
assert_doc_not_exists(&store, "users", "nonexistent");
assert_doc_field(&store, "users", &id, "name", &json!("Alice"));
assert_collection_count(&store, "users", 101);
assert_query_count(&store, "users", Query::eq("name", "Alice"), 1);
assert_unique_field(&store, "users", "email", "alice@ex.com");
```

### HTTP Test Helpers

```rust
use adapto_test_utils::http::*;

// Build test requests
let req = TestRequest::get("/api/users")
    .header("accept", "application/json")
    .bearer("jwt-token-here")
    .query("page", "1")
    .query("limit", "10");

let req = TestRequest::post("/api/users")
    .json_body(&json!({"name": "Alice"}));

let req = TestRequest::put("/api/users/123")
    .content_type("application/json")
    .text_body(r#"{"name":"Bob"}"#);

// Build test responses
let resp = TestResponse::ok("<h1>Hello</h1>");
let resp = TestResponse::json(200, &json!({"users": []}));
let resp = TestResponse::not_found();
let resp = TestResponse::redirect("/login");

// Response inspection
resp.text();                // body as string
resp.json_body();           // Option<Value>
resp.is_success();          // 2xx
resp.is_redirect();         // 3xx
resp.is_client_error();     // 4xx
resp.is_server_error();     // 5xx
resp.header("content-type");

// Assertions
assert_status(&resp, 200);
assert_body_contains(&resp, "Hello");
assert_json_field(&resp, "users.0.name", &json!("Alice"));
assert_header(&resp, "content-type", "application/json");
```

### Event Builders

Build client protocol messages for WebSocket handler testing:

```rust
use adapto_test_utils::builders::*;

// Click event
let msg = EventBuilder::click("handle_delete")
    .session("test-session")
    .component("user-list")
    .payload_field("id", json!("user-123"))
    .build();

// Input event
let msg = EventBuilder::input("handle_search", "query text")
    .build();

// Form submission
let msg = FormBuilder::new("handle_register")
    .field("name", "Alice")
    .field("email", json!("alice@example.com"))
    .field("age", json!(30))
    .build();

// Patch response builder
let msg = PatchBuilder::new(1)
    .replace_text("#counter", "42")
    .replace_html("#list", "<ul>...</ul>")
    .set_attr("#btn", "disabled", "true")
    .remove_attr("#btn", "disabled")
    .add_class("#item", "active")
    .remove_class("#item", "loading")
    .flash(FlashLevel::Success, "Saved!")
    .redirect("/dashboard")
    .build();

// State builder
let state = StateBuilder::new()
    .set("counter", json!(0))
    .set("name", json!("Alice"))
    .build();
```

### Fixtures

Deterministic test data:

```rust
use adapto_test_utils::fixtures::*;

let tenant_id = test_tenant_id();    // fixed UUID
let user_id = test_user_id();        // fixed UUID
let session_id = test_session_id();  // "test-session-001"
let request_id = test_request_id();  // fixed UUID

// Full context
let ctx = test_ctx();                          // authenticated, with tenant
let ctx = test_ctx_with_permissions(&["users.read", "users.write"]);
let ctx = test_ctx_anonymous();                // no user, no tenant
let ctx = test_ctx_no_tenant();                // authenticated, no tenant
```

### Mocks

```rust
use adapto_test_utils::mock::*;

// Mock audit sink
let audit = MockAuditSink::new();
audit.write(event);
let events = audit.events();
audit.clear();

// Mock clock (deterministic time)
let clock = MockClock::new(chrono::Utc::now());
clock.advance(chrono::Duration::hours(1));
let now = clock.now();
clock.set(specific_time);

// Mock secret provider (fixed test secret)
let secrets = MockSecretProvider::new();
let key = secrets.secret(); // b"test-secret-key-for-tests"
```

### Snapshot Assertions

```rust
use adapto_test_utils::snapshot::*;

// Exact JSON equality
assert_json_eq(&actual, &expected);

// Subset matching (actual must contain all keys/values from subset)
assert_json_includes(&actual, &json!({"name": "Alice"}));

// Shape checking (verify keys exist)
assert_json_shape(&value, &["id", "name", "email"]);

// Array length
assert_json_array_len(&value, 3);

// Diff two JSON values
let diffs = json_diff(&a, &b); // Vec<String> describing differences
```

### Assertion Macros

```rust
use adapto_test_utils::assertions::*;

// Patch assertions
assert_patch_contains_text(&msg, "#counter", "42");
assert_patch_contains_html(&msg, "#list");
assert_patch_op_count(&msg, 3);

// State assertions
assert_state_eq(&store, "counter", &json!(0));
assert_state_dirty(&store, "counter");
assert_state_clean(&store, "counter");

// Validation assertions
assert_validation_valid(&result);
assert_validation_invalid(&result);
assert_validation_error(&result, "email", "invalid_email");
assert_no_validation_error(&result, "name");
```

---

## 15. Deployment

### Cross-Compilation

Build for Linux deployment from macOS:

```bash
# Install cargo-zigbuild
cargo install cargo-zigbuild

# Build for Linux
cargo zigbuild --release --target x86_64-unknown-linux-gnu
```

### Environment Configuration

The app reads these environment variables (via `.from_env()`):

| Variable | Description | Default |
|---|---|---|
| `PORT` | HTTP listen port | 3000 |
| `BIND_ADDR` | Bind address | `0.0.0.0` |
| `STORE_PATH` | Persistent store directory | None (in-memory) |

### Production Setup

```rust
App::new("My App")
    .from_env()                          // reads PORT, BIND_ADDR, STORE_PATH
    .health_check("/health")             // for load balancer probes
    .on_shutdown(|| {
        println!("Shutting down...");
    })
    .with_middleware(|router| {
        router.layer(tower_http::cors::CorsLayer::permissive())
    })
    .run()
    .await?;
```

### Store Persistence

For production, always open with a path:

```rust
let store = AdaptoStore::open(Some("./data"))?;
```

This creates WAL-backed storage at `./data/store.wal`. Call `store.compact()` periodically to snapshot and truncate the WAL.

### Systemd Service

Example `/etc/systemd/system/myapp.service`:

```ini
[Unit]
Description=My Adapto App
After=network.target

[Service]
ExecStart=/opt/myapp/myapp
WorkingDirectory=/opt/myapp
Environment=PORT=8080
Environment=STORE_PATH=/opt/myapp/data
Restart=always
User=deploy

[Install]
WantedBy=multi-user.target
```

### Monitoring

Use the health check endpoint for uptime monitoring:

```bash
curl http://localhost:8080/health
# Response: "ok" (200)
```

Use `store.stats()` for database metrics and `ResponseCache::stats()` for AI cache hit rates.
