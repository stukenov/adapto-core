# Adapto

[![crates.io](https://img.shields.io/crates/v/adapto_app.svg)](https://crates.io/crates/adapto_app)
[![license](https://img.shields.io/crates/l/adapto_app.svg)](LICENSE)

A Rust web framework for building data-driven sites and apps. Embedded document database, declarative routing, multilingual support — all in one workspace.

**Production-proven** on [myqaz.kz](https://myqaz.kz) — 140K+ pages, 200K+ documents, Kazakh/Russian bilingual.

## Crates

| Crate | Version | Description |
|-------|---------|-------------|
| [`adapto_store`](crates/adapto_store) | [![](https://img.shields.io/crates/v/adapto_store.svg)](https://crates.io/crates/adapto_store) | Embedded document DB — JSON, BTree indexes, WAL, mmap DiskCollection |
| [`adapto_app`](crates/adapto_app) | [![](https://img.shields.io/crates/v/adapto_app.svg)](https://crates.io/crates/adapto_app) | HTTP app builder on axum — routes, multilingual, WebSocket |
| [`adapto_ui`](crates/adapto_ui) | [![](https://img.shields.io/crates/v/adapto_ui.svg)](https://crates.io/crates/adapto_ui) | CSS components + `html_escape()` |
| [`adapto_client_protocol`](crates/adapto_client_protocol) | [![](https://img.shields.io/crates/v/adapto_client_protocol.svg)](https://crates.io/crates/adapto_client_protocol) | WebSocket protocol types |

## Quick Start

```toml
[dependencies]
adapto_app = "0.1"
adapto_store = "0.1"
```

```rust
use adapto_app::{App, PageResponse};
use adapto_store::{AdaptoStore, Query};
use serde_json::json;

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

## Multilingual Routes

Register one route, serve multiple languages:

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
let keys = companies.index_keys("bin"); // keys without loading docs

// Slugify (Cyrillic → Latin)
assert_eq!(slugify("Привет Мир"), "privet-mir");
assert_eq!(slugify("Қазақстан"), "qazaqstan");
```

## Cross-Compilation

Build on macOS for Linux deployment:

```bash
cargo install cargo-zigbuild
cargo zigbuild --release --target x86_64-unknown-linux-gnu
```

## License

MIT — [Saken Tukenov](https://github.com/stukenov)
