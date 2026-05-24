# Permissions

Permissions должны быть доступны на уровне route, component, action и field.

## Route permission

```html
<route>
  path: "/customers"
  auth: required
  permission: "customers.read"
</route>
```

## Action permission

```rust
#[permission("customers.write")]
action async fn save(form: CustomerForm, ctx: Ctx) {
  ...
}
```

## Template permission

```html
{#can "customers.delete"}
  <button on:click="delete_customer(customer.id)">Delete</button>
{/can}
```

## Field permission

```html
<Field name="credit_limit" visible-if-can="customers.credit.read" />
```
