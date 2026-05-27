# adapto_live

Live updates for Adapto — WebSocket session management, DOM patching, event handling, action interpreter, dependency-driven re-render.

Part of the [Adapto](https://github.com/stukenov/adapto-core) web framework.

## Features

- **WebSocket sessions** — manage connected clients with SessionManager
- **DOM patching** — efficient PatchOps (SetText, SetAttr, InsertNode, RemoveNode, etc.)
- **Event handling** — client events routed to server-side action handlers
- **Action interpreter** — assignments, if/else, for loops, method calls
- **Dependency tracking** — re-render only components affected by state changes

## Quick Start

```toml
[dependencies]
adapto_live = "0.2"
```

```rust
use adapto_live::session::{SessionManager, LiveSession};
use adapto_live::patch::PatchOp;

// Session management
let manager = SessionManager::new();
let session = manager.create_session("user_42")?;

// DOM patching
let patches = vec![
    PatchOp::SetText { target: "counter".into(), value: "42".into() },
    PatchOp::SetAttr { target: "btn".into(), attr: "disabled".into(), value: "true".into() },
];
session.send_patches(&patches)?;
```

## License

MIT — [Saken Tukenov](https://github.com/stukenov)
