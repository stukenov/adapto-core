# adapto

Umbrella crate re-exporting all Adapto framework crates — one dependency to get the full stack.

Part of the [Adapto](https://github.com/stukenov/adapto-core) web framework.

## Features

- **Default features** — app, ui, forms, auth, audit, macros, live
- **`full` feature** — adds ai, db, parser, compiler, ssr
- **`no-default-features`** — store only, minimal footprint
- **Prelude** — common types re-exported for convenience

## Quick Start

```toml
[dependencies]
adapto = "0.2"

# Or with all features
# adapto = { version = "0.2", features = ["full"] }

# Or minimal (store only)
# adapto = { version = "0.2", default-features = false }
```

```rust
use adapto::prelude::*;

let store = AdaptoStore::open(None)?;
let users = store.collection("users");

users.insert(json!({"name": "Alice"}))?;
users.create_index("name", true)?;

let doc = users.find_one(Query::eq("name", "Alice"))?;
```

## License

MIT — [Saken Tukenov](https://github.com/stukenov)
