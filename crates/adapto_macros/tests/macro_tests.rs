use adapto_macros::Resource;
use serde::{Deserialize, Serialize};

#[derive(Resource, Serialize, Deserialize, Debug, Clone)]
#[resource(collection = "users")]
pub struct User {
    #[field(required, max_length = 100)]
    pub name: String,

    #[field(required, unique, format = "email")]
    pub email: String,

    #[field(default = "active")]
    pub status: String,
}

#[test]
fn test_collection_name() {
    assert_eq!(User::collection_name(), "users");
}

#[test]
fn test_field_names() {
    assert_eq!(User::field_names(), &["name", "email", "status"]);
}

#[test]
fn test_to_value_and_back() {
    let user = User {
        name: "Alice".into(),
        email: "alice@test.com".into(),
        status: "active".into(),
    };
    let val = user.to_value();
    assert_eq!(val["name"], "Alice");
    assert_eq!(val["email"], "alice@test.com");
}

#[test]
fn test_get_field() {
    let user = User {
        name: "Bob".into(),
        email: "bob@test.com".into(),
        status: "lead".into(),
    };
    assert_eq!(user.get_field("name"), Some("Bob".to_string()));
    assert_eq!(user.get_field("email"), Some("bob@test.com".to_string()));
    assert_eq!(user.get_field("unknown"), None);
}

#[test]
fn test_insert_and_find() {
    let store = adapto_store::AdaptoStore::open(None).unwrap();
    User::ensure_indexes(&store);

    let user = User {
        name: "Charlie".into(),
        email: "charlie@test.com".into(),
        status: "active".into(),
    };
    let id = user.insert_into(&store).unwrap();

    let (found_id, found) = User::find_by_id(&store, &id).unwrap();
    assert_eq!(found_id, id);
    assert_eq!(found.name, "Charlie");
    assert_eq!(found.email, "charlie@test.com");
}

#[test]
fn test_find_all() {
    let store = adapto_store::AdaptoStore::open(None).unwrap();

    let u1 = User {
        name: "A".into(),
        email: "a@t.com".into(),
        status: "active".into(),
    };
    let u2 = User {
        name: "B".into(),
        email: "b@t.com".into(),
        status: "inactive".into(),
    };
    u1.insert_into(&store).unwrap();
    u2.insert_into(&store).unwrap();

    let all = User::find_all(&store, adapto_store::Query::new());
    assert_eq!(all.len(), 2);
}

#[test]
fn test_count() {
    let store = adapto_store::AdaptoStore::open(None).unwrap();
    let u = User {
        name: "X".into(),
        email: "x@t.com".into(),
        status: "active".into(),
    };
    u.insert_into(&store).unwrap();
    assert!(User::count(&store) >= 1);
}

#[test]
fn test_delete() {
    let store = adapto_store::AdaptoStore::open(None).unwrap();
    let u = User {
        name: "Del".into(),
        email: "del@t.com".into(),
        status: "active".into(),
    };
    let id = u.insert_into(&store).unwrap();
    assert!(User::delete(&store, &id));
    assert!(User::find_by_id(&store, &id).is_none());
}
