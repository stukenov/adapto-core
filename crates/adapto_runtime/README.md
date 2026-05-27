# adapto_runtime

Runtime types for Adapto — reactive state store, request context, permissions, configuration, core type aliases.

Part of the [Adapto](https://github.com/stukenov/adapto-core) web framework.

## Features

- **StateStore** — reactive key-value store with dirty tracking
- **Ctx** — request context with user, tenant, permissions, and metadata
- **PermissionSet** — role-based permission checking
- **Config** — typed configuration management
- **Type aliases** — TenantId, UserId, SessionId, RequestId, RouteId

## Quick Start

```toml
[dependencies]
adapto_runtime = "0.2"
```

```rust
use adapto_runtime::state::StateStore;
use adapto_runtime::ctx::Ctx;
use adapto_runtime::permissions::PermissionSet;

// Reactive state
let mut state = StateStore::new();
state.set("counter", 0.into());
state.set("counter", 1.into());
assert!(state.is_dirty("counter"));

// Request context
let ctx = Ctx::new()
    .user_id("user_42")
    .tenant_id("tenant_1")
    .permissions(PermissionSet::from(&["read", "write"]));

assert!(ctx.permissions().has("read"));
```

## License

MIT — [Saken Tukenov](https://github.com/stukenov)
