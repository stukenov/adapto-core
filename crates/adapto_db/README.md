# adapto_db

Database abstraction for Adapto — async DatabasePool trait, parameterized SQL generation, migration runner.

Part of the [Adapto](https://github.com/stukenov/adapto-core) web framework.

## Features

- **DatabasePool trait** — async connection pool abstraction
- **SQL generation** — parameterized insert, update, delete, upsert builders
- **Migration runner** — versioned migrations with up/down support
- **In-memory pool** — test-friendly implementation without real database

## Quick Start

```toml
[dependencies]
adapto_db = "0.2"
```

```rust
use adapto_db::{InMemoryPool, DatabasePool};
use adapto_db::sql::{insert_sql, update_sql};
use adapto_db::migration::MigrationRunner;

// SQL generation
let (sql, params) = insert_sql("users", &["name", "email"], &["Alice", "a@b.com"]);
// INSERT INTO users (name, email) VALUES ($1, $2)

// In-memory pool for testing
let pool = InMemoryPool::new();
let rows = pool.query("SELECT * FROM users", &[]).await?;

// Migrations
let runner = MigrationRunner::new(&pool, "./migrations");
runner.run_pending().await?;
```

## License

MIT — [Saken Tukenov](https://github.com/stukenov)
