# Audit

Actions по умолчанию пишут audit events.

```rust
#[audit("customer.updated")]
action async fn save(form: CustomerForm, ctx: Ctx) {
  ...
}
```

## Audit event

```json
{
  "event": "customer.updated",
  "tenant_id": "...",
  "user_id": "...",
  "route": "/customers/123",
  "action": "save",
  "timestamp": "...",
  "request_id": "..."
}
```
