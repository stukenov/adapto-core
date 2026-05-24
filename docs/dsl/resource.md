# Resource DSL

Для CRUD/admin-приложений нужен resource layer.

```html
<resource name="Customer" table="customers">
  tenant: required
  primary_key: id

  field id: Uuid readonly
  field name: String required max=120 searchable
  field email: Email required unique
  field phone: String optional
  field status: Enum[active, inactive, blocked] default=active
  field created_at: DateTime readonly

  permission read: "customers.read"
  permission create: "customers.create"
  permission update: "customers.update"
  permission delete: "customers.delete"
</resource>
```

## Генерация

Из этого compiler может генерировать:

* Rust model;
* repository;
* validation schema;
* admin list page;
* create/edit forms;
* permissions;
* audit events;
* migrations draft;
* API endpoints optional.
