# `<script lang="rust">`

Содержит server-side state, actions, loaders и handlers.

```html
<script lang="rust">
  use crate::db::CustomerRepo;

  prop id: Uuid

  state customer: Customer
  state loading: bool = false
  state error: Option<String> = None

  load async fn load_customer(ctx: Ctx) {
    customer = CustomerRepo::find(ctx.tenant_id, id).await?;
  }

  action async fn save(form: CustomerForm, ctx: Ctx) {
    ctx.require("customers.write")?;
    CustomerRepo::update(ctx.tenant_id, id, form).await?;
    flash.success("Customer saved");
  }
</script>
```

## Типы объявлений

```txt
prop      route/component input
state     reactive server-side state
memo      derived state
load      server loader before render
action    event handler from browser
server    server-only function, not callable from browser
resource  DB/resource binding
```
