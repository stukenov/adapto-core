use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// A stored document with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    /// Auto-generated UUID v4.
    pub id: String,
    /// The JSON document body.
    pub data: Value,
    /// When the document was first inserted.
    pub created_at: DateTime<Utc>,
    /// When the document was last modified.
    pub updated_at: DateTime<Utc>,
    /// Optional tenant scope.
    pub tenant_id: Option<String>,
}

impl Document {
    /// Create a new document wrapping the given JSON value.
    /// An `_id` field inside `data` is ignored — the canonical ID lives in `self.id`.
    pub fn new(data: Value, tenant_id: Option<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            data,
            created_at: now,
            updated_at: now,
            tenant_id,
        }
    }

    /// Access a field by dot-notation path (e.g. `"address.city"`).
    /// Returns `None` if any segment is missing.
    pub fn get(&self, field: &str) -> Option<&Value> {
        resolve_path(&self.data, field)
    }

    /// Convenience: get a field as `&str`.
    pub fn get_str(&self, field: &str) -> Option<&str> {
        self.get(field).and_then(Value::as_str)
    }

    /// Convenience: get a field as `i64`.
    pub fn get_i64(&self, field: &str) -> Option<i64> {
        self.get(field).and_then(Value::as_i64)
    }

    /// Convenience: get a field as `f64`.
    pub fn get_f64(&self, field: &str) -> Option<f64> {
        self.get(field).and_then(Value::as_f64)
    }

    /// Convenience: get a field as `bool`.
    pub fn get_bool(&self, field: &str) -> Option<bool> {
        self.get(field).and_then(Value::as_bool)
    }

    /// Convenience: get a field as an array.
    pub fn get_array(&self, field: &str) -> Option<&Vec<Value>> {
        self.get(field).and_then(Value::as_array)
    }
}

/// Resolve a dot-notation path against a JSON value.
pub fn resolve_path<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    let mut current = value;
    for segment in path.split('.') {
        match current {
            Value::Object(map) => {
                current = map.get(segment)?;
            }
            _ => return None,
        }
    }
    Some(current)
}

/// Mutably resolve a dot-notation path, creating intermediate objects as needed.
/// Returns a mutable reference to the *parent* object and the final key.
pub fn resolve_path_mut<'a>(
    value: &'a mut Value,
    path: &str,
) -> Option<(&'a mut serde_json::Map<String, Value>, String)> {
    let segments: Vec<&str> = path.split('.').collect();
    if segments.is_empty() {
        return None;
    }

    let (parents, last) = segments.split_at(segments.len() - 1);
    let last_key = last[0].to_string();

    let mut current = value;
    for segment in parents {
        if !current.is_object() {
            return None;
        }
        let obj = current.as_object_mut().unwrap();
        if !obj.contains_key(*segment) {
            obj.insert(segment.to_string(), Value::Object(serde_json::Map::new()));
        }
        current = obj.get_mut(*segment).unwrap();
    }

    current.as_object_mut().map(|m| (m, last_key))
}
