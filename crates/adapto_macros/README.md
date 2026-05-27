# adapto_macros

Derive macros for Adapto — `#[derive(Resource)]` generates collection CRUD operations automatically.

Part of the [Adapto](https://github.com/stukenov/adapto-core) web framework.

## Features

- **#[derive(Resource)]** — generates insert, find, update, delete, exists methods
- **Collection mapping** — `#[resource(collection = "...")]` attribute
- **Index management** — `ensure_indexes` for automatic index creation
- **Query helpers** — `find_one_by`, `find_all`, `delete_where`

## Quick Start

```toml
[dependencies]
adapto_macros = "0.2"
adapto_store = "0.2"
```

```rust
use adapto_macros::Resource;
use adapto_store::AdaptoStore;
use serde::{Deserialize, Serialize};

#[derive(Resource, Serialize, Deserialize)]
#[resource(collection = "users")]
struct User {
    name: String,
    email: String,
}

let store = AdaptoStore::open(None)?;
User::ensure_indexes(&store);

// Generated methods
let id = User::insert_into(&store, &user)?;
let found = User::find_by_id(&store, &id)?;
let all = User::find_all(&store)?;
let by_email = User::find_one_by(&store, "email", "alice@example.com")?;
User::update_in(&store, &id, &updated_user)?;
User::delete_where(&store, "email", "alice@example.com")?;
let exists = User::exists(&store, "email", "alice@example.com")?;
```

## License

MIT — [Saken Tukenov](https://github.com/stukenov)
