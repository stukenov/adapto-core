use adapto_store::{AdaptoStore, Query};
use serde_json::Value;

pub fn temp_store() -> AdaptoStore {
    AdaptoStore::open(None).unwrap()
}

pub struct StoreSeeder<'a> {
    store: &'a AdaptoStore,
    collection: String,
}

impl<'a> StoreSeeder<'a> {
    pub fn new(store: &'a AdaptoStore, collection: &str) -> Self {
        Self {
            store,
            collection: collection.to_string(),
        }
    }

    pub fn insert(&self, doc: Value) -> String {
        self.store.collection(&self.collection).insert(doc).unwrap()
    }

    pub fn insert_many(&self, docs: Vec<Value>) -> Vec<String> {
        self.store
            .collection(&self.collection)
            .insert_many(docs)
            .unwrap()
    }

    pub fn seed_n(&self, n: usize, template: impl Fn(usize) -> Value) -> Vec<String> {
        let docs: Vec<Value> = (0..n).map(&template).collect();
        self.insert_many(docs)
    }

    pub fn with_index(self, field: &str, unique: bool) -> Self {
        self.store
            .collection(&self.collection)
            .create_index(field, unique)
            .ok();
        self
    }

    pub fn count(&self) -> u64 {
        self.store.collection(&self.collection).count_all()
    }
}

pub fn assert_doc_exists(store: &AdaptoStore, collection: &str, id: &str) {
    let doc = store.collection(collection).find_by_id(id).unwrap();
    assert!(
        doc.is_some(),
        "Expected document {:?} to exist in {:?}, but it was not found",
        id,
        collection,
    );
}

pub fn assert_doc_not_exists(store: &AdaptoStore, collection: &str, id: &str) {
    let doc = store.collection(collection).find_by_id(id).unwrap();
    assert!(
        doc.is_none(),
        "Expected document {:?} to NOT exist in {:?}, but it was found",
        id,
        collection,
    );
}

pub fn assert_doc_field(
    store: &AdaptoStore,
    collection: &str,
    id: &str,
    field: &str,
    expected: &Value,
) {
    let doc = store
        .collection(collection)
        .find_by_id(id)
        .unwrap()
        .unwrap_or_else(|| panic!("Document {:?} not found in {:?}", id, collection));
    let actual = doc.get(field);
    assert_eq!(
        actual,
        Some(expected),
        "Document {:?} field {:?}: expected {:?}, got {:?}",
        id,
        field,
        expected,
        actual,
    );
}

pub fn assert_collection_count(store: &AdaptoStore, collection: &str, expected: u64) {
    let actual = store.collection(collection).count_all();
    assert_eq!(
        actual, expected,
        "Collection {:?}: expected {} documents, found {}",
        collection, expected, actual,
    );
}

pub fn assert_query_count(store: &AdaptoStore, collection: &str, query: Query, expected: u64) {
    let actual = store.collection(collection).count(query).unwrap();
    assert_eq!(
        actual, expected,
        "Collection {:?}: expected {} matching documents, found {}",
        collection, expected, actual,
    );
}

pub fn assert_unique_field(store: &AdaptoStore, collection: &str, field: &str, value: &str) {
    let count = store
        .collection(collection)
        .count(Query::eq(field, value))
        .unwrap();
    assert!(
        count <= 1,
        "Expected field {:?}={:?} to be unique in {:?}, but found {} documents",
        field,
        value,
        collection,
        count,
    );
}
