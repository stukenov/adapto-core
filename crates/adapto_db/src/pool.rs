use crate::error::DbError;
use std::future::Future;
use std::pin::Pin;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
pub type DbResult<T> = Result<T, DbError>;

pub trait DatabasePool: Send + Sync {
    fn execute(&self, sql: &str, params: &[serde_json::Value]) -> BoxFuture<'_, DbResult<u64>>;

    fn query_one(
        &self,
        sql: &str,
        params: &[serde_json::Value],
    ) -> BoxFuture<'_, DbResult<serde_json::Value>>;

    fn query_all(
        &self,
        sql: &str,
        params: &[serde_json::Value],
    ) -> BoxFuture<'_, DbResult<Vec<serde_json::Value>>>;

    fn in_transaction<'a>(
        &'a self,
        ops: Vec<(String, Vec<serde_json::Value>)>,
    ) -> BoxFuture<'a, DbResult<()>>;

    fn health_check(&self) -> BoxFuture<'_, DbResult<()>>;
}

pub struct InMemoryPool {
    tables: std::sync::RwLock<
        std::collections::HashMap<String, Vec<serde_json::Map<String, serde_json::Value>>>,
    >,
}

impl InMemoryPool {
    pub fn new() -> Self {
        Self {
            tables: std::sync::RwLock::new(std::collections::HashMap::new()),
        }
    }

    pub fn create_table(&self, name: &str) {
        self.tables
            .write()
            .unwrap()
            .entry(name.into())
            .or_default();
    }

    pub fn insert_row(&self, table: &str, row: serde_json::Map<String, serde_json::Value>) {
        self.tables
            .write()
            .unwrap()
            .entry(table.into())
            .or_default()
            .push(row);
    }

    pub fn table_rows(&self, table: &str) -> Vec<serde_json::Value> {
        self.tables
            .read()
            .unwrap()
            .get(table)
            .map(|rows| rows.iter().map(|r| serde_json::Value::Object(r.clone())).collect())
            .unwrap_or_default()
    }

    pub fn table_count(&self, table: &str) -> usize {
        self.tables
            .read()
            .unwrap()
            .get(table)
            .map(|r| r.len())
            .unwrap_or(0)
    }

    pub fn clear_table(&self, table: &str) {
        if let Some(rows) = self.tables.write().unwrap().get_mut(table) {
            rows.clear();
        }
    }
}

impl Default for InMemoryPool {
    fn default() -> Self {
        Self::new()
    }
}

impl DatabasePool for InMemoryPool {
    fn execute(&self, _sql: &str, _params: &[serde_json::Value]) -> BoxFuture<'_, DbResult<u64>> {
        Box::pin(async { Ok(0) })
    }

    fn query_one(
        &self,
        _sql: &str,
        _params: &[serde_json::Value],
    ) -> BoxFuture<'_, DbResult<serde_json::Value>> {
        Box::pin(async { Err(DbError::NotFound) })
    }

    fn query_all(
        &self,
        _sql: &str,
        _params: &[serde_json::Value],
    ) -> BoxFuture<'_, DbResult<Vec<serde_json::Value>>> {
        Box::pin(async { Ok(Vec::new()) })
    }

    fn in_transaction<'a>(
        &'a self,
        _ops: Vec<(String, Vec<serde_json::Value>)>,
    ) -> BoxFuture<'a, DbResult<()>> {
        Box::pin(async { Ok(()) })
    }

    fn health_check(&self) -> BoxFuture<'_, DbResult<()>> {
        Box::pin(async { Ok(()) })
    }
}
