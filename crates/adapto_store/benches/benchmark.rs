use adapto_store::{AdaptoStore, Filter, Query, SortDir, Update};
use serde_json::json;
use std::sync::Arc;
use std::time::Instant;

fn main() {
    println!("=== AdaptoStore Benchmark Suite ===\n");

    bench_insert();
    bench_find_by_id();
    bench_query_eq();
    bench_query_range();
    bench_query_complex();
    bench_update();
    bench_delete();
    bench_indexed_vs_scan();
    bench_bulk_insert();
    bench_sort();
    bench_wal_persistence();

    bench_concurrent_writes_same_collection();
    bench_concurrent_writes_diff_collections();
    bench_concurrent_read_write();

    println!("\n=== Done ===");
}

fn bench_concurrent_writes_same_collection() {
    println!("--- CONCURRENT WRITES: SAME COLLECTION ---");
    let store = Arc::new(AdaptoStore::open(None).unwrap());

    let thread_counts = [1, 2, 4, 8];
    let docs_per_thread = 5_000;

    for &threads in &thread_counts {
        let store = Arc::clone(&store);
        // Fresh store each round
        let store = Arc::new(AdaptoStore::open(None).unwrap());

        let start = Instant::now();
        let handles: Vec<_> = (0..threads)
            .map(|t| {
                let store = Arc::clone(&store);
                std::thread::spawn(move || {
                    let col = store.collection("shared");
                    for i in 0..docs_per_thread {
                        col.insert(json!({
                            "thread": t,
                            "idx": i,
                            "data": format!("t{t}-doc{i}")
                        })).unwrap();
                    }
                })
            })
            .collect();

        for h in handles {
            h.join().unwrap();
        }
        let elapsed = start.elapsed();
        let total = threads * docs_per_thread;
        let per_op = elapsed.as_nanos() / total as u128;
        let col = store.collection("shared");
        let count = col.count_all();
        println!("  {threads} threads x {docs_per_thread} docs = {total} total: {elapsed:>10.2?}  ({per_op} ns/op, {} ops/sec) [count={count}]",
            1_000_000_000u128 / per_op.max(1));
    }
    println!();
}

fn bench_concurrent_writes_diff_collections() {
    println!("--- CONCURRENT WRITES: DIFFERENT COLLECTIONS ---");
    let docs_per_thread = 5_000;

    let thread_counts = [1, 2, 4, 8];

    for &threads in &thread_counts {
        let store = Arc::new(AdaptoStore::open(None).unwrap());

        let start = Instant::now();
        let handles: Vec<_> = (0..threads)
            .map(|t| {
                let store = Arc::clone(&store);
                std::thread::spawn(move || {
                    let col = store.collection(&format!("col_{t}"));
                    for i in 0..docs_per_thread {
                        col.insert(json!({
                            "thread": t,
                            "idx": i,
                            "data": format!("t{t}-doc{i}")
                        })).unwrap();
                    }
                })
            })
            .collect();

        for h in handles {
            h.join().unwrap();
        }
        let elapsed = start.elapsed();
        let total = threads * docs_per_thread;
        let per_op = elapsed.as_nanos() / total as u128;
        let stats = store.stats();
        println!("  {threads} threads x {docs_per_thread} docs ({threads} collections): {elapsed:>10.2?}  ({per_op} ns/op, {} ops/sec) [total={}]",
            1_000_000_000u128 / per_op.max(1), stats.total_documents);
    }
    println!();
}

fn bench_concurrent_read_write() {
    println!("--- CONCURRENT READ+WRITE MIX ---");
    let store = Arc::new(AdaptoStore::open(None).unwrap());

    // Pre-populate
    let col = store.collection("mixed");
    let mut ids = Vec::new();
    for i in 0..10_000 {
        ids.push(col.insert(json!({
            "name": format!("User {i}"),
            "age": 20 + (i % 50),
            "score": i
        })).unwrap());
    }

    let readers = 4;
    let writers = 2;
    let ops_per_thread = 5_000;

    let start = Instant::now();
    let mut handles = Vec::new();

    // Reader threads
    for _ in 0..readers {
        let store = Arc::clone(&store);
        let ids = ids.clone();
        handles.push(std::thread::spawn(move || {
            let col = store.collection("mixed");
            let mut found = 0u64;
            for i in 0..ops_per_thread {
                let id = &ids[i % ids.len()];
                if col.find_by_id(id).unwrap().is_some() {
                    found += 1;
                }
            }
            found
        }));
    }

    // Writer threads
    for t in 0..writers {
        let store = Arc::clone(&store);
        handles.push(std::thread::spawn(move || {
            let col = store.collection("mixed");
            for i in 0..ops_per_thread {
                col.insert(json!({
                    "name": format!("Writer{t}-{i}"),
                    "age": 30,
                    "score": i
                })).unwrap();
            }
            ops_per_thread as u64
        }));
    }

    let results: Vec<u64> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    let elapsed = start.elapsed();

    let total_ops = (readers + writers) * ops_per_thread;
    let per_op = elapsed.as_nanos() / total_ops as u128;
    let reader_found: u64 = results[..readers].iter().sum();
    let writer_wrote: u64 = results[readers..].iter().sum();

    println!("  {readers} readers + {writers} writers, {ops_per_thread} ops each:");
    println!("    Total time: {elapsed:>10.2?}  ({per_op} ns/op, {} ops/sec)",
        1_000_000_000u128 / per_op.max(1));
    println!("    Reads completed: {reader_found}");
    println!("    Writes completed: {writer_wrote}");
    let stats = store.stats();
    println!("    Final doc count: {}", stats.total_documents);
    println!();
}

fn bench_insert() {
    let store = AdaptoStore::open(None).unwrap();
    let col = store.collection("users");

    let sizes = [100, 1_000, 10_000, 100_000];
    println!("--- INSERT ---");
    for &n in &sizes {
        let store = AdaptoStore::open(None).unwrap();
        let col = store.collection("users");

        let start = Instant::now();
        for i in 0..n {
            col.insert(json!({
                "name": format!("User {i}"),
                "email": format!("user{i}@test.com"),
                "age": 20 + (i % 50),
                "company": format!("Company {}", i % 100),
                "active": i % 3 != 0,
                "score": i as f64 * 1.5
            })).unwrap();
        }
        let elapsed = start.elapsed();
        let per_op = elapsed.as_nanos() / n as u128;
        println!("  {n:>7} docs: {elapsed:>10.2?}  ({per_op} ns/op, {} ops/sec)",
            1_000_000_000u128 / per_op.max(1));
    }
    println!();
}

fn bench_find_by_id() {
    let store = AdaptoStore::open(None).unwrap();
    let col = store.collection("users");

    let mut ids = Vec::new();
    for i in 0..10_000 {
        ids.push(col.insert(json!({
            "name": format!("User {i}"),
            "age": 20 + (i % 50)
        })).unwrap());
    }

    println!("--- FIND BY ID (10K docs) ---");
    let n = 10_000;
    let start = Instant::now();
    for id in &ids {
        let _ = col.find_by_id(id).unwrap();
    }
    let elapsed = start.elapsed();
    let per_op = elapsed.as_nanos() / n as u128;
    println!("  {n} lookups: {elapsed:>10.2?}  ({per_op} ns/op, {} ops/sec)",
        1_000_000_000u128 / per_op.max(1));
    println!();
}

fn bench_query_eq() {
    let store = AdaptoStore::open(None).unwrap();
    let col = store.collection("users");

    for i in 0..10_000 {
        col.insert(json!({
            "name": format!("User {i}"),
            "status": if i % 2 == 0 { "active" } else { "inactive" },
            "age": 20 + (i % 50)
        })).unwrap();
    }

    println!("--- QUERY EQ (10K docs) ---");
    let n = 1_000;
    let start = Instant::now();
    for _ in 0..n {
        let _ = col.find(Query::eq("status", "active")).count();
    }
    let elapsed = start.elapsed();
    let per_op = elapsed.as_nanos() / n as u128;
    println!("  {n} eq queries: {elapsed:>10.2?}  ({per_op} ns/op, {} ops/sec)",
        1_000_000_000u128 / per_op.max(1));
    println!();
}

fn bench_query_range() {
    let store = AdaptoStore::open(None).unwrap();
    let col = store.collection("users");

    for i in 0..10_000 {
        col.insert(json!({
            "name": format!("User {i}"),
            "age": 20 + (i % 50),
            "score": i as f64 * 1.5
        })).unwrap();
    }

    println!("--- QUERY RANGE (10K docs, age > 40 AND age < 60) ---");
    let n = 1_000;
    let start = Instant::now();
    for _ in 0..n {
        let q = Query::filter(Filter::And(vec![
            Filter::Gt("age".into(), json!(40)),
            Filter::Lt("age".into(), json!(60)),
        ]));
        let _ = col.find(q).count();
    }
    let elapsed = start.elapsed();
    let per_op = elapsed.as_nanos() / n as u128;
    println!("  {n} range queries: {elapsed:>10.2?}  ({per_op} ns/op, {} ops/sec)",
        1_000_000_000u128 / per_op.max(1));
    println!();
}

fn bench_query_complex() {
    let store = AdaptoStore::open(None).unwrap();
    let col = store.collection("users");

    let statuses = ["active", "pending", "inactive"];
    let tags = ["dev", "ops", "data"];
    for i in 0..10_000 {
        let status = statuses[i % 3];
        let tag = tags[i % 3];
        col.insert(json!({
            "name": format!("User {i}"),
            "status": status,
            "age": 20 + (i % 50),
            "company": format!("Company {}", i % 20),
            "tags": tag
        })).unwrap();
    }

    println!("--- COMPLEX QUERY (10K docs, OR + AND + Contains) ---");
    let n = 500;
    let start = Instant::now();
    for _ in 0..n {
        let q = Query::filter(Filter::Or(vec![
            Filter::And(vec![
                Filter::Eq("status".into(), json!("active")),
                Filter::Gte("age".into(), json!(30)),
            ]),
            Filter::Contains("company".into(), "Company 1".into()),
        ])).sort("age", SortDir::Desc).limit(50);
        let _ = col.find(q).count();
    }
    let elapsed = start.elapsed();
    let per_op = elapsed.as_nanos() / n as u128;
    println!("  {n} complex queries: {elapsed:>10.2?}  ({per_op} ns/op, {} ops/sec)",
        1_000_000_000u128 / per_op.max(1));
    println!();
}

fn bench_update() {
    let store = AdaptoStore::open(None).unwrap();
    let col = store.collection("users");

    let mut ids = Vec::new();
    for i in 0..10_000 {
        ids.push(col.insert(json!({
            "name": format!("User {i}"),
            "age": 20 + (i % 50),
            "score": 0
        })).unwrap());
    }

    println!("--- UPDATE BY ID (10K docs) ---");
    let n = 10_000;
    let start = Instant::now();
    for (i, id) in ids.iter().enumerate() {
        col.update_by_id(id, Update::Set(vec![
            ("score".into(), json!(i * 10)),
        ])).unwrap();
    }
    let elapsed = start.elapsed();
    let per_op = elapsed.as_nanos() / n as u128;
    println!("  {n} updates: {elapsed:>10.2?}  ({per_op} ns/op, {} ops/sec)",
        1_000_000_000u128 / per_op.max(1));

    // Update with $inc
    let start = Instant::now();
    for id in &ids {
        col.update_by_id(id, Update::Inc("score".into(), 1.0)).unwrap();
    }
    let elapsed = start.elapsed();
    let per_op = elapsed.as_nanos() / n as u128;
    println!("  {n} $inc ops: {elapsed:>10.2?}  ({per_op} ns/op, {} ops/sec)",
        1_000_000_000u128 / per_op.max(1));
    println!();
}

fn bench_delete() {
    let store = AdaptoStore::open(None).unwrap();
    let col = store.collection("users");

    let mut ids = Vec::new();
    for i in 0..10_000 {
        ids.push(col.insert(json!({
            "name": format!("User {i}"),
            "age": 20 + (i % 50)
        })).unwrap());
    }

    println!("--- DELETE BY ID (10K docs) ---");
    let n = ids.len();
    let start = Instant::now();
    for id in &ids {
        col.delete_by_id(id).unwrap();
    }
    let elapsed = start.elapsed();
    let per_op = elapsed.as_nanos() / n as u128;
    println!("  {n} deletes: {elapsed:>10.2?}  ({per_op} ns/op, {} ops/sec)",
        1_000_000_000u128 / per_op.max(1));
    println!();
}

fn bench_indexed_vs_scan() {
    println!("--- INDEXED vs FULL SCAN (10K docs) ---");

    // Without index — eq
    let store = AdaptoStore::open(None).unwrap();
    let col = store.collection("users");
    for i in 0..10_000 {
        col.insert(json!({
            "name": format!("User {i}"),
            "email": format!("user{i}@test.com"),
            "age": 20 + (i % 50)
        })).unwrap();
    }

    let n = 1_000;
    let start = Instant::now();
    for i in 0..n {
        let _ = col.find(Query::eq("email", format!("user{i}@test.com"))).count();
    }
    let scan_elapsed = start.elapsed();
    let scan_per_op = scan_elapsed.as_nanos() / n as u128;

    // With index — eq
    col.create_index("email", true).unwrap();
    let start = Instant::now();
    for i in 0..n {
        let _ = col.find(Query::eq("email", format!("user{i}@test.com"))).count();
    }
    let idx_elapsed = start.elapsed();
    let idx_per_op = idx_elapsed.as_nanos() / n as u128;

    let speedup = if idx_per_op > 0 { scan_per_op / idx_per_op } else { 0 };
    println!("  Eq query (find 1 in 10K):");
    println!("    Full scan:  {scan_elapsed:>10.2?}  ({scan_per_op} ns/op)");
    println!("    Indexed:    {idx_elapsed:>10.2?}  ({idx_per_op} ns/op)");
    println!("    Speedup:    {speedup}x");

    // Range query with index
    col.create_index("age", false).unwrap();

    let start = Instant::now();
    for _ in 0..n {
        let q = Query::filter(Filter::And(vec![
            Filter::Gte("age".into(), json!(40)),
            Filter::Lt("age".into(), json!(50)),
        ]));
        let _ = col.find(q).count();
    }
    let range_idx_elapsed = start.elapsed();
    let range_idx_per_op = range_idx_elapsed.as_nanos() / n as u128;

    // Drop index to compare
    col.drop_index("idx_age").unwrap();
    let start = Instant::now();
    for _ in 0..n {
        let q = Query::filter(Filter::And(vec![
            Filter::Gte("age".into(), json!(40)),
            Filter::Lt("age".into(), json!(50)),
        ]));
        let _ = col.find(q).count();
    }
    let range_scan_elapsed = start.elapsed();
    let range_scan_per_op = range_scan_elapsed.as_nanos() / n as u128;

    let range_speedup = if range_idx_per_op > 0 { range_scan_per_op / range_idx_per_op } else { 0 };
    println!("  Range query (age 40..50 in 10K):");
    println!("    Full scan:  {range_scan_elapsed:>10.2?}  ({range_scan_per_op} ns/op)");
    println!("    Indexed:    {range_idx_elapsed:>10.2?}  ({range_idx_per_op} ns/op)");
    println!("    Speedup:    {range_speedup}x");

    println!();
}

fn bench_bulk_insert() {
    println!("--- BULK INSERT (insert_many) ---");
    let sizes = [1_000, 10_000, 50_000];

    for &n in &sizes {
        let store = AdaptoStore::open(None).unwrap();
        let col = store.collection("data");

        let docs: Vec<_> = (0..n).map(|i| json!({
            "key": format!("item-{i}"),
            "value": i,
            "nested": {
                "x": i * 2,
                "y": format!("val-{}", i % 100)
            }
        })).collect();

        let start = Instant::now();
        col.insert_many(docs).unwrap();
        let elapsed = start.elapsed();
        let per_op = elapsed.as_nanos() / n as u128;
        println!("  {n:>6} docs: {elapsed:>10.2?}  ({per_op} ns/op, {} ops/sec)",
            1_000_000_000u128 / per_op.max(1));
    }
    println!();
}

fn bench_sort() {
    println!("--- SORT (10K docs) ---");
    let store = AdaptoStore::open(None).unwrap();
    let col = store.collection("users");

    for i in 0..10_000 {
        col.insert(json!({
            "name": format!("User {}", 10_000 - i),
            "age": 20 + (i % 50),
            "score": (i as f64 * 3.14).sin() * 100.0
        })).unwrap();
    }

    let n = 100;
    let start = Instant::now();
    for _ in 0..n {
        let q = Query::new().sort("score", SortDir::Asc);
        let _ = col.find(q).count();
    }
    let elapsed = start.elapsed();
    let per_op = elapsed.as_nanos() / n as u128;
    println!("  {n} sort queries (full 10K): {elapsed:>10.2?}  ({per_op} ns/op)");

    let start = Instant::now();
    for _ in 0..n {
        let q = Query::new().sort("score", SortDir::Desc).limit(10);
        let _ = col.find(q).count();
    }
    let elapsed = start.elapsed();
    let per_op = elapsed.as_nanos() / n as u128;
    println!("  {n} sort+limit(10) queries: {elapsed:>10.2?}  ({per_op} ns/op)");
    println!();
}

fn bench_wal_persistence() {
    println!("--- WAL PERSISTENCE ---");
    let dir = std::env::temp_dir().join("adapto_bench_wal");
    let _ = std::fs::remove_dir_all(&dir);
    let path = dir.to_str().unwrap();

    // Write
    let n = 10_000;
    let start = Instant::now();
    {
        let store = AdaptoStore::open(Some(path)).unwrap();
        let col = store.collection("wal_test");
        for i in 0..n {
            col.insert(json!({"key": format!("k{i}"), "val": i})).unwrap();
        }
    }
    let write_elapsed = start.elapsed();
    let write_per_op = write_elapsed.as_nanos() / n as u128;
    println!("  Write {n} docs (WAL): {write_elapsed:>10.2?}  ({write_per_op} ns/op)");

    // Reopen (replay)
    let start = Instant::now();
    let store = AdaptoStore::open(Some(path)).unwrap();
    let replay_elapsed = start.elapsed();
    let stats = store.stats();
    println!("  Replay {n} docs:       {replay_elapsed:>10.2?}");
    println!("  WAL size:              {} bytes", stats.wal_size_bytes);
    println!("  Doc count after open:  {}", stats.total_documents);

    // Compact
    let start = Instant::now();
    store.compact().unwrap();
    let compact_elapsed = start.elapsed();
    let stats_after = store.stats();
    println!("  Compact:               {compact_elapsed:>10.2?}");
    println!("  WAL size after:        {} bytes", stats_after.wal_size_bytes);

    // Cleanup
    let _ = std::fs::remove_dir_all(&dir);
    println!();
}
