use serde_json::Value;
use std::collections::{HashMap, HashSet};

/// Server-side state store for a single live session.
///
/// Tracks which keys have been modified since the last `clear_dirty` call,
/// enabling efficient diff-based updates over the wire.
#[derive(Debug, Clone, Default)]
pub struct StateStore {
    values: HashMap<String, Value>,
    dirty: HashSet<String>,
}

impl StateStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.values.get(key)
    }

    pub fn set(&mut self, key: &str, value: Value) {
        self.values.insert(key.to_string(), value);
        self.dirty.insert(key.to_string());
    }

    pub fn get_dirty(&self) -> &HashSet<String> {
        &self.dirty
    }

    pub fn clear_dirty(&mut self) {
        self.dirty.clear();
    }

    pub fn is_dirty(&self, key: &str) -> bool {
        self.dirty.contains(key)
    }

    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.values.keys()
    }

    pub fn to_map(&self) -> &HashMap<String, Value> {
        &self.values
    }

    /// Merge another map of values into this store, marking all merged keys
    /// as dirty.
    pub fn merge(&mut self, other: HashMap<String, Value>) {
        for (key, value) in other {
            self.dirty.insert(key.clone());
            self.values.insert(key, value);
        }
    }
}
