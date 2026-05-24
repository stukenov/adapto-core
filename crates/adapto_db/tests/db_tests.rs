use adapto_db::migration::{ColumnDef, MigrationPlan};
use adapto_db::query::{Direction, Query};
use adapto_db::repository::InMemoryRepository;
use adapto_runtime::types::TenantId;
use serde_json::json;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Test data
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
struct Customer {
    name: String,
    email: String,
}

fn tenant_a() -> TenantId {
    TenantId(Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap())
}

fn tenant_b() -> TenantId {
    TenantId(Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap())
}

// ---------------------------------------------------------------------------
// 1. InMemoryRepository: create and find
// ---------------------------------------------------------------------------

#[test]
fn repo_create_and_find() {
    let repo = InMemoryRepository::<Customer>::new();
    let id = Uuid::new_v4();
    let customer = Customer {
        name: "Alice".into(),
        email: "alice@example.com".into(),
    };
    repo.create(&tenant_a(), id, customer.clone());

    let found = repo.find(&tenant_a(), &id);
    assert_eq!(found, Some(customer));
}

// ---------------------------------------------------------------------------
// 2. InMemoryRepository: for_tenant returns only tenant records
// ---------------------------------------------------------------------------

#[test]
fn repo_for_tenant() {
    let repo = InMemoryRepository::<Customer>::new();
    let id1 = Uuid::new_v4();
    let id2 = Uuid::new_v4();

    repo.create(
        &tenant_a(),
        id1,
        Customer {
            name: "Alice".into(),
            email: "a@x.com".into(),
        },
    );
    repo.create(
        &tenant_a(),
        id2,
        Customer {
            name: "Bob".into(),
            email: "b@x.com".into(),
        },
    );
    repo.create(
        &tenant_b(),
        Uuid::new_v4(),
        Customer {
            name: "Charlie".into(),
            email: "c@x.com".into(),
        },
    );

    let tenant_a_records = repo.for_tenant(&tenant_a());
    assert_eq!(tenant_a_records.len(), 2);
}

// ---------------------------------------------------------------------------
// 3. InMemoryRepository: update existing
// ---------------------------------------------------------------------------

#[test]
fn repo_update_existing() {
    let repo = InMemoryRepository::<Customer>::new();
    let id = Uuid::new_v4();
    repo.create(
        &tenant_a(),
        id,
        Customer {
            name: "Alice".into(),
            email: "a@x.com".into(),
        },
    );

    let updated = Customer {
        name: "Alice Updated".into(),
        email: "alice.new@x.com".into(),
    };
    let result = repo.update(&tenant_a(), &id, updated.clone());
    assert_eq!(result, Some(updated.clone()));

    let found = repo.find(&tenant_a(), &id).unwrap();
    assert_eq!(found.name, "Alice Updated");
}

// ---------------------------------------------------------------------------
// 4. InMemoryRepository: update nonexistent returns None
// ---------------------------------------------------------------------------

#[test]
fn repo_update_nonexistent() {
    let repo = InMemoryRepository::<Customer>::new();
    let result = repo.update(
        &tenant_a(),
        &Uuid::new_v4(),
        Customer {
            name: "Ghost".into(),
            email: "ghost@x.com".into(),
        },
    );
    assert_eq!(result, None);
}

// ---------------------------------------------------------------------------
// 5. InMemoryRepository: delete existing
// ---------------------------------------------------------------------------

#[test]
fn repo_delete_existing() {
    let repo = InMemoryRepository::<Customer>::new();
    let id = Uuid::new_v4();
    repo.create(
        &tenant_a(),
        id,
        Customer {
            name: "Alice".into(),
            email: "a@x.com".into(),
        },
    );

    assert!(repo.delete(&tenant_a(), &id));
    assert_eq!(repo.find(&tenant_a(), &id), None);
}

// ---------------------------------------------------------------------------
// 6. InMemoryRepository: delete nonexistent returns false
// ---------------------------------------------------------------------------

#[test]
fn repo_delete_nonexistent() {
    let repo = InMemoryRepository::<Customer>::new();
    assert!(!repo.delete(&tenant_a(), &Uuid::new_v4()));
}

// ---------------------------------------------------------------------------
// 7. InMemoryRepository: search with predicate
// ---------------------------------------------------------------------------

#[test]
fn repo_search_with_predicate() {
    let repo = InMemoryRepository::<Customer>::new();
    repo.create(
        &tenant_a(),
        Uuid::new_v4(),
        Customer {
            name: "Alice".into(),
            email: "alice@x.com".into(),
        },
    );
    repo.create(
        &tenant_a(),
        Uuid::new_v4(),
        Customer {
            name: "Bob".into(),
            email: "bob@x.com".into(),
        },
    );
    repo.create(
        &tenant_a(),
        Uuid::new_v4(),
        Customer {
            name: "Alina".into(),
            email: "alina@x.com".into(),
        },
    );

    let results = repo.search(&tenant_a(), |c| c.name.starts_with("Al"));
    assert_eq!(results.len(), 2);
}

// ---------------------------------------------------------------------------
// 8. InMemoryRepository: count
// ---------------------------------------------------------------------------

#[test]
fn repo_count() {
    let repo = InMemoryRepository::<Customer>::new();
    assert_eq!(repo.count(&tenant_a()), 0);

    repo.create(
        &tenant_a(),
        Uuid::new_v4(),
        Customer {
            name: "Alice".into(),
            email: "a@x.com".into(),
        },
    );
    repo.create(
        &tenant_a(),
        Uuid::new_v4(),
        Customer {
            name: "Bob".into(),
            email: "b@x.com".into(),
        },
    );

    assert_eq!(repo.count(&tenant_a()), 2);
}

// ---------------------------------------------------------------------------
// 9. InMemoryRepository: all_unscoped returns everything
// ---------------------------------------------------------------------------

#[test]
fn repo_all_unscoped() {
    let repo = InMemoryRepository::<Customer>::new();
    repo.create(
        &tenant_a(),
        Uuid::new_v4(),
        Customer {
            name: "Alice".into(),
            email: "a@x.com".into(),
        },
    );
    repo.create(
        &tenant_b(),
        Uuid::new_v4(),
        Customer {
            name: "Bob".into(),
            email: "b@x.com".into(),
        },
    );

    let all = repo.all_unscoped();
    assert_eq!(all.len(), 2);
}

// ---------------------------------------------------------------------------
// 10. InMemoryRepository: tenant isolation
// ---------------------------------------------------------------------------

#[test]
fn repo_tenant_isolation() {
    let repo = InMemoryRepository::<Customer>::new();
    let id_a = Uuid::new_v4();
    let id_b = Uuid::new_v4();

    repo.create(
        &tenant_a(),
        id_a,
        Customer {
            name: "Alice".into(),
            email: "a@x.com".into(),
        },
    );
    repo.create(
        &tenant_b(),
        id_b,
        Customer {
            name: "Bob".into(),
            email: "b@x.com".into(),
        },
    );

    // Tenant A cannot see Tenant B's records.
    assert_eq!(repo.find(&tenant_a(), &id_b), None);
    assert_eq!(repo.find(&tenant_b(), &id_a), None);

    // Each tenant sees only their own records.
    assert_eq!(repo.for_tenant(&tenant_a()).len(), 1);
    assert_eq!(repo.for_tenant(&tenant_b()).len(), 1);

    // Deleting from wrong tenant does nothing.
    assert!(!repo.delete(&tenant_a(), &id_b));
    assert_eq!(repo.find(&tenant_b(), &id_b).unwrap().name, "Bob");
}

// ---------------------------------------------------------------------------
// 11. Query builder: simple eq
// ---------------------------------------------------------------------------

#[test]
fn query_simple_eq() {
    let (sql, params) = Query::table("users")
        .where_eq("name", json!("Alice"))
        .to_sql();

    assert_eq!(sql, "SELECT * FROM users WHERE name = $1");
    assert_eq!(params, vec![json!("Alice")]);
}

// ---------------------------------------------------------------------------
// 12. Query builder: multiple conditions
// ---------------------------------------------------------------------------

#[test]
fn query_multiple_conditions() {
    let (sql, params) = Query::table("users")
        .where_eq("status", json!("active"))
        .where_gt("age", json!(18))
        .to_sql();

    assert_eq!(
        sql,
        "SELECT * FROM users WHERE status = $1 AND age > $2"
    );
    assert_eq!(params, vec![json!("active"), json!(18)]);
}

// ---------------------------------------------------------------------------
// 13. Query builder: order_by
// ---------------------------------------------------------------------------

#[test]
fn query_order_by() {
    let (sql, _) = Query::table("users")
        .order_by("name", Direction::Asc)
        .to_sql();

    assert_eq!(sql, "SELECT * FROM users ORDER BY name ASC");
}

// ---------------------------------------------------------------------------
// 14. Query builder: limit and offset
// ---------------------------------------------------------------------------

#[test]
fn query_limit_and_offset() {
    let (sql, _) = Query::table("users").limit(10).offset(20).to_sql();

    assert_eq!(sql, "SELECT * FROM users LIMIT 10 OFFSET 20");
}

// ---------------------------------------------------------------------------
// 15. Query builder: to_sql generates valid SQL
// ---------------------------------------------------------------------------

#[test]
fn query_to_sql_full() {
    let (sql, params) = Query::table("orders")
        .where_eq("tenant_id", json!("abc"))
        .where_like("description", "%urgent%")
        .order_by("created_at", Direction::Desc)
        .limit(25)
        .offset(50)
        .to_sql();

    assert_eq!(
        sql,
        "SELECT * FROM orders WHERE tenant_id = $1 AND description LIKE $2 ORDER BY created_at DESC LIMIT 25 OFFSET 50"
    );
    assert_eq!(params.len(), 2);
    assert_eq!(params[0], json!("abc"));
    assert_eq!(params[1], json!("%urgent%"));
}

// ---------------------------------------------------------------------------
// 16. Query builder: like condition
// ---------------------------------------------------------------------------

#[test]
fn query_like_condition() {
    let (sql, params) = Query::table("products")
        .where_like("name", "%widget%")
        .to_sql();

    assert_eq!(sql, "SELECT * FROM products WHERE name LIKE $1");
    assert_eq!(params, vec![json!("%widget%")]);
}

// ---------------------------------------------------------------------------
// 17. Migration: create_table generates SQL
// ---------------------------------------------------------------------------

#[test]
fn migration_create_table() {
    let migration = MigrationPlan::create_table(
        "customers",
        vec![
            ColumnDef::new("id", "UUID").primary_key(),
            ColumnDef::new("name", "VARCHAR(255)").not_null(),
            ColumnDef::new("email", "VARCHAR(255)").not_null().unique(),
            ColumnDef::new("created_at", "TIMESTAMPTZ")
                .not_null()
                .default_value("NOW()"),
        ],
    );

    assert_eq!(migration.name, "create_customers");
    assert!(migration.up.contains("CREATE TABLE customers"));
    assert!(migration.up.contains("id UUID PRIMARY KEY"));
    assert!(migration.up.contains("name VARCHAR(255) NOT NULL"));
    assert!(migration.up.contains("email VARCHAR(255) NOT NULL UNIQUE"));
    assert!(migration.up.contains("created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()"));
    assert!(migration.down.contains("DROP TABLE IF EXISTS customers"));
}

// ---------------------------------------------------------------------------
// 18. ColumnDef with all options
// ---------------------------------------------------------------------------

#[test]
fn column_def_all_options() {
    let col = ColumnDef::new("status", "VARCHAR(50)")
        .not_null()
        .unique()
        .default_value("'active'");

    assert_eq!(col.name, "status");
    assert_eq!(col.sql_type, "VARCHAR(50)");
    assert!(!col.nullable);
    assert!(!col.primary_key);
    assert!(col.unique);
    assert_eq!(col.default, Some("'active'".to_string()));
}
