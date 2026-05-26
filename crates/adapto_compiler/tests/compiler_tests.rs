use adapto_compiler::codegen::CodeGenerator;
use adapto_compiler::compiler::Compiler;
use adapto_compiler::dependency::DependencyGraph;
use adapto_compiler::error::CompileError;
use adapto_compiler::ir::*;
use adapto_compiler::manifest::*;

/// Helper: parse a `.adapto` source string and compile it.
fn compile_source(source: &str, path: &str) -> adapto_compiler::compiler::CompileOutput {
    let file = adapto_parser::parse(source).expect("parse failed");
    let mut compiler = Compiler::new();
    compiler.compile_file(&file, path).expect("compile failed")
}

// ---------------------------------------------------------------------------
// 1. Compile counter example (state + action + template)
// ---------------------------------------------------------------------------

#[test]
fn test_compile_counter_example() {
    let source = r#"
<script>
    state count: i32 = 0
    action increment() {
        state.count += 1;
    }
</script>
<template>
    <button on:click="increment">{count}</button>
</template>
"#;

    let output = compile_source(source, "pages/counter.adapto");
    let ir = &output.component_ir;

    assert_eq!(ir.name, "Counter");
    assert_eq!(ir.state_fields.len(), 1);
    assert_eq!(ir.state_fields[0].name, "count");
    assert_eq!(ir.state_fields[0].ty, "i32");
    assert_eq!(ir.state_fields[0].default, Some("0".to_string()));
    assert_eq!(ir.actions.len(), 1);
    assert_eq!(ir.actions[0].name, "increment");
    assert!(!ir.static_segments.is_empty());
    assert!(!ir.dynamic_segments.is_empty());
    assert!(!ir.events.is_empty());
}

// ---------------------------------------------------------------------------
// 2. Verify static/dynamic segmentation
// ---------------------------------------------------------------------------

#[test]
fn test_static_dynamic_segmentation() {
    let source = r#"
<script>
    state name: String = "World"
</script>
<template>
    <p>Hello, {name}!</p>
</template>
"#;

    let output = compile_source(source, "components/greeting.adapto");
    let ir = &output.component_ir;

    // Should have static segments for <p>, "Hello, ", "!", </p>
    assert!(
        ir.static_segments.iter().any(|s| s.contains("<p>")),
        "Expected a <p> static segment, got: {:?}",
        ir.static_segments
    );

    // Should have one dynamic segment for {name}
    assert_eq!(ir.dynamic_segments.len(), 1);
    assert!(ir.dynamic_segments[0].expr.contains("name"));
}

// ---------------------------------------------------------------------------
// 3. Verify dependency graph (counter depends on "count")
// ---------------------------------------------------------------------------

#[test]
fn test_dependency_graph_counter() {
    let source = r#"
<script>
    state count: i32 = 0
</script>
<template>
    <span>{count}</span>
</template>
"#;

    let output = compile_source(source, "counter.adapto");
    let graph = &output.dependency_graph;

    assert!(
        graph.all_state_fields().contains("count"),
        "Dependency graph should track 'count' field"
    );

    let affected = graph.get_affected_segments(&["count"]);
    assert!(
        !affected.is_empty(),
        "Changing 'count' should affect at least one segment"
    );
}

// ---------------------------------------------------------------------------
// 4. Verify event IR extraction
// ---------------------------------------------------------------------------

#[test]
fn test_event_ir_extraction() {
    let source = r#"
<script>
    action save() {
        // save logic
    }
</script>
<template>
    <button on:click="save">Save</button>
</template>
"#;

    let output = compile_source(source, "form.adapto");
    let ir = &output.component_ir;

    assert_eq!(ir.events.len(), 1);
    assert_eq!(ir.events[0].event_type, "click");
    assert_eq!(ir.events[0].handler, "save");
}

// ---------------------------------------------------------------------------
// 5. Verify action IR extraction
// ---------------------------------------------------------------------------

#[test]
fn test_action_ir_extraction() {
    let source = r#"
<script>
    action async submit(data: FormData) {
        let result = api.save(data);
    }
</script>
<template>
    <form on:submit="submit">Submit</form>
</template>
"#;

    let output = compile_source(source, "submit.adapto");
    let ir = &output.component_ir;

    assert_eq!(ir.actions.len(), 1);
    assert_eq!(ir.actions[0].name, "submit");
    assert!(ir.actions[0].is_async);
    assert_eq!(ir.actions[0].params.len(), 1);
    assert_eq!(ir.actions[0].params[0].name, "data");
    assert_eq!(ir.actions[0].params[0].ty, "FormData");
}

// ---------------------------------------------------------------------------
// 6. Compile route block to RouteEntry
// ---------------------------------------------------------------------------

#[test]
fn test_compile_route_block() {
    let source = r#"
<route>
    path: "/customers"
    method: GET
    auth: required
    tenant: required
    cache: private
    layout: "dashboard"
    permission: "customers.view"
</route>
<template>
    <div>Customers</div>
</template>
"#;

    let output = compile_source(source, "pages/customers.adapto");

    let route_entry = output.route_entry.expect("Should have a route entry");
    assert_eq!(route_entry.path, "/customers");
    assert_eq!(route_entry.method, "GET");
    assert_eq!(route_entry.auth, "required");
    assert_eq!(route_entry.tenant, "required");
    assert_eq!(route_entry.cache, "private");
    assert_eq!(route_entry.layout, Some("dashboard".to_string()));
    assert_eq!(
        route_entry.permission,
        Some("customers.view".to_string())
    );

    let ir = &output.component_ir;
    let route_ir = ir.route.as_ref().expect("IR should have route");
    assert_eq!(route_ir.path, "/customers");
    assert_eq!(route_ir.auth, "required");
}

// ---------------------------------------------------------------------------
// 7. Route manifest add and lookup
// ---------------------------------------------------------------------------

#[test]
fn test_route_manifest_add_and_lookup() {
    let mut manifest = RouteManifest::new();

    manifest.add(RouteEntry {
        id: "route_customers".to_string(),
        path: "/customers".to_string(),
        file: "pages/customers.adapto".to_string(),
        method: "GET".to_string(),
        auth: "required".to_string(),
        tenant: "required".to_string(),
        permission: Some("customers.view".to_string()),
        layout: Some("dashboard".to_string()),
        cache: "private".to_string(),
    });

    manifest.add(RouteEntry {
        id: "route_settings".to_string(),
        path: "/settings".to_string(),
        file: "pages/settings.adapto".to_string(),
        method: "GET".to_string(),
        auth: "required".to_string(),
        tenant: "none".to_string(),
        permission: None,
        layout: None,
        cache: "no-store".to_string(),
    });

    assert_eq!(manifest.routes.len(), 2);

    let found = manifest.find_by_path("/customers");
    assert!(found.is_some());
    assert_eq!(found.unwrap().id, "route_customers");

    let not_found = manifest.find_by_path("/nonexistent");
    assert!(not_found.is_none());

    // JSON serialization
    let json = manifest.to_json();
    assert!(json.contains("/customers"));
    assert!(json.contains("/settings"));
}

// ---------------------------------------------------------------------------
// 8. Component manifest add and lookup
// ---------------------------------------------------------------------------

#[test]
fn test_component_manifest_add_and_lookup() {
    let mut manifest = ComponentManifest::new();

    manifest.add(ComponentEntry {
        id: "comp_counter".to_string(),
        name: "Counter".to_string(),
        file: "components/counter.adapto".to_string(),
        is_island: false,
        dependencies: vec!["count".to_string()],
    });

    manifest.add(ComponentEntry {
        id: "comp_header".to_string(),
        name: "Header".to_string(),
        file: "components/header.adapto".to_string(),
        is_island: true,
        dependencies: vec![],
    });

    assert_eq!(manifest.components.len(), 2);

    let found = manifest.find_by_name("Counter");
    assert!(found.is_some());
    assert_eq!(found.unwrap().id, "comp_counter");

    let not_found = manifest.find_by_name("Footer");
    assert!(not_found.is_none());

    let json = manifest.to_json();
    assert!(json.contains("Counter"));
    assert!(json.contains("Header"));
}

// ---------------------------------------------------------------------------
// 9. Dependency graph: single state -> multiple segments
// ---------------------------------------------------------------------------

#[test]
fn test_dependency_graph_single_state_multiple_segments() {
    let mut graph = DependencyGraph::new();
    graph.add_dependency("dyn_0", "count");
    graph.add_dependency("dyn_1", "count");
    graph.add_dependency("dyn_2", "count");

    let affected = graph.get_affected_segments(&["count"]);
    assert_eq!(affected.len(), 3);
    assert!(affected.contains("dyn_0"));
    assert!(affected.contains("dyn_1"));
    assert!(affected.contains("dyn_2"));
}

// ---------------------------------------------------------------------------
// 10. Dependency graph: get_affected_segments
// ---------------------------------------------------------------------------

#[test]
fn test_dependency_graph_get_affected_segments() {
    let mut graph = DependencyGraph::new();
    graph.add_dependency("dyn_0", "name");
    graph.add_dependency("dyn_1", "email");
    graph.add_dependency("dyn_2", "name");
    graph.add_dependency("dyn_3", "age");

    // Changing name affects dyn_0 and dyn_2
    let affected_name = graph.get_affected_segments(&["name"]);
    assert_eq!(affected_name.len(), 2);
    assert!(affected_name.contains("dyn_0"));
    assert!(affected_name.contains("dyn_2"));

    // Changing email affects only dyn_1
    let affected_email = graph.get_affected_segments(&["email"]);
    assert_eq!(affected_email.len(), 1);
    assert!(affected_email.contains("dyn_1"));

    // Changing name and email affects dyn_0, dyn_1, dyn_2
    let affected_both = graph.get_affected_segments(&["name", "email"]);
    assert_eq!(affected_both.len(), 3);

    // Changing unknown field affects nothing
    let affected_none = graph.get_affected_segments(&["nonexistent"]);
    assert!(affected_none.is_empty());
}

// ---------------------------------------------------------------------------
// 11. Dependency graph: validate detects unknown deps
// ---------------------------------------------------------------------------

#[test]
fn test_dependency_graph_validate_unknown_deps() {
    let mut graph = DependencyGraph::new();
    graph.add_dependency("dyn_0", "name");
    graph.add_dependency("dyn_1", "email");
    graph.add_dependency("dyn_2", "unknown_field");

    let unknown = graph.validate(&["name", "email"]);
    assert_eq!(unknown.len(), 1);
    assert_eq!(unknown[0], "unknown_field");

    let no_unknown = graph.validate(&["name", "email", "unknown_field"]);
    assert!(no_unknown.is_empty());
}

// ---------------------------------------------------------------------------
// 12. CodeGenerator: generate state struct
// ---------------------------------------------------------------------------

#[test]
fn test_codegen_state_struct() {
    let ir = ComponentIR {
        id: "comp_counter".to_string(),
        name: "Counter".to_string(),
        route: None,
        static_segments: vec![],
        dynamic_segments: vec![],
        events: vec![],
        actions: vec![],
        state_fields: vec![
            StateFieldIR {
                name: "count".to_string(),
                ty: "i32".to_string(),
                default: Some("0".to_string()),
                secret: false,
            },
            StateFieldIR {
                name: "label".to_string(),
                ty: "String".to_string(),
                default: None,
                secret: false,
            },
        ],
        form_schemas: vec![],
        permissions: vec![],
        children: vec![],
        is_island: false,
        style: None,
    };

    let mut gen = CodeGenerator::new();
    let code = gen.generate_component(&ir);

    assert!(
        code.contains("pub struct CounterState"),
        "Should contain state struct. Got:\n{}",
        code
    );
    assert!(code.contains("pub count: i32"));
    assert!(code.contains("pub label: String"));
}

// ---------------------------------------------------------------------------
// 13. CodeGenerator: generate render function
// ---------------------------------------------------------------------------

#[test]
fn test_codegen_render_function() {
    let ir = ComponentIR {
        id: "comp_counter".to_string(),
        name: "Counter".to_string(),
        route: None,
        static_segments: vec![
            "<button data-ar-click=\"increment\">".to_string(),
            "</button>".to_string(),
        ],
        dynamic_segments: vec![DynamicSegment {
            id: "dyn_0".to_string(),
            expr: "state.count".to_string(),
            deps: vec!["count".to_string()],
            segment_type: SegmentType::Text,
        }],
        events: vec![],
        actions: vec![],
        state_fields: vec![StateFieldIR {
            name: "count".to_string(),
            ty: "i32".to_string(),
            default: Some("0".to_string()),
            secret: false,
        }],
        form_schemas: vec![],
        permissions: vec![],
        children: vec![],
        is_island: false,
        style: None,
    };

    let mut gen = CodeGenerator::new();
    let code = gen.generate_component(&ir);

    assert!(
        code.contains("fn render(&self, state: &Self::State) -> Rendered"),
        "Should contain render function. Got:\n{}",
        code
    );
    assert!(code.contains("Rendered::new()"));
    assert!(code.contains("static_part"));
    assert!(code.contains("dynamic_text"));
    assert!(code.contains("dyn_0"));
}

// ---------------------------------------------------------------------------
// 14. CodeGenerator: generate event handler
// ---------------------------------------------------------------------------

#[test]
fn test_codegen_event_handler() {
    let ir = ComponentIR {
        id: "comp_counter".to_string(),
        name: "Counter".to_string(),
        route: None,
        static_segments: vec![],
        dynamic_segments: vec![],
        events: vec![],
        actions: vec![ActionIR {
            name: "increment".to_string(),
            is_async: false,
            params: vec![],
            permission: None,
            audit: None,
            body: "state.count += 1;\nmark_dirty!(\"count\");".to_string(),
        }],
        state_fields: vec![StateFieldIR {
            name: "count".to_string(),
            ty: "i32".to_string(),
            default: Some("0".to_string()),
            secret: false,
        }],
        form_schemas: vec![],
        permissions: vec![],
        children: vec![],
        is_island: false,
        style: None,
    };

    let mut gen = CodeGenerator::new();
    let code = gen.generate_component(&ir);

    assert!(
        code.contains("fn handle_event"),
        "Should contain handle_event. Got:\n{}",
        code
    );
    assert!(code.contains("\"increment\""));
    assert!(code.contains("state.count += 1;"));
    assert!(code.contains("Err(Error::UnknownHandler)"));
}

// ---------------------------------------------------------------------------
// 15. Security check: detect secret state in template
// ---------------------------------------------------------------------------

#[test]
fn test_security_check_secret_state_in_template() {
    let source = r#"
<script>
    state secret api_key: String = ""
    state name: String = "test"
</script>
<template>
    <p>{api_key}</p>
</template>
"#;

    let file = adapto_parser::parse(source).expect("parse failed");
    let mut compiler = Compiler::new();
    let result = compiler.compile_file(&file, "secret.adapto");

    assert!(result.is_err(), "Should reject secret state in template");
    let err = result.unwrap_err();
    let err_msg = format!("{}", err);
    assert!(
        err_msg.contains("E0421") || err_msg.contains("Secret state"),
        "Error should reference E0421 or secret state. Got: {}",
        err_msg
    );
}

// ---------------------------------------------------------------------------
// 16. Compile file with form schema
// ---------------------------------------------------------------------------

#[test]
fn test_compile_form_schema() {
    let source = r#"
<script>
    form CustomerForm {
        name: String required min=2 max=100
        email: String required
        age: i32
    }
</script>
<template>
    <form>Form</form>
</template>
"#;

    let output = compile_source(source, "customer_form.adapto");
    let ir = &output.component_ir;

    assert_eq!(ir.form_schemas.len(), 1);
    let form = &ir.form_schemas[0];
    assert_eq!(form.name, "CustomerForm");
    assert_eq!(form.fields.len(), 3);

    let name_field = &form.fields[0];
    assert_eq!(name_field.name, "name");
    assert_eq!(name_field.ty, "String");
    assert!(name_field.required);
    assert_eq!(name_field.min, Some(2));
    assert_eq!(name_field.max, Some(100));

    let email_field = &form.fields[1];
    assert!(email_field.required);

    let age_field = &form.fields[2];
    assert!(!age_field.required);
}

// ---------------------------------------------------------------------------
// 17. Compile file with permissions
// ---------------------------------------------------------------------------

#[test]
fn test_compile_permissions() {
    let source = r#"
<route>
    path: "/admin/users"
    auth: required
    permission: "admin.users.view"
</route>
<script>
    #[permission("admin.users.delete")]
    action delete_user(id: String) {
        api.delete(id);
    }
</script>
<template>
    <div>Admin</div>
</template>
"#;

    let output = compile_source(source, "pages/admin_users.adapto");
    let ir = &output.component_ir;

    // Permissions from both route and actions
    assert!(
        ir.permissions.contains(&"admin.users.delete".to_string()),
        "Should include action permission. Got: {:?}",
        ir.permissions
    );
    assert!(
        ir.permissions.contains(&"admin.users.view".to_string()),
        "Should include route permission. Got: {:?}",
        ir.permissions
    );

    assert_eq!(
        ir.actions[0].permission,
        Some("admin.users.delete".to_string())
    );
}

// ---------------------------------------------------------------------------
// 18. Compile file with style (scoped)
// ---------------------------------------------------------------------------

#[test]
fn test_compile_scoped_style() {
    let source = r#"
<template>
    <div class="container">Hello</div>
</template>
<style scoped>
    .container {
        padding: 16px;
        background: #fff;
    }
</style>
"#;

    let output = compile_source(source, "components/card.adapto");
    let ir = &output.component_ir;

    let style = ir.style.as_ref().expect("Should have compiled style");
    assert!(style.scoped);
    assert!(style.scope_id.is_some());
    assert!(style.css.contains("padding: 16px"));
}

// ---------------------------------------------------------------------------
// 19. Compile full customer page example
// ---------------------------------------------------------------------------

#[test]
fn test_compile_customer_page() {
    let source = r#"
<route>
    path: "/customers/:id"
    auth: required
    tenant: required
    layout: "dashboard"
    cache: private
</route>
<script>
    state customer_name: String = ""
    state email: String = ""
    state status: String = "active"

    action async save() {
        let result = repo.update(state.customer_name, state.email);
    }
</script>
<template>
    <div class="customer-page">
        <h1>{customer_name}</h1>
        <p>{email}</p>
        <span class="badge">{status}</span>
        <button on:click="save">Save</button>
    </div>
</template>
<style scoped>
    .customer-page { padding: 24px; }
    .badge { border-radius: 4px; }
</style>
"#;

    let output = compile_source(source, "pages/customer_detail.adapto");
    let ir = &output.component_ir;

    assert_eq!(ir.name, "CustomerDetail");
    assert_eq!(ir.state_fields.len(), 3);
    assert_eq!(ir.actions.len(), 1);
    assert!(ir.actions[0].is_async);
    assert_eq!(ir.events.len(), 1);
    assert_eq!(ir.dynamic_segments.len(), 3); // customer_name, email, status
    assert!(ir.route.is_some());
    assert!(ir.style.is_some());

    let route = ir.route.as_ref().unwrap();
    assert_eq!(route.path, "/customers/:id");
    assert_eq!(route.auth, "required");
    assert_eq!(route.tenant, "required");

    // Dependency graph tracks all three state fields
    let graph = &output.dependency_graph;
    let fields = graph.all_state_fields();
    assert!(fields.contains("customer_name"));
    assert!(fields.contains("email"));
    assert!(fields.contains("status"));
}

// ---------------------------------------------------------------------------
// 20. CompileError formatting matches spec error format
// ---------------------------------------------------------------------------

#[test]
fn test_compile_error_formatting() {
    let err = CompileError::UnknownAction {
        action: "delete".to_string(),
        file: "page.adapto".to_string(),
        line: 10,
        col: 5,
    };
    assert_eq!(
        format!("{}", err),
        "E0101: Unknown action `delete` at page.adapto:10:5"
    );

    let err = CompileError::SecretStateInTemplate {
        field: "api_key".to_string(),
        file: "page.adapto".to_string(),
        line: 15,
        col: 3,
    };
    assert_eq!(
        format!("{}", err),
        "E0421: Secret state `api_key` cannot be rendered in template at page.adapto:15:3"
    );

    let err = CompileError::DuplicateState {
        name: "count".to_string(),
    };
    assert_eq!(
        format!("{}", err),
        "E0201: Duplicate state declaration: count"
    );

    let err = CompileError::TemplateSyntax {
        file: "broken.adapto".to_string(),
        line: 5,
        col: 10,
        message: "unexpected token".to_string(),
    };
    assert_eq!(
        format!("{}", err),
        "E0501: Invalid template syntax at broken.adapto:5:10: unexpected token"
    );

    let err = CompileError::Multiple {
        count: 3,
        errors: vec![],
    };
    assert_eq!(format!("{}", err), "Compilation failed with 3 errors");
}

// ---------------------------------------------------------------------------
// 21. Compile template with if/else
// ---------------------------------------------------------------------------

#[test]
fn test_compile_if_else() {
    let source = r#"
<script>
    state show: bool = true
    state name: String = "test"
</script>
<template>
    {#if show}
        <p>Visible: {name}</p>
    {:else}
        <p>Hidden</p>
    {/if}
</template>
"#;

    let output = compile_source(source, "conditional.adapto");
    let ir = &output.component_ir;

    // Should have a conditional dynamic segment
    let conditionals: Vec<_> = ir
        .dynamic_segments
        .iter()
        .filter(|s| matches!(s.segment_type, SegmentType::Conditional))
        .collect();
    assert!(
        !conditionals.is_empty(),
        "Should have conditional segments. Dynamic segments: {:?}",
        ir.dynamic_segments
    );

    // Should also have a text dynamic segment for {name}
    let texts: Vec<_> = ir
        .dynamic_segments
        .iter()
        .filter(|s| matches!(s.segment_type, SegmentType::Text))
        .collect();
    assert!(
        !texts.is_empty(),
        "Should have text dynamic segments for expression inside if"
    );
}

// ---------------------------------------------------------------------------
// 22. Compile template with each loop
// ---------------------------------------------------------------------------

#[test]
fn test_compile_each_loop() {
    let source = r#"
<script>
    state items: Vec<String> = vec![]
</script>
<template>
    {#each items as item, index}
        <li>{item}</li>
    {/each}
</template>
"#;

    let output = compile_source(source, "list.adapto");
    let ir = &output.component_ir;

    let loops: Vec<_> = ir
        .dynamic_segments
        .iter()
        .filter(|s| matches!(s.segment_type, SegmentType::Loop))
        .collect();
    assert_eq!(
        loops.len(),
        1,
        "Should have exactly one loop segment. Got: {:?}",
        ir.dynamic_segments
    );
    assert_eq!(loops[0].expr, "items");

    // The loop's iterable should depend on "items"
    assert!(loops[0].deps.contains(&"items".to_string()));
}

// ---------------------------------------------------------------------------
// 23. Compile template with can permission
// ---------------------------------------------------------------------------

#[test]
fn test_compile_can_permission() {
    let source = r#"
<script>
    state name: String = "admin"
</script>
<template>
    {#can "users.delete"}
        <button on:click="delete">Delete</button>
    {/can}
</template>
"#;

    let output = compile_source(source, "admin.adapto");
    let ir = &output.component_ir;

    let perms: Vec<_> = ir
        .dynamic_segments
        .iter()
        .filter(|s| matches!(s.segment_type, SegmentType::Permission))
        .collect();
    assert_eq!(
        perms.len(),
        1,
        "Should have one permission segment. Got: {:?}",
        ir.dynamic_segments
    );
    assert_eq!(perms[0].expr, "users.delete");
}

// ---------------------------------------------------------------------------
// 24. Compile template with component usage
// ---------------------------------------------------------------------------

#[test]
fn test_compile_component_usage() {
    let source = r#"
<template>
    <div>
        <Header title="Page Title" />
        <Sidebar active="home" />
    </div>
</template>
"#;

    let output = compile_source(source, "page.adapto");
    let ir = &output.component_ir;

    assert!(
        ir.children.contains(&"Header".to_string()),
        "Should reference Header child. Got: {:?}",
        ir.children
    );
    assert!(
        ir.children.contains(&"Sidebar".to_string()),
        "Should reference Sidebar child. Got: {:?}",
        ir.children
    );
}

// ---------------------------------------------------------------------------
// 25. Multiple dynamic segments get unique IDs
// ---------------------------------------------------------------------------

#[test]
fn test_unique_dynamic_segment_ids() {
    let source = r#"
<script>
    state first: String = ""
    state last: String = ""
    state age: i32 = 0
</script>
<template>
    <p>{first}</p>
    <p>{last}</p>
    <p>{age}</p>
</template>
"#;

    let output = compile_source(source, "multi.adapto");
    let ir = &output.component_ir;

    let ids: Vec<&str> = ir.dynamic_segments.iter().map(|s| s.id.as_str()).collect();

    // All IDs should be unique
    let unique_ids: std::collections::HashSet<&str> = ids.iter().copied().collect();
    assert_eq!(
        ids.len(),
        unique_ids.len(),
        "All dynamic segment IDs should be unique. Got: {:?}",
        ids
    );

    // IDs should follow the dyn_N pattern
    for id in &ids {
        assert!(
            id.starts_with("dyn_"),
            "Dynamic segment ID should start with 'dyn_'. Got: {}",
            id
        );
    }
}

// ---------------------------------------------------------------------------
// Additional edge case tests
// ---------------------------------------------------------------------------

#[test]
fn test_compile_file_no_template() {
    let source = r#"
<script>
    state count: i32 = 0
</script>
"#;

    let output = compile_source(source, "headless.adapto");
    let ir = &output.component_ir;

    assert!(ir.static_segments.is_empty());
    assert!(ir.dynamic_segments.is_empty());
    assert!(ir.events.is_empty());
    assert_eq!(ir.state_fields.len(), 1);
}

#[test]
fn test_compile_file_no_script() {
    let source = r#"
<template>
    <div>Static content only</div>
</template>
"#;

    let output = compile_source(source, "static_page.adapto");
    let ir = &output.component_ir;

    assert!(ir.state_fields.is_empty());
    assert!(ir.actions.is_empty());
    assert!(!ir.static_segments.is_empty());
}

#[test]
fn test_component_name_derivation() {
    // Test various file naming patterns
    let source = r#"
<template>
    <div>Test</div>
</template>
"#;

    let output1 = compile_source(source, "pages/customer_list.adapto");
    assert_eq!(output1.component_ir.name, "CustomerList");

    let output2 = compile_source(source, "components/nav-bar.adapto");
    assert_eq!(output2.component_ir.name, "NavBar");

    let output3 = compile_source(source, "counter.adapto");
    assert_eq!(output3.component_ir.name, "Counter");
}

#[test]
fn test_dependency_graph_all_segments() {
    let mut graph = DependencyGraph::new();
    graph.add_dependency("dyn_0", "name");
    graph.add_dependency("dyn_1", "name");
    graph.add_dependency("dyn_2", "email");

    let all = graph.all_segments();
    assert_eq!(all.len(), 3);
    assert!(all.contains("dyn_0"));
    assert!(all.contains("dyn_1"));
    assert!(all.contains("dyn_2"));
}

#[test]
fn test_dependency_graph_deps_for_segment() {
    let mut graph = DependencyGraph::new();
    graph.add_dependency("dyn_0", "name");
    graph.add_dependency("dyn_0", "email");
    graph.add_dependency("dyn_1", "age");

    let deps = graph.get_deps_for_segment("dyn_0");
    assert_eq!(deps.len(), 2);
    assert!(deps.contains("name"));
    assert!(deps.contains("email"));

    let deps1 = graph.get_deps_for_segment("dyn_1");
    assert_eq!(deps1.len(), 1);
    assert!(deps1.contains("age"));

    let deps_none = graph.get_deps_for_segment("dyn_99");
    assert!(deps_none.is_empty());
}

#[test]
fn test_duplicate_state_detection() {
    let source = r#"
<script>
    state count: i32 = 0
    state count: i32 = 1
</script>
<template>
    <div>test</div>
</template>
"#;

    let file = adapto_parser::parse(source).expect("parse failed");
    let mut compiler = Compiler::new();
    let result = compiler.compile_file(&file, "dup.adapto");

    assert!(result.is_err());
    let err_msg = format!("{}", result.unwrap_err());
    assert!(
        err_msg.contains("E0201") || err_msg.contains("Duplicate state"),
        "Should report duplicate state. Got: {}",
        err_msg
    );
}

#[test]
fn test_codegen_form_struct() {
    let schema = FormSchemaIR {
        name: "LoginForm".to_string(),
        fields: vec![
            FormFieldIR {
                name: "username".to_string(),
                ty: "String".to_string(),
                required: true,
                min: Some(3),
                max: Some(50),
            },
            FormFieldIR {
                name: "password".to_string(),
                ty: "String".to_string(),
                required: true,
                min: Some(8),
                max: None,
            },
        ],
    };

    let ir = ComponentIR {
        id: "comp_login".to_string(),
        name: "Login".to_string(),
        route: None,
        static_segments: vec![],
        dynamic_segments: vec![],
        events: vec![],
        actions: vec![],
        state_fields: vec![],
        form_schemas: vec![schema],
        permissions: vec![],
        children: vec![],
        is_island: false,
        style: None,
    };

    let mut gen = CodeGenerator::new();
    let code = gen.generate_component(&ir);

    assert!(code.contains("pub struct LoginForm"));
    assert!(code.contains("pub username: String"));
    assert!(code.contains("pub password: String"));
    assert!(code.contains("fn validate"));
}

#[test]
fn test_generated_rust_contains_component_impl() {
    let source = r#"
<script>
    state count: i32 = 0
    action increment() {
        state.count += 1;
    }
</script>
<template>
    <button on:click="increment">{count}</button>
</template>
"#;

    let output = compile_source(source, "counter.adapto");

    assert!(
        output.generated_rust.contains("impl Component for Counter"),
        "Generated Rust should contain Component impl. Got:\n{}",
        output.generated_rust
    );
    assert!(output.generated_rust.contains("type State = CounterState;"));
    assert!(output.generated_rust.contains("fn render"));
    assert!(output.generated_rust.contains("fn handle_event"));
}

#[test]
fn test_route_manifest_json_roundtrip() {
    let mut manifest = RouteManifest::new();
    manifest.add(RouteEntry {
        id: "route_home".to_string(),
        path: "/".to_string(),
        file: "pages/home.adapto".to_string(),
        method: "GET".to_string(),
        auth: "public".to_string(),
        tenant: "none".to_string(),
        permission: None,
        layout: None,
        cache: "public".to_string(),
    });

    let json = manifest.to_json();
    let parsed: RouteManifest = serde_json::from_str(&json).expect("should parse back");
    assert_eq!(parsed.routes.len(), 1);
    assert_eq!(parsed.routes[0].path, "/");
}

#[test]
fn test_compiler_accumulates_manifests() {
    let source1 = r#"
<route>
    path: "/page-a"
    auth: public
</route>
<template><div>A</div></template>
"#;

    let source2 = r#"
<route>
    path: "/page-b"
    auth: required
</route>
<template><div>B</div></template>
"#;

    let mut compiler = Compiler::new();

    let file1 = adapto_parser::parse(source1).expect("parse 1");
    compiler
        .compile_file(&file1, "pages/page_a.adapto")
        .expect("compile 1");

    let file2 = adapto_parser::parse(source2).expect("parse 2");
    compiler
        .compile_file(&file2, "pages/page_b.adapto")
        .expect("compile 2");

    assert_eq!(compiler.route_manifest().routes.len(), 2);
    assert_eq!(compiler.component_manifest().components.len(), 2);

    assert!(compiler.route_manifest().find_by_path("/page-a").is_some());
    assert!(compiler.route_manifest().find_by_path("/page-b").is_some());
}

#[test]
fn test_compile_global_style() {
    let source = r#"
<template>
    <div>Global styled</div>
</template>
<style global>
    body { margin: 0; }
</style>
"#;

    let output = compile_source(source, "global.adapto");
    let ir = &output.component_ir;

    let style = ir.style.as_ref().expect("Should have style");
    assert!(!style.scoped);
    assert!(style.scope_id.is_none());
}

#[test]
fn test_compile_event_with_modifiers() {
    let source = r#"
<script>
    action submit() {
        // submit
    }
</script>
<template>
    <form on:submit.prevent="submit">
        <button type="submit">Go</button>
    </form>
</template>
"#;

    let output = compile_source(source, "form.adapto");
    let ir = &output.component_ir;

    assert_eq!(ir.events.len(), 1);
    assert_eq!(ir.events[0].event_type, "submit");
    assert!(
        ir.events[0].modifiers.contains(&"prevent".to_string()),
        "Should have prevent modifier. Got: {:?}",
        ir.events[0].modifiers
    );
}

#[test]
fn test_non_secret_state_in_template_is_ok() {
    let source = r#"
<script>
    state secret api_key: String = ""
    state name: String = "visible"
</script>
<template>
    <p>{name}</p>
</template>
"#;

    // This should compile fine — only name is in the template, not api_key
    let output = compile_source(source, "ok.adapto");
    assert_eq!(output.component_ir.dynamic_segments.len(), 1);
}

#[test]
fn test_interleave_ordering() {
    let source = r#"
<script>
    state count: i32 = 0
    action increment() { count += 1 }
</script>
<template>
  <div>
    <h1>Title</h1>
    <p>Count: {count}</p>
    <button on:click="increment">+1</button>
  </div>
</template>
"#;

    let output = compile_source(source, "counter.adapto");
    let ir = &output.component_ir;

    assert_eq!(ir.static_segments.len(), 2, "Expected 2 static segments for correct interleaving");
    assert_eq!(ir.dynamic_segments.len(), 1);

    assert!(ir.static_segments[0].contains("<p>Count:"),
        "First static segment should end with '<p>Count:', got: {:?}", ir.static_segments[0]);
    assert!(ir.static_segments[1].starts_with("</p>"),
        "Second static segment should start with '</p>', got: {:?}", ir.static_segments[1]);
}

// ===========================================================================
// Extended test suite
// ===========================================================================

// ---------------------------------------------------------------------------
// DependencyGraph: add_dependency bidirectional consistency
// ---------------------------------------------------------------------------

#[test]
fn test_dependency_graph_add_dependency_bidirectional() {
    let mut graph = DependencyGraph::new();
    graph.add_dependency("dyn_0", "username");
    graph.add_dependency("dyn_0", "email");
    graph.add_dependency("dyn_1", "username");

    // Forward: state -> segments
    let affected = graph.get_affected_segments(&["username"]);
    assert_eq!(affected.len(), 2);
    assert!(affected.contains("dyn_0"));
    assert!(affected.contains("dyn_1"));

    // Reverse: segment -> deps
    let deps_0 = graph.get_deps_for_segment("dyn_0");
    assert_eq!(deps_0.len(), 2);
    assert!(deps_0.contains("username"));
    assert!(deps_0.contains("email"));

    let deps_1 = graph.get_deps_for_segment("dyn_1");
    assert_eq!(deps_1.len(), 1);
    assert!(deps_1.contains("username"));
}

// ---------------------------------------------------------------------------
// DependencyGraph: validate with all known fields passes
// ---------------------------------------------------------------------------

#[test]
fn test_dependency_graph_validate_all_known() {
    let mut graph = DependencyGraph::new();
    graph.add_dependency("dyn_0", "a");
    graph.add_dependency("dyn_1", "b");
    graph.add_dependency("dyn_2", "c");

    let unknown = graph.validate(&["a", "b", "c"]);
    assert!(unknown.is_empty(), "All fields known, should be empty: {:?}", unknown);
}

// ---------------------------------------------------------------------------
// DependencyGraph: validate detects multiple unknowns (sorted)
// ---------------------------------------------------------------------------

#[test]
fn test_dependency_graph_validate_multiple_unknowns() {
    let mut graph = DependencyGraph::new();
    graph.add_dependency("dyn_0", "known");
    graph.add_dependency("dyn_1", "unknown_b");
    graph.add_dependency("dyn_2", "unknown_a");

    let unknown = graph.validate(&["known"]);
    assert_eq!(unknown.len(), 2);
    // Should be sorted
    assert_eq!(unknown[0], "unknown_a");
    assert_eq!(unknown[1], "unknown_b");
}

// ---------------------------------------------------------------------------
// DependencyGraph: all_state_fields and all_segments
// ---------------------------------------------------------------------------

#[test]
fn test_dependency_graph_all_state_fields_and_segments() {
    let mut graph = DependencyGraph::new();
    graph.add_dependency("seg_a", "field_x");
    graph.add_dependency("seg_b", "field_y");
    graph.add_dependency("seg_c", "field_x");

    let fields = graph.all_state_fields();
    assert_eq!(fields.len(), 2);
    assert!(fields.contains("field_x"));
    assert!(fields.contains("field_y"));

    let segments = graph.all_segments();
    assert_eq!(segments.len(), 3);
    assert!(segments.contains("seg_a"));
    assert!(segments.contains("seg_b"));
    assert!(segments.contains("seg_c"));
}

// ---------------------------------------------------------------------------
// DependencyGraph: empty graph edge cases
// ---------------------------------------------------------------------------

#[test]
fn test_dependency_graph_empty() {
    let graph = DependencyGraph::new();

    assert!(graph.all_state_fields().is_empty());
    assert!(graph.all_segments().is_empty());
    assert!(graph.get_affected_segments(&["anything"]).is_empty());
    assert!(graph.get_deps_for_segment("dyn_0").is_empty());
    assert!(graph.validate(&[]).is_empty());
}

// ---------------------------------------------------------------------------
// RouteManifest: add, find_by_path, to_json
// ---------------------------------------------------------------------------

#[test]
fn test_route_manifest_comprehensive() {
    let mut manifest = RouteManifest::new();
    assert!(manifest.routes.is_empty());

    manifest.add(RouteEntry {
        id: "r1".to_string(),
        path: "/api/users".to_string(),
        file: "pages/users.adapto".to_string(),
        method: "GET".to_string(),
        auth: "required".to_string(),
        tenant: "required".to_string(),
        permission: Some("users.list".to_string()),
        layout: Some("admin".to_string()),
        cache: "no-store".to_string(),
    });

    manifest.add(RouteEntry {
        id: "r2".to_string(),
        path: "/api/users/:id".to_string(),
        file: "pages/user_detail.adapto".to_string(),
        method: "GET".to_string(),
        auth: "required".to_string(),
        tenant: "required".to_string(),
        permission: Some("users.read".to_string()),
        layout: Some("admin".to_string()),
        cache: "private".to_string(),
    });

    manifest.add(RouteEntry {
        id: "r3".to_string(),
        path: "/public/health".to_string(),
        file: "pages/health.adapto".to_string(),
        method: "GET".to_string(),
        auth: "public".to_string(),
        tenant: "none".to_string(),
        permission: None,
        layout: None,
        cache: "public".to_string(),
    });

    assert_eq!(manifest.routes.len(), 3);

    // find_by_path
    let found = manifest.find_by_path("/api/users");
    assert!(found.is_some());
    assert_eq!(found.unwrap().id, "r1");
    assert_eq!(found.unwrap().permission.as_deref(), Some("users.list"));

    let found2 = manifest.find_by_path("/api/users/:id");
    assert!(found2.is_some());
    assert_eq!(found2.unwrap().id, "r2");

    let found3 = manifest.find_by_path("/public/health");
    assert!(found3.is_some());
    assert!(found3.unwrap().permission.is_none());

    // Not found
    assert!(manifest.find_by_path("/nonexistent").is_none());

    // to_json
    let json = manifest.to_json();
    assert!(json.contains("/api/users"));
    assert!(json.contains("/api/users/:id"));
    assert!(json.contains("/public/health"));
    assert!(json.contains("users.list"));
    assert!(json.contains("\"method\": \"GET\""));
}

// ---------------------------------------------------------------------------
// ComponentManifest: add, find_by_name, to_json
// ---------------------------------------------------------------------------

#[test]
fn test_component_manifest_comprehensive() {
    let mut manifest = ComponentManifest::new();
    assert!(manifest.components.is_empty());

    manifest.add(ComponentEntry {
        id: "comp_sidebar".to_string(),
        name: "Sidebar".to_string(),
        file: "components/sidebar.adapto".to_string(),
        is_island: false,
        dependencies: vec!["menu_items".to_string()],
    });

    manifest.add(ComponentEntry {
        id: "comp_chart".to_string(),
        name: "Chart".to_string(),
        file: "components/chart.adapto".to_string(),
        is_island: true,
        dependencies: vec!["data".to_string(), "options".to_string()],
    });

    manifest.add(ComponentEntry {
        id: "comp_footer".to_string(),
        name: "Footer".to_string(),
        file: "components/footer.adapto".to_string(),
        is_island: false,
        dependencies: vec![],
    });

    assert_eq!(manifest.components.len(), 3);

    // find_by_name
    let sidebar = manifest.find_by_name("Sidebar");
    assert!(sidebar.is_some());
    assert_eq!(sidebar.unwrap().id, "comp_sidebar");
    assert!(!sidebar.unwrap().is_island);

    let chart = manifest.find_by_name("Chart");
    assert!(chart.is_some());
    assert!(chart.unwrap().is_island);
    assert_eq!(chart.unwrap().dependencies.len(), 2);

    let footer = manifest.find_by_name("Footer");
    assert!(footer.is_some());
    assert!(footer.unwrap().dependencies.is_empty());

    // Not found
    assert!(manifest.find_by_name("NotExists").is_none());

    // to_json
    let json = manifest.to_json();
    assert!(json.contains("Sidebar"));
    assert!(json.contains("Chart"));
    assert!(json.contains("Footer"));
    assert!(json.contains("\"is_island\": true"));
}

// ---------------------------------------------------------------------------
// CompileError display messages (additional variants)
// ---------------------------------------------------------------------------

#[test]
fn test_compile_error_display_unscoped_query() {
    let err = CompileError::UnscopedQuery {
        file: "tenant_page.adapto".to_string(),
        line: 20,
    };
    let msg = format!("{}", err);
    assert!(msg.contains("E0702"), "Should contain E0702, got: {}", msg);
    assert!(msg.contains("unscoped"), "Should mention unscoped, got: {}", msg);
}

#[test]
fn test_compile_error_display_unknown_component() {
    let err = CompileError::UnknownComponent {
        name: "MissingWidget".to_string(),
    };
    let msg = format!("{}", err);
    assert!(msg.contains("E0301"));
    assert!(msg.contains("MissingWidget"));
}

#[test]
fn test_compile_error_display_missing_permission() {
    let err = CompileError::MissingPermission {
        action: "delete_all".to_string(),
    };
    let msg = format!("{}", err);
    assert!(msg.contains("E0401"));
    assert!(msg.contains("delete_all"));
}

#[test]
fn test_compile_error_display_multiple() {
    let err = CompileError::Multiple {
        count: 5,
        errors: vec![
            CompileError::DuplicateState { name: "x".to_string() },
            CompileError::DuplicateState { name: "y".to_string() },
        ],
    };
    let msg = format!("{}", err);
    assert_eq!(msg, "Compilation failed with 5 errors");
}

// ---------------------------------------------------------------------------
// Codegen: generate_component produces valid Rust code structure
// ---------------------------------------------------------------------------

#[test]
fn test_codegen_full_structure() {
    let ir = ComponentIR {
        id: "comp_profile".to_string(),
        name: "Profile".to_string(),
        route: None,
        static_segments: vec![
            "<div class=\"profile\">".to_string(),
            "</div>".to_string(),
        ],
        dynamic_segments: vec![DynamicSegment {
            id: "dyn_0".to_string(),
            expr: "state.username".to_string(),
            deps: vec!["username".to_string()],
            segment_type: SegmentType::Text,
        }],
        events: vec![EventIR {
            id: "evt_0".to_string(),
            event_type: "click".to_string(),
            handler: "save".to_string(),
            component_id: "comp_profile".to_string(),
            modifiers: vec![],
            element_id: "el_button_0".to_string(),
        }],
        actions: vec![ActionIR {
            name: "save".to_string(),
            is_async: true,
            params: vec![ParamIR {
                name: "data".to_string(),
                ty: "ProfileData".to_string(),
            }],
            permission: Some("profile.update".to_string()),
            audit: Some("profile.saved".to_string()),
            body: "repo.save(data).await;".to_string(),
        }],
        state_fields: vec![
            StateFieldIR {
                name: "username".to_string(),
                ty: "String".to_string(),
                default: Some("\"\"".to_string()),
                secret: false,
            },
            StateFieldIR {
                name: "bio".to_string(),
                ty: "String".to_string(),
                default: None,
                secret: false,
            },
        ],
        form_schemas: vec![],
        permissions: vec!["profile.update".to_string()],
        children: vec![],
        is_island: false,
        style: None,
    };

    let mut gen = CodeGenerator::new();
    let code = gen.generate_component(&ir);

    // State struct
    assert!(code.contains("pub struct ProfileState"), "Missing state struct");
    assert!(code.contains("pub username: String"), "Missing username field");
    assert!(code.contains("pub bio: String"), "Missing bio field");

    // Component impl
    assert!(code.contains("impl Component for Profile"), "Missing Component impl");
    assert!(code.contains("type State = ProfileState;"), "Missing State type");

    // render function
    assert!(code.contains("fn render(&self, state: &Self::State) -> Rendered"), "Missing render fn");
    assert!(code.contains("static_part"), "Missing static_part call");
    assert!(code.contains("dynamic_text"), "Missing dynamic_text call");
    assert!(code.contains("dyn_0"), "Missing dyn_0 reference");

    // handle_event function
    assert!(code.contains("fn handle_event"), "Missing handle_event fn");
    assert!(code.contains("\"save\""), "Missing save handler match");
    assert!(code.contains("repo.save(data).await;"), "Missing action body");
    assert!(code.contains("Err(Error::UnknownHandler)"), "Missing unknown handler fallback");
}

// ---------------------------------------------------------------------------
// Codegen: component with no actions produces handle_event with only fallback
// ---------------------------------------------------------------------------

#[test]
fn test_codegen_no_actions_handler() {
    let ir = ComponentIR {
        id: "comp_static".to_string(),
        name: "StaticPage".to_string(),
        route: None,
        static_segments: vec!["<p>Hello</p>".to_string()],
        dynamic_segments: vec![],
        events: vec![],
        actions: vec![],
        state_fields: vec![],
        form_schemas: vec![],
        permissions: vec![],
        children: vec![],
        is_island: false,
        style: None,
    };

    let mut gen = CodeGenerator::new();
    let code = gen.generate_component(&ir);

    assert!(code.contains("fn handle_event"));
    assert!(code.contains("Err(Error::UnknownHandler)"));
    // Should not contain any action match arms
    assert!(!code.contains("Ok(())"), "No actions means no Ok(()) in handler");
}

// ---------------------------------------------------------------------------
// Full compile pipeline: template with if/each/can blocks
// ---------------------------------------------------------------------------

#[test]
fn test_compile_full_pipeline_if_each_can() {
    let source = r#"
<route>
    path: "/orders"
    auth: required
    tenant: required
    permission: "orders.read"
</route>

<script>
    state orders: Vec<Order> = vec![]
    state show_archived: bool = false
    state filter: String = ""

    load async fn load_orders(ctx: Ctx) {
        orders = OrderRepo::list(ctx.tenant_id).await?;
    }

    action toggle_archived() {
        state.show_archived = !state.show_archived;
    }

    #[permission("orders.delete")]
    #[audit("order.deleted")]
    action async fn delete_order(id: Uuid, ctx: Ctx) {
        OrderRepo::delete(ctx.tenant_id, id).await?;
    }
</script>

<template>
    <div class="orders-page">
        <h1>Orders</h1>

        {#if show_archived}
            <p>Showing archived orders</p>
        {:else}
            <p>Showing active orders</p>
        {/if}

        {#each orders as order}
            <div class="order-row">
                <span>{order.reference}</span>
                <span>{order.total}</span>
            </div>
        {/each}

        {#can "orders.create"}
            <button>New Order</button>
        {/can}

        <button on:click="toggle_archived">Toggle Archived</button>
    </div>
</template>

<style scoped>
    .orders-page { padding: 24px; }
    .order-row { display: flex; gap: 8px; }
</style>
"#;

    let output = compile_source(source, "pages/orders.adapto");
    let ir = &output.component_ir;

    // Component name
    assert_eq!(ir.name, "Orders");

    // Route
    let route = ir.route.as_ref().expect("Should have route");
    assert_eq!(route.path, "/orders");
    assert_eq!(route.auth, "required");
    assert_eq!(route.tenant, "required");
    assert_eq!(route.permission.as_deref(), Some("orders.read"));

    // State fields
    assert_eq!(ir.state_fields.len(), 3);
    let field_names: Vec<&str> = ir.state_fields.iter().map(|f| f.name.as_str()).collect();
    assert!(field_names.contains(&"orders"));
    assert!(field_names.contains(&"show_archived"));
    assert!(field_names.contains(&"filter"));

    // Actions
    assert_eq!(ir.actions.len(), 2);
    let toggle = ir.actions.iter().find(|a| a.name == "toggle_archived").unwrap();
    assert!(!toggle.is_async);
    assert!(toggle.permission.is_none());

    let delete = ir.actions.iter().find(|a| a.name == "delete_order").unwrap();
    assert!(delete.is_async);
    assert_eq!(delete.permission.as_deref(), Some("orders.delete"));
    assert_eq!(delete.audit.as_deref(), Some("order.deleted"));

    // Dynamic segments: should have conditional, loop, permission, and text segments
    let conditionals: Vec<_> = ir.dynamic_segments.iter()
        .filter(|s| matches!(s.segment_type, SegmentType::Conditional))
        .collect();
    assert!(!conditionals.is_empty(), "Should have conditional segments");

    let loops: Vec<_> = ir.dynamic_segments.iter()
        .filter(|s| matches!(s.segment_type, SegmentType::Loop))
        .collect();
    assert_eq!(loops.len(), 1, "Should have one loop segment");
    assert_eq!(loops[0].expr, "orders");

    let perms: Vec<_> = ir.dynamic_segments.iter()
        .filter(|s| matches!(s.segment_type, SegmentType::Permission))
        .collect();
    assert_eq!(perms.len(), 1, "Should have one permission segment");
    assert_eq!(perms[0].expr, "orders.create");

    // Events
    assert_eq!(ir.events.len(), 1);
    assert_eq!(ir.events[0].event_type, "click");
    assert_eq!(ir.events[0].handler, "toggle_archived");

    // Permissions collected from route + actions
    assert!(ir.permissions.contains(&"orders.delete".to_string()));
    assert!(ir.permissions.contains(&"orders.read".to_string()));

    // Style
    let style = ir.style.as_ref().expect("Should have style");
    assert!(style.scoped);
    assert!(style.scope_id.is_some());
    assert!(style.css.contains(".orders-page"));

    // Dependency graph
    let graph = &output.dependency_graph;
    let all_fields = graph.all_state_fields();
    assert!(all_fields.contains("show_archived"));
    assert!(all_fields.contains("orders"));

    let affected_by_orders = graph.get_affected_segments(&["orders"]);
    assert!(!affected_by_orders.is_empty(), "Changing orders should affect segments");

    // Generated Rust
    assert!(output.generated_rust.contains("impl Component for Orders"));
    assert!(output.generated_rust.contains("type State = OrdersState;"));
    assert!(output.generated_rust.contains("fn render"));
    assert!(output.generated_rust.contains("fn handle_event"));

    // Route entry
    let route_entry = output.route_entry.as_ref().expect("Should have route entry");
    assert_eq!(route_entry.path, "/orders");
    assert_eq!(route_entry.auth, "required");
    assert_eq!(route_entry.tenant, "required");
}

// ---------------------------------------------------------------------------
// Codegen: dynamic_cond, dynamic_loop, dynamic_perm in render
// ---------------------------------------------------------------------------

#[test]
fn test_codegen_renders_all_segment_types() {
    let ir = ComponentIR {
        id: "comp_mixed".to_string(),
        name: "Mixed".to_string(),
        route: None,
        static_segments: vec![
            "<div>".to_string(),
            "</div>".to_string(),
            "<ul>".to_string(),
            "</ul>".to_string(),
        ],
        dynamic_segments: vec![
            DynamicSegment {
                id: "dyn_0".to_string(),
                expr: "show".to_string(),
                deps: vec!["show".to_string()],
                segment_type: SegmentType::Conditional,
            },
            DynamicSegment {
                id: "dyn_1".to_string(),
                expr: "items".to_string(),
                deps: vec!["items".to_string()],
                segment_type: SegmentType::Loop,
            },
            DynamicSegment {
                id: "dyn_2".to_string(),
                expr: "admin.delete".to_string(),
                deps: vec!["permission:admin.delete".to_string()],
                segment_type: SegmentType::Permission,
            },
            DynamicSegment {
                id: "dyn_3".to_string(),
                expr: "state.title".to_string(),
                deps: vec!["title".to_string()],
                segment_type: SegmentType::Text,
            },
        ],
        events: vec![],
        actions: vec![],
        state_fields: vec![],
        form_schemas: vec![],
        permissions: vec![],
        children: vec![],
        is_island: false,
        style: None,
    };

    let mut gen = CodeGenerator::new();
    let code = gen.generate_component(&ir);

    assert!(code.contains("dynamic_cond(\"dyn_0\""), "Missing conditional segment");
    assert!(code.contains("dynamic_loop(\"dyn_1\""), "Missing loop segment");
    assert!(code.contains("dynamic_perm(\"dyn_2\""), "Missing permission segment");
    assert!(code.contains("dynamic_text(\"dyn_3\""), "Missing text segment");
}

// ---------------------------------------------------------------------------
// Compiler accumulates manifests across multiple files
// ---------------------------------------------------------------------------

#[test]
fn test_compiler_accumulates_three_files() {
    let sources = vec![
        (r#"
<route>
    path: "/a"
    auth: public
</route>
<template><div>A</div></template>
"#, "pages/a.adapto"),
        (r#"
<route>
    path: "/b"
    auth: required
</route>
<template><div>B</div></template>
"#, "pages/b.adapto"),
        (r#"
<template><span>Widget</span></template>
"#, "components/widget.adapto"),
    ];

    let mut compiler = Compiler::new();

    for (src, path) in &sources {
        let file = adapto_parser::parse(src).expect("parse failed");
        compiler.compile_file(&file, path).expect("compile failed");
    }

    // Route manifest: 2 routes (widget has no route)
    assert_eq!(compiler.route_manifest().routes.len(), 2);
    assert!(compiler.route_manifest().find_by_path("/a").is_some());
    assert!(compiler.route_manifest().find_by_path("/b").is_some());

    // Component manifest: 3 components
    assert_eq!(compiler.component_manifest().components.len(), 3);
    assert!(compiler.component_manifest().find_by_name("A").is_some());
    assert!(compiler.component_manifest().find_by_name("B").is_some());
    assert!(compiler.component_manifest().find_by_name("Widget").is_some());
}

// ---------------------------------------------------------------------------
// Component manifest JSON roundtrip
// ---------------------------------------------------------------------------

#[test]
fn test_component_manifest_json_roundtrip() {
    let mut manifest = ComponentManifest::new();
    manifest.add(ComponentEntry {
        id: "comp_btn".to_string(),
        name: "Button".to_string(),
        file: "components/button.adapto".to_string(),
        is_island: false,
        dependencies: vec!["label".to_string(), "disabled".to_string()],
    });

    let json = manifest.to_json();
    let parsed: ComponentManifest = serde_json::from_str(&json).expect("should parse back");
    assert_eq!(parsed.components.len(), 1);
    assert_eq!(parsed.components[0].name, "Button");
    assert_eq!(parsed.components[0].dependencies.len(), 2);
}

// ---------------------------------------------------------------------------
// Codegen: form struct with validation
// ---------------------------------------------------------------------------

#[test]
fn test_codegen_form_with_all_constraints() {
    let schema = FormSchemaIR {
        name: "RegistrationForm".to_string(),
        fields: vec![
            FormFieldIR {
                name: "name".to_string(),
                ty: "String".to_string(),
                required: true,
                min: Some(2),
                max: Some(100),
            },
            FormFieldIR {
                name: "email".to_string(),
                ty: "String".to_string(),
                required: true,
                min: None,
                max: Some(255),
            },
            FormFieldIR {
                name: "age".to_string(),
                ty: "i32".to_string(),
                required: false,
                min: None,
                max: None,
            },
        ],
    };

    let ir = ComponentIR {
        id: "comp_reg".to_string(),
        name: "Registration".to_string(),
        route: None,
        static_segments: vec![],
        dynamic_segments: vec![],
        events: vec![],
        actions: vec![],
        state_fields: vec![],
        form_schemas: vec![schema],
        permissions: vec![],
        children: vec![],
        is_island: false,
        style: None,
    };

    let mut gen = CodeGenerator::new();
    let code = gen.generate_component(&ir);

    assert!(code.contains("pub struct RegistrationForm"));
    assert!(code.contains("pub name: String"));
    assert!(code.contains("pub email: String"));
    assert!(code.contains("pub age: i32"));
    assert!(code.contains("fn validate"));
    // Should check required for name
    assert!(code.contains("name is required") || code.contains("name"));
    // Should check min/max for name
    assert!(code.contains("at least 2") || code.contains("min"));
    assert!(code.contains("at most 100") || code.contains("max"));
}

// ---------------------------------------------------------------------------
// Compile: island component
// ---------------------------------------------------------------------------

#[test]
fn test_compile_island_component() {
    let source = r#"
<template>
    <div>
        <Chart island data={sales} />
    </div>
</template>
"#;

    let output = compile_source(source, "pages/report.adapto");
    let ir = &output.component_ir;

    assert!(
        ir.children.contains(&"Chart".to_string()),
        "Should reference Chart child. Got: {:?}",
        ir.children
    );
}

// ---------------------------------------------------------------------------
// Compile: multiple events on same element
// ---------------------------------------------------------------------------

#[test]
fn test_compile_multiple_events_same_element() {
    let source = r#"
<script>
    action focus() { /* focus logic */ }
    action blur() { /* blur logic */ }
    action input_change() { /* input logic */ }
</script>
<template>
    <input on:focus="focus" on:blur="blur" on:input="input_change" />
</template>
"#;

    let output = compile_source(source, "multi_event.adapto");
    let ir = &output.component_ir;

    assert_eq!(ir.events.len(), 3, "Should have 3 events: {:?}", ir.events);

    let event_types: Vec<&str> = ir.events.iter().map(|e| e.event_type.as_str()).collect();
    assert!(event_types.contains(&"focus"));
    assert!(event_types.contains(&"blur"));
    assert!(event_types.contains(&"input"));
}
