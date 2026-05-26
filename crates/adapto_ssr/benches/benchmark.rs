use adapto_compiler::compiler::Compiler;
use adapto_parser;
use adapto_runtime::state::StateStore;
use adapto_ssr::renderer::Renderer;
use serde_json::json;
use std::time::Instant;

fn main() {
    println!("=== Adapto SSR Benchmark Suite ===\n");
    bench_render_minimal();
    bench_render_counter();
    bench_render_full_page();
    bench_render_with_layout();
    bench_render_throughput();
    bench_full_pipeline();
    println!("\n=== Done ===");
}

const COUNTER_DSL: &str = r#"
<route>
  path: "/counter"
  method: GET
  auth: public
</route>

<script lang="rust">
  state count: i32 = 0

  action fn increment() {
    count += 1;
  }
</script>

<template>
  <div class="counter">
    <h1>Counter</h1>
    <span>{count}</span>
    <button on:click="increment">+1</button>
  </div>
</template>

<style scoped>
  .counter { text-align: center; }
</style>
"#;

const FULL_PAGE_DSL: &str = r#"
<route>
  path: "/customers"
  method: GET
  auth: required
  tenant: required
  permission: "customers.read"
  layout: "dashboard"
  cache: "no-store"
</route>

<script lang="rust">
  use CustomerRepo

  prop id: Uuid
  state query: String = ""
  state customers: Vec<Customer> = []
  state loading: bool = false

  memo total: usize = customers.len()

  load async fn load(ctx: Ctx) {
    customers = CustomerRepo::for_tenant(ctx.tenant_id).await?;
  }

  action async fn search(ctx: Ctx) {
    loading = true;
    customers = CustomerRepo::search(ctx.tenant_id, query.clone()).await?;
    loading = false;
  }
</script>

<template>
  <div class="page">
    <h1>Customers ({total})</h1>
    <input bind:value="query" on:input.debounce.300="search" placeholder="Search..." />
    {#if loading}
      <div class="spinner">Loading...</div>
    {:else}
      {#each customers as customer (customer.id)}
        <div class="row">
          <span>{customer.name}</span>
          <span>{customer.email}</span>
        </div>
      {/each}
    {/if}
  </div>
</template>

<style scoped>
  .page { padding: 16px; }
  .row { display: flex; gap: 8px; }
</style>
"#;

const LAYOUT: &str = r#"<!DOCTYPE html>
<html lang="en">
<head><meta charset="utf-8"><title>App</title></head>
<body><main>{slot}</main></body>
</html>"#;

fn compile(dsl: &str) -> adapto_compiler::compiler::CompileOutput {
    let ast = adapto_parser::parse(dsl).unwrap();
    let mut compiler = Compiler::new();
    compiler.compile_file(&ast, "bench.adapto").unwrap()
}

fn bench_render_minimal() {
    let dsl = "<template>\n  <p>Hello</p>\n</template>\n";
    let output = compile(dsl);
    let renderer = Renderer::new(b"bench-secret-key");
    let state = StateStore::new();
    let n = 20_000;
    let start = Instant::now();
    for _ in 0..n {
        let _ = renderer.render_component(&output.component_ir, &state).unwrap();
    }
    let elapsed = start.elapsed();
    let per_op = elapsed.as_nanos() / n as u128;
    println!("--- RENDER COMPONENT (minimal) ---");
    println!("  {n} renders: {elapsed:>10.2?}  ({per_op} ns/op, {} ops/sec)",
        1_000_000_000u128 / per_op.max(1));
    println!();
}

fn bench_render_counter() {
    let output = compile(COUNTER_DSL);
    let renderer = Renderer::new(b"bench-secret-key");
    let mut state = StateStore::new();
    state.set("count", json!(42));
    let n = 10_000;
    let start = Instant::now();
    for _ in 0..n {
        let _ = renderer.render_component(&output.component_ir, &state).unwrap();
    }
    let elapsed = start.elapsed();
    let per_op = elapsed.as_nanos() / n as u128;
    println!("--- RENDER COMPONENT (counter with state) ---");
    println!("  {n} renders: {elapsed:>10.2?}  ({per_op} ns/op, {} ops/sec)",
        1_000_000_000u128 / per_op.max(1));
    println!();
}

fn bench_render_full_page() {
    let output = compile(FULL_PAGE_DSL);
    let renderer = Renderer::new(b"bench-secret-key");
    let mut state = StateStore::new();
    state.set("query", json!(""));
    state.set("loading", json!(false));
    state.set("total", json!(5));
    state.set("customers", json!([
        {"name": "Alice Corp", "email": "alice@example.com"},
        {"name": "Bob Inc", "email": "bob@example.com"},
        {"name": "Carol LLC", "email": "carol@example.com"},
        {"name": "Dave Ltd", "email": "dave@example.com"},
        {"name": "Eve SA", "email": "eve@example.com"},
    ]));
    let n = 5_000;
    let start = Instant::now();
    for _ in 0..n {
        let _ = renderer.render_component(&output.component_ir, &state).unwrap();
    }
    let elapsed = start.elapsed();
    let per_op = elapsed.as_nanos() / n as u128;
    println!("--- RENDER COMPONENT (full page with 5 customers) ---");
    println!("  {n} renders: {elapsed:>10.2?}  ({per_op} ns/op, {} ops/sec)",
        1_000_000_000u128 / per_op.max(1));
    println!();
}

fn bench_render_with_layout() {
    let output = compile(COUNTER_DSL);
    let renderer = Renderer::new(b"bench-secret-key");
    let mut state = StateStore::new();
    state.set("count", json!(0));
    let n = 5_000;
    let start = Instant::now();
    for _ in 0..n {
        let _ = renderer.render_page(&output.component_ir, &state, Some(LAYOUT)).unwrap();
    }
    let elapsed = start.elapsed();
    let per_op = elapsed.as_nanos() / n as u128;
    println!("--- RENDER PAGE (with layout) ---");
    println!("  {n} renders: {elapsed:>10.2?}  ({per_op} ns/op, {} ops/sec)",
        1_000_000_000u128 / per_op.max(1));
    println!();
}

fn bench_render_throughput() {
    let output = compile(FULL_PAGE_DSL);
    let renderer = Renderer::new(b"bench-secret-key");
    let mut state = StateStore::new();
    state.set("query", json!(""));
    state.set("loading", json!(false));
    state.set("total", json!(0));
    state.set("customers", json!([]));
    let n = 5_000;
    let html = renderer.render_page(&output.component_ir, &state, Some(LAYOUT)).unwrap();
    let output_size = html.len();
    let start = Instant::now();
    for _ in 0..n {
        let _ = renderer.render_page(&output.component_ir, &state, Some(LAYOUT)).unwrap();
    }
    let elapsed = start.elapsed();
    let total_bytes = output_size * n;
    let mb_per_sec = total_bytes as f64 / elapsed.as_secs_f64() / 1_048_576.0;
    println!("--- RENDER THROUGHPUT ---");
    println!("  {:.1} MB/s ({n} x {} bytes output)", mb_per_sec, output_size);
    println!();
}

fn bench_full_pipeline() {
    let n = 1_000;
    let renderer = Renderer::new(b"bench-secret-key");
    let mut state = StateStore::new();
    state.set("count", json!(0));
    let start = Instant::now();
    for _ in 0..n {
        let ast = adapto_parser::parse(COUNTER_DSL).unwrap();
        let mut compiler = Compiler::new();
        let output = compiler.compile_file(&ast, "counter.adapto").unwrap();
        let _ = renderer.render_page(&output.component_ir, &state, Some(LAYOUT)).unwrap();
    }
    let elapsed = start.elapsed();
    let per_op = elapsed.as_nanos() / n as u128;
    println!("--- FULL PIPELINE (parse -> compile -> render) ---");
    println!("  {n} iterations: {elapsed:>10.2?}  ({per_op} ns/op, {} ops/sec)",
        1_000_000_000u128 / per_op.max(1));
    println!();
}
