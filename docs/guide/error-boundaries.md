# Error boundaries

```html
<error-boundary>
  <template #error="err">
    <Alert tone="danger">{err.message}</Alert>
  </template>

  <CustomerTable customers={customers} />
</error-boundary>
```

## На уровне route

```html
<route>
  error: "app/errors/500.adapto"
  not_found: "app/errors/404.adapto"
</route>
```
