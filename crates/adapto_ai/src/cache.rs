use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
struct CacheEntry {
    value: String,
    created: Instant,
    hits: u64,
}

pub struct ResponseCache {
    entries: Arc<Mutex<HashMap<String, CacheEntry>>>,
    ttl: Duration,
    max_entries: usize,
}

impl ResponseCache {
    pub fn new(ttl: Duration, max_entries: usize) -> Self {
        Self {
            entries: Arc::new(Mutex::new(HashMap::new())),
            ttl,
            max_entries,
        }
    }

    pub fn get(&self, key: &str) -> Option<String> {
        let mut entries = self.entries.lock().unwrap();
        if let Some(entry) = entries.get_mut(key) {
            if entry.created.elapsed() < self.ttl {
                entry.hits += 1;
                return Some(entry.value.clone());
            }
            entries.remove(key);
        }
        None
    }

    pub fn set(&self, key: &str, value: &str) {
        let mut entries = self.entries.lock().unwrap();
        if entries.len() >= self.max_entries && !entries.contains_key(key) {
            if let Some(oldest_key) = entries
                .iter()
                .min_by_key(|(_, e)| e.created)
                .map(|(k, _)| k.clone())
            {
                entries.remove(&oldest_key);
            }
        }
        entries.insert(
            key.into(),
            CacheEntry {
                value: value.into(),
                created: Instant::now(),
                hits: 0,
            },
        );
    }

    pub fn invalidate(&self, key: &str) -> bool {
        self.entries.lock().unwrap().remove(key).is_some()
    }

    pub fn clear(&self) {
        self.entries.lock().unwrap().clear();
    }

    pub fn len(&self) -> usize {
        self.entries.lock().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.lock().unwrap().is_empty()
    }

    pub fn stats(&self) -> CacheStats {
        let entries = self.entries.lock().unwrap();
        let total_hits: u64 = entries.values().map(|e| e.hits).sum();
        CacheStats {
            entries: entries.len(),
            total_hits,
            max_entries: self.max_entries,
            ttl: self.ttl,
        }
    }

    pub fn cleanup_expired(&self) -> usize {
        let mut entries = self.entries.lock().unwrap();
        let before = entries.len();
        entries.retain(|_, e| e.created.elapsed() < self.ttl);
        before - entries.len()
    }

    pub fn cache_key(model: &str, prompt: &str) -> String {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        model.hash(&mut hasher);
        prompt.hash(&mut hasher);
        format!("ai:{}:{:x}", model, hasher.finish())
    }
}

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub entries: usize,
    pub total_hits: u64,
    pub max_entries: usize,
    pub ttl: Duration,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_and_get() {
        let cache = ResponseCache::new(Duration::from_secs(60), 100);
        cache.set("key1", "value1");
        assert_eq!(cache.get("key1"), Some("value1".into()));
    }

    #[test]
    fn miss_returns_none() {
        let cache = ResponseCache::new(Duration::from_secs(60), 100);
        assert!(cache.get("missing").is_none());
    }

    #[test]
    fn expired_returns_none() {
        let cache = ResponseCache::new(Duration::from_millis(1), 100);
        cache.set("key", "val");
        std::thread::sleep(Duration::from_millis(10));
        assert!(cache.get("key").is_none());
    }

    #[test]
    fn invalidate_removes() {
        let cache = ResponseCache::new(Duration::from_secs(60), 100);
        cache.set("k", "v");
        assert!(cache.invalidate("k"));
        assert!(cache.get("k").is_none());
        assert!(!cache.invalidate("k"));
    }

    #[test]
    fn max_entries_evicts_oldest() {
        let cache = ResponseCache::new(Duration::from_secs(60), 2);
        cache.set("a", "1");
        std::thread::sleep(Duration::from_millis(1));
        cache.set("b", "2");
        std::thread::sleep(Duration::from_millis(1));
        cache.set("c", "3");
        assert_eq!(cache.len(), 2);
        assert!(cache.get("a").is_none());
        assert!(cache.get("b").is_some());
        assert!(cache.get("c").is_some());
    }

    #[test]
    fn clear_removes_all() {
        let cache = ResponseCache::new(Duration::from_secs(60), 100);
        cache.set("a", "1");
        cache.set("b", "2");
        cache.clear();
        assert!(cache.is_empty());
    }

    #[test]
    fn stats_tracks_hits() {
        let cache = ResponseCache::new(Duration::from_secs(60), 100);
        cache.set("k", "v");
        cache.get("k");
        cache.get("k");
        let stats = cache.stats();
        assert_eq!(stats.entries, 1);
        assert_eq!(stats.total_hits, 2);
    }

    #[test]
    fn cache_key_deterministic() {
        let k1 = ResponseCache::cache_key("gpt-4", "hello");
        let k2 = ResponseCache::cache_key("gpt-4", "hello");
        let k3 = ResponseCache::cache_key("gpt-4", "world");
        assert_eq!(k1, k2);
        assert_ne!(k1, k3);
    }

    #[test]
    fn cleanup_expired_removes_old() {
        let cache = ResponseCache::new(Duration::from_millis(1), 100);
        cache.set("a", "1");
        cache.set("b", "2");
        std::thread::sleep(Duration::from_millis(10));
        let removed = cache.cleanup_expired();
        assert_eq!(removed, 2);
        assert!(cache.is_empty());
    }
}
