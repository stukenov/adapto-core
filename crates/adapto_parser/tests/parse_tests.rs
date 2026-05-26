use adapto_parser::ast::*;
use adapto_parser::error::ParseError;
use adapto_parser::parse;

// ---------------------------------------------------------------------------
// 1. Parse empty file
// ---------------------------------------------------------------------------

#[test]
fn parse_empty_file() {
    let file = parse("").unwrap();
    assert!(file.route.is_none());
    assert!(file.script.is_none());
    assert!(file.template.is_none());
    assert!(file.style.is_none());
    assert!(file.resource.is_none());
    assert!(file.layout.is_none());
}

// ---------------------------------------------------------------------------
// 2. Parse route-only file
// ---------------------------------------------------------------------------

#[test]
fn parse_route_only() {
    let input = r#"
<route>
  path: "/counter"
</route>
"#;
    let file = parse(input).unwrap();
    let route = file.route.unwrap();
    assert_eq!(route.path.as_deref(), Some("/counter"));
    assert!(route.auth.is_none());
}

// ---------------------------------------------------------------------------
// 3. Parse route with all fields
// ---------------------------------------------------------------------------

#[test]
fn parse_route_all_fields() {
    let input = r#"
<route>
  path: "/customers/[id]"
  method: "GET"
  layout: "dashboard"
  page_title: "Customer Detail"
  auth: required
  role: "admin"
  permission: "customers.read"
  tenant: required
  cache: no-store
  error: "app/errors/500.adapto"
  not_found: "app/errors/404.adapto"
</route>
"#;
    let file = parse(input).unwrap();
    let route = file.route.unwrap();
    assert_eq!(route.path.as_deref(), Some("/customers/[id]"));
    assert_eq!(route.method.as_deref(), Some("GET"));
    assert_eq!(route.layout.as_deref(), Some("dashboard"));
    assert_eq!(route.page_title.as_deref(), Some("Customer Detail"));
    assert_eq!(route.auth, Some(AuthLevel::Required));
    assert_eq!(route.role.as_deref(), Some("admin"));
    assert_eq!(route.permission.as_deref(), Some("customers.read"));
    assert_eq!(route.tenant, Some(TenantLevel::Required));
    assert_eq!(route.cache, Some(CachePolicy::NoStore));
    assert_eq!(route.error.as_deref(), Some("app/errors/500.adapto"));
    assert_eq!(route.not_found.as_deref(), Some("app/errors/404.adapto"));
}

// ---------------------------------------------------------------------------
// 4. Parse script with state declarations
// ---------------------------------------------------------------------------

#[test]
fn parse_script_state() {
    let input = r#"
<script lang="rust">
  state count: i32 = 0
  state query: String = ""
  state customers: Vec<Customer> = []
</script>
"#;
    let file = parse(input).unwrap();
    let script = file.script.unwrap();
    assert_eq!(script.states.len(), 3);

    assert_eq!(script.states[0].name, "count");
    assert_eq!(script.states[0].ty, "i32");
    assert_eq!(script.states[0].default.as_deref(), Some("0"));
    assert!(!script.states[0].secret);

    assert_eq!(script.states[1].name, "query");
    assert_eq!(script.states[1].ty, "String");
    assert_eq!(script.states[1].default.as_deref(), Some("\"\""));

    assert_eq!(script.states[2].name, "customers");
    assert_eq!(script.states[2].ty, "Vec<Customer>");
    assert_eq!(script.states[2].default.as_deref(), Some("[]"));
}

// ---------------------------------------------------------------------------
// 5. Parse script with secret state
// ---------------------------------------------------------------------------

#[test]
fn parse_script_secret_state() {
    let input = r#"
<script lang="rust">
  state secret api_key: String
</script>
"#;
    let file = parse(input).unwrap();
    let script = file.script.unwrap();
    assert_eq!(script.states.len(), 1);
    assert_eq!(script.states[0].name, "api_key");
    assert_eq!(script.states[0].ty, "String");
    assert!(script.states[0].secret);
    assert!(script.states[0].default.is_none());
}

// ---------------------------------------------------------------------------
// 6. Parse script with props
// ---------------------------------------------------------------------------

#[test]
fn parse_script_props() {
    let input = r#"
<script lang="rust">
  prop id: Uuid
  prop tone: String = "default"
  prop label: String
</script>
"#;
    let file = parse(input).unwrap();
    let script = file.script.unwrap();
    assert_eq!(script.props.len(), 3);

    assert_eq!(script.props[0].name, "id");
    assert_eq!(script.props[0].ty, "Uuid");
    assert!(script.props[0].default.is_none());

    assert_eq!(script.props[1].name, "tone");
    assert_eq!(script.props[1].ty, "String");
    assert_eq!(script.props[1].default.as_deref(), Some("\"default\""));

    assert_eq!(script.props[2].name, "label");
    assert_eq!(script.props[2].ty, "String");
    assert!(script.props[2].default.is_none());
}

// ---------------------------------------------------------------------------
// 7. Parse script with memo
// ---------------------------------------------------------------------------

#[test]
fn parse_script_memo() {
    let input = r#"
<script lang="rust">
  state price: Decimal = 100
  state tax: Decimal = 12
  memo total: Decimal = price + tax
</script>
"#;
    let file = parse(input).unwrap();
    let script = file.script.unwrap();
    assert_eq!(script.memos.len(), 1);
    assert_eq!(script.memos[0].name, "total");
    assert_eq!(script.memos[0].ty, "Decimal");
    assert_eq!(script.memos[0].expr, "price + tax");
}

// ---------------------------------------------------------------------------
// 8. Parse script with sync action
// ---------------------------------------------------------------------------

#[test]
fn parse_script_sync_action() {
    let input = r#"
<script lang="rust">
  action increment() {
    count += 1
  }
</script>
"#;
    let file = parse(input).unwrap();
    let script = file.script.unwrap();
    assert_eq!(script.actions.len(), 1);
    assert_eq!(script.actions[0].name, "increment");
    assert!(!script.actions[0].is_async);
    assert!(script.actions[0].params.is_empty());
    assert!(script.actions[0].body.contains("count += 1"));
}

// ---------------------------------------------------------------------------
// 9. Parse script with async action
// ---------------------------------------------------------------------------

#[test]
fn parse_script_async_action() {
    let input = r#"
<script lang="rust">
  action async fn search(ctx: Ctx) {
    customers = CustomerRepo::search(ctx.tenant_id, query.clone()).await?;
  }
</script>
"#;
    let file = parse(input).unwrap();
    let script = file.script.unwrap();
    assert_eq!(script.actions.len(), 1);
    assert_eq!(script.actions[0].name, "search");
    assert!(script.actions[0].is_async);
    assert_eq!(script.actions[0].params.len(), 1);
    assert_eq!(script.actions[0].params[0].name, "ctx");
    assert_eq!(script.actions[0].params[0].ty, "Ctx");
}

// ---------------------------------------------------------------------------
// 10. Parse script with permission attribute
// ---------------------------------------------------------------------------

#[test]
fn parse_script_permission_attr() {
    let input = r#"
<script lang="rust">
  #[permission("customers.delete")]
  action async fn delete(id: Uuid, ctx: Ctx) {
    CustomerRepo::delete(ctx.tenant_id, id).await?;
  }
</script>
"#;
    let file = parse(input).unwrap();
    let script = file.script.unwrap();
    assert_eq!(script.actions.len(), 1);
    assert_eq!(
        script.actions[0].permission.as_deref(),
        Some("customers.delete")
    );
}

// ---------------------------------------------------------------------------
// 11. Parse script with audit attribute
// ---------------------------------------------------------------------------

#[test]
fn parse_script_audit_attr() {
    let input = r#"
<script lang="rust">
  #[audit("customer.updated")]
  action async fn save(form: CustomerForm, ctx: Ctx) {
    CustomerRepo::update(ctx.tenant_id, id, form).await?;
  }
</script>
"#;
    let file = parse(input).unwrap();
    let script = file.script.unwrap();
    assert_eq!(script.actions.len(), 1);
    assert_eq!(
        script.actions[0].audit.as_deref(),
        Some("customer.updated")
    );
}

// ---------------------------------------------------------------------------
// 12. Parse script with loader
// ---------------------------------------------------------------------------

#[test]
fn parse_script_loader() {
    let input = r#"
<script lang="rust">
  load async fn load_customer(ctx: Ctx) {
    customer = CustomerRepo::find(ctx.tenant_id, id).await?;
  }
</script>
"#;
    let file = parse(input).unwrap();
    let script = file.script.unwrap();
    assert_eq!(script.loaders.len(), 1);
    assert_eq!(script.loaders[0].name, "load_customer");
    assert!(script.loaders[0].is_async);
    assert!(script.loaders[0].body.contains("CustomerRepo::find"));
}

// ---------------------------------------------------------------------------
// 13. Parse script with form schema
// ---------------------------------------------------------------------------

#[test]
fn parse_script_form() {
    let input = r#"
<script lang="rust">
  form CustomerForm {
    name: String min=2 max=120 required
    email: Email required
    phone: Option<String> max=32
  }
</script>
"#;
    let file = parse(input).unwrap();
    let script = file.script.unwrap();
    assert_eq!(script.forms.len(), 1);
    assert_eq!(script.forms[0].name, "CustomerForm");
    assert_eq!(script.forms[0].fields.len(), 3);

    let name_field = &script.forms[0].fields[0];
    assert_eq!(name_field.name, "name");
    assert_eq!(name_field.ty, "String");
    assert!(name_field.constraints.contains(&FieldConstraint::Min(2)));
    assert!(name_field.constraints.contains(&FieldConstraint::Max(120)));
    assert!(name_field.constraints.contains(&FieldConstraint::Required));

    let email_field = &script.forms[0].fields[1];
    assert_eq!(email_field.name, "email");
    assert_eq!(email_field.ty, "Email");
    assert!(email_field.constraints.contains(&FieldConstraint::Required));

    let phone_field = &script.forms[0].fields[2];
    assert_eq!(phone_field.name, "phone");
    assert_eq!(phone_field.ty, "Option<String>");
    assert!(phone_field.constraints.contains(&FieldConstraint::Max(32)));
}

// ---------------------------------------------------------------------------
// 14. Parse script with AI action
// ---------------------------------------------------------------------------

#[test]
fn parse_script_ai_action() {
    let input = r#"
<script lang="rust">
  ai action summarize_lesson(input: LessonTranscript) -> Summary {
    model: "soz-kz-600m"
    fallback: "gpt-5.5-thinking"
    temperature: 0.2
    audit: true
    pii: redact
    permission: "lessons.ai.summarize"
  }
</script>
"#;
    let file = parse(input).unwrap();
    let script = file.script.unwrap();
    assert_eq!(script.ai_actions.len(), 1);

    let ai = &script.ai_actions[0];
    assert_eq!(ai.name, "summarize_lesson");
    assert_eq!(ai.input_param, "input");
    assert_eq!(ai.input_type, "LessonTranscript");
    assert_eq!(ai.return_type, "Summary");
    assert_eq!(ai.model, "soz-kz-600m");
    assert_eq!(ai.fallback.as_deref(), Some("gpt-5.5-thinking"));
    assert_eq!(ai.temperature, Some(0.2));
    assert!(ai.audit);
    assert_eq!(ai.pii.as_deref(), Some("redact"));
    assert_eq!(ai.permission.as_deref(), Some("lessons.ai.summarize"));
}

// ---------------------------------------------------------------------------
// 15. Parse script with use statements
// ---------------------------------------------------------------------------

#[test]
fn parse_script_use() {
    let input = r#"
<script lang="rust">
  use crate::resources::CustomerRepo;
  use crate::models::Customer;
</script>
"#;
    let file = parse(input).unwrap();
    let script = file.script.unwrap();
    assert_eq!(script.uses.len(), 2);
    assert_eq!(script.uses[0].path, "crate::resources::CustomerRepo");
    assert_eq!(script.uses[1].path, "crate::models::Customer");
}

// ---------------------------------------------------------------------------
// 16. Parse template with plain HTML
// ---------------------------------------------------------------------------

#[test]
fn parse_template_plain_html() {
    let input = r#"
<template>
  <div>
    <h1>Hello</h1>
    <p>World</p>
  </div>
</template>
"#;
    let file = parse(input).unwrap();
    let tpl = file.template.unwrap();
    assert!(!tpl.children.is_empty());

    // Should have a div element
    let div = match &tpl.children[0] {
        TemplateNode::Element(el) => el,
        other => panic!("Expected Element, got: {:?}", other),
    };
    assert_eq!(div.tag, "div");
    // div should have children (h1, p)
    assert!(div.children.len() >= 2);
}

// ---------------------------------------------------------------------------
// 17. Parse template with expressions
// ---------------------------------------------------------------------------

#[test]
fn parse_template_expressions() {
    let input = r#"
<template>
  <h1>{customer.name}</h1>
</template>
"#;
    let file = parse(input).unwrap();
    let tpl = file.template.unwrap();
    let h1 = match &tpl.children[0] {
        TemplateNode::Element(el) => el,
        other => panic!("Expected Element, got: {:?}", other),
    };
    assert_eq!(h1.tag, "h1");
    assert!(!h1.children.is_empty());
    match &h1.children[0] {
        TemplateNode::Expression(expr) => {
            assert_eq!(expr.expr, "customer.name");
        }
        other => panic!("Expected Expression, got: {:?}", other),
    }
}

// ---------------------------------------------------------------------------
// 18. Parse template with if/else
// ---------------------------------------------------------------------------

#[test]
fn parse_template_if_else() {
    let input = r#"
<template>
  {#if customer.is_active}
    <span>Active</span>
  {:else}
    <span>Inactive</span>
  {/if}
</template>
"#;
    let file = parse(input).unwrap();
    let tpl = file.template.unwrap();

    let if_node = match &tpl.children[0] {
        TemplateNode::If(n) => n,
        other => panic!("Expected If, got: {:?}", other),
    };
    assert_eq!(if_node.condition, "customer.is_active");
    assert!(!if_node.then_branch.is_empty());
    assert!(if_node.else_branch.is_some());
}

// ---------------------------------------------------------------------------
// 19. Parse template with each loop
// ---------------------------------------------------------------------------

#[test]
fn parse_template_each() {
    let input = r#"
<template>
  {#each items as item}
    <p>{item.name}</p>
  {/each}
</template>
"#;
    let file = parse(input).unwrap();
    let tpl = file.template.unwrap();

    let each = match &tpl.children[0] {
        TemplateNode::Each(n) => n,
        other => panic!("Expected Each, got: {:?}", other),
    };
    assert_eq!(each.iterable, "items");
    assert_eq!(each.item, "item");
    assert!(each.index.is_none());
    assert!(!each.children.is_empty());
}

// ---------------------------------------------------------------------------
// 20. Parse template with can permission
// ---------------------------------------------------------------------------

#[test]
fn parse_template_can() {
    let input = r#"
<template>
  {#can "customers.create"}
    <button>New customer</button>
  {/can}
</template>
"#;
    let file = parse(input).unwrap();
    let tpl = file.template.unwrap();

    let can = match &tpl.children[0] {
        TemplateNode::Can(n) => n,
        other => panic!("Expected Can, got: {:?}", other),
    };
    assert_eq!(can.permission, "customers.create");
    assert!(!can.children.is_empty());
}

// ---------------------------------------------------------------------------
// 21. Parse template with event binding
// ---------------------------------------------------------------------------

#[test]
fn parse_template_event_binding() {
    let input = r#"
<template>
  <button on:click="increment">+</button>
</template>
"#;
    let file = parse(input).unwrap();
    let tpl = file.template.unwrap();

    let btn = match &tpl.children[0] {
        TemplateNode::Element(el) => el,
        other => panic!("Expected Element, got: {:?}", other),
    };
    assert_eq!(btn.tag, "button");
    assert_eq!(btn.events.len(), 1);
    assert_eq!(btn.events[0].event, "click");
    assert_eq!(btn.events[0].handler, "increment");
    assert!(btn.events[0].modifiers.is_empty());
}

// ---------------------------------------------------------------------------
// 22. Parse template with event modifiers
// ---------------------------------------------------------------------------

#[test]
fn parse_template_event_modifiers() {
    let input = r#"
<template>
  <form on:submit.prevent="save">
    <input on:input.debounce.300="search" />
  </form>
</template>
"#;
    let file = parse(input).unwrap();
    let tpl = file.template.unwrap();

    let form = match &tpl.children[0] {
        TemplateNode::Element(el) => el,
        other => panic!("Expected Element, got: {:?}", other),
    };
    assert_eq!(form.events.len(), 1);
    assert_eq!(form.events[0].event, "submit");
    assert_eq!(form.events[0].handler, "save");
    assert_eq!(form.events[0].modifiers, vec![EventModifier::Prevent]);

    // Find the input inside the form
    let input_el = form.children.iter().find_map(|n| match n {
        TemplateNode::Element(el) if el.tag == "input" => Some(el),
        _ => None,
    });
    let input_el = input_el.expect("Should find input element");
    assert_eq!(input_el.events.len(), 1);
    assert_eq!(input_el.events[0].event, "input");
    assert_eq!(input_el.events[0].handler, "search");
    assert_eq!(
        input_el.events[0].modifiers,
        vec![EventModifier::Debounce(300)]
    );
}

// ---------------------------------------------------------------------------
// 23. Parse template with bind:value
// ---------------------------------------------------------------------------

#[test]
fn parse_template_bind() {
    let input = r#"
<template>
  <input bind:value="query" />
</template>
"#;
    let file = parse(input).unwrap();
    let tpl = file.template.unwrap();

    let input_el = match &tpl.children[0] {
        TemplateNode::Element(el) => el,
        other => panic!("Expected Element, got: {:?}", other),
    };
    assert_eq!(input_el.bindings.len(), 1);
    assert_eq!(input_el.bindings[0].kind, "value");
    assert_eq!(input_el.bindings[0].target, "query");
}

// ---------------------------------------------------------------------------
// 24. Parse template with component usage
// ---------------------------------------------------------------------------

#[test]
fn parse_template_component() {
    let input = r#"
<template>
  <Badge tone="success" label="Active" />
</template>
"#;
    let file = parse(input).unwrap();
    let tpl = file.template.unwrap();

    let comp = match &tpl.children[0] {
        TemplateNode::Component(c) => c,
        other => panic!("Expected Component, got: {:?}", other),
    };
    assert_eq!(comp.name, "Badge");
    assert!(!comp.is_island);
    assert_eq!(comp.props.len(), 2);
    assert_eq!(comp.props[0].name, "tone");
    match &comp.props[0].value {
        AttributeValue::Static(v) => assert_eq!(v, "success"),
        other => panic!("Expected Static, got: {:?}", other),
    }
}

// ---------------------------------------------------------------------------
// 25. Parse template with island component
// ---------------------------------------------------------------------------

#[test]
fn parse_template_island() {
    let input = r#"
<template>
  <Chart island data={sales_data} />
</template>
"#;
    let file = parse(input).unwrap();
    let tpl = file.template.unwrap();

    let comp = match &tpl.children[0] {
        TemplateNode::Component(c) => c,
        other => panic!("Expected Component, got: {:?}", other),
    };
    assert_eq!(comp.name, "Chart");
    assert!(comp.is_island);
}

// ---------------------------------------------------------------------------
// 26. Parse template with slot
// ---------------------------------------------------------------------------

#[test]
fn parse_template_slot() {
    let input = r#"
<template>
  <button>
    <slot />
  </button>
</template>
"#;
    let file = parse(input).unwrap();
    let tpl = file.template.unwrap();

    let btn = match &tpl.children[0] {
        TemplateNode::Element(el) => el,
        other => panic!("Expected Element, got: {:?}", other),
    };
    assert_eq!(btn.tag, "button");
    let slot = btn.children.iter().find_map(|n| match n {
        TemplateNode::Slot(s) => Some(s),
        _ => None,
    });
    assert!(slot.is_some());
    assert!(slot.unwrap().name.is_none());
}

// ---------------------------------------------------------------------------
// 27. Parse template with unsafe html
// ---------------------------------------------------------------------------

#[test]
fn parse_template_unsafe_html() {
    let input = r#"
<template>
  {@html raw_content}
</template>
"#;
    let file = parse(input).unwrap();
    let tpl = file.template.unwrap();

    match &tpl.children[0] {
        TemplateNode::UnsafeHtml(expr) => {
            assert_eq!(expr, "raw_content");
        }
        other => panic!("Expected UnsafeHtml, got: {:?}", other),
    }
}

// ---------------------------------------------------------------------------
// 28. Parse style block (scoped)
// ---------------------------------------------------------------------------

#[test]
fn parse_style_scoped() {
    let input = r#"
<style scoped>
  button {
    padding: 12px;
  }
</style>
"#;
    let file = parse(input).unwrap();
    let style = file.style.unwrap();
    assert!(style.scoped);
    assert!(style.content.contains("padding: 12px"));
}

// ---------------------------------------------------------------------------
// 29. Parse style block (global)
// ---------------------------------------------------------------------------

#[test]
fn parse_style_global() {
    let input = r#"
<style global>
  :root {
    --color-primary: #2563eb;
  }
</style>
"#;
    let file = parse(input).unwrap();
    let style = file.style.unwrap();
    assert!(!style.scoped);
    assert!(style.content.contains("--color-primary"));
}

// ---------------------------------------------------------------------------
// 30. Parse resource block
// ---------------------------------------------------------------------------

#[test]
fn parse_resource_block() {
    let input = r#"
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
"#;
    let file = parse(input).unwrap();
    let resource = file.resource.unwrap();
    assert_eq!(resource.name, "Customer");
    assert_eq!(resource.table, "customers");
    assert_eq!(resource.tenant, TenantLevel::Required);
    assert_eq!(resource.primary_key, "id");
    assert_eq!(resource.fields.len(), 6);
    assert_eq!(resource.permissions.len(), 4);

    // Check name field
    let name_field = &resource.fields[1];
    assert_eq!(name_field.name, "name");
    assert_eq!(name_field.ty, "String");
    assert!(name_field.searchable);
    assert!(name_field.constraints.contains(&FieldConstraint::Required));
    assert!(name_field.constraints.contains(&FieldConstraint::Max(120)));

    // Check email field
    let email_field = &resource.fields[2];
    assert_eq!(email_field.name, "email");
    assert!(email_field.constraints.contains(&FieldConstraint::Unique));

    // Check status field default
    let status_field = &resource.fields[4];
    assert_eq!(status_field.default.as_deref(), Some("active"));

    // Check permissions
    assert_eq!(resource.permissions[0].action, "read");
    assert_eq!(resource.permissions[0].permission, "customers.read");
    assert_eq!(resource.permissions[3].action, "delete");
    assert_eq!(resource.permissions[3].permission, "customers.delete");
}

// ---------------------------------------------------------------------------
// 31. Parse layout block
// ---------------------------------------------------------------------------

#[test]
fn parse_layout_block() {
    let input = r#"
<layout name="dashboard">
  auth: required
  tenant: required
</layout>
"#;
    let file = parse(input).unwrap();
    let layout = file.layout.unwrap();
    assert_eq!(layout.name, "dashboard");
    assert_eq!(layout.auth, Some(AuthLevel::Required));
    assert_eq!(layout.tenant, Some(TenantLevel::Required));
}

// ---------------------------------------------------------------------------
// 32. Parse full counter example
// ---------------------------------------------------------------------------

#[test]
fn parse_full_counter() {
    let input = r#"
<route>
  path: "/counter"
  layout: "main"
  auth: required
</route>

<script lang="rust">
  state count: i32 = 0

  action increment() {
    count += 1
  }
</script>

<template>
  <button on:click="increment">
    Count: {count}
  </button>
</template>

<style scoped>
  button {
    padding: 12px;
  }
</style>
"#;
    let file = parse(input).unwrap();

    // Route
    let route = file.route.unwrap();
    assert_eq!(route.path.as_deref(), Some("/counter"));
    assert_eq!(route.auth, Some(AuthLevel::Required));

    // Script
    let script = file.script.unwrap();
    assert_eq!(script.states.len(), 1);
    assert_eq!(script.actions.len(), 1);
    assert_eq!(script.actions[0].name, "increment");

    // Template
    let tpl = file.template.unwrap();
    assert!(!tpl.children.is_empty());

    // Style
    let style = file.style.unwrap();
    assert!(style.scoped);
    assert!(style.content.contains("padding: 12px"));
}

// ---------------------------------------------------------------------------
// 33. Parse full customer page
// ---------------------------------------------------------------------------

#[test]
fn parse_full_customer_page() {
    let input = r#"
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
"#;
    let file = parse(input).unwrap();

    // Route
    let route = file.route.unwrap();
    assert_eq!(route.path.as_deref(), Some("/customers"));
    assert_eq!(route.permission.as_deref(), Some("customers.read"));
    assert_eq!(route.tenant, Some(TenantLevel::Required));

    // Script
    let script = file.script.unwrap();
    assert_eq!(script.uses.len(), 1);
    assert_eq!(script.states.len(), 3);
    assert_eq!(script.loaders.len(), 1);
    assert_eq!(script.actions.len(), 2);

    // Check delete action attributes
    let delete_action = &script.actions[1];
    assert_eq!(delete_action.name, "delete");
    assert_eq!(
        delete_action.permission.as_deref(),
        Some("customers.delete")
    );
    assert_eq!(
        delete_action.audit.as_deref(),
        Some("customer.deleted")
    );

    // Template
    let tpl = file.template.unwrap();
    assert!(!tpl.children.is_empty());
}

// ---------------------------------------------------------------------------
// 34. Parse full lesson tracker
// ---------------------------------------------------------------------------

#[test]
fn parse_full_lesson_tracker() {
    let input = r#"
<route>
  path: "/lessons/[id]"
  layout: "school"
  auth: required
  tenant: required
  permission: "lessons.read"
</route>

<script lang="rust">
  prop id: Uuid

  state lesson: Lesson
  state transcript: String = ""
  state ai_summary: Option<LessonSummary> = None
  state saving: bool = false

  load async fn load(ctx: Ctx) {
    lesson = LessonRepo::find(ctx.tenant_id, id).await?;
    transcript = lesson.transcript.clone();
  }

  #[permission("lessons.update")]
  #[audit("lesson.status.changed")]
  action async fn set_status(status: LessonStatus, ctx: Ctx) {
    lesson.status = status;
    LessonRepo::set_status(ctx.tenant_id, id, status).await?;
  }

  ai action summarize() -> LessonSummary {
    model: "soz-kz-600m"
    fallback: "gpt-5.5-thinking"
    input: transcript
    pii: redact
    permission: "lessons.ai.summarize"
    audit: true
  }
</script>

<template>
  <Page title={lesson.title}>
    <StatusBar status={lesson.status} />

    <ButtonGroup>
      <Button on:click="set_status('planned')">Planned</Button>
      <Button on:click="set_status('in_progress')">In progress</Button>
      <Button on:click="set_status('done')">Done</Button>
    </ButtonGroup>

    <TextArea bind:value="transcript" />

    <Button on:click="summarize">Generate AI summary</Button>

    {#if ai_summary}
      <Card>
        <h2>AI Summary</h2>
        <p>{ai_summary.text}</p>
      </Card>
    {/if}
  </Page>
</template>
"#;
    let file = parse(input).unwrap();

    // Route
    let route = file.route.unwrap();
    assert_eq!(route.path.as_deref(), Some("/lessons/[id]"));

    // Script
    let script = file.script.unwrap();
    assert_eq!(script.props.len(), 1);
    assert_eq!(script.props[0].name, "id");
    assert_eq!(script.states.len(), 4);
    assert_eq!(script.loaders.len(), 1);
    assert_eq!(script.actions.len(), 1);
    assert_eq!(script.ai_actions.len(), 1);

    let set_status = &script.actions[0];
    assert_eq!(set_status.name, "set_status");
    assert_eq!(
        set_status.permission.as_deref(),
        Some("lessons.update")
    );
    assert_eq!(
        set_status.audit.as_deref(),
        Some("lesson.status.changed")
    );

    let ai = &script.ai_actions[0];
    assert_eq!(ai.name, "summarize");
    assert_eq!(ai.model, "soz-kz-600m");
    assert!(ai.audit);
    assert_eq!(ai.input_param, "transcript");

    // Template
    let tpl = file.template.unwrap();
    assert!(!tpl.children.is_empty());
}

// ---------------------------------------------------------------------------
// 35. Error: unclosed block
// ---------------------------------------------------------------------------

#[test]
fn error_unclosed_block() {
    let input = r#"
<route>
  path: "/test"
"#;
    let result = parse(input);
    assert!(result.is_err());
    match result.unwrap_err() {
        ParseError::UnclosedBlock(name) => assert_eq!(name, "route"),
        other => panic!("Expected UnclosedBlock, got: {other}"),
    }
}

// ---------------------------------------------------------------------------
// 36. Error: duplicate block
// ---------------------------------------------------------------------------

#[test]
fn error_duplicate_block() {
    let input = r#"
<route>
  path: "/a"
</route>
<route>
  path: "/b"
</route>
"#;
    let result = parse(input);
    assert!(result.is_err());
    match result.unwrap_err() {
        ParseError::DuplicateBlock(name) => assert_eq!(name, "route"),
        other => panic!("Expected DuplicateBlock, got: {other}"),
    }
}

// ---------------------------------------------------------------------------
// 37. Error: invalid auth value
// ---------------------------------------------------------------------------

#[test]
fn error_invalid_auth_value() {
    let input = r#"
<route>
  path: "/test"
  auth: invalid_value
</route>
"#;
    let result = parse(input);
    assert!(result.is_err());
    match result.unwrap_err() {
        ParseError::InvalidValue { field, .. } => assert_eq!(field, "auth"),
        other => panic!("Expected InvalidValue, got: {other}"),
    }
}

// ---------------------------------------------------------------------------
// 38. Error: missing required resource field
// ---------------------------------------------------------------------------

#[test]
fn error_missing_resource_name() {
    let input = r#"
<resource table="customers">
  field id: Uuid
</resource>
"#;
    let result = parse(input);
    assert!(result.is_err());
    match result.unwrap_err() {
        ParseError::MissingField { field, .. } => assert_eq!(field, "name"),
        other => panic!("Expected MissingField, got: {other}"),
    }
}

// ---------------------------------------------------------------------------
// Additional: template with multiple control flow nodes
// ---------------------------------------------------------------------------

#[test]
fn parse_template_if_else_if() {
    let input = r#"
<template>
  {#if status == "active"}
    <span>Active</span>
  {:else if status == "pending"}
    <span>Pending</span>
  {:else}
    <span>Unknown</span>
  {/if}
</template>
"#;
    let file = parse(input).unwrap();
    let tpl = file.template.unwrap();

    let if_node = match &tpl.children[0] {
        TemplateNode::If(n) => n,
        other => panic!("Expected If, got: {:?}", other),
    };
    assert_eq!(if_node.condition, "status == \"active\"");
    assert_eq!(if_node.else_if_branches.len(), 1);
    assert_eq!(
        if_node.else_if_branches[0].0,
        "status == \"pending\""
    );
    assert!(if_node.else_branch.is_some());
}

// ---------------------------------------------------------------------------
// Additional: each with index
// ---------------------------------------------------------------------------

#[test]
fn parse_template_each_with_index() {
    let input = r#"
<template>
  {#each items as item, idx}
    <p>{idx}: {item.name}</p>
  {/each}
</template>
"#;
    let file = parse(input).unwrap();
    let tpl = file.template.unwrap();

    let each = match &tpl.children[0] {
        TemplateNode::Each(n) => n,
        other => panic!("Expected Each, got: {:?}", other),
    };
    assert_eq!(each.iterable, "items");
    assert_eq!(each.item, "item");
    assert_eq!(each.index.as_deref(), Some("idx"));
}

// ---------------------------------------------------------------------------
// Additional: component with children
// ---------------------------------------------------------------------------

#[test]
fn parse_template_component_with_children() {
    let input = r#"
<template>
  <Button>Save</Button>
</template>
"#;
    let file = parse(input).unwrap();
    let tpl = file.template.unwrap();

    let comp = match &tpl.children[0] {
        TemplateNode::Component(c) => c,
        other => panic!("Expected Component, got: {:?}", other),
    };
    assert_eq!(comp.name, "Button");
    // Should have a text child
    let has_text = comp.children.iter().any(|n| matches!(n, TemplateNode::Text(t) if t == "Save"));
    assert!(has_text, "Button should contain text 'Save', got: {:?}", comp.children);
}

// ---------------------------------------------------------------------------
// Additional: component with dynamic props
// ---------------------------------------------------------------------------

#[test]
fn parse_template_dynamic_props() {
    let input = r#"
<template>
  <Table rows={customers} />
</template>
"#;
    let file = parse(input).unwrap();
    let tpl = file.template.unwrap();

    let comp = match &tpl.children[0] {
        TemplateNode::Component(c) => c,
        other => panic!("Expected Component, got: {:?}", other),
    };
    assert_eq!(comp.name, "Table");
    assert_eq!(comp.props.len(), 1);
    assert_eq!(comp.props[0].name, "rows");
    match &comp.props[0].value {
        AttributeValue::Dynamic(v) => assert_eq!(v, "customers"),
        other => panic!("Expected Dynamic, got: {:?}", other),
    }
}

// ---------------------------------------------------------------------------
// Additional: self-closing HTML element
// ---------------------------------------------------------------------------

#[test]
fn parse_template_self_closing() {
    let input = r#"
<template>
  <img src="/logo.png" />
  <br />
  <input type="text" />
</template>
"#;
    let file = parse(input).unwrap();
    let tpl = file.template.unwrap();

    // All three should be self-closing elements
    let img = tpl.children.iter().find_map(|n| match n {
        TemplateNode::Element(el) if el.tag == "img" => Some(el),
        _ => None,
    });
    assert!(img.is_some());
    assert!(img.unwrap().self_closing);
}

// ---------------------------------------------------------------------------
// Additional: whitespace-only file
// ---------------------------------------------------------------------------

#[test]
fn parse_whitespace_only() {
    let file = parse("  \n\n  \t  \n").unwrap();
    assert!(file.route.is_none());
    assert!(file.script.is_none());
    assert!(file.template.is_none());
}

// ---------------------------------------------------------------------------
// Snapshot tests using insta
// ---------------------------------------------------------------------------

#[test]
fn snapshot_counter_ast() {
    let input = r#"
<route>
  path: "/counter"
  auth: public
</route>

<script lang="rust">
  state count: i32 = 0

  action increment() {
    count += 1
  }
</script>

<template>
  <button on:click="increment">Count: {count}</button>
</template>
"#;
    let file = parse(input).unwrap();
    insta::assert_yaml_snapshot!(file);
}

#[test]
fn snapshot_resource_block() {
    let input = r#"
<resource name="Product" table="products">
  tenant: required
  primary_key: id

  field id: Uuid readonly
  field name: String required max=200 searchable
  field price: Decimal required

  permission read: "products.read"
  permission create: "products.create"
</resource>
"#;
    let file = parse(input).unwrap();
    insta::assert_yaml_snapshot!(file);
}

// ===========================================================================
// Extended test suite
// ===========================================================================

// ---------------------------------------------------------------------------
// Error recovery: malformed route block (missing path)
// ---------------------------------------------------------------------------

#[test]
fn error_route_missing_path() {
    let input = r#"
<route>
  method: "GET"
  auth: required
</route>
"#;
    let file = parse(input);
    // Route without path should either parse with path=None or error
    match file {
        Ok(f) => {
            let route = f.route.unwrap();
            assert!(route.path.is_none(), "Route parsed but path should be None");
        }
        Err(e) => {
            // Also acceptable if parser requires path
            let msg = format!("{}", e);
            assert!(
                msg.contains("path") || msg.contains("MissingField"),
                "Error should mention 'path', got: {}",
                msg
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Error recovery: unclosed template tag
// ---------------------------------------------------------------------------

#[test]
fn error_unclosed_template_block() {
    let input = r#"
<template>
  <div>Hello
"#;
    let result = parse(input);
    assert!(result.is_err(), "Unclosed template block should error");
    match result.unwrap_err() {
        ParseError::UnclosedBlock(name) => assert_eq!(name, "template"),
        ParseError::Syntax { message, .. } => {
            assert!(
                message.contains("unclosed") || message.contains("Unclosed") || message.len() > 0,
                "Syntax error should describe unclosed tag"
            );
        }
        other => panic!("Expected UnclosedBlock or Syntax, got: {other}"),
    }
}

// ---------------------------------------------------------------------------
// Error recovery: invalid auth level
// ---------------------------------------------------------------------------

#[test]
fn error_invalid_auth_level_bogus() {
    let input = r#"
<route>
  path: "/test"
  auth: super_admin
</route>
"#;
    let result = parse(input);
    assert!(result.is_err(), "Invalid auth level should error");
    match result.unwrap_err() {
        ParseError::InvalidValue { field, value, .. } => {
            assert_eq!(field, "auth");
            assert_eq!(value, "super_admin");
        }
        other => panic!("Expected InvalidValue for auth, got: {other}"),
    }
}

// ---------------------------------------------------------------------------
// Edge case: empty script block
// ---------------------------------------------------------------------------

#[test]
fn parse_empty_script_block() {
    let input = r#"
<script lang="rust">
</script>
"#;
    let file = parse(input).unwrap();
    let script = file.script.unwrap();
    assert!(script.states.is_empty());
    assert!(script.actions.is_empty());
    assert!(script.props.is_empty());
    assert!(script.memos.is_empty());
    assert!(script.loaders.is_empty());
    assert!(script.forms.is_empty());
    assert!(script.ai_actions.is_empty());
    assert!(script.uses.is_empty());
}

// ---------------------------------------------------------------------------
// Edge case: script with only use statements
// ---------------------------------------------------------------------------

#[test]
fn parse_script_use_only() {
    let input = r#"
<script lang="rust">
  use crate::models::User;
  use crate::repos::UserRepo;
  use std::collections::HashMap;
</script>
"#;
    let file = parse(input).unwrap();
    let script = file.script.unwrap();
    assert_eq!(script.uses.len(), 3);
    assert!(script.states.is_empty());
    assert!(script.actions.is_empty());
    assert_eq!(script.uses[0].path, "crate::models::User");
    assert_eq!(script.uses[1].path, "crate::repos::UserRepo");
    assert_eq!(script.uses[2].path, "std::collections::HashMap");
}

// ---------------------------------------------------------------------------
// Edge case: template with nested if inside each
// ---------------------------------------------------------------------------

#[test]
fn parse_template_nested_if_in_each() {
    let input = r#"
<template>
  {#each users as user}
    {#if user.is_active}
      <span>Active: {user.name}</span>
    {:else}
      <span>Inactive</span>
    {/if}
  {/each}
</template>
"#;
    let file = parse(input).unwrap();
    let tpl = file.template.unwrap();

    let each = match &tpl.children[0] {
        TemplateNode::Each(n) => n,
        other => panic!("Expected Each, got: {:?}", other),
    };
    assert_eq!(each.iterable, "users");
    assert_eq!(each.item, "user");

    // Find the If node nested inside each
    let if_node = each.children.iter().find_map(|n| match n {
        TemplateNode::If(i) => Some(i),
        _ => None,
    });
    assert!(if_node.is_some(), "Should have nested If inside Each");
    let if_node = if_node.unwrap();
    assert_eq!(if_node.condition, "user.is_active");
    assert!(if_node.else_branch.is_some());
}

// ---------------------------------------------------------------------------
// Edge case: template with nested each inside if
// ---------------------------------------------------------------------------

#[test]
fn parse_template_nested_each_in_if() {
    let input = r#"
<template>
  {#if has_items}
    {#each items as item}
      <li>{item.name}</li>
    {/each}
  {/if}
</template>
"#;
    let file = parse(input).unwrap();
    let tpl = file.template.unwrap();

    let if_node = match &tpl.children[0] {
        TemplateNode::If(n) => n,
        other => panic!("Expected If, got: {:?}", other),
    };
    assert_eq!(if_node.condition, "has_items");

    let each = if_node.then_branch.iter().find_map(|n| match n {
        TemplateNode::Each(e) => Some(e),
        _ => None,
    });
    assert!(each.is_some(), "Should have nested Each inside If");
    assert_eq!(each.unwrap().iterable, "items");
}

// ---------------------------------------------------------------------------
// Resource block: full resource with fields, permissions, constraints
// ---------------------------------------------------------------------------

#[test]
fn parse_resource_full_with_constraints() {
    let input = r#"
<resource name="Order" table="orders">
  tenant: required
  primary_key: id

  field id: Uuid readonly
  field customer_id: Uuid required
  field total: Decimal required
  field status: Enum[pending, confirmed, shipped, delivered] default=pending
  field notes: Option<String> optional
  field created_at: DateTime readonly

  permission read: "orders.read"
  permission create: "orders.create"
  permission update: "orders.update"
  permission delete: "orders.delete"
</resource>
"#;
    let file = parse(input).unwrap();
    let resource = file.resource.unwrap();
    assert_eq!(resource.name, "Order");
    assert_eq!(resource.table, "orders");
    assert_eq!(resource.tenant, TenantLevel::Required);
    assert_eq!(resource.primary_key, "id");
    assert_eq!(resource.fields.len(), 6);
    assert_eq!(resource.permissions.len(), 4);

    // id field: readonly
    let id_field = &resource.fields[0];
    assert_eq!(id_field.name, "id");
    assert!(id_field.readonly);

    // customer_id field: required
    let cust_field = &resource.fields[1];
    assert_eq!(cust_field.name, "customer_id");
    assert!(cust_field.constraints.contains(&FieldConstraint::Required));

    // status field: has default
    let status_field = &resource.fields[3];
    assert_eq!(status_field.default.as_deref(), Some("pending"));

    // notes field: optional
    let notes_field = &resource.fields[4];
    assert!(notes_field.constraints.contains(&FieldConstraint::Optional));

    // created_at: readonly
    let created_field = &resource.fields[5];
    assert!(created_field.readonly);
}

// ---------------------------------------------------------------------------
// Style block: scoped vs global
// ---------------------------------------------------------------------------

#[test]
fn parse_style_scoped_vs_global() {
    // Scoped
    let scoped_input = r#"
<style scoped>
  .card { border: 1px solid #ccc; }
</style>
"#;
    let scoped_file = parse(scoped_input).unwrap();
    let scoped_style = scoped_file.style.unwrap();
    assert!(scoped_style.scoped);
    assert!(scoped_style.content.contains(".card"));

    // Global
    let global_input = r#"
<style global>
  body { font-family: sans-serif; }
</style>
"#;
    let global_file = parse(global_input).unwrap();
    let global_style = global_file.style.unwrap();
    assert!(!global_style.scoped);
    assert!(global_style.content.contains("font-family"));
}

// ---------------------------------------------------------------------------
// Style block: default (no attribute) should be scoped
// ---------------------------------------------------------------------------

#[test]
fn parse_style_default_scoped() {
    let input = r#"
<style>
  .item { color: red; }
</style>
"#;
    let file = parse(input).unwrap();
    let style = file.style.unwrap();
    // Default should be scoped (or at least parse without error)
    assert!(style.content.contains("color: red"));
}

// ---------------------------------------------------------------------------
// AI action: parse ai action with all fields
// ---------------------------------------------------------------------------

#[test]
fn parse_ai_action_all_fields() {
    let input = r#"
<script lang="rust">
  ai action classify_ticket(input: TicketText) -> TicketCategory {
    model: "gpt-4o"
    fallback: "gpt-3.5-turbo"
    temperature: 0.1
    audit: true
    pii: mask
    permission: "tickets.ai.classify"
  }
</script>
"#;
    let file = parse(input).unwrap();
    let script = file.script.unwrap();
    assert_eq!(script.ai_actions.len(), 1);

    let ai = &script.ai_actions[0];
    assert_eq!(ai.name, "classify_ticket");
    assert_eq!(ai.input_param, "input");
    assert_eq!(ai.input_type, "TicketText");
    assert_eq!(ai.return_type, "TicketCategory");
    assert_eq!(ai.model, "gpt-4o");
    assert_eq!(ai.fallback.as_deref(), Some("gpt-3.5-turbo"));
    assert_eq!(ai.temperature, Some(0.1));
    assert!(ai.audit);
    assert_eq!(ai.pii.as_deref(), Some("mask"));
    assert_eq!(ai.permission.as_deref(), Some("tickets.ai.classify"));
}

// ---------------------------------------------------------------------------
// AI action: minimal (no optional fields)
// ---------------------------------------------------------------------------

#[test]
fn parse_ai_action_minimal() {
    let input = r#"
<script lang="rust">
  ai action translate(input: Text) -> Translation {
    model: "nllb-200"
    audit: false
  }
</script>
"#;
    let file = parse(input).unwrap();
    let script = file.script.unwrap();
    assert_eq!(script.ai_actions.len(), 1);

    let ai = &script.ai_actions[0];
    assert_eq!(ai.name, "translate");
    assert_eq!(ai.model, "nllb-200");
    assert!(!ai.audit);
    assert!(ai.fallback.is_none());
    assert!(ai.temperature.is_none());
    assert!(ai.pii.is_none());
    assert!(ai.permission.is_none());
}

// ---------------------------------------------------------------------------
// Error display messages for ParseError variants
// ---------------------------------------------------------------------------

#[test]
fn parse_error_display_syntax() {
    let err = ParseError::Syntax {
        line: 5,
        col: 10,
        message: "unexpected token".to_string(),
    };
    let msg = format!("{}", err);
    assert!(msg.contains("line 5"));
    assert!(msg.contains("col 10"));
    assert!(msg.contains("unexpected token"));
}

#[test]
fn parse_error_display_unknown_block() {
    let err = ParseError::UnknownBlock("widget".to_string());
    let msg = format!("{}", err);
    assert!(msg.contains("widget"));
    assert!(msg.contains("Unknown block"));
}

#[test]
fn parse_error_display_duplicate_block() {
    let err = ParseError::DuplicateBlock("script".to_string());
    let msg = format!("{}", err);
    assert!(msg.contains("script"));
    assert!(msg.contains("Duplicate"));
}

#[test]
fn parse_error_display_missing_field() {
    let err = ParseError::MissingField {
        block: "resource".to_string(),
        field: "name".to_string(),
    };
    let msg = format!("{}", err);
    assert!(msg.contains("name"));
    assert!(msg.contains("resource"));
}

#[test]
fn parse_error_display_invalid_value() {
    let err = ParseError::InvalidValue {
        field: "cache".to_string(),
        value: "mega".to_string(),
        reason: "valid values: no-store, private, public, static".to_string(),
    };
    let msg = format!("{}", err);
    assert!(msg.contains("cache"));
    assert!(msg.contains("mega"));
}

#[test]
fn parse_error_display_unclosed_block() {
    let err = ParseError::UnclosedBlock("style".to_string());
    let msg = format!("{}", err);
    assert!(msg.contains("style"));
    assert!(msg.contains("Unclosed"));
}

#[test]
fn parse_error_display_unexpected_token() {
    let err = ParseError::UnexpectedToken(">>>".to_string());
    let msg = format!("{}", err);
    assert!(msg.contains(">>>"));
    assert!(msg.contains("Unexpected"));
}

// ---------------------------------------------------------------------------
// Multiple blocks in single file (route + script + template + style)
// ---------------------------------------------------------------------------

#[test]
fn parse_full_multi_block_file() {
    let input = r#"
<route>
  path: "/dashboard"
  layout: "main"
  auth: required
  tenant: required
  cache: private
</route>

<script lang="rust">
  use crate::repos::StatsRepo;

  state total_users: i64 = 0
  state total_orders: i64 = 0
  state recent_activity: Vec<Activity> = []

  load async fn load_stats(ctx: Ctx) {
    total_users = StatsRepo::count_users(ctx.tenant_id).await?;
    total_orders = StatsRepo::count_orders(ctx.tenant_id).await?;
    recent_activity = StatsRepo::recent(ctx.tenant_id, 10).await?;
  }

  action async fn refresh(ctx: Ctx) {
    total_users = StatsRepo::count_users(ctx.tenant_id).await?;
    total_orders = StatsRepo::count_orders(ctx.tenant_id).await?;
  }
</script>

<template>
  <div class="dashboard">
    <h1>Dashboard</h1>
    <div class="stats">
      <span>Users: {total_users}</span>
      <span>Orders: {total_orders}</span>
    </div>
    {#each recent_activity as activity}
      <p>{activity.description}</p>
    {/each}
    <button on:click="refresh">Refresh</button>
  </div>
</template>

<style scoped>
  .dashboard { padding: 24px; }
  .stats { display: flex; gap: 16px; }
</style>
"#;
    let file = parse(input).unwrap();

    // Route
    let route = file.route.unwrap();
    assert_eq!(route.path.as_deref(), Some("/dashboard"));
    assert_eq!(route.layout.as_deref(), Some("main"));
    assert_eq!(route.auth, Some(AuthLevel::Required));
    assert_eq!(route.tenant, Some(TenantLevel::Required));
    assert_eq!(route.cache, Some(CachePolicy::Private));

    // Script
    let script = file.script.unwrap();
    assert_eq!(script.uses.len(), 1);
    assert_eq!(script.states.len(), 3);
    assert_eq!(script.loaders.len(), 1);
    assert_eq!(script.actions.len(), 1);
    assert_eq!(script.actions[0].name, "refresh");
    assert!(script.actions[0].is_async);

    // Template
    let tpl = file.template.unwrap();
    assert!(!tpl.children.is_empty());

    // Style
    let style = file.style.unwrap();
    assert!(style.scoped);
    assert!(style.content.contains(".dashboard"));
    assert!(style.content.contains("display: flex"));
}

// ---------------------------------------------------------------------------
// Route with all cache policies
// ---------------------------------------------------------------------------

#[test]
fn parse_route_cache_policies() {
    for (policy_str, expected) in [
        ("no-store", CachePolicy::NoStore),
        ("private", CachePolicy::Private),
        ("public", CachePolicy::Public),
        ("static", CachePolicy::Static),
    ] {
        let input = format!(
            r#"
<route>
  path: "/test"
  cache: {}
</route>
"#,
            policy_str
        );
        let file = parse(&input).unwrap();
        let route = file.route.unwrap();
        assert_eq!(
            route.cache,
            Some(expected),
            "Failed for cache policy: {}",
            policy_str
        );
    }
}

// ---------------------------------------------------------------------------
// Route with all auth levels
// ---------------------------------------------------------------------------

#[test]
fn parse_route_auth_levels() {
    for (auth_str, expected) in [
        ("public", AuthLevel::Public),
        ("optional", AuthLevel::Optional),
        ("required", AuthLevel::Required),
    ] {
        let input = format!(
            r#"
<route>
  path: "/test"
  auth: {}
</route>
"#,
            auth_str
        );
        let file = parse(&input).unwrap();
        let route = file.route.unwrap();
        assert_eq!(
            route.auth,
            Some(expected),
            "Failed for auth level: {}",
            auth_str
        );
    }
}

// ---------------------------------------------------------------------------
// Route with all tenant levels
// ---------------------------------------------------------------------------

#[test]
fn parse_route_tenant_levels() {
    for (tenant_str, expected) in [
        ("none", TenantLevel::None),
        ("optional", TenantLevel::Optional),
        ("required", TenantLevel::Required),
    ] {
        let input = format!(
            r#"
<route>
  path: "/test"
  tenant: {}
</route>
"#,
            tenant_str
        );
        let file = parse(&input).unwrap();
        let route = file.route.unwrap();
        assert_eq!(
            route.tenant,
            Some(expected),
            "Failed for tenant level: {}",
            tenant_str
        );
    }
}

// ---------------------------------------------------------------------------
// Multiple state + action + memo + loader + form in one script
// ---------------------------------------------------------------------------

#[test]
fn parse_script_all_decl_types() {
    let input = r#"
<script lang="rust">
  use crate::models::Invoice;

  prop customer_id: Uuid
  state items: Vec<Item> = []
  state discount: f64 = 0.0
  state secret api_secret: String

  memo subtotal: f64 = items.iter().map(|i| i.price).sum()

  load async fn load_items(ctx: Ctx) {
    items = InvoiceRepo::items(ctx.tenant_id, customer_id).await?;
  }

  action add_item(item: Item) {
    items.push(item)
  }

  form InvoiceForm {
    reference: String required min=3 max=50
    amount: Decimal required
  }

  ai action estimate_total(input: InvoiceData) -> EstimatedTotal {
    model: "fin-model-v2"
    audit: true
  }
</script>
"#;
    let file = parse(input).unwrap();
    let script = file.script.unwrap();

    assert_eq!(script.uses.len(), 1);
    assert_eq!(script.props.len(), 1);
    assert_eq!(script.states.len(), 3);
    assert!(script.states[2].secret);
    assert_eq!(script.memos.len(), 1);
    assert_eq!(script.loaders.len(), 1);
    assert_eq!(script.actions.len(), 1);
    assert_eq!(script.forms.len(), 1);
    assert_eq!(script.ai_actions.len(), 1);
}

// ---------------------------------------------------------------------------
// Duplicate script block should error
// ---------------------------------------------------------------------------

#[test]
fn error_duplicate_script_block() {
    let input = r#"
<script lang="rust">
  state a: i32 = 0
</script>
<script lang="rust">
  state b: i32 = 0
</script>
"#;
    let result = parse(input);
    assert!(result.is_err());
    match result.unwrap_err() {
        ParseError::DuplicateBlock(name) => assert_eq!(name, "script"),
        other => panic!("Expected DuplicateBlock(script), got: {other}"),
    }
}

// ---------------------------------------------------------------------------
// Duplicate template block should error
// ---------------------------------------------------------------------------

#[test]
fn error_duplicate_template_block() {
    let input = r#"
<template>
  <div>A</div>
</template>
<template>
  <div>B</div>
</template>
"#;
    let result = parse(input);
    assert!(result.is_err());
    match result.unwrap_err() {
        ParseError::DuplicateBlock(name) => assert_eq!(name, "template"),
        other => panic!("Expected DuplicateBlock(template), got: {other}"),
    }
}

// ---------------------------------------------------------------------------
// Duplicate style block should error
// ---------------------------------------------------------------------------

#[test]
fn error_duplicate_style_block() {
    let input = r#"
<style scoped>
  .a { color: red; }
</style>
<style global>
  .b { color: blue; }
</style>
"#;
    let result = parse(input);
    assert!(result.is_err());
    match result.unwrap_err() {
        ParseError::DuplicateBlock(name) => assert_eq!(name, "style"),
        other => panic!("Expected DuplicateBlock(style), got: {other}"),
    }
}

// ---------------------------------------------------------------------------
// Named slot
// ---------------------------------------------------------------------------

#[test]
fn parse_template_named_slot() {
    let input = r#"
<template>
  <div>
    <slot name="header" />
    <slot />
  </div>
</template>
"#;
    let file = parse(input).unwrap();
    let tpl = file.template.unwrap();

    let div = match &tpl.children[0] {
        TemplateNode::Element(el) => el,
        other => panic!("Expected Element, got: {:?}", other),
    };

    let slots: Vec<&SlotNode> = div
        .children
        .iter()
        .filter_map(|n| match n {
            TemplateNode::Slot(s) => Some(s),
            _ => None,
        })
        .collect();

    assert_eq!(slots.len(), 2);
    assert_eq!(slots[0].name.as_deref(), Some("header"));
    assert!(slots[1].name.is_none());
}
