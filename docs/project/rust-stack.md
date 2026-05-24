# Recommended Rust stack

```txt
HTTP server:       axum
Async runtime:     tokio
WebSocket:         axum ws / tokio-tungstenite
Parsing:           pest / chumsky / nom
Codegen:           quote + syn / custom generator
Serialization:     serde
Database:          sqlx
Validation:        garde / validator / custom
Tracing:           tracing + opentelemetry
Config:            figment / config
CLI:               clap
Assets:            lightningcss / swc optional
Testing:           cargo test + insta snapshots
```
