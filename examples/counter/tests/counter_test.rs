use adapto_compiler::compiler::Compiler;
use adapto_runtime::state::StateStore;
use serde_json::json;

const COUNTER_DSL: &str = r#"
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
  <div>
    <p>Count: {count}</p>
    <button on:click="increment">+</button>
  </div>
</template>
"#;

#[test]
fn parse_counter() {
    let ast = adapto_parser::parse(COUNTER_DSL).unwrap();
    assert!(ast.route.is_some());
    assert!(ast.script.is_some());
    assert!(ast.template.is_some());

    let route = ast.route.unwrap();
    assert_eq!(route.path.as_deref(), Some("/counter"));

    let script = ast.script.unwrap();
    assert_eq!(script.states.len(), 1);
    assert_eq!(script.states[0].name, "count");
    assert_eq!(script.actions.len(), 1);
    assert_eq!(script.actions[0].name, "increment");
}

#[test]
fn compile_counter() {
    let ast = adapto_parser::parse(COUNTER_DSL).unwrap();
    let mut compiler = Compiler::new();
    let output = compiler
        .compile_file(&ast, "counter.adapto")
        .unwrap();

    assert!(!output.component_ir.static_segments.is_empty());
    assert!(!output.component_ir.dynamic_segments.is_empty());
    assert!(!output.component_ir.events.is_empty());
    assert!(!output.generated_rust.is_empty());
}

#[test]
fn counter_state_dirty_tracking() {
    let mut state = StateStore::new();
    state.set("count", json!(0));
    state.clear_dirty();

    assert!(!state.is_dirty("count"));

    state.set("count", json!(1));
    assert!(state.is_dirty("count"));
}

#[test]
fn counter_dependency_graph() {
    let ast = adapto_parser::parse(COUNTER_DSL).unwrap();
    let mut compiler = Compiler::new();
    let output = compiler
        .compile_file(&ast, "counter.adapto")
        .unwrap();

    let affected = output.dependency_graph.get_affected_segments(&["count"]);
    assert!(
        !affected.is_empty(),
        "count should affect at least one segment"
    );
}

#[test]
fn counter_route_manifest() {
    let ast = adapto_parser::parse(COUNTER_DSL).unwrap();
    let mut compiler = Compiler::new();
    let output = compiler
        .compile_file(&ast, "counter.adapto")
        .unwrap();

    assert!(output.route_entry.is_some());
    let route = output.route_entry.unwrap();
    assert_eq!(route.path, "/counter");
}
