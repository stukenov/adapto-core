use adapto_store::{
    AdaptoStore, Filter, Query, SortDir, StoreError, Update,
};
use serde_json::json;

// ===================================================================
// Helpers
// ===================================================================

fn mem_store() -> AdaptoStore {
    AdaptoStore::open(None).unwrap()
}

/// Minimal temp directory helper (avoids pulling in the `tempdir` crate).
mod tempdir {
    use std::path::PathBuf;

    pub struct TempDir(PathBuf);

    impl TempDir {
        pub fn new() -> Self {
            let id = uuid::Uuid::new_v4();
            let path = std::env::temp_dir().join(format!("adapto_store_test_{id}"));
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        pub fn path(&self) -> &str {
            self.0.to_str().unwrap()
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
}

// ===================================================================
// 1. Basic CRUD (8 tests)
// ===================================================================

#[test]
fn insert_and_find_by_id() {
    let store = mem_store();
    let col = store.collection("users");
    let id = col.insert(json!({"name": "Alice"})).unwrap();
    let doc = col.find_by_id(&id).unwrap().unwrap();
    assert_eq!(doc.get_str("name"), Some("Alice"));
}

#[test]
fn insert_returns_unique_ids() {
    let store = mem_store();
    let col = store.collection("items");
    let id1 = col.insert(json!({"x": 1})).unwrap();
    let id2 = col.insert(json!({"x": 2})).unwrap();
    assert_ne!(id1, id2);
}

#[test]
fn update_by_id_modifies_document() {
    let store = mem_store();
    let col = store.collection("users");
    let id = col.insert(json!({"name": "Alice", "age": 25})).unwrap();

    let changed = col
        .update_by_id(&id, Update::Set(vec![("age".into(), json!(26))]))
        .unwrap();
    assert!(changed);

    let doc = col.find_by_id(&id).unwrap().unwrap();
    assert_eq!(doc.get_i64("age"), Some(26));
}

#[test]
fn delete_by_id_removes_document() {
    let store = mem_store();
    let col = store.collection("users");
    let id = col.insert(json!({"name": "Bob"})).unwrap();
    assert!(col.delete_by_id(&id).unwrap());
    assert!(col.find_by_id(&id).unwrap().is_none());
}

#[test]
fn insert_many_and_count() {
    let store = mem_store();
    let col = store.collection("items");
    let docs = vec![json!({"a": 1}), json!({"a": 2}), json!({"a": 3})];
    let ids = col.insert_many(docs).unwrap();
    assert_eq!(ids.len(), 3);
    assert_eq!(col.count_all(), 3);
}

#[test]
fn count_with_filter() {
    let store = mem_store();
    let col = store.collection("items");
    col.insert_many(vec![
        json!({"status": "active"}),
        json!({"status": "active"}),
        json!({"status": "archived"}),
    ])
    .unwrap();

    let count = col.count(Query::eq("status", "active")).unwrap();
    assert_eq!(count, 2);
}

#[test]
fn find_by_id_nonexistent_returns_none() {
    let store = mem_store();
    let col = store.collection("users");
    assert!(col.find_by_id("nonexistent-id").unwrap().is_none());
}

#[test]
fn delete_by_id_nonexistent_returns_false() {
    let store = mem_store();
    let col = store.collection("users");
    assert!(!col.delete_by_id("nonexistent-id").unwrap());
}

// ===================================================================
// 2. Query Operators (10 tests)
// ===================================================================

#[test]
fn query_eq() {
    let store = mem_store();
    let col = store.collection("items");
    col.insert(json!({"color": "red"})).unwrap();
    col.insert(json!({"color": "blue"})).unwrap();

    let results: Vec<_> = col.find(Query::eq("color", "red")).collect();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].get_str("color"), Some("red"));
}

#[test]
fn query_ne() {
    let store = mem_store();
    let col = store.collection("items");
    col.insert(json!({"color": "red"})).unwrap();
    col.insert(json!({"color": "blue"})).unwrap();
    col.insert(json!({"color": "green"})).unwrap();

    let results: Vec<_> = col
        .find(Query::filter(Filter::Ne("color".into(), json!("red"))))
        .collect();
    assert_eq!(results.len(), 2);
}

#[test]
fn query_gt_gte_lt_lte() {
    let store = mem_store();
    let col = store.collection("items");
    for i in 1..=5 {
        col.insert(json!({"val": i})).unwrap();
    }

    let gt3: Vec<_> = col
        .find(Query::filter(Filter::Gt("val".into(), json!(3))))
        .collect();
    assert_eq!(gt3.len(), 2); // 4, 5

    let gte3: Vec<_> = col
        .find(Query::filter(Filter::Gte("val".into(), json!(3))))
        .collect();
    assert_eq!(gte3.len(), 3); // 3, 4, 5

    let lt3: Vec<_> = col
        .find(Query::filter(Filter::Lt("val".into(), json!(3))))
        .collect();
    assert_eq!(lt3.len(), 2); // 1, 2

    let lte3: Vec<_> = col
        .find(Query::filter(Filter::Lte("val".into(), json!(3))))
        .collect();
    assert_eq!(lte3.len(), 3); // 1, 2, 3
}

#[test]
fn query_in() {
    let store = mem_store();
    let col = store.collection("items");
    col.insert(json!({"tag": "a"})).unwrap();
    col.insert(json!({"tag": "b"})).unwrap();
    col.insert(json!({"tag": "c"})).unwrap();

    let results: Vec<_> = col
        .find(Query::filter(Filter::In(
            "tag".into(),
            vec![json!("a"), json!("c")],
        )))
        .collect();
    assert_eq!(results.len(), 2);
}

#[test]
fn query_nin() {
    let store = mem_store();
    let col = store.collection("items");
    col.insert(json!({"tag": "a"})).unwrap();
    col.insert(json!({"tag": "b"})).unwrap();
    col.insert(json!({"tag": "c"})).unwrap();

    let results: Vec<_> = col
        .find(Query::filter(Filter::Nin(
            "tag".into(),
            vec![json!("a"), json!("c")],
        )))
        .collect();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].get_str("tag"), Some("b"));
}

#[test]
fn query_exists() {
    let store = mem_store();
    let col = store.collection("items");
    col.insert(json!({"name": "Alice", "email": "alice@example.com"}))
        .unwrap();
    col.insert(json!({"name": "Bob"})).unwrap();

    let with_email: Vec<_> = col
        .find(Query::filter(Filter::Exists("email".into(), true)))
        .collect();
    assert_eq!(with_email.len(), 1);

    let without_email: Vec<_> = col
        .find(Query::filter(Filter::Exists("email".into(), false)))
        .collect();
    assert_eq!(without_email.len(), 1);
}

#[test]
fn query_regex() {
    let store = mem_store();
    let col = store.collection("items");
    col.insert(json!({"name": "Alice"})).unwrap();
    col.insert(json!({"name": "Alicia"})).unwrap();
    col.insert(json!({"name": "Bob"})).unwrap();

    let results: Vec<_> = col
        .find(Query::filter(Filter::Regex("name".into(), "^Ali".into())))
        .collect();
    assert_eq!(results.len(), 2);
}

#[test]
fn query_contains() {
    let store = mem_store();
    let col = store.collection("items");
    col.insert(json!({"desc": "hello world"})).unwrap();
    col.insert(json!({"desc": "goodbye world"})).unwrap();
    col.insert(json!({"desc": "hello there"})).unwrap();

    let results: Vec<_> = col
        .find(Query::filter(Filter::Contains(
            "desc".into(),
            "hello".into(),
        )))
        .collect();
    assert_eq!(results.len(), 2);
}

#[test]
fn query_and() {
    let store = mem_store();
    let col = store.collection("items");
    col.insert(json!({"color": "red", "size": 10})).unwrap();
    col.insert(json!({"color": "red", "size": 20})).unwrap();
    col.insert(json!({"color": "blue", "size": 10})).unwrap();

    let results: Vec<_> = col
        .find(Query::filter(Filter::And(vec![
            Filter::Eq("color".into(), json!("red")),
            Filter::Gt("size".into(), json!(15)),
        ])))
        .collect();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].get_i64("size"), Some(20));
}

#[test]
fn query_or_and_not() {
    let store = mem_store();
    let col = store.collection("items");
    col.insert(json!({"color": "red"})).unwrap();
    col.insert(json!({"color": "blue"})).unwrap();
    col.insert(json!({"color": "green"})).unwrap();

    // OR: red or blue
    let results: Vec<_> = col
        .find(Query::filter(Filter::Or(vec![
            Filter::Eq("color".into(), json!("red")),
            Filter::Eq("color".into(), json!("blue")),
        ])))
        .collect();
    assert_eq!(results.len(), 2);

    // NOT red => blue + green
    let results: Vec<_> = col
        .find(Query::filter(Filter::Not(Box::new(Filter::Eq(
            "color".into(),
            json!("red"),
        )))))
        .collect();
    assert_eq!(results.len(), 2);
}

// ===================================================================
// 3. Query Features (5 tests)
// ===================================================================

#[test]
fn query_sort_asc() {
    let store = mem_store();
    let col = store.collection("items");
    col.insert(json!({"name": "Charlie"})).unwrap();
    col.insert(json!({"name": "Alice"})).unwrap();
    col.insert(json!({"name": "Bob"})).unwrap();

    let results: Vec<_> = col
        .find(Query::new().sort("name", SortDir::Asc))
        .collect();
    let names: Vec<&str> = results.iter().filter_map(|d| d.get_str("name")).collect();
    assert_eq!(names, vec!["Alice", "Bob", "Charlie"]);
}

#[test]
fn query_sort_desc() {
    let store = mem_store();
    let col = store.collection("items");
    for i in 1..=5 {
        col.insert(json!({"val": i})).unwrap();
    }

    let results: Vec<_> = col
        .find(Query::new().sort("val", SortDir::Desc))
        .collect();
    let vals: Vec<i64> = results.iter().filter_map(|d| d.get_i64("val")).collect();
    assert_eq!(vals, vec![5, 4, 3, 2, 1]);
}

#[test]
fn query_limit() {
    let store = mem_store();
    let col = store.collection("items");
    for i in 0..10 {
        col.insert(json!({"i": i})).unwrap();
    }

    let results: Vec<_> = col.find(Query::new().limit(3)).collect();
    assert_eq!(results.len(), 3);
}

#[test]
fn query_skip() {
    let store = mem_store();
    let col = store.collection("items");
    for i in 1..=5 {
        col.insert(json!({"val": i})).unwrap();
    }

    let results: Vec<_> = col
        .find(Query::new().sort("val", SortDir::Asc).skip(3))
        .collect();
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].get_i64("val"), Some(4));
}

#[test]
fn query_projection() {
    let store = mem_store();
    let col = store.collection("items");
    col.insert(json!({"name": "Alice", "age": 30, "email": "a@b.com"}))
        .unwrap();

    let results: Vec<_> = col
        .find(Query::new().project(&["name", "age"]))
        .collect();
    assert_eq!(results.len(), 1);
    let data = &results[0].data;
    assert!(data.get("name").is_some());
    assert!(data.get("age").is_some());
    assert!(data.get("email").is_none()); // projected out
}

#[test]
fn query_combined_sort_skip_limit() {
    let store = mem_store();
    let col = store.collection("items");
    for i in 1..=10 {
        col.insert(json!({"val": i})).unwrap();
    }

    // Sort asc, skip 2, limit 3 => vals 3,4,5
    let results: Vec<_> = col
        .find(Query::new().sort("val", SortDir::Asc).skip(2).limit(3))
        .collect();
    let vals: Vec<i64> = results.iter().filter_map(|d| d.get_i64("val")).collect();
    assert_eq!(vals, vec![3, 4, 5]);
}

// ===================================================================
// 4. Index (6 tests)
// ===================================================================

#[test]
fn create_index_and_list() {
    let store = mem_store();
    let col = store.collection("users");
    col.create_index("email", false).unwrap();

    let indexes = col.indexes();
    assert_eq!(indexes.len(), 1);
    assert_eq!(indexes[0].name, "idx_email");
    assert!(!indexes[0].unique);
}

#[test]
fn unique_index_prevents_duplicates() {
    let store = mem_store();
    let col = store.collection("users");
    col.create_index("email", true).unwrap();

    col.insert(json!({"email": "alice@test.com"})).unwrap();
    let result = col.insert(json!({"email": "alice@test.com"}));
    assert!(matches!(result, Err(StoreError::DuplicateKey { .. })));
}

#[test]
fn unique_index_allows_different_values() {
    let store = mem_store();
    let col = store.collection("users");
    col.create_index("email", true).unwrap();

    col.insert(json!({"email": "alice@test.com"})).unwrap();
    col.insert(json!({"email": "bob@test.com"})).unwrap();
    assert_eq!(col.count_all(), 2);
}

#[test]
fn index_backfills_existing_documents() {
    let store = mem_store();
    let col = store.collection("users");

    col.insert(json!({"name": "Alice"})).unwrap();
    col.insert(json!({"name": "Bob"})).unwrap();

    col.create_index("name", false).unwrap();
    let indexes = col.indexes();
    assert_eq!(indexes[0].entries, 2);
}

#[test]
fn compound_index() {
    let store = mem_store();
    let col = store.collection("users");
    col.create_compound_index(&["last_name", "first_name"], true)
        .unwrap();

    col.insert(json!({"first_name": "Alice", "last_name": "Smith"}))
        .unwrap();
    // Same last+first should fail.
    let result = col.insert(json!({"first_name": "Alice", "last_name": "Smith"}));
    assert!(matches!(result, Err(StoreError::DuplicateKey { .. })));

    // Different first name is fine.
    col.insert(json!({"first_name": "Bob", "last_name": "Smith"}))
        .unwrap();
    assert_eq!(col.count_all(), 2);
}

#[test]
fn drop_index() {
    let store = mem_store();
    let col = store.collection("users");
    col.create_index("email", false).unwrap();
    assert_eq!(col.indexes().len(), 1);

    col.drop_index("idx_email").unwrap();
    assert_eq!(col.indexes().len(), 0);

    // Dropping nonexistent index fails.
    let result = col.drop_index("idx_nonexistent");
    assert!(matches!(result, Err(StoreError::IndexNotFound(_))));
}

// ===================================================================
// 5. Update Operators (5 tests)
// ===================================================================

#[test]
fn update_set() {
    let store = mem_store();
    let col = store.collection("users");
    let id = col
        .insert(json!({"name": "Alice", "age": 25}))
        .unwrap();

    col.update_by_id(
        &id,
        Update::Set(vec![
            ("name".into(), json!("Alicia")),
            ("city".into(), json!("NYC")),
        ]),
    )
    .unwrap();

    let doc = col.find_by_id(&id).unwrap().unwrap();
    assert_eq!(doc.get_str("name"), Some("Alicia"));
    assert_eq!(doc.get_str("city"), Some("NYC"));
}

#[test]
fn update_unset() {
    let store = mem_store();
    let col = store.collection("users");
    let id = col
        .insert(json!({"name": "Alice", "temp": true}))
        .unwrap();

    col.update_by_id(&id, Update::Unset(vec!["temp".into()]))
        .unwrap();

    let doc = col.find_by_id(&id).unwrap().unwrap();
    assert!(doc.get("temp").is_none());
    assert_eq!(doc.get_str("name"), Some("Alice"));
}

#[test]
fn update_inc() {
    let store = mem_store();
    let col = store.collection("counters");
    let id = col.insert(json!({"name": "hits", "count": 10})).unwrap();

    col.update_by_id(&id, Update::Inc("count".into(), 5.0))
        .unwrap();

    let doc = col.find_by_id(&id).unwrap().unwrap();
    assert_eq!(doc.get_i64("count"), Some(15));
}

#[test]
fn update_push_and_pull() {
    let store = mem_store();
    let col = store.collection("users");
    let id = col
        .insert(json!({"name": "Alice", "tags": ["a", "b"]}))
        .unwrap();

    // Push
    col.update_by_id(&id, Update::Push("tags".into(), json!("c")))
        .unwrap();
    let doc = col.find_by_id(&id).unwrap().unwrap();
    assert_eq!(doc.get_array("tags").unwrap().len(), 3);

    // Pull
    col.update_by_id(&id, Update::Pull("tags".into(), json!("b")))
        .unwrap();
    let doc = col.find_by_id(&id).unwrap().unwrap();
    let tags = doc.get_array("tags").unwrap();
    assert_eq!(tags.len(), 2);
    assert!(!tags.contains(&json!("b")));
}

#[test]
fn update_rename() {
    let store = mem_store();
    let col = store.collection("users");
    let id = col
        .insert(json!({"fname": "Alice", "age": 30}))
        .unwrap();

    col.update_by_id(&id, Update::Rename("fname".into(), "first_name".into()))
        .unwrap();

    let doc = col.find_by_id(&id).unwrap().unwrap();
    assert!(doc.get("fname").is_none());
    assert_eq!(doc.get_str("first_name"), Some("Alice"));
}

#[test]
fn update_multi() {
    let store = mem_store();
    let col = store.collection("users");
    let id = col
        .insert(json!({"name": "Alice", "age": 25, "temp": true}))
        .unwrap();

    col.update_by_id(
        &id,
        Update::Multi(vec![
            Update::Set(vec![("name".into(), json!("Alicia"))]),
            Update::Inc("age".into(), 1.0),
            Update::Unset(vec!["temp".into()]),
        ]),
    )
    .unwrap();

    let doc = col.find_by_id(&id).unwrap().unwrap();
    assert_eq!(doc.get_str("name"), Some("Alicia"));
    assert_eq!(doc.get_i64("age"), Some(26));
    assert!(doc.get("temp").is_none());
}

#[test]
fn update_query_multiple_documents() {
    let store = mem_store();
    let col = store.collection("items");
    col.insert_many(vec![
        json!({"status": "draft", "views": 0}),
        json!({"status": "draft", "views": 0}),
        json!({"status": "published", "views": 0}),
    ])
    .unwrap();

    let result = col
        .update(
            Query::eq("status", "draft"),
            Update::Set(vec![("status".into(), json!("published"))]),
        )
        .unwrap();

    assert_eq!(result.matched, 2);
    assert_eq!(result.modified, 2);
}

// ===================================================================
// 6. WAL / Persistence (4 tests)
// ===================================================================

#[test]
fn wal_persistence_write_reopen_verify() {
    let dir = tempdir::TempDir::new();

    // Write data.
    {
        let store = AdaptoStore::open(Some(dir.path())).unwrap();
        let col = store.collection("users");
        col.insert(json!({"name": "Alice", "age": 30})).unwrap();
        col.insert(json!({"name": "Bob", "age": 25})).unwrap();
    }

    // Reopen and verify.
    {
        let store = AdaptoStore::open(Some(dir.path())).unwrap();
        let col = store.collection("users");
        assert_eq!(col.count_all(), 2);

        let alice = col
            .find_one(Query::eq("name", "Alice"))
            .unwrap()
            .unwrap();
        assert_eq!(alice.get_i64("age"), Some(30));
    }
}

#[test]
fn wal_replay_preserves_deletes() {
    let dir = tempdir::TempDir::new();

    let id;
    {
        let store = AdaptoStore::open(Some(dir.path())).unwrap();
        let col = store.collection("items");
        id = col.insert(json!({"x": 1})).unwrap();
        col.insert(json!({"x": 2})).unwrap();
        col.delete_by_id(&id).unwrap();
    }

    {
        let store = AdaptoStore::open(Some(dir.path())).unwrap();
        let col = store.collection("items");
        assert_eq!(col.count_all(), 1);
        assert!(col.find_by_id(&id).unwrap().is_none());
    }
}

#[test]
fn wal_compaction() {
    let dir = tempdir::TempDir::new();

    {
        let store = AdaptoStore::open(Some(dir.path())).unwrap();
        let col = store.collection("items");
        // Insert, then delete most — the WAL accumulates entries for both.
        for i in 0..50 {
            col.insert(json!({"i": i})).unwrap();
        }
        // Delete 40 of them so the live set is small but the WAL has 90 entries.
        let to_delete: Vec<_> = col
            .find(Query::filter(Filter::Lt("i".into(), json!(40))))
            .map(|d| d.id.clone())
            .collect();
        for id in &to_delete {
            col.delete_by_id(id).unwrap();
        }

        let stats_before = store.stats();
        assert!(stats_before.wal_size_bytes > 0);
        assert_eq!(stats_before.total_documents, 10);

        store.compact().unwrap();

        let stats_after = store.stats();
        // After compaction the WAL holds only a snapshot of 10 docs, which is
        // smaller than the 90 insert+delete entries that were there before.
        assert!(
            stats_after.wal_size_bytes < stats_before.wal_size_bytes,
            "expected compacted size {} < original size {}",
            stats_after.wal_size_bytes,
            stats_before.wal_size_bytes
        );
    }

    // Verify data survives compaction + reopen.
    {
        let store = AdaptoStore::open(Some(dir.path())).unwrap();
        let col = store.collection("items");
        assert_eq!(col.count_all(), 10);
    }
}

#[test]
fn in_memory_mode_no_wal() {
    let store = mem_store();
    let stats = store.stats();
    assert!(stats.in_memory);
    assert_eq!(stats.wal_size_bytes, 0);

    // Operations still work.
    let col = store.collection("items");
    col.insert(json!({"x": 1})).unwrap();
    assert_eq!(col.count_all(), 1);
}

// ===================================================================
// 7. Tenant Isolation (4 tests)
// ===================================================================

#[test]
fn tenant_insert_scoped() {
    let store = mem_store();
    let t1 = store.tenant("acme");
    let col = t1.collection("items");
    let id = col.insert(json!({"name": "Widget"})).unwrap();

    let doc = col.find_by_id(&id).unwrap().unwrap();
    assert_eq!(doc.tenant_id.as_deref(), Some("acme"));
}

#[test]
fn tenant_query_only_sees_own_data() {
    let store = mem_store();

    let t1 = store.tenant("acme");
    let t2 = store.tenant("globex");

    t1.collection("items")
        .insert(json!({"name": "Widget A"}))
        .unwrap();
    t1.collection("items")
        .insert(json!({"name": "Widget B"}))
        .unwrap();
    t2.collection("items")
        .insert(json!({"name": "Gadget X"}))
        .unwrap();

    // Tenant 1 sees 2 items.
    assert_eq!(t1.collection("items").count_all(), 2);

    // Tenant 2 sees 1 item.
    assert_eq!(t2.collection("items").count_all(), 1);

    // Unscoped sees all 3.
    assert_eq!(store.collection("items").count_all(), 3);
}

#[test]
fn tenant_cross_tenant_find_by_id_blocked() {
    let store = mem_store();

    let t1 = store.tenant("acme");
    let t2 = store.tenant("globex");

    let id = t1
        .collection("items")
        .insert(json!({"name": "Secret"}))
        .unwrap();

    // Tenant 2 cannot see tenant 1's document by ID.
    assert!(t2.collection("items").find_by_id(&id).unwrap().is_none());
}

#[test]
fn tenant_delete_scoped() {
    let store = mem_store();

    let t1 = store.tenant("acme");
    let t2 = store.tenant("globex");

    let id_t1 = t1
        .collection("items")
        .insert(json!({"name": "Acme Item"}))
        .unwrap();
    t2.collection("items")
        .insert(json!({"name": "Globex Item"}))
        .unwrap();

    // Tenant 2 cannot delete tenant 1's document.
    assert!(!t2.collection("items").delete_by_id(&id_t1).unwrap());

    // Tenant 1 can delete its own.
    assert!(t1.collection("items").delete_by_id(&id_t1).unwrap());

    // Globex item still exists.
    assert_eq!(t2.collection("items").count_all(), 1);
}

// ===================================================================
// 8. Edge Cases (5 tests)
// ===================================================================

#[test]
fn empty_collection_queries() {
    let store = mem_store();
    let col = store.collection("empty");

    assert_eq!(col.count_all(), 0);
    assert_eq!(col.count(Query::new()).unwrap(), 0);
    assert!(col.find_one(Query::new()).unwrap().is_none());

    let results: Vec<_> = col.find(Query::new()).collect();
    assert!(results.is_empty());
}

#[test]
fn dot_notation_field_access() {
    let store = mem_store();
    let col = store.collection("users");
    let id = col
        .insert(json!({
            "name": "Alice",
            "address": {
                "city": "NYC",
                "zip": "10001"
            }
        }))
        .unwrap();

    let doc = col.find_by_id(&id).unwrap().unwrap();
    assert_eq!(doc.get_str("address.city"), Some("NYC"));
    assert_eq!(doc.get_str("address.zip"), Some("10001"));
    assert!(doc.get("address.nonexistent").is_none());
}

#[test]
fn nested_document_query() {
    let store = mem_store();
    let col = store.collection("users");

    col.insert(json!({"name": "Alice", "address": {"city": "NYC"}}))
        .unwrap();
    col.insert(json!({"name": "Bob", "address": {"city": "LA"}}))
        .unwrap();

    let results: Vec<_> = col.find(Query::eq("address.city", "NYC")).collect();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].get_str("name"), Some("Alice"));
}

#[test]
fn drop_collection_and_recreate() {
    let store = mem_store();
    let col = store.collection("temp");
    col.insert(json!({"x": 1})).unwrap();
    assert_eq!(col.count_all(), 1);

    store.drop_collection("temp").unwrap();

    // Collection is gone from the listing.
    assert!(!store.collections().contains(&"temp".to_string()));

    // Re-creating it starts fresh.
    let col = store.collection("temp");
    assert_eq!(col.count_all(), 0);
}

#[test]
fn store_stats() {
    let store = mem_store();

    let col = store.collection("users");
    col.insert_many(vec![json!({"a": 1}), json!({"a": 2})]).unwrap();
    col.create_index("a", false).unwrap();

    store.collection("logs");

    let stats = store.stats();
    assert_eq!(stats.collections, 2);
    assert_eq!(stats.total_documents, 2);
    assert_eq!(stats.total_indexes, 1);
    assert!(stats.in_memory);
}

#[test]
fn list_collections() {
    let store = mem_store();
    store.collection("alpha");
    store.collection("beta");
    store.collection("gamma");

    let mut names = store.collections();
    names.sort();
    assert_eq!(names, vec!["alpha", "beta", "gamma"]);
}

#[test]
fn cursor_iteration() {
    let store = mem_store();
    let col = store.collection("items");
    for i in 0..5 {
        col.insert(json!({"val": i})).unwrap();
    }

    let cursor = col.find(Query::new());
    assert_eq!(cursor.count(), 5);

    let mut count = 0;
    for _doc in col.find(Query::new()) {
        count += 1;
    }
    assert_eq!(count, 5);
}

#[test]
fn find_one_returns_first_match() {
    let store = mem_store();
    let col = store.collection("items");
    col.insert(json!({"status": "active", "name": "A"})).unwrap();
    col.insert(json!({"status": "active", "name": "B"})).unwrap();

    let doc = col.find_one(Query::eq("status", "active")).unwrap();
    assert!(doc.is_some());
}

#[test]
fn wal_persists_indexes() {
    let dir = tempdir::TempDir::new();

    {
        let store = AdaptoStore::open(Some(dir.path())).unwrap();
        let col = store.collection("users");
        col.create_index("email", true).unwrap();
        col.insert(json!({"email": "alice@test.com"})).unwrap();
    }

    {
        let store = AdaptoStore::open(Some(dir.path())).unwrap();
        let col = store.collection("users");

        // The unique index should have been replayed.
        let indexes = col.indexes();
        assert_eq!(indexes.len(), 1);
        assert!(indexes[0].unique);

        // Duplicate should still fail.
        let result = col.insert(json!({"email": "alice@test.com"}));
        assert!(matches!(result, Err(StoreError::DuplicateKey { .. })));
    }
}

// ---------------------------------------------------------------------------
// 14. WAL crash recovery (BUG-001)
// ---------------------------------------------------------------------------

#[test]
fn wal_recovery_truncated_last_line() {
    let dir = tempdir::TempDir::new();

    // Write valid data.
    {
        let store = AdaptoStore::open(Some(dir.path())).unwrap();
        let col = store.collection("users");
        col.insert(json!({"name": "Alice"})).unwrap();
        col.insert(json!({"name": "Bob"})).unwrap();
    }

    // Simulate crash: append garbage to WAL (truncated JSON).
    let wal_path = format!("{}/store.wal", dir.path());
    {
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(&wal_path)
            .unwrap();
        writeln!(f, r#"{{"Insert":{{"collection":"users","doc_id":"crash","data":{{"name":"Trun"#).unwrap();
    }

    // Reopen should recover — skip corrupted last line, keep valid entries.
    let store = AdaptoStore::open(Some(dir.path())).unwrap();
    let col = store.collection("users");
    assert_eq!(col.count_all(), 2);
}

#[test]
fn wal_recovery_trailing_garbage_bytes() {
    let dir = tempdir::TempDir::new();

    {
        let store = AdaptoStore::open(Some(dir.path())).unwrap();
        let col = store.collection("items");
        col.insert(json!({"x": 1})).unwrap();
        col.insert(json!({"x": 2})).unwrap();
        col.insert(json!({"x": 3})).unwrap();
    }

    // Append invalid non-JSON line.
    let wal_path = format!("{}/store.wal", dir.path());
    {
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(&wal_path)
            .unwrap();
        writeln!(f, "not valid json at all").unwrap();
    }

    let store = AdaptoStore::open(Some(dir.path())).unwrap();
    let col = store.collection("items");
    assert_eq!(col.count_all(), 3);
}

#[test]
fn wal_recovery_empty_trailing_line() {
    let dir = tempdir::TempDir::new();

    {
        let store = AdaptoStore::open(Some(dir.path())).unwrap();
        let col = store.collection("data");
        col.insert(json!({"v": 42})).unwrap();
    }

    // Empty lines at end should be harmless.
    let wal_path = format!("{}/store.wal", dir.path());
    {
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(&wal_path)
            .unwrap();
        writeln!(f).unwrap();
        writeln!(f).unwrap();
    }

    let store = AdaptoStore::open(Some(dir.path())).unwrap();
    assert_eq!(store.collection("data").count_all(), 1);
}

#[test]
fn wal_mid_file_corruption_still_errors() {
    let dir = tempdir::TempDir::new();

    {
        let store = AdaptoStore::open(Some(dir.path())).unwrap();
        let col = store.collection("test");
        col.insert(json!({"a": 1})).unwrap();
        col.insert(json!({"a": 2})).unwrap();
        col.insert(json!({"a": 3})).unwrap();
    }

    // Insert garbage in the MIDDLE (not at end).
    let wal_path = format!("{}/store.wal", dir.path());
    let content = std::fs::read_to_string(&wal_path).unwrap();
    let lines: Vec<&str> = content.lines().collect();
    assert!(lines.len() >= 3);
    let mut corrupted = String::new();
    corrupted.push_str(lines[0]);
    corrupted.push('\n');
    corrupted.push_str("CORRUPTED LINE IN MIDDLE\n");
    corrupted.push_str(lines[1]);
    corrupted.push('\n');
    corrupted.push_str(lines[2]);
    corrupted.push('\n');
    std::fs::write(&wal_path, corrupted).unwrap();

    // Mid-file corruption should still error (not silently skip).
    let result = AdaptoStore::open(Some(dir.path()));
    assert!(result.is_err());
}

#[test]
fn wal_recovery_preserves_data_after_truncation() {
    let dir = tempdir::TempDir::new();

    {
        let store = AdaptoStore::open(Some(dir.path())).unwrap();
        let col = store.collection("users");
        col.insert(json!({"name": "Alice", "age": 30})).unwrap();
        col.insert(json!({"name": "Bob", "age": 25})).unwrap();
    }

    // Append truncated entry.
    let wal_path = format!("{}/store.wal", dir.path());
    {
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(&wal_path)
            .unwrap();
        writeln!(f, r#"{{"Insert":{{"collection":"users","doc_"#).unwrap();
    }

    // Recovery should preserve both valid docs.
    let store = AdaptoStore::open(Some(dir.path())).unwrap();
    let col = store.collection("users");
    assert_eq!(col.count_all(), 2);

    // Data should still be correct.
    let doc = col.find_one(Query::eq("name", "Alice")).unwrap().unwrap();
    assert_eq!(doc.get_i64("age"), Some(30));

    // New writes should work after recovery.
    col.insert(json!({"name": "Charlie", "age": 35})).unwrap();
    assert_eq!(col.count_all(), 3);
}
