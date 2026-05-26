# adapto_client_protocol

WebSocket protocol types for Adapto client-server communication — DOM patch operations, server messages, action dispatching.

Part of the [Adapto](https://github.com/stukenov/adapto-core) web framework.

## Features

- **PatchOp** — DOM patch operations: `ReplaceHtml`, `SetAttribute`, `RemoveElement`
- **ServerMessage** — typed server-to-client messages with payload
- **PatchMessage** — batched patch operations for efficient DOM updates

## Usage

This crate is used internally by `adapto_app` for WebSocket communication. You typically don't need to depend on it directly.

```toml
[dependencies]
adapto_client_protocol = "0.1"
```

```rust
use adapto_client_protocol::patch::{PatchOp, PatchMessage};

let patch = PatchMessage {
    ops: vec![
        PatchOp::ReplaceHtml {
            target: "#content".to_string(),
            html: "<p>Updated</p>".to_string(),
        },
    ],
};
```

## License

MIT — [Saken Tukenov](https://github.com/stukenov)
