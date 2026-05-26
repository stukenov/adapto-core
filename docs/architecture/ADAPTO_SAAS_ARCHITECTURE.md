# Adapto: The SaaS Construction Kit

## Architecture Design Document

**Version**: 1.0  
**Date**: 2026-05-25  
**Status**: Proposed

---

## Table of Contents

1. [Philosophy](#1-philosophy)
2. [Developer Journey](#2-developer-journey)
3. [Convention System](#3-convention-system)
4. [The `.crud::<T>()` Primitive](#4-the-crudt-primitive)
5. [Resource Derive Macro — Full Specification](#5-resource-derive-macro--full-specification)
6. [Auto-Views System](#6-auto-views-system)
7. [REST API Auto-Generation](#7-rest-api-auto-generation)
8. [Middleware & SaaS Primitives](#8-middleware--saas-primitives)
9. [Relationships Between Resources](#9-relationships-between-resources)
10. [Auto-Dashboard](#10-auto-dashboard)
11. [Validation Pipeline](#11-validation-pipeline)
12. [Seed & Fixtures System](#12-seed--fixtures-system)
13. [CLI Generator](#13-cli-generator)
14. [Error Handling](#14-error-handling)
15. [Convention Table](#15-convention-table)
16. [Implementation Blueprint](#16-implementation-blueprint)
17. [Comparison Matrix](#17-comparison-matrix)

---

## 1. Philosophy

Adapto operates on one axiom: **the shape of your data IS your application**.

When you define a Rust struct with `#[derive(Resource)]`, you have already told the framework everything it needs to know: the database schema, the API endpoints, the admin UI, the validation rules, the search indexes, and the audit trail. Every line of code you write after that is customization — not construction.

Three principles govern every design decision:

**Inevitability.** Each API should feel like the only way it could work. When a developer guesses how something might work, they should be right.

**Zero-to-working in one method call.** `.crud::<Customer>()` produces a complete, production-grade CRUD interface. Not a scaffold. Not a starting point. A working product.

**Escape hatches at every layer.** Conventions are defaults, never constraints. Override a single field's rendering, a single endpoint's behavior, or the entire view — at whatever granularity you need.

---

## 2. Developer Journey

### Step 1: Create a new project

```bash
$ cargo install adapto-cli
$ adapto new invoicely
    Created  invoicely/Cargo.toml
    Created  invoicely/src/main.rs
    Created  invoicely/src/resources/mod.rs
    Created  invoicely/seeds/development.rs
    Created  invoicely/.env
    Created  invoicely/.env.example

$ cd invoicely
```

The generated `Cargo.toml`:

```toml
[package]
name = "invoicely"
version = "0.1.0"
edition = "2021"

[dependencies]
adapto = { version = "0.1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
```

The generated `src/main.rs`:

```rust
use adapto::prelude::*;

mod resources;

#[tokio::main]
async fn main() {
    App::new("Invoicely")
        .port(3000)
        .store_path("./data/invoicely")
        .run()
        .await
        .unwrap();
}
```

Running `cargo run` immediately serves a dashboard at `localhost:3000` with zero resources — a blank canvas with the full Adapto chrome (nav, sidebar, system stats).

### Step 2: Define a resource

```bash
$ adapto generate resource Invoice \
    number:string:required:unique \
    client_name:string:required \
    amount_cents:i64:required \
    status:enum:draft,sent,paid,overdue \
    due_date:string:format=date \
    notes:string
```

This creates `src/resources/invoice.rs`:

```rust
use adapto::prelude::*;

#[derive(Resource, Serialize, Deserialize, Debug, Clone)]
#[resource(label = "Invoice")]
pub struct Invoice {
    #[field(required, unique)]
    pub number: String,

    #[field(required, max_length = 200)]
    pub client_name: String,

    #[field(required, label = "Amount (cents)")]
    pub amount_cents: i64,

    #[field(default = "draft", one_of = ["draft", "sent", "paid", "overdue"])]
    pub status: String,

    #[field(format = "date", label = "Due Date")]
    pub due_date: String,

    #[field(max_length = 2000, input = "textarea")]
    pub notes: String,
}
```

And updates `src/resources/mod.rs`:

```rust
pub mod invoice;
pub use invoice::Invoice;
```

### Step 3: Register it

Add one line to `main.rs`:

```rust
App::new("Invoicely")
    .port(3000)
    .store_path("./data/invoicely")
    .crud::<Invoice>()           // <-- this line
    .run()
    .await
    .unwrap();
```

That single `.crud::<Invoice>()` call produces:

- **Database**: Collection `"invoices"` with indexes on `number` (unique) and `status`
- **WebSocket handlers**: `show_invoices`, `show_invoice`, `create_invoice`, `update_invoice`, `delete_invoice`, `search_invoices`
- **REST API**: `GET/POST /api/invoices`, `GET/PUT/DELETE /api/invoices/:id`
- **UI**: List table, detail card, create/edit form — all auto-generated
- **Navigation**: "Invoices" appears in the sidebar
- **Dashboard**: Invoice count widget on the home page
- **Audit**: All mutations logged automatically

### Step 4: Add more resources

```rust
App::new("Invoicely")
    .port(3000)
    .store_path("./data/invoicely")
    .crud::<Invoice>()
    .crud::<Client>()
    .crud::<Payment>()
    .crud::<LineItem>()
    .seed::<Invoice>(seeds::invoices)
    .auth(AuthConfig::jwt("SECRET"))
    .tenant_mode(TenantMode::Header("X-Tenant-Id"))
    .audit()
    .run()
    .await
    .unwrap();
```

Seven resources, authentication, multi-tenancy, audit logging. That is the entire application.

### Step 5: Customize

Override only what needs to differ from the default:

```rust
App::new("Invoicely")
    .port(3000)
    .store_path("./data/invoicely")
    .crud::<Invoice>()
        .list_columns(&["number", "client_name", "amount_cents", "status", "due_date"])
        .hide_field("notes", ViewKind::List)
        .custom_column("amount", |doc| {
            format!("${:.2}", doc.get_i64("amount_cents").unwrap_or(0) as f64 / 100.0)
        })
        .action("mark_paid", |ctx| {
            ctx.store.collection("invoices").update_one(
                &ctx.id(),
                &Update::set("status", "paid"),
            )?;
            ctx.refresh()
        })
        .done()
    .crud::<Client>()
    .run()
    .await
    .unwrap();
```

---

## 3. Convention System

Every convention follows one rule: **derive from the struct definition, override with attributes or builder methods**.

### 3.1 Naming Conventions

| Source | Convention | Example | Override |
|--------|-----------|---------|----------|
| Struct name | Collection = `snake_case(plural(name))` | `Invoice` -> `"invoices"` | `#[resource(collection = "billing_invoices")]` |
| Struct name | Label = struct name with spaces | `LineItem` -> `"Line Item"` | `#[resource(label = "Invoice Line")]` |
| Struct name | Label plural = pluralized label | `LineItem` -> `"Line Items"` | `#[resource(label_plural = "Invoice Lines")]` |
| Struct name | Route prefix = `/{collection}` | `Invoice` -> `"/invoices"` | `#[resource(route = "/billing/invoices")]` |
| Struct name | API prefix = `/api/{collection}` | `Invoice` -> `"/api/invoices"` | `#[resource(api_prefix = "/api/v1/invoices")]` |
| Field name | Column header = `Title Case(name)` | `client_name` -> `"Client Name"` | `#[field(label = "Customer")]` |
| Field name | Form label = `Title Case(name)` | `due_date` -> `"Due Date"` | `#[field(label = "Payment Deadline")]` |

### 3.2 Pluralization

Built-in English pluralization rules (a minimal, deterministic set — no NLP):

```
*s      -> *ses     (status -> statuses)
*y      -> *ies     (company -> companies)  [consonant + y only]
*ch|sh  -> *ches|shes
*x|z    -> *xes|zes
*        -> *s       (default)
```

Irregular forms are handled by a hardcoded table:

```
person -> people, child -> children, man -> men, woman -> women,
mouse -> mice, goose -> geese, tooth -> teeth, foot -> feet,
ox -> oxen, datum -> data, index -> indices
```

When the algorithm fails, the developer overrides with `#[resource(label_plural = "...")]` or `#[resource(collection = "...")]`. The fallback is always explicit — never surprising.

### 3.3 Type Conventions

| Rust Type | Default Input | Default Display | Default Index |
|-----------|--------------|-----------------|---------------|
| `String` | `<input type="text">` | Plain text | None |
| `String` + `format = "email"` | `<input type="email">` | Mailto link | None |
| `String` + `format = "url"` | `<input type="url">` | Clickable link | None |
| `String` + `format = "date"` | `<input type="date">` | Formatted date | None |
| `String` + `format = "datetime"` | `<input type="datetime-local">` | Formatted datetime | None |
| `String` + `format = "phone"` | `<input type="tel">` | Phone link | None |
| `String` + `format = "color"` | `<input type="color">` | Color swatch | None |
| `String` + `one_of = [...]` | `<select>` with options | Badge | BTree index |
| `String` + `input = "textarea"` | `<textarea>` | Truncated text | None |
| `String` + `input = "richtext"` | Rich text editor | HTML rendered | None |
| `i64` / `i32` | `<input type="number">` | Formatted number | None |
| `i64` + `format = "currency"` | `<input type="number" step="0.01">` | `$1,234.56` | None |
| `f64` / `f32` | `<input type="number" step="any">` | Formatted decimal | None |
| `bool` | Toggle switch | Yes/No badge | None |
| `String` + `unique` | Text input | Plain text | Unique index |
| Any + `index` | — | — | BTree index |

---

## 4. The `.crud::<T>()` Primitive

### 4.1 Signature

```rust
impl App {
    /// Register full CRUD for a resource type.
    ///
    /// This is the core primitive of Adapto. A single call registers:
    /// - Database collection with indexes
    /// - WebSocket action handlers (list, detail, form, create, update, delete, search)
    /// - REST API endpoints (GET, POST, PUT, DELETE)
    /// - Auto-generated views (list table, detail card, create/edit form)
    /// - Navigation sidebar entry
    /// - Dashboard stats widget
    /// - Audit logging for all mutations
    pub fn crud<T: Resource>(self) -> CrudBuilder<T>
    where
        T: Resource + Serialize + DeserializeOwned + Clone + Send + Sync + 'static,
    {
        CrudBuilder::new(self)
    }
}
```

### 4.2 CrudBuilder — Customization Chain

```rust
pub struct CrudBuilder<T: Resource> {
    app: App,
    config: CrudConfig,
    _marker: PhantomData<T>,
}

pub struct CrudConfig {
    /// Which fields appear in the list view, and in what order.
    /// Default: all fields from `T::field_names()`.
    pub list_columns: Option<Vec<String>>,

    /// Fields hidden from specific views.
    pub hidden_fields: HashMap<String, HashSet<ViewKind>>,

    /// Custom column renderers: field_name -> fn(doc) -> HTML.
    pub custom_columns: HashMap<String, Box<dyn Fn(&Document) -> String + Send + Sync>>,

    /// Additional action buttons on the detail view.
    pub custom_actions: Vec<CustomAction>,

    /// Custom action handlers beyond the standard CRUD.
    pub custom_handlers: HashMap<String, Box<dyn ActionHandler>>,

    /// Whether to generate REST API endpoints. Default: true.
    pub rest_api: bool,

    /// Whether to add to the navigation sidebar. Default: true.
    pub nav_item: bool,

    /// Whether to show on the dashboard. Default: true.
    pub dashboard_widget: bool,

    /// Whether to enable audit logging for mutations. Default: true.
    pub audit: bool,

    /// Override the default list page size. Default: 25.
    pub page_size: usize,

    /// Fields that are searchable. Default: all String fields.
    pub searchable_fields: Option<Vec<String>>,

    /// Fields that are sortable. Default: all fields.
    pub sortable_fields: Option<Vec<String>>,

    /// Custom validation function run before create/update.
    pub validator: Option<Box<dyn Fn(&Value) -> Result<(), ValidationErrors> + Send + Sync>>,
}

impl<T: Resource> CrudBuilder<T> {
    /// Specify which columns appear in the list view, and in what order.
    pub fn list_columns(mut self, columns: &[&str]) -> Self { ... }

    /// Hide a field from a specific view type.
    pub fn hide_field(mut self, field: &str, view: ViewKind) -> Self { ... }

    /// Add a custom computed column to the list view.
    pub fn custom_column<F>(mut self, name: &str, renderer: F) -> Self
    where
        F: Fn(&Document) -> String + Send + Sync + 'static,
    { ... }

    /// Add a custom action button to the detail view.
    pub fn action<F>(mut self, name: &str, handler: F) -> Self
    where
        F: Fn(&mut ActionContext<'_>) -> ActionResult + Send + Sync + 'static,
    { ... }

    /// Add a custom WebSocket action handler.
    pub fn on<F>(mut self, action: &str, handler: F) -> Self
    where
        F: Fn(&mut ActionContext<'_>) -> ActionResult + Send + Sync + 'static,
    { ... }

    /// Disable REST API generation for this resource.
    pub fn no_rest(mut self) -> Self { ... }

    /// Exclude from navigation sidebar.
    pub fn no_nav(mut self) -> Self { ... }

    /// Exclude from dashboard.
    pub fn no_dashboard(mut self) -> Self { ... }

    /// Disable audit logging for this resource.
    pub fn no_audit(mut self) -> Self { ... }

    /// Set page size for list view.
    pub fn page_size(mut self, size: usize) -> Self { ... }

    /// Specify searchable fields (default: all String fields).
    pub fn searchable(mut self, fields: &[&str]) -> Self { ... }

    /// Add a custom validator.
    pub fn validate<F>(mut self, f: F) -> Self
    where
        F: Fn(&Value) -> Result<(), ValidationErrors> + Send + Sync + 'static,
    { ... }

    /// Finish configuration and return the App for further chaining.
    pub fn done(self) -> App { ... }
}
```

### 4.3 What `.crud::<T>()` Registers

When `CrudBuilder::done()` is called (or when the next `.crud()` / `.run()` is called), these registrations happen:

**Database layer:**
```
Collection: "{T::collection_name()}"
Indexes:    For each #[field(unique)]  -> unique BTree index
            For each #[field(index)]   -> BTree index
            For each #[field(one_of)]  -> BTree index
```

**WebSocket handlers (auto-prefixed with collection name):**
```
"{collection}:list"    -> show list view with pagination
"{collection}:detail"  -> show detail view for id
"{collection}:form"    -> show create/edit form
"{collection}:create"  -> validate + insert + show detail
"{collection}:update"  -> validate + update + show detail
"{collection}:delete"  -> delete + show list
"{collection}:search"  -> filter list by query string
"{collection}:sort"    -> re-render list with sort order
"{collection}:page"    -> navigate to page N
"{collection}:bulk"    -> bulk action (delete, update status, etc.)
```

**REST endpoints:**
```
GET    /api/{collection}        -> list (paginated, filtered, sorted)
GET    /api/{collection}/:id    -> get by ID
POST   /api/{collection}        -> create
PUT    /api/{collection}/:id    -> update
DELETE /api/{collection}/:id    -> delete
```

**UI registrations:**
```
Navigation: sidebar entry with icon + label
Dashboard:  stats card (count, recent trend)
Views:      list table, detail card, form (all auto-generated)
```

### 4.4 How Chaining Works

The `CrudBuilder` must eventually resolve back to `App`. This happens in three ways:

```rust
// 1. Explicit .done() — when customizing before the next resource
App::new("MyApp")
    .crud::<Invoice>()
        .list_columns(&["number", "client_name", "amount_cents"])
        .action("mark_paid", mark_paid_handler)
        .done()                                // <-- returns App
    .crud::<Client>()
        .done()
    .run().await;

// 2. Implicit — next .crud() call auto-finishes the previous builder
App::new("MyApp")
    .crud::<Invoice>()                         // starts InvoiceCrudBuilder
    .crud::<Client>()                          // finishes Invoice, starts Client
    .run().await;

// 3. Implicit — .run() auto-finishes the last builder
App::new("MyApp")
    .crud::<Invoice>()
    .run().await;                              // finishes Invoice, then runs
```

This is implemented via a `From<CrudBuilder<T>>` impl on `App`:

```rust
impl<T: Resource> From<CrudBuilder<T>> for App {
    fn from(builder: CrudBuilder<T>) -> App {
        builder.done()
    }
}
```

---

## 5. Resource Derive Macro — Full Specification

### 5.1 Container Attributes `#[resource(...)]`

| Attribute | Type | Default | Description |
|-----------|------|---------|-------------|
| `collection` | `string` | `snake_case(plural(struct_name))` | Collection name in the store |
| `label` | `string` | `Title Case(struct_name)` | Human-readable singular |
| `label_plural` | `string` | `plural(label)` | Human-readable plural |
| `route` | `string` | `"/{collection}"` | Route prefix for UI views |
| `api_prefix` | `string` | `"/api/{collection}"` | Route prefix for REST API |
| `icon` | `string` | `"folder"` | Sidebar icon name |
| `timestamps` | `bool` | `true` | Auto-add `created_at`, `updated_at` fields |
| `soft_delete` | `bool` | `false` | Use `deleted_at` instead of physical delete |
| `views` | `"auto" \| "none"` | `"auto"` | Whether to auto-generate views |

### 5.2 Field Attributes `#[field(...)]`

| Attribute | Type | Example | Effect |
|-----------|------|---------|--------|
| `required` | flag | `#[field(required)]` | Non-empty validation; form field marked required |
| `unique` | flag | `#[field(unique)]` | Unique index; validation error on duplicate |
| `index` | flag | `#[field(index)]` | Non-unique BTree index for fast queries |
| `default` | `string` | `#[field(default = "draft")]` | Default value on creation |
| `max_length` | `usize` | `#[field(max_length = 200)]` | Max string length validation |
| `min_length` | `usize` | `#[field(min_length = 3)]` | Min string length validation |
| `min` | `i64` | `#[field(min = 0)]` | Min numeric value |
| `max` | `i64` | `#[field(max = 100)]` | Max numeric value |
| `format` | `string` | `#[field(format = "email")]` | Input type + validation pattern |
| `one_of` | `[string]` | `#[field(one_of = ["a", "b"])]` | Enum-like constraint; renders as `<select>` |
| `label` | `string` | `#[field(label = "Email Address")]` | Override display label |
| `input` | `string` | `#[field(input = "textarea")]` | Override input widget type |
| `hidden` | flag | `#[field(hidden)]` | Hidden from all auto-views |
| `read_only` | flag | `#[field(read_only)]` | Shown but not editable |
| `belongs_to` | `string` | `#[field(belongs_to = "projects")]` | Foreign key relationship |
| `searchable` | `bool` | `#[field(searchable = false)]` | Exclude from search (String fields searchable by default) |
| `sortable` | `bool` | `#[field(sortable = false)]` | Exclude from sort options |

### 5.3 Generated Code

For this definition:

```rust
#[derive(Resource, Serialize, Deserialize, Debug, Clone)]
#[resource(label = "Invoice")]
pub struct Invoice {
    #[field(required, unique)]
    pub number: String,

    #[field(required, max_length = 200)]
    pub client_name: String,

    #[field(required)]
    pub amount_cents: i64,

    #[field(default = "draft", one_of = ["draft", "sent", "paid", "overdue"])]
    pub status: String,

    #[field(format = "date")]
    pub due_date: String,

    #[field(input = "textarea")]
    pub notes: String,
}
```

The macro generates:

```rust
// ── Trait implementation ──────────────────────────────────────────────

impl Resource for Invoice {
    fn collection_name() -> &'static str { "invoices" }
    fn resource_label() -> &'static str { "Invoice" }
    fn resource_label_plural() -> &'static str { "Invoices" }
    fn route_prefix() -> &'static str { "/invoices" }
    fn api_prefix() -> &'static str { "/api/invoices" }
    fn icon() -> &'static str { "folder" }

    fn field_names() -> &'static [&'static str] {
        &["number", "client_name", "amount_cents", "status", "due_date", "notes"]
    }

    fn field_defs() -> Vec<FieldDef> {
        vec![
            FieldDef {
                name: "number",
                label: "Number",
                rust_type: "String",
                required: true,
                unique: true,
                index: false,
                default: None,
                max_length: None,
                min_length: None,
                min: None,
                max: None,
                format: None,
                one_of: None,
                input: InputKind::Text,
                hidden: false,
                read_only: false,
                belongs_to: None,
                searchable: true,
                sortable: true,
            },
            FieldDef {
                name: "client_name",
                label: "Client Name",
                rust_type: "String",
                required: true,
                unique: false,
                index: false,
                default: None,
                max_length: Some(200),
                min_length: None,
                min: None,
                max: None,
                format: None,
                one_of: None,
                input: InputKind::Text,
                hidden: false,
                read_only: false,
                belongs_to: None,
                searchable: true,
                sortable: true,
            },
            FieldDef {
                name: "amount_cents",
                label: "Amount Cents",
                rust_type: "i64",
                required: true,
                unique: false,
                index: false,
                default: None,
                max_length: None,
                min_length: None,
                min: None,
                max: None,
                format: None,
                one_of: None,
                input: InputKind::Number,
                hidden: false,
                read_only: false,
                belongs_to: None,
                searchable: false,
                sortable: true,
            },
            FieldDef {
                name: "status",
                label: "Status",
                rust_type: "String",
                required: false,
                unique: false,
                index: true, // auto-indexed because one_of
                default: Some("draft"),
                max_length: None,
                min_length: None,
                min: None,
                max: None,
                format: None,
                one_of: Some(&["draft", "sent", "paid", "overdue"]),
                input: InputKind::Select,
                hidden: false,
                read_only: false,
                belongs_to: None,
                searchable: false,
                sortable: true,
            },
            FieldDef {
                name: "due_date",
                label: "Due Date",
                rust_type: "String",
                required: false,
                unique: false,
                index: false,
                default: None,
                max_length: None,
                min_length: None,
                min: None,
                max: None,
                format: Some("date"),
                one_of: None,
                input: InputKind::Date,
                hidden: false,
                read_only: false,
                belongs_to: None,
                searchable: false,
                sortable: true,
            },
            FieldDef {
                name: "notes",
                label: "Notes",
                rust_type: "String",
                required: false,
                unique: false,
                index: false,
                default: None,
                max_length: None,
                min_length: None,
                min: None,
                max: None,
                format: None,
                one_of: None,
                input: InputKind::Textarea,
                hidden: false,
                read_only: false,
                belongs_to: None,
                searchable: true,
                sortable: false,
            },
        ]
    }

    fn ensure_indexes(store: &AdaptoStore) {
        let col = store.collection("invoices");
        let _ = col.create_index("number", true);  // unique
        let _ = col.create_index("status", false);  // one_of -> indexed
    }

    fn validate(value: &Value) -> Result<(), ValidationErrors> {
        let mut errors = ValidationErrors::new();

        // number: required, unique (checked at store level)
        match value.get("number").and_then(|v| v.as_str()) {
            None | Some("") => errors.add("number", "Number is required"),
            _ => {}
        }

        // client_name: required, max_length = 200
        match value.get("client_name").and_then(|v| v.as_str()) {
            None | Some("") => errors.add("client_name", "Client Name is required"),
            Some(s) if s.len() > 200 => {
                errors.add("client_name", "Client Name must be at most 200 characters")
            }
            _ => {}
        }

        // amount_cents: required
        if value.get("amount_cents").and_then(|v| v.as_i64()).is_none() {
            errors.add("amount_cents", "Amount Cents is required");
        }

        // status: one_of
        if let Some(s) = value.get("status").and_then(|v| v.as_str()) {
            if !["draft", "sent", "paid", "overdue"].contains(&s) {
                errors.add("status", "Status must be one of: draft, sent, paid, overdue");
            }
        }

        errors.into_result()
    }

    fn apply_defaults(value: &mut Value) {
        let obj = value.as_object_mut().unwrap();
        if !obj.contains_key("status") {
            obj.insert("status".into(), json!("draft"));
        }
    }
}

// ── Convenience methods ──────────────────────────────────────────────

impl Invoice {
    pub fn collection_name() -> &'static str { "invoices" }

    pub fn from_document(doc: &Document) -> Option<Self> {
        serde_json::from_value(doc.data.clone()).ok()
    }

    pub fn to_value(&self) -> Value {
        serde_json::to_value(self).unwrap_or(Value::Null)
    }

    pub fn store_collection(store: &AdaptoStore) -> Collection<'_> {
        store.collection("invoices")
    }

    pub fn find_all(store: &AdaptoStore) -> Vec<(String, Self)> {
        let col = store.collection("invoices");
        col.find(&Query::all())
            .map(|cursor| {
                cursor
                    .map(|doc| {
                        let item = Self::from_document(&doc).unwrap();
                        (doc.id.clone(), item)
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn find_by_id(store: &AdaptoStore, id: &str) -> Option<(String, Self)> {
        let col = store.collection("invoices");
        col.find_by_id(id).ok().flatten().and_then(|doc| {
            Self::from_document(&doc).map(|item| (doc.id.clone(), item))
        })
    }

    pub fn insert(store: &AdaptoStore, mut value: Value) -> Result<String, StoreError> {
        Self::apply_defaults(&mut value);
        Self::validate(&value)?;
        store.collection("invoices").insert(value)
    }

    pub fn update(store: &AdaptoStore, id: &str, update: &Update) -> Result<UpdateResult, StoreError> {
        store.collection("invoices").update_one(id, update)
    }

    pub fn delete(store: &AdaptoStore, id: &str) -> Result<bool, StoreError> {
        store.collection("invoices").delete_by_id(id)
    }
}
```

### 5.4 The `Resource` Trait

```rust
/// The core trait that connects a Rust struct to the Adapto framework.
///
/// Implemented automatically by `#[derive(Resource)]`. Provides all metadata
/// the framework needs to generate views, routes, handlers, and validation.
pub trait Resource: Serialize + DeserializeOwned + Clone + Send + Sync + 'static {
    /// Collection name in the store (e.g., "invoices").
    fn collection_name() -> &'static str;

    /// Human-readable singular label (e.g., "Invoice").
    fn resource_label() -> &'static str;

    /// Human-readable plural label (e.g., "Invoices").
    fn resource_label_plural() -> &'static str;

    /// Route prefix for UI views (e.g., "/invoices").
    fn route_prefix() -> &'static str;

    /// Route prefix for REST API (e.g., "/api/invoices").
    fn api_prefix() -> &'static str;

    /// Sidebar icon name.
    fn icon() -> &'static str;

    /// Ordered list of field names.
    fn field_names() -> &'static [&'static str];

    /// Full field definitions with metadata.
    fn field_defs() -> Vec<FieldDef>;

    /// Create necessary indexes in the store.
    fn ensure_indexes(store: &AdaptoStore);

    /// Validate a JSON value against field constraints.
    fn validate(value: &Value) -> Result<(), ValidationErrors>;

    /// Apply default values to missing fields.
    fn apply_defaults(value: &mut Value);
}
```

---

## 6. Auto-Views System

### 6.1 Architecture

Views are pure functions: `fn(data, config) -> String`. They produce HTML strings that the WebSocket handler sends to the client via `PatchOp::ReplaceHtml`. There is no virtual DOM, no component tree — just server-rendered HTML patched into the live page.

The view system has three layers:

```
Layer 3: Custom views       (developer-written HTML)
Layer 2: Configured views   (auto-generated with overrides)
Layer 1: Default views      (auto-generated from Resource trait)
```

Each layer fully replaces the one below it. There is no partial mixing within a single view — either you use the auto-generated view (with configuration) or you replace it entirely.

### 6.2 List View

The default list view for a resource with N fields:

```rust
pub fn render_auto_list<T: Resource>(
    store: &AdaptoStore,
    config: &ListConfig,
    search: &str,
    page: usize,
    sort_field: Option<&str>,
    sort_dir: SortDir,
) -> String
```

Generated HTML structure:

```html
<!-- Search + Actions Bar -->
<div class="au-list-toolbar">
  <div class="au-list-search">
    <input type="search"
           class="au-input au-input--sm"
           placeholder="Search invoices..."
           data-action="invoices:search"
           data-debounce="300"
           value="{current_search}">
  </div>
  <div class="au-list-actions">
    <button class="au-btn au-btn--primary au-btn--sm"
            data-action="invoices:form">
      + New Invoice
    </button>
  </div>
</div>

<!-- Table -->
<div class="au-table-container">
  <table class="au-table">
    <thead>
      <tr>
        <th class="au-table__check">
          <input type="checkbox" data-action="invoices:select-all">
        </th>
        <!-- One <th> per visible field -->
        <th class="au-table__th au-table__th--sortable {active_class}"
            data-action="invoices:sort"
            data-field="number"
            data-dir="{next_dir}">
          Number
          <svg class="au-sort-icon">...</svg>
        </th>
        <th class="au-table__th au-table__th--sortable"
            data-action="invoices:sort"
            data-field="client_name"
            data-dir="asc">
          Client Name
        </th>
        <!-- ... more columns ... -->
        <th class="au-table__th au-table__th--actions">Actions</th>
      </tr>
    </thead>
    <tbody>
      <!-- One <tr> per document -->
      <tr class="au-table__row" data-id="{doc_id}">
        <td class="au-table__check">
          <input type="checkbox" value="{doc_id}" data-bulk="invoices">
        </td>
        <td class="au-table__td">{render_cell("number", value, field_def)}</td>
        <td class="au-table__td">{render_cell("client_name", value, field_def)}</td>
        <td class="au-table__td">{render_cell("amount_cents", value, field_def)}</td>
        <td class="au-table__td">{render_cell("status", value, field_def)}</td>
        <td class="au-table__td">{render_cell("due_date", value, field_def)}</td>
        <td class="au-table__td au-table__td--actions">
          <button class="au-btn au-btn--ghost au-btn--xs"
                  data-action="invoices:detail"
                  data-id="{doc_id}">
            View
          </button>
        </td>
      </tr>
    </tbody>
  </table>
</div>

<!-- Pagination -->
<div class="au-pagination">
  <span class="au-pagination__info">
    Showing {start}-{end} of {total}
  </span>
  <div class="au-pagination__controls">
    <button class="au-btn au-btn--ghost au-btn--xs"
            data-action="invoices:page" data-page="{prev}"
            {disabled_if_first}>
      Previous
    </button>
    <!-- Page number buttons -->
    <button class="au-btn au-btn--ghost au-btn--xs au-btn--active"
            data-action="invoices:page" data-page="1">1</button>
    <button class="au-btn au-btn--ghost au-btn--xs"
            data-action="invoices:page" data-page="2">2</button>
    <!-- ... -->
    <button class="au-btn au-btn--ghost au-btn--xs"
            data-action="invoices:page" data-page="{next}"
            {disabled_if_last}>
      Next
    </button>
  </div>
</div>

<!-- Bulk Actions Bar (hidden until selection) -->
<div class="au-bulk-bar" id="invoices-bulk-bar" style="display:none">
  <span class="au-bulk-bar__count">
    <span id="invoices-bulk-count">0</span> selected
  </span>
  <button class="au-btn au-btn--danger au-btn--sm"
          data-action="invoices:bulk"
          data-op="delete">
    Delete Selected
  </button>
</div>
```

### 6.3 Cell Rendering

Each field type has a default cell renderer:

```rust
fn render_cell(field_name: &str, value: &Value, def: &FieldDef) -> String {
    let raw = value.get(field_name);

    match (def.input, def.format.as_deref(), &def.one_of) {
        // Enum / one_of -> colored badge
        (InputKind::Select, _, Some(options)) => {
            let val = raw.and_then(|v| v.as_str()).unwrap_or("");
            let color = badge_color_for(val, options);
            format!(r#"<span class="au-badge au-badge--{color}">{}</span>"#, html_escape(val))
        }

        // Email -> mailto link
        (_, Some("email"), _) => {
            let val = raw.and_then(|v| v.as_str()).unwrap_or("");
            format!(r#"<a href="mailto:{0}" class="au-link">{0}</a>"#, html_escape(val))
        }

        // URL -> clickable link
        (_, Some("url"), _) => {
            let val = raw.and_then(|v| v.as_str()).unwrap_or("");
            format!(r#"<a href="{0}" class="au-link" target="_blank">{0}</a>"#, html_escape(val))
        }

        // Date -> formatted
        (_, Some("date"), _) => {
            let val = raw.and_then(|v| v.as_str()).unwrap_or("");
            format!(r#"<time datetime="{0}">{0}</time>"#, html_escape(val))
        }

        // Bool -> Yes/No badge
        (InputKind::Toggle, _, _) => {
            let val = raw.and_then(|v| v.as_bool()).unwrap_or(false);
            if val {
                r#"<span class="au-badge au-badge--green">Yes</span>"#.to_string()
            } else {
                r#"<span class="au-badge au-badge--neutral">No</span>"#.to_string()
            }
        }

        // Number -> formatted
        (InputKind::Number, _, _) => {
            let val = raw.and_then(|v| v.as_i64()).unwrap_or(0);
            format!("{}", val)
        }

        // Textarea -> truncated
        (InputKind::Textarea, _, _) => {
            let val = raw.and_then(|v| v.as_str()).unwrap_or("");
            let truncated = if val.len() > 80 {
                format!("{}...", html_escape(&val[..80]))
            } else {
                html_escape(val).to_string()
            };
            format!(r#"<span class="au-text-secondary">{truncated}</span>"#)
        }

        // Default: plain text
        _ => {
            let val = raw.and_then(|v| v.as_str()).unwrap_or("");
            html_escape(val).to_string()
        }
    }
}

/// Deterministic color assignment for enum badges.
/// Maps the option's position in the one_of list to a color.
fn badge_color_for(value: &str, options: &[&str]) -> &'static str {
    let colors = ["blue", "green", "amber", "red", "purple", "teal", "pink", "neutral"];
    let idx = options.iter().position(|o| *o == value).unwrap_or(0);
    colors[idx % colors.len()]
}
```

### 6.4 Detail View

```html
<!-- Back navigation -->
<button class="au-btn au-btn--ghost au-btn--sm"
        data-action="invoices:list"
        style="margin-bottom: var(--au-space-4)">
  <svg class="au-icon"><!-- arrow left --></svg>
  Back to Invoices
</button>

<!-- Detail card -->
<div class="au-card">
  <div class="au-card__header">
    <h2 class="au-card__title">{primary_field_value}</h2>
    <div class="au-card__actions">
      <button class="au-btn au-btn--sm"
              data-action="invoices:form"
              data-id="{doc_id}">
        Edit
      </button>
      <button class="au-btn au-btn--danger au-btn--sm"
              data-action="invoices:delete"
              data-id="{doc_id}"
              data-confirm="Delete this invoice?">
        Delete
      </button>
      <!-- Custom action buttons -->
      <button class="au-btn au-btn--sm"
              data-action="invoices:mark_paid"
              data-id="{doc_id}">
        Mark Paid
      </button>
    </div>
  </div>

  <div class="au-card__body">
    <dl class="au-detail-grid">
      <!-- One <dt>/<dd> pair per visible field -->
      <dt class="au-detail-grid__label">Number</dt>
      <dd class="au-detail-grid__value">{value}</dd>

      <dt class="au-detail-grid__label">Client Name</dt>
      <dd class="au-detail-grid__value">{value}</dd>

      <dt class="au-detail-grid__label">Amount Cents</dt>
      <dd class="au-detail-grid__value">{formatted_value}</dd>

      <dt class="au-detail-grid__label">Status</dt>
      <dd class="au-detail-grid__value">
        <span class="au-badge au-badge--blue">draft</span>
      </dd>

      <dt class="au-detail-grid__label">Due Date</dt>
      <dd class="au-detail-grid__value">
        <time datetime="2026-06-15">2026-06-15</time>
      </dd>

      <dt class="au-detail-grid__label">Notes</dt>
      <dd class="au-detail-grid__value au-detail-grid__value--full">
        {full_text}
      </dd>
    </dl>
  </div>

  <!-- Related resources (if belongs_to / has_many configured) -->
  <div class="au-card__section">
    <h3 class="au-card__subtitle">Line Items</h3>
    <!-- Embedded mini-list of related resources -->
    <table class="au-table au-table--compact">
      ...
    </table>
  </div>

  <!-- Metadata footer -->
  <div class="au-card__footer">
    <span class="au-text-xs au-text-tertiary">
      Created {created_at} | Updated {updated_at} | ID: {doc_id}
    </span>
  </div>
</div>
```

The detail grid uses CSS Grid for clean label-value alignment:

```css
.au-detail-grid {
  display: grid;
  grid-template-columns: minmax(120px, max-content) 1fr;
  gap: var(--au-space-1) var(--au-space-6);
  margin: 0;
}

.au-detail-grid__label {
  font-size: var(--au-text-sm);
  font-weight: var(--au-weight-medium);
  color: var(--au-color-text-secondary);
  padding: var(--au-space-2) 0;
}

.au-detail-grid__value {
  font-size: var(--au-text-sm);
  padding: var(--au-space-2) 0;
}

.au-detail-grid__value--full {
  grid-column: 1 / -1;
  padding-top: 0;
}
```

### 6.5 Form View

```html
<button class="au-btn au-btn--ghost au-btn--sm"
        data-action="invoices:list"
        style="margin-bottom: var(--au-space-4)">
  <svg class="au-icon"><!-- arrow left --></svg>
  Back to Invoices
</button>

<div class="au-card" style="max-width: 640px">
  <div class="au-card__header">
    <h2 class="au-card__title">{create_or_edit} Invoice</h2>
  </div>

  <div class="au-card__body">
    <!-- One form group per editable field -->

    <!-- String (required, unique) -->
    <div class="au-form-group">
      <label class="au-form-group__label" for="form_number">
        Number <span class="au-form-group__required">*</span>
      </label>
      <input type="text"
             id="form_number"
             class="au-input"
             placeholder="INV-001"
             value="{existing_value}"
             required>
      <span class="au-form-group__error" id="error_number"></span>
    </div>

    <!-- String (required, max_length) -->
    <div class="au-form-group">
      <label class="au-form-group__label" for="form_client_name">
        Client Name <span class="au-form-group__required">*</span>
      </label>
      <input type="text"
             id="form_client_name"
             class="au-input"
             maxlength="200"
             value="{existing_value}"
             required>
      <span class="au-form-group__hint">Max 200 characters</span>
      <span class="au-form-group__error" id="error_client_name"></span>
    </div>

    <!-- i64 (required) -->
    <div class="au-form-group">
      <label class="au-form-group__label" for="form_amount_cents">
        Amount Cents <span class="au-form-group__required">*</span>
      </label>
      <input type="number"
             id="form_amount_cents"
             class="au-input"
             value="{existing_value}"
             required>
      <span class="au-form-group__error" id="error_amount_cents"></span>
    </div>

    <!-- one_of -> select -->
    <div class="au-form-group">
      <label class="au-form-group__label" for="form_status">
        Status
      </label>
      <select id="form_status" class="au-input">
        <option value="draft" {selected}>Draft</option>
        <option value="sent" {selected}>Sent</option>
        <option value="paid" {selected}>Paid</option>
        <option value="overdue" {selected}>Overdue</option>
      </select>
    </div>

    <!-- format = "date" -->
    <div class="au-form-group">
      <label class="au-form-group__label" for="form_due_date">
        Due Date
      </label>
      <input type="date"
             id="form_due_date"
             class="au-input"
             value="{existing_value}">
    </div>

    <!-- input = "textarea" -->
    <div class="au-form-group">
      <label class="au-form-group__label" for="form_notes">
        Notes
      </label>
      <textarea id="form_notes"
                class="au-input"
                rows="4"
                placeholder="Optional notes...">{existing_value}</textarea>
    </div>
  </div>

  <div class="au-card__footer" style="display:flex;gap:var(--au-space-3);justify-content:flex-end">
    <button class="au-btn au-btn--ghost"
            data-action="invoices:list">
      Cancel
    </button>
    <button class="au-btn au-btn--primary"
            data-action="invoices:create"
            data-collect-form="true">
      Create Invoice
    </button>
  </div>
</div>
```

### 6.6 Edge Cases

**50+ fields**: When a resource has more than 8 fields, the form renders in a two-column grid layout. When more than 16, it groups fields into collapsible sections (the first 8 expanded, the rest collapsed under "More Fields"). The list view defaults to showing only the first 6 fields + actions; remaining fields are accessible in the detail view.

**Nested objects**: Not directly supported as top-level form fields. A `Value` field renders as a read-only JSON viewer in the detail view and a JSON textarea in the form. Proper nested object support is a future extension.

**File uploads**: Handled by a dedicated `#[field(input = "file")]` attribute. The form renders a file input; the file is uploaded via a separate HTTP POST to `/api/{collection}/{id}/files/{field_name}`, and the document stores the file path/URL as a string.

**Long text in tables**: Truncated to 80 characters with an ellipsis. Full text visible in the detail view.

### 6.7 Custom View Override

Replace the auto-generated view entirely:

```rust
App::new("MyApp")
    .crud::<Invoice>()
        .list_view(|store, search, page, sort| {
            // Return your own HTML string
            my_custom_invoice_table(store, search, page, sort)
        })
        .detail_view(|store, id| {
            my_custom_invoice_detail(store, id)
        })
        .form_view(|store, id| {
            my_custom_invoice_form(store, id)
        })
        .done()
    .run().await;
```

---

## 7. REST API Auto-Generation

### 7.1 Endpoint Specification

Every `.crud::<T>()` call generates these REST endpoints:

```
GET    /api/{collection}        -> List
GET    /api/{collection}/:id    -> Get one
POST   /api/{collection}        -> Create
PUT    /api/{collection}/:id    -> Full update
PATCH  /api/{collection}/:id    -> Partial update
DELETE /api/{collection}/:id    -> Delete
```

### 7.2 Response Envelope

Every response uses a consistent JSON envelope:

```json
// Success: single resource
{
  "data": {
    "id": "abc123",
    "number": "INV-001",
    "client_name": "Acme Corp",
    "amount_cents": 150000,
    "status": "draft",
    "due_date": "2026-06-15",
    "notes": "",
    "created_at": "2026-05-25T10:30:00Z",
    "updated_at": "2026-05-25T10:30:00Z"
  }
}

// Success: list
{
  "data": [
    { "id": "abc123", ... },
    { "id": "def456", ... }
  ],
  "meta": {
    "total": 142,
    "page": 1,
    "per_page": 25,
    "total_pages": 6
  }
}

// Error
{
  "error": {
    "code": "VALIDATION_FAILED",
    "message": "Validation failed for 2 fields",
    "details": {
      "number": ["Number is required"],
      "client_name": ["Client Name must be at most 200 characters"]
    }
  }
}
```

### 7.3 Query Parameters

**Filtering** — any field can be filtered:

```
GET /api/invoices?status=draft
GET /api/invoices?status=draft,sent          # OR within a field
GET /api/invoices?amount_cents.gte=10000      # numeric operators
GET /api/invoices?client_name.contains=Acme   # string operators
```

Supported operators: `.eq`, `.ne`, `.gt`, `.gte`, `.lt`, `.lte`, `.contains`, `.starts_with`, `.in`, `.nin`.

**Sorting**:

```
GET /api/invoices?sort=created_at             # ascending
GET /api/invoices?sort=-created_at            # descending (- prefix)
GET /api/invoices?sort=-status,created_at     # multi-field sort
```

**Pagination**:

```
GET /api/invoices?page=2&per_page=25          # page-based (default)
```

**Field selection**:

```
GET /api/invoices?fields=number,client_name,status
```

**Full-text search**:

```
GET /api/invoices?q=acme+corp
```

Searches all fields where `searchable = true` (default: all String fields). Uses case-insensitive substring matching via the store's `contains` filter.

### 7.4 Implementation

```rust
/// REST API handler for listing resources.
async fn rest_list<T: Resource>(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ListParams>,
) -> impl IntoResponse {
    let collection = state.store.collection(T::collection_name());

    // Build query from params
    let mut query = Query::new();

    // Apply field filters
    for (key, value) in &params.filters {
        let (field, op) = parse_filter_key(key);
        query = query.filter(match op {
            "eq" => Filter::eq(field, value.clone()),
            "ne" => Filter::ne(field, value.clone()),
            "gt" => Filter::gt(field, value.clone()),
            "gte" => Filter::gte(field, value.clone()),
            "lt" => Filter::lt(field, value.clone()),
            "lte" => Filter::lte(field, value.clone()),
            "contains" => Filter::contains(field, value.as_str().unwrap_or("")),
            "in" => Filter::in_values(field, parse_csv(value)),
            _ => Filter::eq(field, value.clone()),
        });
    }

    // Apply search
    if let Some(q) = &params.q {
        let searchable = T::field_defs()
            .iter()
            .filter(|f| f.searchable)
            .map(|f| f.name)
            .collect::<Vec<_>>();
        let search_filters: Vec<Filter> = searchable
            .iter()
            .map(|field| Filter::contains(field, q))
            .collect();
        if !search_filters.is_empty() {
            query = query.filter(Filter::or(search_filters));
        }
    }

    // Apply sort
    if let Some(sort) = &params.sort {
        for field in sort.split(',') {
            let (field, dir) = if field.starts_with('-') {
                (&field[1..], SortDir::Desc)
            } else {
                (field, SortDir::Asc)
            };
            query = query.sort(field, dir);
        }
    }

    // Apply pagination
    let page = params.page.unwrap_or(1);
    let per_page = params.per_page.unwrap_or(25).min(100);
    let total = collection.count(&query).unwrap_or(0);
    query = query.skip((page - 1) * per_page).limit(per_page);

    // Execute
    let docs = collection.find(&query).unwrap_or_default();

    // Apply field selection
    let data: Vec<Value> = if let Some(fields) = &params.fields {
        let field_list: HashSet<&str> = fields.split(',').collect();
        docs.map(|doc| filter_fields(&doc, &field_list)).collect()
    } else {
        docs.map(|doc| doc.to_api_value()).collect()
    };

    Json(json!({
        "data": data,
        "meta": {
            "total": total,
            "page": page,
            "per_page": per_page,
            "total_pages": (total + per_page - 1) / per_page
        }
    }))
}
```

### 7.5 OpenAPI Auto-Generation

An `/api/_openapi.json` endpoint is auto-generated from all registered resources:

```rust
App::new("MyApp")
    .crud::<Invoice>()
    .crud::<Client>()
    .openapi(true)     // default: true in dev, false in prod
    .run().await;
```

The OpenAPI spec is built at startup by iterating over all registered `Resource` types and their `field_defs()`. Each resource produces:

- A schema definition in `#/components/schemas/{ResourceName}`
- Five operations under `#/paths/api/{collection}`
- Request/response schemas derived from `FieldDef` metadata

Served at `/api/_openapi.json` (JSON) and `/api/_docs` (Swagger UI).

---

## 8. Middleware & SaaS Primitives

### 8.1 Authentication

```rust
/// Authentication configuration.
pub enum AuthConfig {
    /// JWT-based authentication.
    /// The string is the HMAC secret (HS256) or public key path (RS256).
    Jwt(JwtConfig),

    /// Session-based authentication with server-side storage.
    Session(SessionConfig),

    /// API key authentication (header-based).
    ApiKey(ApiKeyConfig),

    /// No authentication (default).
    None,
}

pub struct JwtConfig {
    /// HMAC secret or path to RSA public key.
    pub secret: String,
    /// Header name. Default: "Authorization" with "Bearer " prefix.
    pub header: String,
    /// Token expiry in seconds. Default: 3600 (1 hour).
    pub expiry_secs: u64,
    /// Refresh token expiry. Default: 604800 (7 days).
    pub refresh_expiry_secs: u64,
    /// Fields to include in the JWT claims from the user document.
    pub claims_fields: Vec<String>,
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            secret: std::env::var("ADAPTO_JWT_SECRET")
                .unwrap_or_else(|_| "change-me-in-production".into()),
            header: "Authorization".into(),
            expiry_secs: 3600,
            refresh_expiry_secs: 604800,
            claims_fields: vec!["email".into(), "role".into()],
        }
    }
}
```

Usage:

```rust
App::new("MyApp")
    .auth(AuthConfig::Jwt(JwtConfig {
        secret: env::var("JWT_SECRET").unwrap(),
        ..Default::default()
    }))
    .crud::<Invoice>()
    .run().await;
```

Auto-generated auth endpoints:

```
POST /api/auth/login     { "email": "...", "password": "..." }
POST /api/auth/register  { "email": "...", "password": "...", "name": "..." }
POST /api/auth/refresh   { "refresh_token": "..." }
POST /api/auth/logout    (invalidates refresh token)
GET  /api/auth/me        (returns current user)
```

The `ActionContext` gains auth-aware fields:

```rust
pub struct ActionContext<'a> {
    pub store: &'a AdaptoStore,
    pub payload: &'a Value,
    pub session: &'a mut HashMap<String, String>,
    pub user: Option<AuthUser>,       // populated by auth middleware
    pub tenant_id: Option<String>,    // populated by tenant middleware
    pub request_id: String,           // unique per request
}

pub struct AuthUser {
    pub id: String,
    pub email: String,
    pub role: String,
    pub claims: HashMap<String, Value>,
}
```

### 8.2 Multi-Tenancy

```rust
/// How tenant identity is determined.
pub enum TenantMode {
    /// Tenant ID from a request header.
    /// Example: `TenantMode::Header("X-Tenant-Id")`
    Header(&'static str),

    /// Tenant ID from the subdomain.
    /// Example: `acme.myapp.com` -> tenant_id = "acme"
    Subdomain,

    /// Tenant ID from a URL path prefix.
    /// Example: `/t/{tenant_id}/invoices` -> tenant_id from path
    PathPrefix,

    /// No multi-tenancy (single tenant). Default.
    None,
}
```

When tenancy is enabled, every store operation is automatically scoped:

```rust
// Without tenancy: direct collection access
let col = store.collection("invoices");
col.find(&query);

// With tenancy: transparently scoped via TenantScope
let tenant = store.tenant(&ctx.tenant_id.unwrap());
let col = tenant.collection("invoices");
col.find(&query);  // only returns this tenant's documents
```

The CRUD handlers automatically use `TenantScope` when `tenant_mode` is set. No developer code changes required — the framework handles scoping transparently.

### 8.3 Rate Limiting

```rust
pub struct RateLimitConfig {
    /// Requests per window. Default: 100.
    pub requests: u32,
    /// Window duration in seconds. Default: 60.
    pub window_secs: u64,
    /// How to identify the client. Default: by auth user, fallback to IP.
    pub key: RateLimitKey,
    /// Per-endpoint overrides.
    pub overrides: HashMap<String, (u32, u64)>,
}

pub enum RateLimitKey {
    /// Rate limit by authenticated user ID.
    User,
    /// Rate limit by tenant ID.
    Tenant,
    /// Rate limit by source IP.
    Ip,
    /// Custom key extractor.
    Custom(Box<dyn Fn(&Request) -> String + Send + Sync>),
}
```

Usage:

```rust
App::new("MyApp")
    .rate_limit(RateLimitConfig {
        requests: 100,
        window_secs: 60,
        key: RateLimitKey::Tenant,
        overrides: HashMap::from([
            ("/api/auth/login".into(), (5, 60)),    // 5 login attempts per minute
            ("/api/auth/register".into(), (3, 60)),  // 3 registrations per minute
        ]),
    })
    .run().await;
```

Rate limit state is stored in the Adapto store itself (collection: `_rate_limits`), so it persists across restarts and works without external dependencies.

Response headers on every request:

```
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 87
X-RateLimit-Reset: 1716652800
```

When exceeded:

```
HTTP 429 Too Many Requests
Retry-After: 23

{
  "error": {
    "code": "RATE_LIMIT_EXCEEDED",
    "message": "Rate limit exceeded. Try again in 23 seconds.",
    "details": { "retry_after": 23 }
  }
}
```

### 8.4 Audit Logging

```rust
App::new("MyApp")
    .audit()           // enable audit logging for all mutations
    .crud::<Invoice>()
    .run().await;
```

Every create, update, and delete operation writes an audit entry to the `_audit_log` collection:

```json
{
  "timestamp": "2026-05-25T10:30:00Z",
  "action": "create",
  "collection": "invoices",
  "document_id": "abc123",
  "user_id": "user_456",
  "tenant_id": "acme",
  "changes": {
    "number": { "to": "INV-001" },
    "status": { "to": "draft" }
  },
  "ip": "192.168.1.1",
  "request_id": "req_789"
}
```

For updates, the `changes` field records both `from` and `to` values:

```json
{
  "action": "update",
  "changes": {
    "status": { "from": "draft", "to": "sent" },
    "updated_at": { "from": "2026-05-25T10:30:00Z", "to": "2026-05-25T11:00:00Z" }
  }
}
```

The audit log is queryable via the REST API:

```
GET /api/_audit?collection=invoices&action=update&user_id=user_456
```

And rendered on the dashboard as "Recent Activity".

### 8.5 CORS

```rust
pub struct CorsConfig {
    /// Allowed origins. Default: `["*"]` in dev, empty in prod.
    pub origins: Vec<String>,
    /// Allowed methods. Default: `["GET", "POST", "PUT", "PATCH", "DELETE", "OPTIONS"]`.
    pub methods: Vec<String>,
    /// Allowed headers. Default: `["Content-Type", "Authorization", "X-Tenant-Id"]`.
    pub headers: Vec<String>,
    /// Max age for preflight cache in seconds. Default: 3600.
    pub max_age: u64,
    /// Whether to allow credentials. Default: true.
    pub credentials: bool,
}
```

### 8.6 Middleware Chain Order

When the app starts, middleware is applied in this order (outermost first):

```
Request ->
  1. Request ID (always on; assigns unique ID)
  2. CORS (if configured)
  3. Rate Limit (if configured)
  4. Auth (if configured; populates ctx.user)
  5. Tenant (if configured; populates ctx.tenant_id)
  6. Audit (if enabled; wraps handler to log mutations)
  7. Handler (CRUD handler or custom handler)
<- Response
```

---

## 9. Relationships Between Resources

### 9.1 Declaration

```rust
#[derive(Resource, Serialize, Deserialize, Debug, Clone)]
pub struct Task {
    #[field(required)]
    pub title: String,

    #[field(belongs_to = "projects")]
    pub project_id: String,

    #[field(belongs_to = "users", label = "Assigned To")]
    pub assignee_id: String,

    #[field(default = "todo", one_of = ["todo", "in_progress", "done"])]
    pub status: String,
}

#[derive(Resource, Serialize, Deserialize, Debug, Clone)]
pub struct Project {
    #[field(required)]
    pub name: String,

    #[field(has_many = "tasks", foreign_key = "project_id")]
    _tasks: (),  // phantom field, not stored

    #[field(belongs_to = "users", label = "Owner")]
    pub owner_id: String,
}
```

### 9.2 What `belongs_to` Generates

For `#[field(belongs_to = "projects")] pub project_id: String`:

**In the form view**: Instead of a plain text input, renders a searchable dropdown populated from the `projects` collection. The dropdown shows the first non-ID String field of the related resource (its "display field") as the option label.

```html
<div class="au-form-group">
  <label class="au-form-group__label" for="form_project_id">
    Project <span class="au-form-group__required">*</span>
  </label>
  <select id="form_project_id" class="au-input au-input--searchable"
          data-action="tasks:search-relation"
          data-collection="projects">
    <option value="">Select a project...</option>
    <option value="proj_123" selected>Website Redesign</option>
    <option value="proj_456">Mobile App v2</option>
  </select>
</div>
```

**In the detail view**: The raw ID is replaced with a clickable link to the related resource:

```html
<dt class="au-detail-grid__label">Project</dt>
<dd class="au-detail-grid__value">
  <a href="/projects/proj_123"
     class="au-link"
     data-action="projects:detail"
     data-id="proj_123">
    Website Redesign
  </a>
</dd>
```

**In the list view**: Shows the display field of the related resource, not the raw ID.

**Index**: A BTree index is created on the foreign key field for fast lookups.

### 9.3 What `has_many` Generates

For `#[field(has_many = "tasks", foreign_key = "project_id")]`:

**In the detail view**: A "Tasks" section appears below the main fields, showing a compact table of related documents:

```html
<div class="au-card__section">
  <div class="au-card__section-header">
    <h3 class="au-card__subtitle">Tasks</h3>
    <span class="au-badge au-badge--neutral">12</span>
    <button class="au-btn au-btn--ghost au-btn--xs"
            data-action="tasks:form"
            data-prefill-project_id="{this_id}">
      + Add Task
    </button>
  </div>
  <table class="au-table au-table--compact">
    <thead>
      <tr>
        <th>Title</th>
        <th>Status</th>
        <th>Assigned To</th>
      </tr>
    </thead>
    <tbody>
      <tr data-action="tasks:detail" data-id="task_001">
        <td>Design mockups</td>
        <td><span class="au-badge au-badge--green">done</span></td>
        <td>Alice</td>
      </tr>
      <!-- ... -->
    </tbody>
  </table>
</div>
```

The compact table shows the first 5 related documents. A "View all N tasks" link appears when there are more.

### 9.4 Cascading Deletes

By default, deleting a parent with `has_many` children is **blocked** with an error:

```json
{
  "error": {
    "code": "DEPENDENT_RECORDS",
    "message": "Cannot delete Project: 12 Tasks still reference it",
    "details": {
      "collection": "tasks",
      "field": "project_id",
      "count": 12
    }
  }
}
```

Override with `#[field(has_many = "tasks", on_delete = "cascade")]` to auto-delete children, or `on_delete = "nullify"` to set the foreign key to null.

```rust
#[field(has_many = "tasks", foreign_key = "project_id", on_delete = "cascade")]
_tasks: (),
```

---

## 10. Auto-Dashboard

### 10.1 Structure

The dashboard is the index page (`/`) and is auto-generated from registered resources:

```html
<div class="au-dashboard">
  <!-- Stats row: one card per resource -->
  <div class="au-dashboard__stats">
    <div class="au-card au-card--stat">
      <div class="au-stat__label">Invoices</div>
      <div class="au-stat__value">142</div>
      <div class="au-stat__trend au-stat__trend--up">+12 this week</div>
    </div>
    <div class="au-card au-card--stat">
      <div class="au-stat__label">Clients</div>
      <div class="au-stat__value">38</div>
      <div class="au-stat__trend au-stat__trend--neutral">No change</div>
    </div>
    <div class="au-card au-card--stat">
      <div class="au-stat__label">Payments</div>
      <div class="au-stat__value">89</div>
      <div class="au-stat__trend au-stat__trend--up">+5 this week</div>
    </div>
  </div>

  <!-- Recent activity (from audit log) -->
  <div class="au-card">
    <div class="au-card__header">
      <h3 class="au-card__title">Recent Activity</h3>
    </div>
    <div class="au-card__body">
      <ul class="au-activity-feed">
        <li class="au-activity-feed__item">
          <span class="au-activity-feed__action">Created</span>
          <span class="au-activity-feed__resource">Invoice INV-142</span>
          <span class="au-activity-feed__time">2 minutes ago</span>
        </li>
        <li class="au-activity-feed__item">
          <span class="au-activity-feed__action">Updated</span>
          <span class="au-activity-feed__resource">Client Acme Corp</span>
          <span class="au-activity-feed__time">15 minutes ago</span>
        </li>
      </ul>
    </div>
  </div>

  <!-- System stats -->
  <div class="au-card">
    <div class="au-card__header">
      <h3 class="au-card__title">System</h3>
    </div>
    <div class="au-card__body">
      <dl class="au-detail-grid">
        <dt>Total Documents</dt>
        <dd>269</dd>
        <dt>Collections</dt>
        <dd>3</dd>
        <dt>WAL Size</dt>
        <dd>124 KB</dd>
        <dt>Uptime</dt>
        <dd>3h 42m</dd>
      </dl>
    </div>
  </div>

  <!-- Custom widgets (developer-provided) -->
  <div class="au-dashboard__custom">
    {custom_widget_html}
  </div>
</div>
```

### 10.2 Dashboard CSS Layout

```css
.au-dashboard {
  display: grid;
  gap: var(--au-space-6);
}

.au-dashboard__stats {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
  gap: var(--au-space-4);
}

.au-card--stat {
  padding: var(--au-space-5);
}

.au-stat__label {
  font-size: var(--au-text-sm);
  color: var(--au-color-text-secondary);
  font-weight: var(--au-weight-medium);
}

.au-stat__value {
  font-size: var(--au-text-3xl);
  font-weight: var(--au-weight-bold);
  font-variant-numeric: tabular-nums;
  margin: var(--au-space-1) 0;
}

.au-stat__trend {
  font-size: var(--au-text-xs);
}

.au-stat__trend--up { color: var(--au-color-green); }
.au-stat__trend--down { color: var(--au-color-red); }
.au-stat__trend--neutral { color: var(--au-color-text-tertiary); }
```

### 10.3 Custom Dashboard Widgets

```rust
App::new("MyApp")
    .crud::<Invoice>()
    .crud::<Client>()
    .dashboard_widget("Revenue This Month", |store| {
        let invoices = store.collection("invoices");
        let paid = invoices.find(&Query::filter(Filter::eq("status", "paid")))
            .map(|c| c.count())
            .unwrap_or(0);
        let total: i64 = invoices
            .find(&Query::filter(Filter::eq("status", "paid")))
            .map(|c| c.map(|d| d.get_i64("amount_cents").unwrap_or(0)).sum())
            .unwrap_or(0);
        format!(
            r#"<div class="au-stat__value">${:.2}</div>
            <div class="au-stat__trend">{} paid invoices</div>"#,
            total as f64 / 100.0,
            paid
        )
    })
    .run().await;
```

---

## 11. Validation Pipeline

### 11.1 Validation Flow

```
User submits form or API request
         |
         v
  1. Type coercion (string "42" -> i64 42)
         |
         v
  2. Apply defaults (missing fields get default values)
         |
         v
  3. Field-level validation (from #[field] attributes)
         |
         v
  4. Custom validator (from .validate() builder method)
         |
         v
  5. Uniqueness check (query store for unique fields)
         |
         v
  6. Insert/Update
```

### 11.2 ValidationErrors Type

```rust
/// A collection of field-level validation errors.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ValidationErrors {
    /// Map of field_name -> list of error messages.
    pub errors: HashMap<String, Vec<String>>,
}

impl ValidationErrors {
    pub fn new() -> Self { Self::default() }

    pub fn add(&mut self, field: &str, message: &str) {
        self.errors
            .entry(field.to_string())
            .or_default()
            .push(message.to_string());
    }

    pub fn is_empty(&self) -> bool { self.errors.is_empty() }

    pub fn into_result(self) -> Result<(), Self> {
        if self.is_empty() { Ok(()) } else { Err(self) }
    }
}
```

### 11.3 Form Error Display

When validation fails on a WebSocket form submission, the server sends patch operations that inject error messages inline:

```rust
// For each field with errors:
PatchOp::ReplaceHtml {
    target: format!("error_{}", field_name),
    html: format!(
        r#"<span class="au-form-group__error au-form-group__error--visible">{}</span>"#,
        errors.join(", ")
    ),
}
// Also add error class to the input:
PatchOp::AddClass {
    target: format!("form_{}", field_name),
    class: "au-input--error".into(),
}
```

CSS for error state:

```css
.au-input--error {
  border-color: var(--au-color-red);
  box-shadow: 0 0 0 3px rgba(var(--au-color-red-rgb), 0.1);
}

.au-form-group__error {
  display: none;
  font-size: var(--au-text-xs);
  color: var(--au-color-red);
  margin-top: var(--au-space-1);
}

.au-form-group__error--visible {
  display: block;
}
```

### 11.4 API Error Response

For REST API validation errors:

```
HTTP 422 Unprocessable Entity

{
  "error": {
    "code": "VALIDATION_FAILED",
    "message": "Validation failed for 2 fields",
    "details": {
      "number": ["Number is required"],
      "client_name": ["Client Name must be at most 200 characters"]
    }
  }
}
```

---

## 12. Seed & Fixtures System

### 12.1 API

```rust
App::new("MyApp")
    .crud::<Invoice>()
    .seed::<Invoice>(seeds::sample_invoices)    // seed if collection empty
    .seed::<Client>(seeds::sample_clients)
    .run().await;
```

The seed function signature:

```rust
/// Seed function: returns a list of documents to insert if the collection is empty.
pub type SeedFn = fn() -> Vec<Value>;

// Example:
mod seeds {
    use serde_json::json;

    pub fn sample_invoices() -> Vec<serde_json::Value> {
        vec![
            json!({
                "number": "INV-001",
                "client_name": "Acme Corp",
                "amount_cents": 150000,
                "status": "paid",
                "due_date": "2026-05-01",
                "notes": "Website redesign project"
            }),
            json!({
                "number": "INV-002",
                "client_name": "Globex Inc",
                "amount_cents": 75000,
                "status": "sent",
                "due_date": "2026-06-15",
                "notes": ""
            }),
        ]
    }
}
```

### 12.2 Behavior

- Seeds run once during `App::run()`, after indexes are created but before the server starts listening.
- A collection is seeded only if it contains zero documents.
- Seeds are idempotent — running the app a second time does not duplicate data.
- Seed data goes through the same validation pipeline as user-submitted data.
- The console logs: `Seeded 10 invoices` or `Skipping seed: invoices already has 142 documents`.

### 12.3 Environment-Aware Seeds

```rust
App::new("MyApp")
    .seed_env("development", |app| {
        app.seed::<Invoice>(seeds::dev_invoices)
           .seed::<Client>(seeds::dev_clients)
    })
    .seed_env("staging", |app| {
        app.seed::<Invoice>(seeds::staging_invoices)
    })
    // No seeds in production
    .run().await;
```

The environment is determined by `ADAPTO_ENV` or `RUST_ENV` environment variable. Default: `"development"`.

### 12.4 Seed Files

For larger datasets, load from JSON files in a `seeds/` directory:

```rust
App::new("MyApp")
    .seed_file::<Invoice>("seeds/invoices.json")
    .run().await;
```

The JSON file format:

```json
[
  { "number": "INV-001", "client_name": "Acme Corp", "amount_cents": 150000 },
  { "number": "INV-002", "client_name": "Globex Inc", "amount_cents": 75000 }
]
```

---

## 13. CLI Generator

### 13.1 `adapto new`

```bash
$ adapto new myapp
    Created  myapp/
    Created  myapp/Cargo.toml
    Created  myapp/src/main.rs
    Created  myapp/src/resources/mod.rs
    Created  myapp/seeds/mod.rs
    Created  myapp/.env
    Created  myapp/.env.example
    Created  myapp/.gitignore
    Created  myapp/README.md

  Next steps:
    cd myapp
    cargo run
    Open http://localhost:3000
```

Options:

```bash
$ adapto new myapp --port 8080
$ adapto new myapp --auth jwt
$ adapto new myapp --tenant header
$ adapto new myapp --full    # all features enabled
```

### 13.2 `adapto generate resource`

```bash
$ adapto generate resource Task \
    title:string:required \
    description:string:textarea \
    project_id:belongs_to:Project \
    assignee_id:belongs_to:User \
    status:enum:todo,in_progress,done \
    priority:enum:low,medium,high,critical \
    due_date:string:date \
    estimated_hours:f64

    Created  src/resources/task.rs
    Updated  src/resources/mod.rs
    Updated  src/main.rs (added .crud::<Task>())
```

Field type syntax: `name:type[:modifiers]`

| Syntax | Generates |
|--------|-----------|
| `title:string` | `pub title: String` |
| `title:string:required` | `#[field(required)] pub title: String` |
| `title:string:required:unique` | `#[field(required, unique)] pub title: String` |
| `email:string:email` | `#[field(format = "email")] pub email: String` |
| `bio:string:textarea` | `#[field(input = "textarea")] pub bio: String` |
| `count:i64` | `pub count: i64` |
| `price:f64` | `pub price: f64` |
| `active:bool` | `pub active: bool` |
| `status:enum:a,b,c` | `#[field(one_of = ["a", "b", "c"])] pub status: String` |
| `project_id:belongs_to:Project` | `#[field(belongs_to = "projects")] pub project_id: String` |
| `date:string:date` | `#[field(format = "date")] pub date: String` |

### 13.3 `adapto generate seed`

```bash
$ adapto generate seed invoices --count 50

    Created  seeds/invoices.rs

  Generated 50 realistic invoice records using field types and constraints.
```

The generator creates plausible fake data based on field metadata:

- `format = "email"` -> `"{first}.{last}@example.com"`
- `format = "date"` -> random date within last 90 days
- `one_of = ["a", "b", "c"]` -> random selection from options
- `required` string -> random word/phrase
- numeric fields -> random within reasonable range

### 13.4 `adapto db`

```bash
$ adapto db stats           # collection counts, WAL size, index info
$ adapto db compact         # trigger WAL compaction
$ adapto db export invoices # export collection as JSON
$ adapto db import invoices data.json  # import from JSON
$ adapto db reset           # delete all data (with confirmation)
```

### 13.5 `adapto dev`

```bash
$ adapto dev    # cargo watch + auto-reload
```

Wraps `cargo watch -x run` with a file watcher that sends a reload signal to connected WebSocket clients when the server restarts.

---

## 14. Error Handling

### 14.1 Error Types

```rust
/// Unified error type for the Adapto framework.
#[derive(Debug, thiserror::Error)]
pub enum AdaptoError {
    #[error("Not found: {collection}/{id}")]
    NotFound { collection: String, id: String },

    #[error("Validation failed")]
    Validation(ValidationErrors),

    #[error("Duplicate value for unique field '{field}'")]
    DuplicateKey { collection: String, field: String, value: String },

    #[error("Cannot delete: {count} {related_collection} records depend on this")]
    DependentRecords { related_collection: String, field: String, count: usize },

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Forbidden: insufficient permissions")]
    Forbidden,

    #[error("Rate limit exceeded")]
    RateLimited { retry_after: u64 },

    #[error("Tenant required but not provided")]
    TenantRequired,

    #[error("Store error: {0}")]
    Store(#[from] StoreError),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl AdaptoError {
    /// HTTP status code for this error.
    pub fn status_code(&self) -> u16 {
        match self {
            Self::NotFound { .. } => 404,
            Self::Validation(_) => 422,
            Self::DuplicateKey { .. } => 409,
            Self::DependentRecords { .. } => 409,
            Self::Unauthorized => 401,
            Self::Forbidden => 403,
            Self::RateLimited { .. } => 429,
            Self::TenantRequired => 400,
            Self::Store(_) => 500,
            Self::Internal(_) => 500,
        }
    }

    /// Convert to the standard JSON error response.
    pub fn to_json(&self) -> Value {
        json!({
            "error": {
                "code": self.error_code(),
                "message": self.to_string(),
                "details": self.details(),
            }
        })
    }
}
```

### 14.2 Error Handling in WebSocket Context

When an error occurs in a WebSocket action handler, the framework catches it and renders an appropriate UI response:

- **Validation errors**: Inline field errors in the current form (see section 11.3).
- **Not found**: Redirect to the list view with a toast notification.
- **Duplicate key**: Highlight the offending field with the error message.
- **Other errors**: Toast notification at the top of the page.

Toast notification HTML:

```html
<div class="au-toast au-toast--error" id="adapto-toast">
  <span class="au-toast__message">Invoice INV-001 not found</span>
  <button class="au-toast__close" onclick="this.parentElement.remove()">x</button>
</div>
```

---

## 15. Convention Table

| Decision | Convention (Default) | Override Mechanism | Example |
|----------|--------------------|--------------------|---------|
| Collection name | `snake_case(plural(struct_name))` | `#[resource(collection = "...")]` | `Invoice` -> `invoices` |
| Singular label | Struct name with spaces | `#[resource(label = "...")]` | `LineItem` -> `Line Item` |
| Plural label | Pluralized label | `#[resource(label_plural = "...")]` | `Line Item` -> `Line Items` |
| Route prefix | `/{collection}` | `#[resource(route = "...")]` | `/invoices` |
| API prefix | `/api/{collection}` | `#[resource(api_prefix = "...")]` | `/api/invoices` |
| Sidebar icon | `"folder"` | `#[resource(icon = "...")]` | `"receipt"` |
| Timestamps | Auto-add `created_at`, `updated_at` | `#[resource(timestamps = false)]` | — |
| Soft delete | Physical delete | `#[resource(soft_delete)]` | Adds `deleted_at` field |
| Views | Auto-generated | `.list_view(fn)`, `.detail_view(fn)`, `.form_view(fn)` | — |
| Field label | `Title_Case(field_name)` | `#[field(label = "...")]` | `client_name` -> `Client Name` |
| Input type | Derived from Rust type | `#[field(input = "...")]` | `String` -> text, `bool` -> toggle |
| Required | Optional (except noted) | `#[field(required)]` | — |
| Searchable | `true` for String fields | `#[field(searchable = false)]` | — |
| Sortable | `true` for all fields | `#[field(sortable = false)]` | — |
| Index | None (except unique/one_of) | `#[field(index)]` | — |
| Page size | 25 | `.page_size(50)` | — |
| REST API | Enabled | `.no_rest()` | — |
| Nav item | Enabled | `.no_nav()` | — |
| Dashboard widget | Enabled | `.no_dashboard()` | — |
| Audit logging | Enabled (when `.audit()` on App) | `.no_audit()` on CrudBuilder | — |
| Cascade delete | Block | `#[field(has_many = "...", on_delete = "cascade")]` | — |
| Auth | None | `.auth(AuthConfig::Jwt(...))` | — |
| Tenancy | Single tenant | `.tenant_mode(TenantMode::Header("X-Tenant-Id"))` | — |
| Rate limit | None | `.rate_limit(RateLimitConfig { ... })` | — |
| CORS | `*` in dev, none in prod | `.cors(CorsConfig { ... })` | — |
| Environment | `"development"` | `ADAPTO_ENV` env var | — |

---

## 16. Implementation Blueprint

### Phase 1: Resource Trait & Derive Macro (Foundation)

**Goal**: Expand the `Resource` trait and `#[derive(Resource)]` macro to generate all metadata the framework needs.

**Modify**: `crates/adapto_macros/src/resource.rs`  
**Modify**: `crates/adapto_app/src/lib.rs` (ResourceMeta -> Resource trait)

Key additions to the macro:
- Parse all `#[field(...)]` attributes (currently only `unique` and `one_of`)
- Generate `field_defs()` returning full `Vec<FieldDef>` metadata
- Generate `validate()` from field constraints
- Generate `apply_defaults()` from `default = "..."` attributes
- Generate `api_prefix()`, `icon()` methods
- Generate convenience methods: `find_all`, `find_by_id`, `insert`, `update`, `delete`

New types to add:
- `FieldDef` struct (in a new `adapto_core` crate or in `adapto_app`)
- `InputKind` enum
- `ValidationErrors` struct
- `ViewKind` enum

**Estimated scope**: ~500 lines of macro code, ~200 lines of types.

### Phase 2: Auto-Views (The Visual Layer)

**Goal**: Generate list, detail, and form views from `Resource::field_defs()`.

**Modify**: `crates/adapto_app/src/views.rs` (currently placeholder)

Implement:
- `render_auto_list<T: Resource>(...)` -> HTML table with search, sort, pagination
- `render_auto_detail<T: Resource>(...)` -> HTML detail card
- `render_auto_form<T: Resource>(...)` -> HTML form with all field types
- `render_cell(...)` -> per-type cell rendering
- `render_input(...)` -> per-type form input rendering

New CSS components (add to `adapto_ui`):
- `.au-detail-grid` — label/value grid for detail views
- `.au-list-toolbar` — search + actions bar
- `.au-pagination` — pagination controls
- `.au-bulk-bar` — bulk selection bar
- `.au-activity-feed` — audit log timeline

**Estimated scope**: ~800 lines of view code, ~200 lines of CSS.

### Phase 3: `.crud::<T>()` Builder

**Goal**: The core developer-facing API.

**Modify**: `crates/adapto_app/src/lib.rs`

Implement:
- `CrudBuilder<T>` struct with all customization methods
- CRUD WebSocket handler dispatch: auto-register `{collection}:list`, `:detail`, `:form`, `:create`, `:update`, `:delete`, `:search`, `:sort`, `:page`
- Generic CRUD handler functions that use `Resource` trait methods
- Auto-registration of nav items, dashboard widgets

The key insight: CRUD handlers become **generic over `T: Resource`**, not per-resource. One set of handler functions serves all resources.

```rust
fn handle_crud_list<T: Resource>(ctx: &mut ActionContext) -> ActionResult {
    let config = ctx.crud_config::<T>();
    let search = ctx.session.get("search").cloned().unwrap_or_default();
    let page = ctx.session.get("page").and_then(|p| p.parse().ok()).unwrap_or(1);
    let html = render_auto_list::<T>(&ctx.store, &config, &search, page, None, SortDir::Asc);
    full_patch(&html, T::route_prefix())
}
```

**Estimated scope**: ~400 lines.

### Phase 4: REST API Layer

**Goal**: Auto-generate REST endpoints alongside WebSocket handlers.

**New**: `crates/adapto_app/src/rest.rs`

Implement:
- Generic REST handlers: `rest_list<T>`, `rest_get<T>`, `rest_create<T>`, `rest_update<T>`, `rest_delete<T>`
- Query parameter parsing (filters, sort, pagination, field selection, search)
- JSON response envelope
- Error response formatting

**Modify**: `crates/adapto_app/src/lib.rs` — register REST routes in `App::run()`

**Estimated scope**: ~600 lines.

### Phase 5: Middleware Stack

**Goal**: Auth, tenancy, rate limiting, audit, CORS.

**Modify**: `crates/adapto_auth/src/lib.rs` (currently empty)  
**Modify**: `crates/adapto_audit/src/lib.rs` (currently empty)  
**New**: `crates/adapto_app/src/middleware.rs`

Implement as axum middleware layers:
- `AuthLayer` — JWT validation, user extraction
- `TenantLayer` — tenant ID extraction from header/subdomain/path
- `RateLimitLayer` — token bucket per key, stored in adapto_store
- `AuditLayer` — wrap handlers to log mutations
- `CorsLayer` — standard CORS headers (use `tower-http` CORS)

**Estimated scope**: ~800 lines across crates.

### Phase 6: Relationships

**Goal**: `belongs_to` and `has_many` support.

**Modify**: `crates/adapto_macros/src/resource.rs` — parse relationship attributes  
**Modify**: `crates/adapto_app/src/views.rs` — relationship rendering in forms and detail views  
**Modify**: CRUD handlers — cascade/block logic on delete

**Estimated scope**: ~400 lines.

### Phase 7: CLI

**Goal**: `adapto new`, `adapto generate resource`, `adapto db`.

**New**: `crates/adapto_cli/` — new crate with `clap`-based CLI

Implement:
- Project scaffolding templates
- Resource code generation from field type syntax
- Database management commands

**Estimated scope**: ~600 lines.

### Phase 8: Dashboard & Polish

**Goal**: Auto-dashboard, OpenAPI, dev experience.

**Modify**: `crates/adapto_app/src/views.rs` — dashboard rendering  
**New**: `crates/adapto_app/src/openapi.rs` — OpenAPI spec generation

**Estimated scope**: ~500 lines.

### Implementation Order & Dependencies

```
Phase 1 (Resource trait)
    |
    +---> Phase 2 (Auto-views)
    |         |
    |         +---> Phase 3 (.crud() builder)
    |                   |
    |                   +---> Phase 4 (REST API)
    |                   |
    |                   +---> Phase 6 (Relationships)
    |                   |
    |                   +---> Phase 8 (Dashboard)
    |
    +---> Phase 5 (Middleware) [independent]
    |
    +---> Phase 7 (CLI) [independent]
```

Phases 5 and 7 can proceed in parallel with phases 2-4.

---

## 17. Comparison Matrix

### Lines of Code for Common SaaS Tasks

| Task | Rails | Django | Laravel | Next.js | **Adapto** |
|------|-------|--------|---------|---------|-----------|
| Define a resource with 6 fields | 8 (model) + 15 (migration) + 30 (controller) + 50 (views) = **103** | 10 (model) + 15 (admin) + 30 (views) + 20 (serializer) = **75** | 12 (model) + 15 (migration) + 40 (controller) + 60 (blade) = **127** | 30 (schema) + 40 (API route) + 80 (React component) = **150** | **12** (struct + derive) |
| Full CRUD with list/detail/form | +50 (routes) + 80 (controller) + 150 (views) = **280** | +40 (urls) + 60 (views) + 100 (templates) = **200** | +50 (routes) + 100 (controller) + 200 (blade) = **350** | +120 (API) + 300 (React pages) = **420** | **1** (`.crud::<T>()`) |
| Add authentication | `devise` gem: **3** lines config + gem install | `django.contrib.auth`: **5** lines settings | `laravel/breeze`: **2** commands + scaffold | `next-auth`: **40** lines config + provider | **1** line (`.auth(...)`) |
| Add multi-tenancy | `acts_as_tenant` gem: **15** lines | `django-tenants`: **20** lines + middleware | Manual: **100+** lines | Manual: **100+** lines | **1** line (`.tenant_mode(...)`) |
| REST API with filtering, pagination, sorting | `ransack` + `kaminari`: **30** lines | DRF + filters: **25** lines | API Resource: **40** lines | Manual: **80+** lines | **0** (included with `.crud()`) |
| Seed data | `db/seeds.rb`: **10** lines | `fixtures/`: **10** lines | `DatabaseSeeder.php`: **15** lines | `prisma db seed`: **20** lines | **1** line (`.seed::<T>(fn)`) |
| Audit logging | `paper_trail` gem: **3** lines | `django-auditlog`: **5** lines | Manual: **50+** lines | Manual: **80+** lines | **1** line (`.audit()`) |

### Steps to Go from Zero to Working SaaS

| Step | Rails | Django | Laravel | Next.js | **Adapto** |
|------|-------|--------|---------|---------|-----------|
| Install toolchain | `rvm` + `gem install rails` | `pip install django` | `composer` + `php` | `node` + `npx create-next-app` | `cargo install adapto-cli` |
| Create project | `rails new` | `django-admin startproject` | `laravel new` | `npx create-next-app` | `adapto new` |
| Create database | `rails db:create` | `python manage.py migrate` | `php artisan migrate` | Prisma setup + migrate | Automatic (embedded) |
| Define model | Write model file | Write model file | Write model file | Write Prisma schema | Write struct |
| Generate migration | `rails g migration` | `makemigrations` | `make:migration` | `prisma migrate` | Not needed (schemaless) |
| Run migration | `rails db:migrate` | `migrate` | `artisan migrate` | `prisma migrate deploy` | Not needed |
| Create controller | `rails g controller` | Write views.py | `make:controller` | Write API route | Not needed |
| Create views/templates | Write ERB/HAML | Write templates | Write Blade | Write React pages | Not needed |
| Configure routes | `config/routes.rb` | `urls.py` | `routes/web.php` | File-based routing | Not needed |
| Add auth | Install Devise, configure | Configure AUTH settings | Install Breeze, scaffold | Install next-auth, configure | `.auth(...)` |
| Add API | Write serializers | Install DRF, write serializers | Write API resources | Write API routes | Included |
| **Total manual steps** | **~12** | **~10** | **~12** | **~10** | **~3** |
| **Total files touched** | **~15** | **~12** | **~18** | **~12** | **~2** |

### Runtime Characteristics

| Metric | Rails | Django | Laravel | Next.js | **Adapto** |
|--------|-------|--------|---------|---------|-----------|
| Language | Ruby | Python | PHP | JavaScript/TypeScript | **Rust** |
| Startup time | ~3s | ~1s | ~2s | ~5s (dev) | **<100ms** |
| Memory (idle) | ~80MB | ~40MB | ~50MB | ~120MB | **<10MB** |
| Requests/sec (simple CRUD) | ~2,000 | ~3,000 | ~1,500 | ~5,000 | **>50,000** |
| External dependencies | PostgreSQL, Redis | PostgreSQL | MySQL, Redis | PostgreSQL, Prisma | **None** (embedded store) |
| Deployment | Server + DB + Redis | Server + DB | Server + DB + Redis | Vercel/Server + DB | **Single binary** |
| Type safety | Runtime errors | Runtime errors | Runtime errors | TypeScript (partial) | **Compile-time** |

---

## Appendix A: The `adapto` Prelude

The `adapto` crate re-exports everything a developer needs:

```rust
pub mod prelude {
    // Core traits
    pub use adapto_app::{App, Resource, ResourceMeta};
    pub use adapto_app::handler::{ActionContext, ActionHandler, ActionResult};

    // Builder types
    pub use adapto_app::{CrudBuilder, CrudConfig, ViewKind};

    // Store
    pub use adapto_store::{AdaptoStore, Collection, Query, Filter, Update, SortDir};
    pub use adapto_store::{Document, Cursor, UpdateResult};

    // Field metadata
    pub use adapto_app::field::{FieldDef, InputKind};

    // Validation
    pub use adapto_app::validation::ValidationErrors;

    // Auth
    pub use adapto_auth::{AuthConfig, JwtConfig, AuthUser};

    // Tenancy
    pub use adapto_store::tenant::{TenantMode, TenantScope, TenantCollection};

    // Middleware
    pub use adapto_app::middleware::{RateLimitConfig, CorsConfig};

    // Derive macro
    pub use adapto_macros::Resource;

    // Serde (re-export for convenience)
    pub use serde::{Deserialize, Serialize};
    pub use serde_json::{json, Value};
}
```

Usage:

```rust
use adapto::prelude::*;
```

One import. Everything available.

---

## Appendix B: Full Example — Project Management SaaS

```rust
use adapto::prelude::*;

// ── Resources ────────────────────────────────────────────────────────

#[derive(Resource, Serialize, Deserialize, Debug, Clone)]
#[resource(icon = "briefcase")]
pub struct Project {
    #[field(required, max_length = 200)]
    pub name: String,

    #[field(input = "textarea", max_length = 2000)]
    pub description: String,

    #[field(default = "active", one_of = ["active", "archived", "completed"])]
    pub status: String,

    #[field(belongs_to = "users", label = "Owner")]
    pub owner_id: String,

    #[field(has_many = "tasks", foreign_key = "project_id", on_delete = "cascade")]
    _tasks: (),
}

#[derive(Resource, Serialize, Deserialize, Debug, Clone)]
#[resource(icon = "check-circle")]
pub struct Task {
    #[field(required, max_length = 300)]
    pub title: String,

    #[field(input = "textarea")]
    pub description: String,

    #[field(belongs_to = "projects")]
    pub project_id: String,

    #[field(belongs_to = "users", label = "Assigned To")]
    pub assignee_id: String,

    #[field(default = "todo", one_of = ["todo", "in_progress", "review", "done"])]
    pub status: String,

    #[field(default = "medium", one_of = ["low", "medium", "high", "critical"])]
    pub priority: String,

    #[field(format = "date")]
    pub due_date: String,

    pub estimated_hours: f64,
}

#[derive(Resource, Serialize, Deserialize, Debug, Clone)]
#[resource(icon = "user")]
pub struct User {
    #[field(required, max_length = 120)]
    pub name: String,

    #[field(required, unique, format = "email")]
    pub email: String,

    #[field(default = "member", one_of = ["admin", "manager", "member"])]
    pub role: String,

    #[field(hidden)]
    pub password_hash: String,

    #[field(has_many = "projects", foreign_key = "owner_id")]
    _projects: (),

    #[field(has_many = "tasks", foreign_key = "assignee_id")]
    _tasks: (),
}

#[derive(Resource, Serialize, Deserialize, Debug, Clone)]
#[resource(icon = "clock")]
pub struct TimeEntry {
    #[field(belongs_to = "tasks")]
    pub task_id: String,

    #[field(belongs_to = "users")]
    pub user_id: String,

    #[field(required, min = 0)]
    pub minutes: i64,

    #[field(format = "date")]
    pub date: String,

    #[field(max_length = 500)]
    pub notes: String,
}

// ── Application ──────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    App::new("ProjectHub")
        .port(3000)
        .store_path("./data/projecthub")

        // Auth & security
        .auth(AuthConfig::Jwt(JwtConfig::default()))
        .tenant_mode(TenantMode::Header("X-Tenant-Id"))
        .rate_limit(RateLimitConfig::default())
        .audit()
        .cors(CorsConfig::permissive())

        // Resources
        .crud::<User>()
            .hide_field("password_hash", ViewKind::All)
            .done()
        .crud::<Project>()
        .crud::<Task>()
            .list_columns(&["title", "project_id", "assignee_id", "status", "priority", "due_date"])
            .page_size(50)
            .done()
        .crud::<TimeEntry>()
            .no_nav()  // accessible only from task detail view
            .done()

        // Dashboard customization
        .dashboard_widget("Hours This Week", |store| {
            // custom widget logic
            "<div class=\"au-stat__value\">142h</div>".to_string()
        })

        // Seeds
        .seed::<User>(seeds::admin_user)

        .run()
        .await
        .unwrap();
}
```

That is the entire application. Four resources, authentication, multi-tenancy, rate limiting, audit logging, custom dashboard widget, seed data. The developer wrote the structs and their relationships. Adapto built everything else.

The result: a production-grade project management SaaS with full CRUD, real-time updates, REST API, admin dashboard, and enterprise features — in under 150 lines of Rust.

---

*"People think focus means saying yes to the thing you've got to focus on. But that's not what it means at all. It means saying no to the hundred other good ideas that there are." — Steve Jobs*

Adapto says no to boilerplate so the developer can say yes to their product.
