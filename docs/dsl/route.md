# `<route>`

Описывает маршрут и системные требования.

```html
<route>
  path: "/customers/[id]"
  method: "GET"
  layout: "dashboard"
  auth: required
  tenant: required
  permission: "customers.read"
  cache: no-store
</route>
```

## Поля

```txt
path          route path
method        GET/POST/etc, default GET
layout        layout name
page_title    HTML title
auth          public | optional | required
role          required role
permission    required permission
tenant        none | optional | required
cache         no-store | private | public | static
```
