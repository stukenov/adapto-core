# Compiler errors

Ошибки должны быть строгими и понятными.

## Unknown action

```txt
E0101: Unknown action `sav`.
  app/customers/page.adapto:42:18

  <Button on:click="sav">Save</Button>
                   ^^^

Did you mean `save`?
```

## Security error

```txt
E0421: Secret state `api_key` cannot be rendered in template.
  app/settings/page.adapto:18:12

  <code>{api_key}</code>
         ^^^^^^^
```

## Tenant error

```txt
E0702: Tenant-required route uses unscoped repository query.
  CustomerRepo::all().await?

Use:
  CustomerRepo::for_tenant(ctx.tenant_id).await?
```
