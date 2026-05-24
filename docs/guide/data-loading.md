# Data loading

Loader выполняется до SSR.

```rust
load async fn load(ctx: Ctx) {
  ctx.require("customers.read")?;
  customers = CustomerRepo::for_tenant(ctx.tenant_id).await?;
}
```

## Правила

* loader не callable из браузера;
* loader может читать DB/API;
* loader должен respect tenant;
* loader errors идут в error boundary.
