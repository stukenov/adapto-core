# adapto_test_utils

Test utilities for the Adapto framework — builders, fixtures, mocks, and assertion helpers.

Part of the [Adapto](https://github.com/stukenov/adapto-core) web framework.

## Features

- **Builders** — Event, Form, Patch, State, TestRequest, TestResponse
- **Fixtures** — `test_ctx`, `test_tenant_id`, seeded stores
- **Mocks** — MockAuditSink, MockClock, MockSecretProvider
- **Assertions** — patch, state, validation, HTTP status, store queries, JSON snapshots

## Quick Start

```toml
[dependencies]
adapto_test_utils = "0.2"
```

```rust
use adapto_test_utils::builders::EventBuilder;
use adapto_test_utils::fixtures::test_ctx;
use adapto_test_utils::mocks::MockAuditSink;
use adapto_test_utils::assertions::{assert_json_includes, assert_status};
use adapto_test_utils::store::StoreSeeder;

// Build test events
let event = EventBuilder::click("submit-btn").build();

// Seed test data
let store = StoreSeeder::new()
    .collection("users", vec![json!({"name": "Alice"})])
    .build()?;

// Assert JSON subsets
assert_json_includes!(actual, json!({"name": "Alice"}));

// Assert HTTP responses
assert_status!(response, 200);
```

## License

MIT — [Saken Tukenov](https://github.com/stukenov)
