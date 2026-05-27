# adapto_auth

Authentication for Adapto apps — PBKDF2-SHA256 password hashing, HS256 JWT tokens, session management, CSRF protection, RBAC, rate limiting.

Part of the [Adapto](https://github.com/stukenov/adapto-core) web framework.

## Features

- **Password hashing** — PBKDF2-SHA256 with configurable iterations and salt
- **JWT tokens** — HS256 encode/decode with expiration and custom claims
- **Session management** — in-memory session store with TTL and cleanup
- **CSRF protection** — token generation and validation
- **RBAC** — role-based access control with permission checks
- **Rate limiting** — per-key rate limiter with configurable windows

## Quick Start

```toml
[dependencies]
adapto_auth = "0.2"
```

```rust
use adapto_auth::{hash_password, verify_password, AuthConfig};
use adapto_auth::jwt::{encode_jwt, decode_jwt};
use adapto_auth::session::InMemorySessionStore;

// Password hashing
let hash = hash_password("secret123")?;
assert!(verify_password("secret123", &hash)?);

// JWT
let config = AuthConfig::default();
let token = encode_jwt(&config, &claims)?;
let decoded = decode_jwt(&config, &token)?;

// Sessions
let sessions = InMemorySessionStore::new();
let session_id = sessions.create("user_42")?;
let user = sessions.get(&session_id)?;
```

## License

MIT — [Saken Tukenov](https://github.com/stukenov)
