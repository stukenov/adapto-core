# Database/resource layer

## Минимальная интеграция

* PostgreSQL через sqlx;
* SQLite/rqlite adapter optional;
* repository generation;
* migrations generation;
* tenant scoping;
* soft delete optional;
* audit triggers optional.

## Пример resource repository

```rust
impl CustomerRepo {
    pub async fn for_tenant(tenant_id: TenantId) -> Result<Vec<Customer>>;
    pub async fn find(tenant_id: TenantId, id: Uuid) -> Result<Customer>;
    pub async fn create(tenant_id: TenantId, form: CustomerForm) -> Result<Customer>;
    pub async fn update(tenant_id: TenantId, id: Uuid, form: CustomerForm) -> Result<Customer>;
}
```
