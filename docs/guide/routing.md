# Routing

## File-based routing

```txt
app/page.adapto                  → /
app/dashboard/page.adapto        → /dashboard
app/customers/page.adapto        → /customers
app/customers/[id]/page.adapto   → /customers/:id
app/customers/layout.adapto      → nested layout
app/api/health/route.rs          → /api/health
```

## Route manifest

```json
{
  "routes": [
    {
      "id": "customers_show",
      "path": "/customers/:id",
      "file": "app/customers/[id]/page.adapto",
      "auth": "required",
      "tenant": "required",
      "permission": "customers.read"
    }
  ]
}
```
