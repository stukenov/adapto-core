use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use crate::error::AuthError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    pub user_id: String,
    #[serde(default)]
    pub data: HashMap<String, serde_json::Value>,
    #[serde(skip)]
    pub created_at: Option<Instant>,
    #[serde(skip)]
    pub last_accessed: Option<Instant>,
}

impl SessionData {
    pub fn new(user_id: &str) -> Self {
        let now = Instant::now();
        Self {
            user_id: user_id.into(),
            data: HashMap::new(),
            created_at: Some(now),
            last_accessed: Some(now),
        }
    }

    pub fn set(&mut self, key: &str, value: serde_json::Value) {
        self.data.insert(key.into(), value);
    }

    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.data.get(key)
    }

    pub fn remove(&mut self, key: &str) -> Option<serde_json::Value> {
        self.data.remove(key)
    }

    fn touch(&mut self) {
        self.last_accessed = Some(Instant::now());
    }
}

pub trait SessionStore: Send + Sync {
    fn create(&self, session_id: &str, data: SessionData) -> Result<(), AuthError>;
    fn get(&self, session_id: &str) -> Result<SessionData, AuthError>;
    fn update(&self, session_id: &str, data: SessionData) -> Result<(), AuthError>;
    fn destroy(&self, session_id: &str) -> Result<(), AuthError>;
    fn exists(&self, session_id: &str) -> bool;
    fn cleanup_expired(&self, max_age: Duration) -> usize;
}

#[derive(Debug, Clone)]
pub struct InMemorySessionStore {
    sessions: Arc<RwLock<HashMap<String, SessionData>>>,
}

impl InMemorySessionStore {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn len(&self) -> usize {
        self.sessions.read().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.sessions.read().unwrap().is_empty()
    }

    pub fn session_ids(&self) -> Vec<String> {
        self.sessions.read().unwrap().keys().cloned().collect()
    }
}

impl Default for InMemorySessionStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionStore for InMemorySessionStore {
    fn create(&self, session_id: &str, data: SessionData) -> Result<(), AuthError> {
        self.sessions
            .write()
            .unwrap()
            .insert(session_id.into(), data);
        Ok(())
    }

    fn get(&self, session_id: &str) -> Result<SessionData, AuthError> {
        let mut sessions = self.sessions.write().unwrap();
        match sessions.get_mut(session_id) {
            Some(data) => {
                data.touch();
                Ok(data.clone())
            }
            None => Err(AuthError::SessionNotFound),
        }
    }

    fn update(&self, session_id: &str, data: SessionData) -> Result<(), AuthError> {
        let mut sessions = self.sessions.write().unwrap();
        if sessions.contains_key(session_id) {
            sessions.insert(session_id.into(), data);
            Ok(())
        } else {
            Err(AuthError::SessionNotFound)
        }
    }

    fn destroy(&self, session_id: &str) -> Result<(), AuthError> {
        self.sessions.write().unwrap().remove(session_id);
        Ok(())
    }

    fn exists(&self, session_id: &str) -> bool {
        self.sessions.read().unwrap().contains_key(session_id)
    }

    fn cleanup_expired(&self, max_age: Duration) -> usize {
        let mut sessions = self.sessions.write().unwrap();
        let before = sessions.len();
        sessions.retain(|_, data| {
            data.last_accessed
                .map_or(false, |t| t.elapsed() < max_age)
        });
        before - sessions.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_and_get() {
        let store = InMemorySessionStore::new();
        let data = SessionData::new("user-1");
        store.create("sess-1", data).unwrap();
        let retrieved = store.get("sess-1").unwrap();
        assert_eq!(retrieved.user_id, "user-1");
    }

    #[test]
    fn get_nonexistent_fails() {
        let store = InMemorySessionStore::new();
        assert!(matches!(
            store.get("nope"),
            Err(AuthError::SessionNotFound)
        ));
    }

    #[test]
    fn update_existing() {
        let store = InMemorySessionStore::new();
        let mut data = SessionData::new("user-1");
        store.create("sess-1", data.clone()).unwrap();
        data.set("role", serde_json::json!("admin"));
        store.update("sess-1", data).unwrap();
        let retrieved = store.get("sess-1").unwrap();
        assert_eq!(retrieved.get("role").unwrap(), "admin");
    }

    #[test]
    fn update_nonexistent_fails() {
        let store = InMemorySessionStore::new();
        assert!(matches!(
            store.update("nope", SessionData::new("x")),
            Err(AuthError::SessionNotFound)
        ));
    }

    #[test]
    fn destroy_removes() {
        let store = InMemorySessionStore::new();
        store.create("sess-1", SessionData::new("user-1")).unwrap();
        assert!(store.exists("sess-1"));
        store.destroy("sess-1").unwrap();
        assert!(!store.exists("sess-1"));
    }

    #[test]
    fn cleanup_expired_removes_old() {
        let store = InMemorySessionStore::new();
        let mut old = SessionData::new("user-1");
        old.last_accessed = Some(Instant::now() - Duration::from_secs(3600));
        store.create("old", old).unwrap();
        store
            .create("new", SessionData::new("user-2"))
            .unwrap();
        let removed = store.cleanup_expired(Duration::from_secs(60));
        assert_eq!(removed, 1);
        assert!(!store.exists("old"));
        assert!(store.exists("new"));
    }

    #[test]
    fn len_and_is_empty() {
        let store = InMemorySessionStore::new();
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);
        store.create("s1", SessionData::new("u1")).unwrap();
        assert!(!store.is_empty());
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn session_data_set_get_remove() {
        let mut data = SessionData::new("user-1");
        data.set("theme", serde_json::json!("dark"));
        assert_eq!(data.get("theme").unwrap(), "dark");
        let removed = data.remove("theme");
        assert!(removed.is_some());
        assert!(data.get("theme").is_none());
    }
}
