# `action`

Action вызывается из браузера через event protocol.

```rust
action increment() {
  count += 1
}
```

## Асинхронный action

```rust
action async fn search() {
  customers = CustomerRepo::search(query).await?;
}
```

## Action с permission

```rust
#[permission("customers.delete")]
action async fn delete_customer(id: Uuid, ctx: Ctx) {
  CustomerRepo::delete(ctx.tenant_id, id).await?;
}
```
