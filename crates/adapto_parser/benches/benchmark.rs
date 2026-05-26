use adapto_parser;
use std::time::Instant;

fn main() {
    println!("=== Adapto Parser Benchmark Suite ===\n");
    bench_parse_minimal();
    bench_parse_counter();
    bench_parse_full_page();
    bench_parse_resource();
    bench_parse_throughput();
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
  state secret password: String

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

const RESOURCE_DSL: &str = r#"
<resource name="Customer" table="customers">
  tenant: required
  primary_key: id

  field id: Uuid readonly
  field name: String required max=120 searchable
  field email: Email required unique
  field phone: String optional
  field status: Enum[active, inactive, blocked] default=active
  field notes: String optional max=2000
  field created_at: DateTime readonly
  field updated_at: DateTime readonly

  permission read: "customers.read"
  permission create: "customers.create"
  permission update: "customers.update"
  permission delete: "customers.delete"
</resource>
"#;

fn bench_parse_minimal() {
    let dsl = "<template>\n  <p>Hello</p>\n</template>\n";
    let n = 10_000;
    let start = Instant::now();
    for _ in 0..n {
        let _ = adapto_parser::parse(dsl).unwrap();
    }
    let elapsed = start.elapsed();
    let per_op = elapsed.as_nanos() / n as u128;
    println!("--- PARSE MINIMAL (template only) ---");
    println!("  {n} parses: {elapsed:>10.2?}  ({per_op} ns/op, {} ops/sec)",
        1_000_000_000u128 / per_op.max(1));
    println!();
}

fn bench_parse_counter() {
    let n = 5_000;
    let start = Instant::now();
    for _ in 0..n {
        let _ = adapto_parser::parse(COUNTER_DSL).unwrap();
    }
    let elapsed = start.elapsed();
    let per_op = elapsed.as_nanos() / n as u128;
    println!("--- PARSE COUNTER (route+script+template+style) ---");
    println!("  {n} parses: {elapsed:>10.2?}  ({per_op} ns/op, {} ops/sec)",
        1_000_000_000u128 / per_op.max(1));
    println!();
}

fn bench_parse_full_page() {
    let n = 2_000;
    let start = Instant::now();
    for _ in 0..n {
        let _ = adapto_parser::parse(FULL_PAGE_DSL).unwrap();
    }
    let elapsed = start.elapsed();
    let per_op = elapsed.as_nanos() / n as u128;
    println!("--- PARSE FULL PAGE (complex: props+state+memo+load+actions+forms+if/each/can) ---");
    println!("  {n} parses: {elapsed:>10.2?}  ({per_op} ns/op, {} ops/sec)",
        1_000_000_000u128 / per_op.max(1));
    println!("  Input size: {} bytes", FULL_PAGE_DSL.len());
    println!();
}

fn bench_parse_resource() {
    let n = 5_000;
    let start = Instant::now();
    for _ in 0..n {
        let _ = adapto_parser::parse(RESOURCE_DSL).unwrap();
    }
    let elapsed = start.elapsed();
    let per_op = elapsed.as_nanos() / n as u128;
    println!("--- PARSE RESOURCE BLOCK ---");
    println!("  {n} parses: {elapsed:>10.2?}  ({per_op} ns/op, {} ops/sec)",
        1_000_000_000u128 / per_op.max(1));
    println!();
}

fn bench_parse_throughput() {
    let total_bytes: usize = FULL_PAGE_DSL.len() * 1_000;
    let n = 1_000;
    let start = Instant::now();
    for _ in 0..n {
        let _ = adapto_parser::parse(FULL_PAGE_DSL).unwrap();
    }
    let elapsed = start.elapsed();
    let mb_per_sec = total_bytes as f64 / elapsed.as_secs_f64() / 1_048_576.0;
    println!("--- THROUGHPUT ---");
    println!("  {:.1} MB/s ({n} x {} bytes)", mb_per_sec, FULL_PAGE_DSL.len());
    println!();
}
