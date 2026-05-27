# adapto_store

Embedded document database for Rust — JSON documents, BTree indexes, WAL persistence, mmap-backed disk collections.

Part of the [Adapto](https://github.com/stukenov/adapto-core) web framework.

## Features

- **Document store** — insert, query, update, delete JSON documents
- **BTree indexes** — unique and compound indexes, O(1) lookups via `Query::eq()`
- **WAL persistence** — write-ahead log with compaction
- **DiskCollection** — mmap-backed storage for 100K+ document datasets
- **Query builder** — Eq, Ne, Gt, Lt, In, Regex, Contains, And, Or, Not + sort/limit/skip/projection
- **Slugify** — Cyrillic transliteration (Russian + Kazakh) to URL-safe slugs

## Quick Start

```toml
[dependencies]
adapto_store = "0.2"
```

```rust
use adapto_store::{AdaptoStore, Query};
use serde_json::json;

let store = AdaptoStore::open(Some("./data"))?;
let users = store.collection("users");

// Insert
let id = users.insert(json!({"name": "Alice", "email": "alice@example.com"}))?;

// Index
users.create_index("email", true)?; // unique

// Query
let doc = users.find_one(Query::eq("email", "alice@example.com"))?;
let all = users.find(Query::new().sort("name", SortDir::Asc).limit(10));

// Update
users.update(
    Query::eq("email", "alice@example.com"),
    Update::Set(vec![("name".into(), json!("Alice Smith"))]),
)?;

// Delete
users.delete_by_id(&id)?;
```

## DiskCollection

For large datasets (100K+ documents), use mmap-backed disk collections:

```rust
let companies = store.disk_collection("companies")?;
companies.bulk_insert(docs)?;
companies.create_index("bin", true)?;

let doc = companies.find_one(Query::eq("bin", "123456789012"))?;
let keys = companies.index_keys("bin"); // Vec<String> — keys without loading docs
```

## Slugify

Cyrillic transliteration for URL-safe slugs:

```rust
use adapto_store::slugify;

assert_eq!(slugify("Привет Мир"), "privet-mir");
assert_eq!(slugify("Қазақстан"), "qazaqstan");
assert_eq!(slugify("Hello World!"), "hello-world");
```

## License

MIT — [Saken Tukenov](https://github.com/stukenov)
