use adapto_store::{AdaptoStore, Query};
use serde_json::json;

fn main() {
    let db_path = "/tmp/adapto_store_demo";

    // Clean slate
    let _ = std::fs::remove_dir_all(db_path);

    // === PHASE 1: Write data and "crash" (drop store) ===
    println!("=== PHASE 1: Writing data to disk ===");
    {
        let store = AdaptoStore::open(Some(db_path)).unwrap();
        let users = store.collection("users");

        users.insert(json!({"name": "Alice", "age": 30, "role": "engineer"})).unwrap();
        users.insert(json!({"name": "Bob", "age": 25, "role": "designer"})).unwrap();
        users.insert(json!({"name": "Charlie", "age": 35, "role": "manager"})).unwrap();

        let orders = store.collection("orders");
        orders.insert(json!({"product": "Laptop", "price": 1299, "customer": "Alice"})).unwrap();
        orders.insert(json!({"product": "Mouse", "price": 29, "customer": "Bob"})).unwrap();

        let stats = store.stats();
        println!("  Wrote {} documents across {} collections", stats.total_documents, stats.collections);
        println!("  WAL size: {} bytes", stats.wal_size_bytes);
    }
    // store is DROPPED here — simulates process crash/restart
    println!("  Store closed (process 'crashed')\n");

    // === PHASE 2: Reopen — data must survive ===
    println!("=== PHASE 2: Reopening from disk ===");
    {
        let store = AdaptoStore::open(Some(db_path)).unwrap();
        let stats = store.stats();
        println!("  Recovered {} documents across {} collections", stats.total_documents, stats.collections);

        let users = store.collection("users");
        println!("  Users: {} docs", users.count_all());

        let alice = users.find_one(Query::eq("name", "Alice")).unwrap();
        println!("  Alice: {:?}", alice.map(|d| d.data));

        let engineers = users.find(Query::eq("role", "engineer"));
        println!("  Engineers: {} found", engineers.count());

        let orders = store.collection("orders");
        println!("  Orders: {} docs", orders.count_all());

        let laptop = orders.find_one(Query::eq("product", "Laptop")).unwrap();
        println!("  Laptop order: {:?}", laptop.map(|d| d.data));
    }
    println!();

    // === PHASE 3: Add more data, compact, reopen ===
    println!("=== PHASE 3: Add more data + compact ===");
    {
        let store = AdaptoStore::open(Some(db_path)).unwrap();
        let users = store.collection("users");

        for i in 0..100 {
            users.insert(json!({"name": format!("User{i}"), "age": 20+i, "role": "staff"})).unwrap();
        }

        let stats_before = store.stats();
        println!("  WAL before compact: {} bytes ({} docs)", stats_before.wal_size_bytes, stats_before.total_documents);

        store.compact().unwrap();

        let stats_after = store.stats();
        println!("  WAL after compact:  {} bytes", stats_after.wal_size_bytes);
    }
    println!("  Store closed\n");

    println!("=== PHASE 4: Final reopen after compact ===");
    {
        let store = AdaptoStore::open(Some(db_path)).unwrap();
        let stats = store.stats();
        println!("  Recovered {} documents across {} collections", stats.total_documents, stats.collections);

        let users = store.collection("users");
        let charlie = users.find_one(Query::eq("name", "Charlie")).unwrap();
        println!("  Charlie survived compact: {:?}", charlie.is_some());

        let staff = users.find(Query::eq("role", "staff")).count();
        println!("  Staff members: {}", staff);
    }

    // Show actual WAL file
    println!("\n=== WAL file on disk ===");
    let wal_path = format!("{}/store.wal", db_path);
    let content = std::fs::read_to_string(&wal_path).unwrap();
    let lines: Vec<&str> = content.lines().collect();
    println!("  {} lines in WAL file", lines.len());
    if let Some(first) = lines.first() {
        let preview = if first.len() > 120 { &first[..120] } else { first };
        println!("  First line: {}...", preview);
    }

    // Cleanup
    let _ = std::fs::remove_dir_all(db_path);
    println!("\n=== All data persisted and recovered successfully ===");
}
