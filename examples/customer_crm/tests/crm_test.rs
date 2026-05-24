use adapto_audit::event::{AuditEvent, AuditStatus};
use adapto_audit::sink::{AuditSink, InMemoryAuditSink};
use adapto_auth::rbac::{RbacStore, Role};
use adapto_compiler::compiler::Compiler;
use adapto_db::repository::InMemoryRepository;
use adapto_forms::schema::{FieldSchema, FieldType, FormSchema};
use adapto_runtime::context::{Ctx, PermissionSet};
use adapto_runtime::state::StateStore;
use adapto_runtime::types::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashSet;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Customer model (shared across tests)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Customer {
    id: Uuid,
    name: String,
    email: String,
    phone: Option<String>,
    status: String,
}

// ---------------------------------------------------------------------------
// Shared DSL
// ---------------------------------------------------------------------------

const CUSTOMER_PAGE_DSL: &str = r#"
<route>
  path: "/customers"
  layout: "dashboard"
  auth: required
  tenant: required
  permission: "customers.read"
</route>

<script lang="rust">
  state query: String = ""
  state customers: Vec<Customer> = []

  action async fn search(ctx: Ctx) {
    ctx.require("customers.read")?;
    customers = CustomerRepo::search(ctx.tenant_id, query.clone()).await?;
  }

  #[permission("customers.delete")]
  #[audit("customer.deleted")]
  action async fn delete(id: Uuid, ctx: Ctx) {
    CustomerRepo::delete(ctx.tenant_id, id).await?;
  }
</script>

<template>
  <div class="customer-list">
    <h1>Customers</h1>
    <input bind:value="query" on:input="search" />
    {#each customers as customer}
      <div class="customer-row">
        <span>{customer.name}</span>
        <span>{customer.email}</span>
        {#can "customers.delete"}
          <button on:click="delete(customer.id)">Delete</button>
        {/can}
      </div>
    {/each}
  </div>
</template>

<style scoped>
  .customer-list { max-width: 960px; margin: 0 auto; }
  .customer-row { display: flex; gap: 1rem; padding: 0.75rem 0; border-bottom: 1px solid #e5e5e5; }
</style>
"#;

const RESOURCE_DSL: &str = r#"
<resource name="Customer" table="customers">
  tenant: required
  primary_key: id

  field id: Uuid required readonly
  field name: String required min=2 max=120 searchable
  field email: String required unique searchable
  field phone: Option<String>
  field status: Enum[active, inactive, blocked] required default=active
  field created_at: DateTime readonly
  field updated_at: DateTime readonly

  permission read: "customers.read"
  permission create: "customers.create"
  permission update: "customers.update"
  permission delete: "customers.delete"
</resource>
"#;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn setup_rbac() -> (RbacStore, UserId, UserId) {
    let mut rbac = RbacStore::new();

    let mut admin_perms = HashSet::new();
    admin_perms.insert("customers.read".into());
    admin_perms.insert("customers.create".into());
    admin_perms.insert("customers.update".into());
    admin_perms.insert("customers.delete".into());
    rbac.add_role(Role {
        name: "admin".into(),
        permissions: admin_perms,
    });

    let mut viewer_perms = HashSet::new();
    viewer_perms.insert("customers.read".into());
    rbac.add_role(Role {
        name: "viewer".into(),
        permissions: viewer_perms,
    });

    let admin = UserId(Uuid::new_v4());
    let viewer = UserId(Uuid::new_v4());
    rbac.assign_role(&admin, "admin");
    rbac.assign_role(&viewer, "viewer");

    (rbac, admin, viewer)
}

fn make_ctx(
    user_id: &UserId,
    tenant_id: &TenantId,
    permissions: PermissionSet,
) -> Ctx {
    Ctx {
        user_id: Some(user_id.clone()),
        tenant_id: Some(tenant_id.clone()),
        request_id: RequestId::default(),
        permissions,
        route: RouteId("/customers".into()),
        session_id: SessionId("test-session".into()),
    }
}

fn customer_form() -> FormSchema {
    FormSchema::new("CustomerForm")
        .field(
            FieldSchema::new("name", FieldType::String)
                .required()
                .min_length(2)
                .max_length(120),
        )
        .field(FieldSchema::new("email", FieldType::Email).required())
        .field(
            FieldSchema::new("phone", FieldType::Optional(Box::new(FieldType::String)))
                .max_length(32),
        )
        .field(
            FieldSchema::new(
                "status",
                FieldType::Enum(vec![
                    "active".into(),
                    "inactive".into(),
                    "blocked".into(),
                ]),
            )
            .required(),
        )
}

fn seed_repo() -> (InMemoryRepository<Customer>, TenantId, TenantId, Uuid, Uuid, Uuid) {
    let repo: InMemoryRepository<Customer> = InMemoryRepository::new();
    let tenant_a = TenantId(Uuid::new_v4());
    let tenant_b = TenantId(Uuid::new_v4());

    let c1 = Uuid::new_v4();
    let c2 = Uuid::new_v4();
    let c3 = Uuid::new_v4();

    repo.create(
        &tenant_a,
        c1,
        Customer {
            id: c1,
            name: "Alice Corp".into(),
            email: "alice@corp.kz".into(),
            phone: Some("+7 701 111 1111".into()),
            status: "active".into(),
        },
    );
    repo.create(
        &tenant_a,
        c2,
        Customer {
            id: c2,
            name: "Bob LLC".into(),
            email: "bob@llc.kz".into(),
            phone: None,
            status: "active".into(),
        },
    );
    repo.create(
        &tenant_b,
        c3,
        Customer {
            id: c3,
            name: "Charlie Inc".into(),
            email: "charlie@inc.kz".into(),
            phone: None,
            status: "inactive".into(),
        },
    );

    (repo, tenant_a, tenant_b, c1, c2, c3)
}

// ===========================================================================
// Tests
// ===========================================================================

// ---- 1. Parse customer page DSL ------------------------------------------

#[test]
fn parse_customer_page_dsl() {
    let ast = adapto_parser::parse(CUSTOMER_PAGE_DSL).unwrap();

    assert!(ast.route.is_some());
    assert!(ast.script.is_some());
    assert!(ast.template.is_some());

    let route = ast.route.as_ref().unwrap();
    assert_eq!(route.path.as_deref(), Some("/customers"));
    assert_eq!(
        route.permission.as_deref(),
        Some("customers.read")
    );

    let script = ast.script.as_ref().unwrap();
    assert_eq!(script.states.len(), 2);
    assert_eq!(script.states[0].name, "query");
    assert_eq!(script.states[1].name, "customers");
    assert_eq!(script.actions.len(), 2);
    assert_eq!(script.actions[0].name, "search");
    assert_eq!(script.actions[1].name, "delete");
}

// ---- 2. Compile customer page --------------------------------------------

#[test]
fn compile_customer_page() {
    let ast = adapto_parser::parse(CUSTOMER_PAGE_DSL).unwrap();
    let mut compiler = Compiler::new();
    let output = compiler
        .compile_file(&ast, "customers/page.adapto")
        .expect("Compile failed");

    assert_eq!(output.component_ir.name, "Page");
    assert!(!output.component_ir.static_segments.is_empty());
    assert!(!output.component_ir.dynamic_segments.is_empty());
    assert!(!output.component_ir.events.is_empty());
    assert!(!output.generated_rust.is_empty());

    // Route entry should exist
    assert!(output.route_entry.is_some());
    let route = output.route_entry.unwrap();
    assert_eq!(route.path, "/customers");
}

// ---- 3. Tenant isolation: create in A, invisible in B --------------------

#[test]
fn tenant_isolation_invisible_across_tenants() {
    let (repo, tenant_a, tenant_b, _, _, _) = seed_repo();

    let a_customers = repo.for_tenant(&tenant_a);
    let b_customers = repo.for_tenant(&tenant_b);

    assert_eq!(a_customers.len(), 2);
    assert_eq!(b_customers.len(), 1);

    // Tenant A cannot see Charlie (tenant B)
    let a_names: Vec<&str> = a_customers.iter().map(|c| c.name.as_str()).collect();
    assert!(!a_names.contains(&"Charlie Inc"));

    // Tenant B cannot see Alice or Bob (tenant A)
    let b_names: Vec<&str> = b_customers.iter().map(|c| c.name.as_str()).collect();
    assert!(!b_names.contains(&"Alice Corp"));
    assert!(!b_names.contains(&"Bob LLC"));
}

// ---- 4. RBAC: admin has delete, viewer doesn't ---------------------------

#[test]
fn rbac_admin_has_delete_viewer_does_not() {
    let (rbac, admin, viewer) = setup_rbac();

    let admin_perms = rbac.get_permissions(&admin);
    let viewer_perms = rbac.get_permissions(&viewer);

    assert!(admin_perms.has("customers.delete"));
    assert!(!viewer_perms.has("customers.delete"));
}

// ---- 5. Permission check: admin can delete -------------------------------

#[test]
fn permission_check_admin_can_delete() {
    let (rbac, admin, _) = setup_rbac();
    let tenant = TenantId(Uuid::new_v4());
    let ctx = make_ctx(&admin, &tenant, rbac.get_permissions(&admin));

    assert!(ctx.require("customers.delete").is_ok());
    assert!(ctx.require("customers.read").is_ok());
    assert!(ctx.require("customers.create").is_ok());
    assert!(ctx.require("customers.update").is_ok());
}

// ---- 6. Permission check: viewer cannot delete ---------------------------

#[test]
fn permission_check_viewer_cannot_delete() {
    let (rbac, _, viewer) = setup_rbac();
    let tenant = TenantId(Uuid::new_v4());
    let ctx = make_ctx(&viewer, &tenant, rbac.get_permissions(&viewer));

    assert!(ctx.require("customers.read").is_ok());
    assert!(ctx.require("customers.delete").is_err());
    assert!(ctx.require("customers.create").is_err());
    assert!(ctx.require("customers.update").is_err());
}

// ---- 7. Form validation: valid data passes -------------------------------

#[test]
fn form_validation_valid_data_passes() {
    let form = customer_form();
    let data: serde_json::Map<String, serde_json::Value> = serde_json::from_value(json!({
        "name": "New Customer",
        "email": "new@customer.kz",
        "phone": "+7 702 222 2222",
        "status": "active"
    }))
    .unwrap();

    let result = form.validate(&data);
    assert!(result.is_valid());
}

// ---- 8. Form validation: invalid email fails -----------------------------

#[test]
fn form_validation_invalid_email_fails() {
    let form = customer_form();
    let data: serde_json::Map<String, serde_json::Value> = serde_json::from_value(json!({
        "name": "Valid Name",
        "email": "not-an-email",
        "status": "active"
    }))
    .unwrap();

    let result = form.validate(&data);
    assert!(!result.is_valid());

    let email_errors = result.field_errors("email");
    assert!(!email_errors.is_empty());
    assert_eq!(email_errors[0].code, "invalid_email");
}

// ---- 9. Form validation: name too short fails ----------------------------

#[test]
fn form_validation_name_too_short_fails() {
    let form = customer_form();
    let data: serde_json::Map<String, serde_json::Value> = serde_json::from_value(json!({
        "name": "X",
        "email": "x@test.kz",
        "status": "active"
    }))
    .unwrap();

    let result = form.validate(&data);
    assert!(!result.is_valid());

    let name_errors = result.field_errors("name");
    assert!(!name_errors.is_empty());
    assert_eq!(name_errors[0].code, "min_length");
}

// ---- 10. Audit event creation --------------------------------------------

#[test]
fn audit_event_creation() {
    let (rbac, admin, _) = setup_rbac();
    let tenant = TenantId(Uuid::new_v4());
    let ctx = make_ctx(&admin, &tenant, rbac.get_permissions(&admin));

    let sink = InMemoryAuditSink::new();

    let event = AuditEvent::new("customer.deleted", &ctx, "delete").success();
    sink.write(event);

    assert_eq!(sink.len(), 1);

    let events = sink.events();
    assert_eq!(events[0].event, "customer.deleted");
    assert_eq!(events[0].action, "delete");
    assert_eq!(events[0].status, AuditStatus::Success);
    assert_eq!(events[0].tenant_id, Some(tenant));
    assert_eq!(events[0].user_id, Some(admin));
}

// ---- 11. State dirty tracking after search -------------------------------

#[test]
fn state_dirty_tracking_after_search() {
    let mut state = StateStore::new();
    state.set("query", json!(""));
    state.set("customers", json!([]));
    state.clear_dirty();

    assert!(!state.is_dirty("query"));
    assert!(!state.is_dirty("customers"));

    state.set("query", json!("alice"));

    assert!(state.is_dirty("query"));
    assert!(!state.is_dirty("customers"));
}

// ---- 12. Dependency graph: customers affects loop segments ---------------

#[test]
fn dependency_graph_customers_affects_segments() {
    let ast = adapto_parser::parse(CUSTOMER_PAGE_DSL).unwrap();
    let mut compiler = Compiler::new();
    let output = compiler
        .compile_file(&ast, "customers/page.adapto")
        .unwrap();

    // The `customers` state drives the {#each} loop, so it must affect segments.
    let affected_customers = output
        .dependency_graph
        .get_affected_segments(&["customers"]);
    assert!(
        !affected_customers.is_empty(),
        "changing 'customers' should affect at least one dynamic segment (the loop)"
    );

    // `query` only appears in bind:value (a binding, not a dynamic expression),
    // so it does not register in the dependency graph as a segment dependency.
    // The dependency graph tracks template expressions, not bindings.
    let affected_query = output.dependency_graph.get_affected_segments(&["query"]);
    // query's binding is handled by the runtime's two-way binding system, not
    // the dependency graph. This is by design.
    assert!(
        affected_query.is_empty(),
        "query is bound via bind:value, not a dynamic segment"
    );
}

// ---- 13. Multiple tenants complete isolation -----------------------------

#[test]
fn multiple_tenants_complete_isolation() {
    let repo: InMemoryRepository<Customer> = InMemoryRepository::new();
    let tenants: Vec<TenantId> = (0..5).map(|_| TenantId(Uuid::new_v4())).collect();

    // Seed each tenant with a different number of customers
    for (i, tenant) in tenants.iter().enumerate() {
        for j in 0..=i {
            let id = Uuid::new_v4();
            repo.create(
                tenant,
                id,
                Customer {
                    id,
                    name: format!("Customer {}_{}", i, j),
                    email: format!("c{}_{}@test.kz", i, j),
                    phone: None,
                    status: "active".into(),
                },
            );
        }
    }

    // Verify each tenant sees only its own customers
    for (i, tenant) in tenants.iter().enumerate() {
        assert_eq!(
            repo.count(tenant),
            i + 1,
            "Tenant {} should have {} customers",
            i,
            i + 1
        );
    }

    // Total across all tenants (admin-only operation)
    let total: usize = tenants.iter().map(|t| repo.count(t)).sum();
    assert_eq!(total, 1 + 2 + 3 + 4 + 5);
}

// ---- 14. Resource DSL parsing --------------------------------------------

#[test]
fn resource_dsl_parsing() {
    let ast = adapto_parser::parse(RESOURCE_DSL).unwrap();
    assert!(ast.resource.is_some());

    let resource = ast.resource.as_ref().unwrap();
    assert_eq!(resource.name, "Customer");
    assert_eq!(resource.table, "customers");
    assert_eq!(resource.primary_key, "id");
    assert_eq!(
        resource.tenant,
        adapto_parser::ast::TenantLevel::Required
    );
    assert_eq!(resource.fields.len(), 7);
    assert_eq!(resource.permissions.len(), 4);

    // Verify specific field properties
    let name_field = resource.fields.iter().find(|f| f.name == "name").unwrap();
    assert!(name_field.searchable);
    assert!(!name_field.readonly);

    let id_field = resource.fields.iter().find(|f| f.name == "id").unwrap();
    assert!(id_field.readonly);

    // Verify permissions cover all CRUD operations
    let perm_actions: Vec<&str> = resource
        .permissions
        .iter()
        .map(|p| p.action.as_str())
        .collect();
    assert!(perm_actions.contains(&"read"));
    assert!(perm_actions.contains(&"create"));
    assert!(perm_actions.contains(&"update"));
    assert!(perm_actions.contains(&"delete"));
}

// ---- 15. Delete customer from repo ---------------------------------------

#[test]
fn delete_customer_from_repo() {
    let (repo, tenant_a, tenant_b, c1, _, c3) = seed_repo();

    assert_eq!(repo.count(&tenant_a), 2);
    assert_eq!(repo.count(&tenant_b), 1);

    // Delete a customer from tenant A
    assert!(repo.delete(&tenant_a, &c1));
    assert_eq!(repo.count(&tenant_a), 1);
    assert!(repo.find(&tenant_a, &c1).is_none());

    // Tenant B is unaffected
    assert_eq!(repo.count(&tenant_b), 1);
    assert!(repo.find(&tenant_b, &c3).is_some());

    // Deleting same customer again returns false
    assert!(!repo.delete(&tenant_a, &c1));

    // Cannot delete tenant B's customer through tenant A
    assert!(!repo.delete(&tenant_a, &c3));
    assert_eq!(repo.count(&tenant_b), 1);
}
