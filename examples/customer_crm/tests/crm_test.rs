use adapto_macros::Resource;
use adapto_store::{AdaptoStore, Query, Update};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Resource, Serialize, Deserialize, Debug, Clone)]
#[resource(collection = "customers")]
pub struct Customer {
    #[field(required, max_length = 120)]
    pub name: String,

    #[field(required, unique, format = "email")]
    pub email: String,

    #[field(required)]
    pub company: String,

    #[field(default = "active", one_of = ["active", "lead", "inactive"])]
    pub status: String,

    pub created_at: String,
}

fn test_store() -> AdaptoStore {
    AdaptoStore::open(None).unwrap()
}

fn sample_customer(name: &str, email: &str) -> Customer {
    Customer {
        name: name.into(),
        email: email.into(),
        company: "TestCorp".into(),
        status: "active".into(),
        created_at: "2026-01-01".into(),
    }
}

#[test]
fn collection_name() {
    assert_eq!(Customer::collection_name(), "customers");
}

#[test]
fn field_names() {
    let fields = Customer::field_names();
    assert!(fields.contains(&"name"));
    assert!(fields.contains(&"email"));
    assert!(fields.contains(&"company"));
    assert!(fields.contains(&"status"));
    assert!(fields.contains(&"created_at"));
}

#[test]
fn insert_and_find_by_id() {
    let store = test_store();
    Customer::ensure_indexes(&store);

    let c = sample_customer("Alice", "alice@test.com");
    let id = c.insert_into(&store).unwrap();

    let (found_id, found) = Customer::find_by_id(&store, &id).unwrap();
    assert_eq!(found_id, id);
    assert_eq!(found.name, "Alice");
    assert_eq!(found.email, "alice@test.com");
}

#[test]
fn find_all_with_query() {
    let store = test_store();
    Customer::ensure_indexes(&store);

    sample_customer("Bob", "bob@t.com").insert_into(&store).unwrap();
    sample_customer("Carol", "carol@t.com").insert_into(&store).unwrap();

    let all = Customer::find_all(&store, Query::new());
    assert_eq!(all.len(), 2);

    let active = Customer::find_all(&store, Query::eq("status", "active"));
    assert_eq!(active.len(), 2);
}

#[test]
fn count_resources() {
    let store = test_store();
    sample_customer("D", "d@t.com").insert_into(&store).unwrap();
    sample_customer("E", "e@t.com").insert_into(&store).unwrap();
    assert_eq!(Customer::count(&store), 2);
}

#[test]
fn delete_resource() {
    let store = test_store();
    let id = sample_customer("F", "f@t.com").insert_into(&store).unwrap();
    assert!(Customer::delete(&store, &id));
    assert!(Customer::find_by_id(&store, &id).is_none());
}

#[test]
fn get_field_by_name() {
    let c = sample_customer("Grace", "grace@t.com");
    assert_eq!(c.get_field("name"), Some("Grace".to_string()));
    assert_eq!(c.get_field("email"), Some("grace@t.com".to_string()));
    assert_eq!(c.get_field("unknown"), None);
}

#[test]
fn to_value_roundtrip() {
    let c = sample_customer("Hank", "hank@t.com");
    let val = c.to_value();
    assert_eq!(val["name"], "Hank");
    assert_eq!(val["email"], "hank@t.com");
    assert_eq!(val["status"], "active");
}

#[test]
fn update_status_via_store() {
    let store = test_store();
    Customer::ensure_indexes(&store);

    let c = sample_customer("Ivy", "ivy@t.com");
    let id = c.insert_into(&store).unwrap();

    let col = store.collection("customers");
    col.update_by_id(&id, Update::Set(vec![("status".into(), json!("lead"))])).unwrap();

    let (_, updated) = Customer::find_by_id(&store, &id).unwrap();
    assert_eq!(updated.status, "lead");
}

#[test]
fn unique_email_constraint() {
    let store = test_store();
    Customer::ensure_indexes(&store);

    sample_customer("J1", "same@t.com").insert_into(&store).unwrap();
    let result = sample_customer("J2", "same@t.com").insert_into(&store);
    assert!(result.is_err());
}

#[test]
fn search_filter() {
    let store = test_store();
    sample_customer("Kate Smith", "kate@t.com").insert_into(&store).unwrap();
    sample_customer("Leo Jones", "leo@t.com").insert_into(&store).unwrap();

    let col = store.collection("customers");
    let results: Vec<_> = col.find(Query::new())
        .filter(|d| d.data.to_string().to_lowercase().contains("kate"))
        .collect();
    assert_eq!(results.len(), 1);
}

#[test]
fn persistence_roundtrip() {
    let dir = "/tmp/adapto_crm_test_persist";
    let _ = std::fs::remove_dir_all(dir);

    {
        let store = AdaptoStore::open(Some(dir)).unwrap();
        Customer::ensure_indexes(&store);
        sample_customer("Persist", "persist@t.com").insert_into(&store).unwrap();
    }

    {
        let store = AdaptoStore::open(Some(dir)).unwrap();
        assert_eq!(Customer::count(&store), 1);
        let all = Customer::find_all(&store, Query::new());
        assert_eq!(all[0].1.name, "Persist");
    }

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn seed_data_five_customers() {
    let store = test_store();
    Customer::ensure_indexes(&store);

    let samples = vec![
        json!({"name": "John Appleseed", "email": "john@apple.com", "company": "Apple Inc.", "status": "active", "created_at": "2025-11-14"}),
        json!({"name": "Sarah Connor", "email": "sarah@cyberdyne.com", "company": "Cyberdyne Systems", "status": "lead", "created_at": "2026-01-08"}),
        json!({"name": "Bruce Wayne", "email": "bruce@wayne.com", "company": "Wayne Enterprises", "status": "active", "created_at": "2025-09-22"}),
        json!({"name": "Ellen Ripley", "email": "ripley@weyland.com", "company": "Weyland Corp", "status": "inactive", "created_at": "2025-06-03"}),
        json!({"name": "Tony Stark", "email": "tony@stark.com", "company": "Stark Industries", "status": "active", "created_at": "2026-03-19"}),
    ];

    let col = store.collection("customers");
    col.insert_many(samples).unwrap();

    assert_eq!(Customer::count(&store), 5);

    let active = Customer::find_all(&store, Query::eq("status", "active"));
    assert_eq!(active.len(), 3);

    let leads = Customer::find_all(&store, Query::eq("status", "lead"));
    assert_eq!(leads.len(), 1);
    assert_eq!(leads[0].1.name, "Sarah Connor");
}
