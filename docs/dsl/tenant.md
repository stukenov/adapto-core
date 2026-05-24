# Tenant isolation

Tenant должен быть встроенным понятием.

```html
<route>
  path: "/customers"
  tenant: required
</route>
```

## Rust context

```rust
ctx.tenant_id
ctx.user_id
ctx.permissions
ctx.request_id
ctx.audit
```

## Scoping

Любой resource query по умолчанию должен требовать tenant scope.

Небезопасный запрос:

```rust
CustomerRepo::all().await?
```

Должен быть запрещен compiler/linter-правилом в tenant-required route.

Правильный запрос:

```rust
CustomerRepo::for_tenant(ctx.tenant_id).await?
```
