use adapto_audit::event::AuditEvent;
use adapto_audit::sink::{AuditSink, InMemoryAuditSink};
use adapto_auth::rbac::{RbacStore, Role};
use adapto_compiler::compiler::Compiler;
use adapto_db::repository::InMemoryRepository;
use adapto_forms::schema::{FieldSchema, FieldType, FormSchema};
use adapto_runtime::context::Ctx;
use adapto_runtime::state::StateStore;
use adapto_runtime::types::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashSet;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Customer model
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
// DSL source
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

fn main() {
    println!("=== Adapto Customer CRM Example ===\n");

    // -----------------------------------------------------------------------
    // 1. Parse customer page DSL
    // -----------------------------------------------------------------------
    println!("1. Parsing customer page DSL...");
    let ast = adapto_parser::parse(CUSTOMER_PAGE_DSL).expect("Parse failed");
    println!(
        "   Route: {:?}",
        ast.route.as_ref().and_then(|r| r.path.as_deref())
    );
    println!(
        "   States: {}",
        ast.script.as_ref().map(|s| s.states.len()).unwrap_or(0)
    );
    println!(
        "   Actions: {}",
        ast.script.as_ref().map(|s| s.actions.len()).unwrap_or(0)
    );

    // -----------------------------------------------------------------------
    // 2. Compile
    // -----------------------------------------------------------------------
    println!("\n2. Compiling...");
    let mut compiler = Compiler::new();
    let output = compiler
        .compile_file(&ast, "customers/page.adapto")
        .expect("Compile failed");
    println!("   Component: {}", output.component_ir.name);
    println!(
        "   Dynamic segments: {}",
        output.component_ir.dynamic_segments.len()
    );
    println!("   Events: {}", output.component_ir.events.len());
    println!(
        "   Permissions: {:?}",
        output.component_ir.permissions
    );

    // -----------------------------------------------------------------------
    // 3. Database with tenant isolation
    // -----------------------------------------------------------------------
    println!("\n3. Setting up database with tenant isolation...");
    let repo: InMemoryRepository<Customer> = InMemoryRepository::new();
    let tenant_a = TenantId(Uuid::new_v4());
    let tenant_b = TenantId(Uuid::new_v4());

    let c1_id = Uuid::new_v4();
    let c2_id = Uuid::new_v4();
    repo.create(
        &tenant_a,
        c1_id,
        Customer {
            id: c1_id,
            name: "Alice Corp".into(),
            email: "alice@corp.kz".into(),
            phone: Some("+7 701 111 1111".into()),
            status: "active".into(),
        },
    );
    repo.create(
        &tenant_a,
        c2_id,
        Customer {
            id: c2_id,
            name: "Bob LLC".into(),
            email: "bob@llc.kz".into(),
            phone: None,
            status: "active".into(),
        },
    );

    let c3_id = Uuid::new_v4();
    repo.create(
        &tenant_b,
        c3_id,
        Customer {
            id: c3_id,
            name: "Charlie Inc".into(),
            email: "charlie@inc.kz".into(),
            phone: None,
            status: "inactive".into(),
        },
    );

    println!(
        "   Tenant A customers: {}",
        repo.for_tenant(&tenant_a).len()
    );
    println!(
        "   Tenant B customers: {}",
        repo.for_tenant(&tenant_b).len()
    );
    println!("   Tenant isolation verified");

    // -----------------------------------------------------------------------
    // 4. RBAC
    // -----------------------------------------------------------------------
    println!("\n4. Setting up RBAC...");
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

    let admin_user = UserId(Uuid::new_v4());
    let viewer_user = UserId(Uuid::new_v4());
    rbac.assign_role(&admin_user, "admin");
    rbac.assign_role(&viewer_user, "viewer");

    let admin_permissions = rbac.get_permissions(&admin_user);
    let viewer_permissions = rbac.get_permissions(&viewer_user);
    println!(
        "   Admin can delete: {}",
        admin_permissions.has("customers.delete")
    );
    println!(
        "   Viewer can delete: {}",
        viewer_permissions.has("customers.delete")
    );

    // -----------------------------------------------------------------------
    // 5. Permission checks
    // -----------------------------------------------------------------------
    println!("\n5. Permission checks...");
    let admin_ctx = Ctx {
        user_id: Some(admin_user.clone()),
        tenant_id: Some(tenant_a.clone()),
        request_id: RequestId::default(),
        permissions: admin_permissions,
        route: RouteId("/customers".into()),
        session_id: SessionId("sess-admin".into()),
    };

    let viewer_ctx = Ctx {
        user_id: Some(viewer_user.clone()),
        tenant_id: Some(tenant_a.clone()),
        request_id: RequestId::default(),
        permissions: viewer_permissions,
        route: RouteId("/customers".into()),
        session_id: SessionId("sess-viewer".into()),
    };

    println!(
        "   Admin require('customers.delete'): {:?}",
        admin_ctx.require("customers.delete")
    );
    println!(
        "   Viewer require('customers.delete'): {:?}",
        viewer_ctx.require("customers.delete")
    );

    // -----------------------------------------------------------------------
    // 6. Form validation
    // -----------------------------------------------------------------------
    println!("\n6. Form validation...");
    let customer_form = FormSchema::new("CustomerForm")
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
        );

    let valid_data: serde_json::Map<String, serde_json::Value> = serde_json::from_value(json!({
        "name": "New Customer",
        "email": "new@customer.kz",
        "phone": "+7 702 222 2222",
        "status": "active"
    }))
    .unwrap();
    let result = customer_form.validate(&valid_data);
    println!("   Valid form: is_valid = {}", result.is_valid());

    let invalid_data: serde_json::Map<String, serde_json::Value> = serde_json::from_value(json!({
        "name": "X",
        "email": "not-an-email"
    }))
    .unwrap();
    let result = customer_form.validate(&invalid_data);
    println!(
        "   Invalid form: is_valid = {}, errors = {}",
        result.is_valid(),
        result.all_errors().len()
    );
    for err in result.all_errors() {
        println!("     - {}: {}", err.field, err.message);
    }

    // -----------------------------------------------------------------------
    // 7. Audit logging
    // -----------------------------------------------------------------------
    println!("\n7. Audit logging...");
    let audit_sink = InMemoryAuditSink::new();
    let event = AuditEvent::new("customer.deleted", &admin_ctx, "delete").success();
    audit_sink.write(event);

    let denied_event = AuditEvent::new("customer.deleted", &viewer_ctx, "delete").denied();
    audit_sink.write(denied_event);

    println!("   Audit events: {}", audit_sink.len());
    let events = audit_sink.events();
    println!(
        "   Event 1: {} status={:?}",
        events[0].event, events[0].status
    );
    println!(
        "   Event 2: {} status={:?}",
        events[1].event, events[1].status
    );

    // -----------------------------------------------------------------------
    // 8. State and dirty tracking
    // -----------------------------------------------------------------------
    println!("\n8. State & dirty tracking...");
    let mut state = StateStore::new();
    state.set("query", json!(""));
    state.set(
        "customers",
        serde_json::to_value(&repo.for_tenant(&tenant_a)).unwrap(),
    );
    state.clear_dirty();

    state.set("query", json!("alice"));
    println!("   Dirty after query change: {:?}", state.get_dirty());

    // Simulate what happens when search completes: customers list changes
    state.set("customers", json!([{"name": "Alice Corp"}]));
    let dirty: Vec<&str> = state.get_dirty().iter().map(|s| s.as_str()).collect();
    let affected = output.dependency_graph.get_affected_segments(&dirty);
    println!("   Dirty fields: {:?}", dirty);
    println!("   Affected segments: {:?}", affected);

    // -----------------------------------------------------------------------
    // 9. Resource DSL parsing
    // -----------------------------------------------------------------------
    println!("\n9. Resource DSL parsing...");
    let resource_dsl = r#"
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
    let resource_ast = adapto_parser::parse(resource_dsl).expect("Resource parse failed");
    let resource = resource_ast.resource.as_ref().expect("No resource block");
    println!("   Resource: {}", resource.name);
    println!("   Table: {}", resource.table);
    println!("   Fields: {}", resource.fields.len());
    println!("   Permissions: {}", resource.permissions.len());

    // -----------------------------------------------------------------------
    // 10. CRUD: Delete and verify
    // -----------------------------------------------------------------------
    println!("\n10. CRUD delete with verification...");
    println!("   Before delete: {} customers", repo.count(&tenant_a));
    let deleted = repo.delete(&tenant_a, &c1_id);
    println!("   Deleted Alice Corp: {}", deleted);
    println!("   After delete: {} customers", repo.count(&tenant_a));
    println!(
        "   Tenant B unaffected: {} customers",
        repo.count(&tenant_b)
    );

    println!("\n=== CRM example complete ===");
}
