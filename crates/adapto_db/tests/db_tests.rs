use adapto_db::query::{Query, Condition, Direction};
use adapto_db::repository::InMemoryRepository;
use adapto_db::migration::{Migration, MigrationPlan, ColumnDef};
use adapto_db::error::DbError;
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

// ===========================================================================
// Query builder
// ===========================================================================

#[test]
fn query_simple_eq() {
    let (sql, params) = Query::table("users")
        .where_eq("name", json!("Alice"))
        .to_sql();

    assert_eq!(sql, "SELECT * FROM users WHERE name = $1");
    assert_eq!(params, vec![json!("Alice")]);
}

#[test]
fn query_ne() {
    let (sql, params) = Query::table("users")
        .where_ne("status", json!("banned"))
        .to_sql();

    assert_eq!(sql, "SELECT * FROM users WHERE status != $1");
    assert_eq!(params, vec![json!("banned")]);
}

#[test]
fn query_gt_lt() {
    let (sql, params) = Query::table("orders")
        .where_gt("amount", json!(100))
        .where_lt("amount", json!(500))
        .to_sql();

    assert_eq!(sql, "SELECT * FROM orders WHERE amount > $1 AND amount < $2");
    assert_eq!(params, vec![json!(100), json!(500)]);
}

#[test]
fn query_gte_lte() {
    let (sql, params) = Query::table("products")
        .where_gte("price", json!(10.0))
        .where_lte("price", json!(99.99))
        .to_sql();

    assert_eq!(
        sql,
        "SELECT * FROM products WHERE price >= $1 AND price <= $2"
    );
    assert_eq!(params, vec![json!(10.0), json!(99.99)]);
}

#[test]
fn query_like() {
    let (sql, params) = Query::table("products")
        .where_like("name", "%widget%")
        .to_sql();

    assert_eq!(sql, "SELECT * FROM products WHERE name LIKE $1");
    assert_eq!(params, vec![json!("%widget%")]);
}

#[test]
fn query_in() {
    let (sql, params) = Query::table("users")
        .where_in("role", vec![json!("admin"), json!("editor")])
        .to_sql();

    assert_eq!(sql, "SELECT * FROM users WHERE role IN ($1, $2)");
    assert_eq!(params, vec![json!("admin"), json!("editor")]);
}

#[test]
fn query_null_checks() {
    let (sql, params) = Query::table("users")
        .where_null("deleted_at")
        .where_not_null("email")
        .to_sql();

    assert_eq!(
        sql,
        "SELECT * FROM users WHERE deleted_at IS NULL AND email IS NOT NULL"
    );
    assert!(params.is_empty());
}

#[test]
fn query_compound_and() {
    let (sql, params) = Query::table("users")
        .and(vec![
            Condition::Eq("status".into(), json!("active")),
            Condition::Gt("age".into(), json!(18)),
        ])
        .to_sql();

    assert_eq!(
        sql,
        "SELECT * FROM users WHERE (status = $1 AND age > $2)"
    );
    assert_eq!(params, vec![json!("active"), json!(18)]);
}

#[test]
fn query_compound_or() {
    let (sql, params) = Query::table("users")
        .or(vec![
            Condition::Eq("role".into(), json!("admin")),
            Condition::Eq("role".into(), json!("superadmin")),
        ])
        .to_sql();

    assert_eq!(
        sql,
        "SELECT * FROM users WHERE (role = $1 OR role = $2)"
    );
    assert_eq!(params, vec![json!("admin"), json!("superadmin")]);
}

#[test]
fn query_order_by_asc() {
    let (sql, _) = Query::table("users")
        .order_by("name", Direction::Asc)
        .to_sql();

    assert_eq!(sql, "SELECT * FROM users ORDER BY name ASC");
}

#[test]
fn query_order_by_desc() {
    let (sql, _) = Query::table("users")
        .order_by("created_at", Direction::Desc)
        .to_sql();

    assert_eq!(sql, "SELECT * FROM users ORDER BY created_at DESC");
}

#[test]
fn query_limit_and_offset() {
    let (sql, _) = Query::table("users").limit(10).offset(20).to_sql();

    assert_eq!(sql, "SELECT * FROM users LIMIT 10 OFFSET 20");
}

#[test]
fn query_limit_only() {
    let (sql, _) = Query::table("users").limit(5).to_sql();

    assert_eq!(sql, "SELECT * FROM users LIMIT 5");
}

#[test]
fn query_no_conditions() {
    let (sql, params) = Query::table("logs").to_sql();

    assert_eq!(sql, "SELECT * FROM logs");
    assert!(params.is_empty());
}

#[test]
fn query_full_combination() {
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

#[test]
fn query_multiple_order_by() {
    let (sql, _) = Query::table("users")
        .order_by("last_name", Direction::Asc)
        .order_by("first_name", Direction::Asc)
        .to_sql();

    assert_eq!(
        sql,
        "SELECT * FROM users ORDER BY last_name ASC, first_name ASC"
    );
}

#[test]
fn query_parameterized_indices_sequential() {
    let (sql, params) = Query::table("t")
        .where_eq("a", json!(1))
        .where_eq("b", json!(2))
        .where_eq("c", json!(3))
        .to_sql();

    assert_eq!(sql, "SELECT * FROM t WHERE a = $1 AND b = $2 AND c = $3");
    assert_eq!(params, vec![json!(1), json!(2), json!(3)]);
}

// ===========================================================================
// InMemoryRepository
// ===========================================================================

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

#[test]
fn repo_find_nonexistent() {
    let repo = InMemoryRepository::<Customer>::new();
    assert_eq!(repo.find(&tenant_a(), &Uuid::new_v4()), None);
}

#[test]
fn repo_for_tenant() {
    let repo = InMemoryRepository::<Customer>::new();
    repo.create(
        &tenant_a(),
        Uuid::new_v4(),
        Customer { name: "Alice".into(), email: "a@x.com".into() },
    );
    repo.create(
        &tenant_a(),
        Uuid::new_v4(),
        Customer { name: "Bob".into(), email: "b@x.com".into() },
    );
    repo.create(
        &tenant_b(),
        Uuid::new_v4(),
        Customer { name: "Charlie".into(), email: "c@x.com".into() },
    );

    assert_eq!(repo.for_tenant(&tenant_a()).len(), 2);
    assert_eq!(repo.for_tenant(&tenant_b()).len(), 1);
}

#[test]
fn repo_update_existing() {
    let repo = InMemoryRepository::<Customer>::new();
    let id = Uuid::new_v4();
    repo.create(
        &tenant_a(),
        id,
        Customer { name: "Alice".into(), email: "a@x.com".into() },
    );

    let updated = Customer { name: "Alice Updated".into(), email: "alice.new@x.com".into() };
    let result = repo.update(&tenant_a(), &id, updated.clone());
    assert_eq!(result, Some(updated.clone()));

    let found = repo.find(&tenant_a(), &id).unwrap();
    assert_eq!(found.name, "Alice Updated");
}

#[test]
fn repo_update_nonexistent() {
    let repo = InMemoryRepository::<Customer>::new();
    let result = repo.update(
        &tenant_a(),
        &Uuid::new_v4(),
        Customer { name: "Ghost".into(), email: "ghost@x.com".into() },
    );
    assert_eq!(result, None);
}

#[test]
fn repo_update_wrong_tenant() {
    let repo = InMemoryRepository::<Customer>::new();
    let id = Uuid::new_v4();
    repo.create(
        &tenant_a(),
        id,
        Customer { name: "Alice".into(), email: "a@x.com".into() },
    );

    let result = repo.update(
        &tenant_b(),
        &id,
        Customer { name: "Hacked".into(), email: "h@x.com".into() },
    );
    assert_eq!(result, None);
    assert_eq!(repo.find(&tenant_a(), &id).unwrap().name, "Alice");
}

#[test]
fn repo_delete_existing() {
    let repo = InMemoryRepository::<Customer>::new();
    let id = Uuid::new_v4();
    repo.create(
        &tenant_a(),
        id,
        Customer { name: "Alice".into(), email: "a@x.com".into() },
    );

    assert!(repo.delete(&tenant_a(), &id));
    assert_eq!(repo.find(&tenant_a(), &id), None);
}

#[test]
fn repo_delete_nonexistent() {
    let repo = InMemoryRepository::<Customer>::new();
    assert!(!repo.delete(&tenant_a(), &Uuid::new_v4()));
}

#[test]
fn repo_delete_wrong_tenant() {
    let repo = InMemoryRepository::<Customer>::new();
    let id = Uuid::new_v4();
    repo.create(
        &tenant_a(),
        id,
        Customer { name: "Alice".into(), email: "a@x.com".into() },
    );

    assert!(!repo.delete(&tenant_b(), &id));
    assert!(repo.find(&tenant_a(), &id).is_some());
}

#[test]
fn repo_search_with_predicate() {
    let repo = InMemoryRepository::<Customer>::new();
    repo.create(
        &tenant_a(),
        Uuid::new_v4(),
        Customer { name: "Alice".into(), email: "alice@x.com".into() },
    );
    repo.create(
        &tenant_a(),
        Uuid::new_v4(),
        Customer { name: "Bob".into(), email: "bob@x.com".into() },
    );
    repo.create(
        &tenant_a(),
        Uuid::new_v4(),
        Customer { name: "Alina".into(), email: "alina@x.com".into() },
    );

    let results = repo.search(&tenant_a(), |c| c.name.starts_with("Al"));
    assert_eq!(results.len(), 2);
}

#[test]
fn repo_search_no_matches() {
    let repo = InMemoryRepository::<Customer>::new();
    repo.create(
        &tenant_a(),
        Uuid::new_v4(),
        Customer { name: "Alice".into(), email: "a@x.com".into() },
    );

    let results = repo.search(&tenant_a(), |c| c.name == "Nobody");
    assert!(results.is_empty());
}

#[test]
fn repo_search_empty_tenant() {
    let repo = InMemoryRepository::<Customer>::new();
    let results = repo.search(&tenant_a(), |_| true);
    assert!(results.is_empty());
}

#[test]
fn repo_count() {
    let repo = InMemoryRepository::<Customer>::new();
    assert_eq!(repo.count(&tenant_a()), 0);

    repo.create(
        &tenant_a(),
        Uuid::new_v4(),
        Customer { name: "Alice".into(), email: "a@x.com".into() },
    );
    repo.create(
        &tenant_a(),
        Uuid::new_v4(),
        Customer { name: "Bob".into(), email: "b@x.com".into() },
    );

    assert_eq!(repo.count(&tenant_a()), 2);
    assert_eq!(repo.count(&tenant_b()), 0);
}

#[test]
fn repo_all_unscoped() {
    let repo = InMemoryRepository::<Customer>::new();
    repo.create(
        &tenant_a(),
        Uuid::new_v4(),
        Customer { name: "Alice".into(), email: "a@x.com".into() },
    );
    repo.create(
        &tenant_b(),
        Uuid::new_v4(),
        Customer { name: "Bob".into(), email: "b@x.com".into() },
    );

    let all = repo.all_unscoped();
    assert_eq!(all.len(), 2);
}

#[test]
fn repo_tenant_isolation() {
    let repo = InMemoryRepository::<Customer>::new();
    let id_a = Uuid::new_v4();
    let id_b = Uuid::new_v4();

    repo.create(
        &tenant_a(),
        id_a,
        Customer { name: "Alice".into(), email: "a@x.com".into() },
    );
    repo.create(
        &tenant_b(),
        id_b,
        Customer { name: "Bob".into(), email: "b@x.com".into() },
    );

    // Cross-tenant lookups return None
    assert_eq!(repo.find(&tenant_a(), &id_b), None);
    assert_eq!(repo.find(&tenant_b(), &id_a), None);

    // Each tenant sees only own records
    assert_eq!(repo.for_tenant(&tenant_a()).len(), 1);
    assert_eq!(repo.for_tenant(&tenant_b()).len(), 1);

    // Cross-tenant delete fails
    assert!(!repo.delete(&tenant_a(), &id_b));
    assert_eq!(repo.find(&tenant_b(), &id_b).unwrap().name, "Bob");
}

// ===========================================================================
// Migration / ColumnDef
// ===========================================================================

#[test]
fn migration_create_table_sql() {
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

#[test]
fn column_def_primary_key_sets_not_nullable() {
    let col = ColumnDef::new("id", "UUID").primary_key();
    assert!(col.primary_key);
    assert!(!col.nullable);
}

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

#[test]
fn column_def_defaults() {
    let col = ColumnDef::new("notes", "TEXT");
    assert!(col.nullable);
    assert!(!col.primary_key);
    assert!(!col.unique);
    assert!(col.default.is_none());
}

#[test]
fn migration_plan_add_ordering() {
    let plan = MigrationPlan::new()
        .add(Migration {
            version: "001".into(),
            name: "create_users".into(),
            up: "CREATE TABLE users ();".into(),
            down: "DROP TABLE users;".into(),
        })
        .add(Migration {
            version: "002".into(),
            name: "create_orders".into(),
            up: "CREATE TABLE orders ();".into(),
            down: "DROP TABLE orders;".into(),
        });

    assert_eq!(plan.migrations.len(), 2);
    assert_eq!(plan.migrations[0].version, "001");
    assert_eq!(plan.migrations[1].version, "002");
}

// ===========================================================================
// Error display
// ===========================================================================

#[test]
fn db_error_display() {
    assert_eq!(DbError::NotFound.to_string(), "Record not found");
    assert_eq!(DbError::Duplicate.to_string(), "Duplicate record");
    assert_eq!(
        DbError::TenantScopeRequired.to_string(),
        "Tenant scope required"
    );
    assert_eq!(
        DbError::QueryError("bad syntax".into()).to_string(),
        "Query error: bad syntax"
    );
    assert_eq!(
        DbError::MigrationError("version conflict".into()).to_string(),
        "Migration error: version conflict"
    );
    assert_eq!(
        DbError::ConnectionError("timeout".into()).to_string(),
        "Connection error: timeout"
    );
}
