# Adapto

[![crates.io](https://img.shields.io/crates/v/adapto_store.svg)](https://crates.io/crates/adapto_store)
[![license](https://img.shields.io/crates/l/adapto_store.svg)](LICENSE)

A full-stack Rust web framework for building data-driven sites and apps. Embedded document database, HTTP routing, component UI, template DSL, live updates, authentication, audit logging — 18 crates, one workspace.

**Production-proven** on [myqaz.kz](https://myqaz.kz) — 140K+ pages, 200K+ documents, Kazakh/Russian bilingual.

## Crates

### Core

| Crate | Version | Description |
|-------|---------|-------------|
| [`adapto`](crates/adapto) | [![](https://img.shields.io/crates/v/adapto.svg)](https://crates.io/crates/adapto) | Umbrella re-export with feature flags |
| [`adapto_store`](crates/adapto_store) | [![](https://img.shields.io/crates/v/adapto_store.svg)](https://crates.io/crates/adapto_store) | Embedded document DB — JSON, BTree indexes, WAL, mmap |
| [`adapto_app`](crates/adapto_app) | [![](https://img.shields.io/crates/v/adapto_app.svg)](https://crates.io/crates/adapto_app) | HTTP app builder on axum — routes, multilingual, WebSocket |
| [`adapto_ui`](crates/adapto_ui) | [![](https://img.shields.io/crates/v/adapto_ui.svg)](https://crates.io/crates/adapto_ui) | HTML component library + `html_escape()` |
| [`adapto_macros`](crates/adapto_macros) | [![](https://img.shields.io/crates/v/adapto_macros.svg)](https://crates.io/crates/adapto_macros) | `#[derive(Resource)]` proc macro |

### Security & Compliance

| Crate | Version | Description |
|-------|---------|-------------|
| [`adapto_auth`](crates/adapto_auth) | [![](https://img.shields.io/crates/v/adapto_auth.svg)](https://crates.io/crates/adapto_auth) | Passwords (PBKDF2), JWT, sessions, CSRF, RBAC, rate limiting |
| [`adapto_audit`](crates/adapto_audit) | [![](https://img.shields.io/crates/v/adapto_audit.svg)](https://crates.io/crates/adapto_audit) | Structured audit events, sinks, PII redaction |
| [`adapto_forms`](crates/adapto_forms) | [![](https://img.shields.io/crates/v/adapto_forms.svg)](https://crates.io/crates/adapto_forms) | Schema validation, cross-field rules, sanitizers |

### Data & AI

| Crate | Version | Description |
|-------|---------|-------------|
| [`adapto_db`](crates/adapto_db) | [![](https://img.shields.io/crates/v/adapto_db.svg)](https://crates.io/crates/adapto_db) | Database pool trait, SQL generation, migrations |
| [`adapto_ai`](crates/adapto_ai) | [![](https://img.shields.io/crates/v/adapto_ai.svg)](https://crates.io/crates/adapto_ai) | LLM client trait, prompt templates, response caching |

### DSL Pipeline

| Crate | Version | Description |
|-------|---------|-------------|
| [`adapto_parser`](crates/adapto_parser) | [![](https://img.shields.io/crates/v/adapto_parser.svg)](https://crates.io/crates/adapto_parser) | `.adapto` template parser (pest PEG) |
| [`adapto_compiler`](crates/adapto_compiler) | [![](https://img.shields.io/crates/v/adapto_compiler.svg)](https://crates.io/crates/adapto_compiler) | AST → ComponentIR, dependency graphs |
| [`adapto_ssr`](crates/adapto_ssr) | [![](https://img.shields.io/crates/v/adapto_ssr.svg)](https://crates.io/crates/adapto_ssr) | Server-side rendering, page wrapping |
| [`adapto_live`](crates/adapto_live) | [![](https://img.shields.io/crates/v/adapto_live.svg)](https://crates.io/crates/adapto_live) | WebSocket sessions, DOM patches, action interpreter |
| [`adapto_runtime`](crates/adapto_runtime) | [![](https://img.shields.io/crates/v/adapto_runtime.svg)](https://crates.io/crates/adapto_runtime) | StateStore, Ctx, permissions, types |
| [`adapto_client_protocol`](crates/adapto_client_protocol) | [![](https://img.shields.io/crates/v/adapto_client_protocol.svg)](https://crates.io/crates/adapto_client_protocol) | WebSocket protocol types |

### Tooling

| Crate | Version | Description |
|-------|---------|-------------|
| [`adapto_cli`](crates/adapto_cli) | [![](https://img.shields.io/crates/v/adapto_cli.svg)](https://crates.io/crates/adapto_cli) | CLI: new, dev, build, check, generate |
| [`adapto_test_utils`](crates/adapto_test_utils) | [![](https://img.shields.io/crates/v/adapto_test_utils.svg)](https://crates.io/crates/adapto_test_utils) | Builders, fixtures, mocks, assertions |

## Quick Start

```toml
[dependencies]
adapto = { version = "0.2", features = ["full"] }
# or pick individual crates:
# adapto_app = "0.2"
# adapto_store = "0.2"
```

```rust
use adapto::prelude::*;

#[tokio::main]
async fn main() {
    let store = AdaptoStore::open(Some("./data")).unwrap();

    // Import data
    let articles = store.collection("articles");
    articles.insert(json!({"title": "Hello", "slug": "hello"})).unwrap();
    articles.create_index("slug", true).unwrap();

    // Build app
    App::new("My App")
        .port(3000)
        .store(store)
        .page("/", |_ctx| "<h1>Home</h1>".to_string())
        .page("/articles/:slug", |ctx| {
            let slug = ctx.param("slug");
            match ctx.store().collection("articles")
                .find_one(Query::eq("slug", slug)).unwrap() {
                Some(doc) => PageResponse::Ok(
                    format!("<h1>{}</h1>", doc.get_str("title").unwrap_or(""))
                ),
                None => PageResponse::NotFound,
            }
        })
        .run()
        .await
        .unwrap();
}
```

## Feature Flags

| Flag | Default | Includes |
|------|---------|----------|
| `app` | yes | adapto_app |
| `ui` | yes | adapto_ui |
| `forms` | yes | adapto_forms |
| `auth` | yes | adapto_auth |
| `audit` | yes | adapto_audit |
| `macros` | yes | adapto_macros |
| `live` | yes | adapto_live, adapto_ssr, adapto_runtime, adapto_client_protocol |
| `ai` | no | adapto_ai |
| `db` | no | adapto_db |
| `parser` | no | adapto_parser, adapto_compiler |
| `full` | no | all of the above |

`adapto_store` is always included.

## Document Database

```rust
use adapto_store::{AdaptoStore, Query, Update, SortDir, slugify};

let store = AdaptoStore::open(Some("./data"))?;
let col = store.collection("users");

// CRUD
let id = col.insert(json!({"name": "Alice", "age": 30}))?;
let doc = col.find_one(Query::eq("name", "Alice"))?;
col.update(Query::eq("name", "Alice"), Update::Set(vec![("age".into(), json!(31))]))?;
col.delete_by_id(&id)?;

// Indexes
col.create_index("email", true)?;          // unique
col.create_compound_index(&["company", "role"], false)?;

// Query builder
let results = col.find(
    Query::new()
        .sort("name", SortDir::Asc)
        .limit(10)
        .skip(20)
        .project(&["name", "email"])
);

// Disk-backed collections for large datasets (100K+ docs)
let companies = store.disk_collection("companies")?;
companies.bulk_insert(docs)?;
companies.create_index("bin", true)?;

// Slugify (Cyrillic → Latin)
assert_eq!(slugify("Привет Мир"), "privet-mir");
assert_eq!(slugify("Қазақстан"), "qazaqstan");
```

## Authentication

```rust
use adapto_auth::password::{hash_password, verify_password};
use adapto_auth::jwt;

// Passwords
let hash = hash_password("my-secret")?;
assert!(verify_password("my-secret", &hash)?);

// JWT
let secret = b"my-secret-key";
let token = jwt::encode("user-123", secret, 3600)?; // 1 hour
let claims = jwt::decode(&token, secret)?;
assert_eq!(claims.sub, "user-123");
```

## Multilingual Routes

```rust
use adapto_app::LangConfig;

#[derive(Clone)]
enum Lang { Ru, Kk }

impl LangConfig for Lang {
    fn code(&self) -> &str {
        match self { Lang::Ru => "ru", Lang::Kk => "kk" }
    }
    fn prefix(&self) -> &str {
        match self { Lang::Ru => "", Lang::Kk => "/kz" }
    }
}

// Registers both GET /about and GET /kz/about
app.languages(vec![Lang::Ru, Lang::Kk])
   .localized_page("/about", |ctx| {
       match ctx.lang_code() {
           "kk" => "<h1>Біз туралы</h1>".to_string(),
           _    => "<h1>О нас</h1>".to_string(),
       }
   })
```

## Cross-Compilation

Build on macOS for Linux deployment:

```bash
cargo install cargo-zigbuild
cargo zigbuild --release --target x86_64-unknown-linux-gnu
```

## License

MIT — [Saken Tukenov](https://github.com/stukenov)
