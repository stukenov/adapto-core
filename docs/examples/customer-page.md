# Пример: customer page

```html
<route>
  path: "/customers"
  layout: "dashboard"
  auth: required
  tenant: required
  permission: "customers.read"
</route>

<script lang="rust">
  use crate::resources::CustomerRepo;

  state query: String = ""
  state customers: Vec<Customer> = []
  state selected: Option<Uuid> = None

  load async fn load(ctx: Ctx) {
    customers = CustomerRepo::for_tenant(ctx.tenant_id).await?;
  }

  action async fn search(ctx: Ctx) {
    ctx.require("customers.read")?;
    customers = CustomerRepo::search(ctx.tenant_id, query.clone()).await?;
  }

  #[permission("customers.delete")]
  #[audit("customer.deleted")]
  action async fn delete(id: Uuid, ctx: Ctx) {
    CustomerRepo::delete(ctx.tenant_id, id).await?;
    customers = CustomerRepo::search(ctx.tenant_id, query.clone()).await?;
  }
</script>

<template>
  <Page title="Customers">
    <Toolbar>
      <Input bind:value="query" on:input.debounce.300="search" placeholder="Search customers" />

      {#can "customers.create"}
        <Button href="/customers/new">New customer</Button>
      {/can}
    </Toolbar>

    <Table rows={customers}>
      <Column label="Name">{row.name}</Column>
      <Column label="Email">{row.email}</Column>
      <Column label="Status">
        <Badge tone={row.status}>{row.status}</Badge>
      </Column>
      <Column label="Actions">
        <Button href="/customers/{row.id}">Open</Button>

        {#can "customers.delete"}
          <Button tone="danger" on:click="delete(row.id)">Delete</Button>
        {/can}
      </Column>
    </Table>
  </Page>
</template>
```
