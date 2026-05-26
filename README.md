# Adapto

A Rust web framework for building data-driven sites and apps.

## Crates

| Crate | Description |
|-------|-------------|
| [`adapto_store`](crates/adapto_store) | Embedded document database — JSON documents, BTree indexes, WAL, mmap-backed disk collections |
| [`adapto_app`](crates/adapto_app) | HTTP app builder on axum — declarative routes, path params, multilingual support, WebSocket |
| [`adapto_ui`](crates/adapto_ui) | CSS component library with `html_escape()` utility |

## Quick Start

```toml
[dependencies]
adapto_app = "0.1"
adapto_store = "0.1"
```

```rust
use adapto_app::App;
use adapto_store::AdaptoStore;
use serde_json::json;

#[tokio::main]
async fn main() {
    let store = AdaptoStore::open(None).unwrap();
    
    // Import data
    let users = store.collection("users");
    users.insert(json!({"name": "Alice", "email": "alice@example.com"})).unwrap();
    users.create_index("email", true).unwrap();

    // Build app
    App::new("My App")
        .port(3000)
        .store(store)
        .page("/users/:id", |ctx| {
            let id = ctx.param("id");
            format!("<h1>User {id}</h1>")
        })
        .run()
        .await
        .unwrap();
}
```

## Multilingual Routes

```rust
use adapto_app::{App, LangConfig};

#[derive(Clone)]
enum Lang { Ru, Kk }

impl LangConfig for Lang {
    fn code(&self) -> &str { match self { Lang::Ru => "ru", Lang::Kk => "kk" } }
    fn prefix(&self) -> &str { match self { Lang::Ru => "", Lang::Kk => "/kz" } }
}

// Registers both /about and /kz/about
app.languages(vec![Lang::Ru, Lang::Kk])
   .localized_page("/about", |ctx| {
       format!("<h1>Lang: {}</h1>", ctx.lang_code())
   })
```

## Document Database

```rust
use adapto_store::{AdaptoStore, Query, slugify};

let store = AdaptoStore::open(Some("./data"))?;
let col = store.collection("articles");

// CRUD
let id = col.insert(json!({"title": "Hello", "slug": "hello"}))?;
let doc = col.find_one(Query::eq("slug", "hello"))?;

// Disk-backed collections for large datasets
let companies = store.disk_collection("companies")?;
companies.bulk_insert(docs)?;
companies.create_index("bin", true)?;
let keys = companies.index_keys("bin"); // extract keys without loading docs

// Slugify
assert_eq!(slugify("Привет Мир"), "privet-mir");
assert_eq!(slugify("Қазақстан"), "qazaqstan");
```

## License

MIT
