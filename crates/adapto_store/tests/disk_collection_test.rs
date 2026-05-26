use adapto_store::{AdaptoStore, Query};
use serde_json::json;
use tempfile::tempdir;

#[test]
fn disk_collection_bulk_insert_and_find() {
    let dir = tempdir().unwrap();
    let store = AdaptoStore::open(Some(dir.path().to_str().unwrap())).unwrap();

    let col = store.disk_collection("companies").unwrap();

    let docs: Vec<serde_json::Value> = (0..1000)
        .map(|i| {
            json!({
                "bin": format!("{:012}", i),
                "name": format!("Company {i}"),
                "datereg": format!("2024-{:02}-01", (i % 12) + 1),
            })
        })
        .collect();

    let count = col.bulk_insert(docs).unwrap();
    assert_eq!(count, 1000);
    assert_eq!(col.count_all(), 1000);

    col.create_index("bin", true).unwrap();

    // find_one by indexed field
    let doc = col.find_one(Query::eq("bin", "000000000042")).unwrap().unwrap();
    assert_eq!(doc.get_str("name").unwrap(), "Company 42");

    // find_one miss
    let miss = col.find_one(Query::eq("bin", "999999999999")).unwrap();
    assert!(miss.is_none());
}

#[test]
fn disk_collection_full_scan_filter() {
    let dir = tempdir().unwrap();
    let store = AdaptoStore::open(Some(dir.path().to_str().unwrap())).unwrap();
    let col = store.disk_collection("test_scan").unwrap();

    let docs: Vec<serde_json::Value> = (0..100)
        .map(|i| {
            json!({
                "id": i,
                "category": if i % 2 == 0 { "even" } else { "odd" },
            })
        })
        .collect();

    col.bulk_insert(docs).unwrap();

    let results: Vec<_> = col.find(Query::eq("category", "even")).collect();
    assert_eq!(results.len(), 50);
}

#[test]
fn disk_collection_persistence() {
    let dir = tempdir().unwrap();
    let path = dir.path().to_str().unwrap();

    // Phase 1: create and populate
    {
        let store = AdaptoStore::open(Some(path)).unwrap();
        let col = store.disk_collection("persistent").unwrap();
        let docs: Vec<serde_json::Value> = (0..500)
            .map(|i| json!({"key": format!("k{i}"), "val": i}))
            .collect();
        col.bulk_insert(docs).unwrap();
        col.create_index("key", true).unwrap();
    }

    // Phase 2: reopen and verify
    {
        let store = AdaptoStore::open(Some(path)).unwrap();
        let col = store.disk_collection("persistent").unwrap();
        assert_eq!(col.count_all(), 500);
        assert_eq!(col.indexes().len(), 1);
        assert_eq!(col.indexes()[0].entries, 500);

        let doc = col.find_one(Query::eq("key", "k42")).unwrap().unwrap();
        assert_eq!(doc.get_i64("val").unwrap(), 42);
    }
}

#[test]
fn disk_collection_large_dataset() {
    let dir = tempdir().unwrap();
    let store = AdaptoStore::open(Some(dir.path().to_str().unwrap())).unwrap();
    let col = store.disk_collection("large").unwrap();

    let docs: Vec<serde_json::Value> = (0..50_000)
        .map(|i| {
            json!({
                "bin": format!("{:012}", i),
                "name": format!("ТОО Компания {i}"),
                "director": format!("Директор {}", i % 1000),
                "datereg": format!("20{:02}-{:02}-01", 18 + (i % 8), (i % 12) + 1),
            })
        })
        .collect();

    let t = std::time::Instant::now();
    col.bulk_insert(docs).unwrap();
    let insert_ms = t.elapsed().as_millis();
    eprintln!("50K insert: {insert_ms}ms");

    col.create_index("bin", true).unwrap();

    let t = std::time::Instant::now();
    let doc = col.find_one(Query::eq("bin", "000000025000")).unwrap().unwrap();
    let lookup_us = t.elapsed().as_micros();
    eprintln!("Indexed lookup: {lookup_us}μs");
    assert_eq!(doc.get_str("name").unwrap(), "ТОО Компания 25000");

    assert!(lookup_us < 5000, "indexed lookup should be under 5ms");
}
