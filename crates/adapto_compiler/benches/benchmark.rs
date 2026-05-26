use adapto_compiler::compiler::Compiler;
use adapto_parser;
use std::time::Instant;

fn main() {
    println!("=== Adapto Compiler Benchmark Suite ===\n");
    bench_compile_minimal();
    bench_compile_counter();
    bench_compile_full_page();
    bench_compile_dependency_graph();
    bench_compile_codegen_size();
    bench_compile_throughput();
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
  use CustomerForm

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

  #[permission("customers.delete")]
  #[audit("customer.deleted")]
  action async fn delete(id: Uuid, ctx: Ctx) {
    CustomerRepo::delete(ctx.tenant_id, id).await?;
  }

  server fn validate_email(email: String) {
    email.contains('@')
  }

  form CustomerForm {
    name: String min=2 max=120 required
    email: Email required
    phone: Option<String> max=32
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
          {#can "customers.delete"}
            <button on:click="delete(customer.id)">Delete</button>
          {/can}
        </div>
      {/each}
    {/if}
  </div>
</template>

<style scoped>
  .page { padding: 16px; }
  .row { display: flex; gap: 8px; }
  .spinner { text-align: center; color: #888; }
</style>
"#;

fn bench_compile_minimal() {
    let dsl = "<template>\n  <p>Hello</p>\n</template>\n";
    let ast = adapto_parser::parse(dsl).unwrap();
    let n = 10_000;
    let start = Instant::now();
    for _ in 0..n {
        let mut compiler = Compiler::new();
        let _ = compiler.compile_file(&ast, "minimal.adapto").unwrap();
    }
    let elapsed = start.elapsed();
    let per_op = elapsed.as_nanos() / n as u128;
    println!("--- COMPILE MINIMAL ---");
    println!("  {n} compiles: {elapsed:>10.2?}  ({per_op} ns/op, {} ops/sec)",
        1_000_000_000u128 / per_op.max(1));
    println!();
}

fn bench_compile_counter() {
    let ast = adapto_parser::parse(COUNTER_DSL).unwrap();
    let n = 5_000;
    let start = Instant::now();
    for _ in 0..n {
        let mut compiler = Compiler::new();
        let _ = compiler.compile_file(&ast, "counter.adapto").unwrap();
    }
    let elapsed = start.elapsed();
    let per_op = elapsed.as_nanos() / n as u128;
    println!("--- COMPILE COUNTER (route+script+template+style) ---");
    println!("  {n} compiles: {elapsed:>10.2?}  ({per_op} ns/op, {} ops/sec)",
        1_000_000_000u128 / per_op.max(1));
    println!();
}

fn bench_compile_full_page() {
    let ast = adapto_parser::parse(FULL_PAGE_DSL).unwrap();
    let n = 2_000;
    let start = Instant::now();
    for _ in 0..n {
        let mut compiler = Compiler::new();
        let _ = compiler.compile_file(&ast, "customers.adapto").unwrap();
    }
    let elapsed = start.elapsed();
    let per_op = elapsed.as_nanos() / n as u128;
    println!("--- COMPILE FULL PAGE (complex) ---");
    println!("  {n} compiles: {elapsed:>10.2?}  ({per_op} ns/op, {} ops/sec)",
        1_000_000_000u128 / per_op.max(1));
    println!();
}

fn bench_compile_dependency_graph() {
    let ast = adapto_parser::parse(FULL_PAGE_DSL).unwrap();
    let n = 5_000;
    let start = Instant::now();
    for _ in 0..n {
        let mut compiler = Compiler::new();
        let output = compiler.compile_file(&ast, "customers.adapto").unwrap();
        let field_set = output.dependency_graph.all_state_fields();
        let fields: Vec<&str> = field_set.iter().map(|s| s.as_str()).collect();
        let _ = output.dependency_graph.get_affected_segments(&fields);
    }
    let elapsed = start.elapsed();
    let per_op = elapsed.as_nanos() / n as u128;
    println!("--- COMPILE + DEPENDENCY LOOKUP ---");
    println!("  {n} compile+lookup: {elapsed:>10.2?}  ({per_op} ns/op, {} ops/sec)",
        1_000_000_000u128 / per_op.max(1));
    println!();
}

fn bench_compile_codegen_size() {
    let ast = adapto_parser::parse(FULL_PAGE_DSL).unwrap();
    let mut compiler = Compiler::new();
    let output = compiler.compile_file(&ast, "customers.adapto").unwrap();
    println!("--- CODEGEN OUTPUT SIZE ---");
    println!("  Input DSL:       {} bytes", FULL_PAGE_DSL.len());
    println!("  Generated Rust:  {} bytes", output.generated_rust.len());
    println!("  IR actions:      {}", output.component_ir.actions.len());
    println!("  IR state fields: {}", output.component_ir.state_fields.len());
    println!("  Static segments: {}", output.component_ir.static_segments.len());
    println!("  Dynamic segments:{}", output.component_ir.dynamic_segments.len());
    println!("  Events:          {}", output.component_ir.events.len());
    println!();
}

fn bench_compile_throughput() {
    let ast = adapto_parser::parse(FULL_PAGE_DSL).unwrap();
    let n = 1_000;
    let total_bytes = FULL_PAGE_DSL.len() * n;
    let start = Instant::now();
    for _ in 0..n {
        let mut compiler = Compiler::new();
        let _ = compiler.compile_file(&ast, "customers.adapto").unwrap();
    }
    let elapsed = start.elapsed();
    let mb_per_sec = total_bytes as f64 / elapsed.as_secs_f64() / 1_048_576.0;
    println!("--- THROUGHPUT ---");
    println!("  {:.1} MB/s ({n} x {} bytes)", mb_per_sec, FULL_PAGE_DSL.len());
    println!();
}
