use adapto_compiler::compiler::Compiler;
use adapto_runtime::state::StateStore;
use serde_json::json;

fn main() {
    println!("=== Adapto Counter Example ===\n");

    // 1. Define counter component in DSL
    let dsl = r#"
<route>
  path: "/counter"
  auth: public
</route>

<script lang="rust">
  state count: i32 = 0

  action increment() {
    count += 1
  }

  action decrement() {
    count -= 1
  }

  action reset() {
    count = 0
  }
</script>

<template>
  <div>
    <h1>Counter</h1>
    <p>Count: {count}</p>
    <button on:click="increment">+</button>
    <button on:click="decrement">-</button>
    <button on:click="reset">Reset</button>
  </div>
</template>
"#;

    // 2. Parse
    println!("Parsing DSL...");
    let ast = adapto_parser::parse(dsl).expect("Parse failed");
    println!(
        "  Route: {:?}",
        ast.route.as_ref().and_then(|r| r.path.as_deref())
    );
    println!(
        "  States: {}",
        ast.script.as_ref().map(|s| s.states.len()).unwrap_or(0)
    );
    println!(
        "  Actions: {}",
        ast.script.as_ref().map(|s| s.actions.len()).unwrap_or(0)
    );
    println!();

    // 3. Compile
    println!("Compiling...");
    let mut compiler = Compiler::new();
    let output = compiler
        .compile_file(&ast, "counter.adapto")
        .expect("Compile failed");
    println!(
        "  Static segments: {}",
        output.component_ir.static_segments.len()
    );
    println!(
        "  Dynamic segments: {}",
        output.component_ir.dynamic_segments.len()
    );
    println!("  Events: {}", output.component_ir.events.len());
    println!();

    // 4. Show generated Rust code
    println!("Generated Rust code:");
    println!("---");
    println!("{}", output.generated_rust);
    println!("---\n");

    // 5. Simulate state changes
    println!("Simulating events...");
    let mut state = StateStore::new();
    state.set("count", json!(0));
    state.clear_dirty();

    // Simulate increment
    let count = state.get("count").and_then(|v| v.as_i64()).unwrap_or(0);
    state.set("count", json!(count + 1));
    println!("  After increment: count = {}", state.get("count").unwrap());

    // Check dirty state
    println!("  Dirty fields: {:?}", state.get_dirty());

    // 6. Show dependency graph
    println!("\nDependency graph:");
    let affected = output.dependency_graph.get_affected_segments(&["count"]);
    println!("  'count' affects segments: {:?}", affected);

    println!("\n=== Counter example complete ===");
}
