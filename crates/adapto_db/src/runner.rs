use crate::error::DbError;
use crate::migration::Migration;
use crate::pool::{DatabasePool, DbResult};

pub struct MigrationRunner<'a> {
    pool: &'a dyn DatabasePool,
    migrations: Vec<Migration>,
    applied: Vec<String>,
}

impl<'a> MigrationRunner<'a> {
    pub fn new(pool: &'a dyn DatabasePool) -> Self {
        Self {
            pool,
            migrations: Vec::new(),
            applied: Vec::new(),
        }
    }

    pub fn add(mut self, migration: Migration) -> Self {
        self.migrations.push(migration);
        self
    }

    pub fn add_all(mut self, migrations: Vec<Migration>) -> Self {
        self.migrations.extend(migrations);
        self
    }

    pub fn mark_applied(&mut self, version: &str) {
        self.applied.push(version.into());
    }

    pub fn pending(&self) -> Vec<&Migration> {
        self.migrations
            .iter()
            .filter(|m| !self.applied.contains(&m.version))
            .collect()
    }

    pub fn applied(&self) -> &[String] {
        &self.applied
    }

    pub async fn run_pending(&mut self) -> DbResult<Vec<String>> {
        let pending: Vec<Migration> = self
            .migrations
            .iter()
            .filter(|m| !self.applied.contains(&m.version))
            .cloned()
            .collect();

        let mut applied = Vec::new();
        for migration in &pending {
            self.pool
                .execute(&migration.up, &[])
                .await
                .map_err(|e| DbError::MigrationError(format!("{}: {}", migration.name, e)))?;
            self.applied.push(migration.version.clone());
            applied.push(migration.version.clone());
        }
        Ok(applied)
    }

    pub async fn rollback_last(&mut self) -> DbResult<Option<String>> {
        if let Some(version) = self.applied.last().cloned() {
            if let Some(migration) = self.migrations.iter().find(|m| m.version == version) {
                self.pool
                    .execute(&migration.down, &[])
                    .await
                    .map_err(|e| {
                        DbError::MigrationError(format!("rollback {}: {}", migration.name, e))
                    })?;
                self.applied.pop();
                return Ok(Some(version));
            }
        }
        Ok(None)
    }

    pub fn status(&self) -> Vec<MigrationStatus> {
        self.migrations
            .iter()
            .map(|m| MigrationStatus {
                version: m.version.clone(),
                name: m.name.clone(),
                applied: self.applied.contains(&m.version),
            })
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct MigrationStatus {
    pub version: String,
    pub name: String,
    pub applied: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pool::InMemoryPool;

    fn test_migrations() -> Vec<Migration> {
        vec![
            Migration {
                version: "001".into(),
                name: "create_users".into(),
                up: "CREATE TABLE users (id UUID PRIMARY KEY)".into(),
                down: "DROP TABLE users".into(),
            },
            Migration {
                version: "002".into(),
                name: "create_posts".into(),
                up: "CREATE TABLE posts (id UUID PRIMARY KEY)".into(),
                down: "DROP TABLE posts".into(),
            },
        ]
    }

    #[tokio::test]
    async fn runner_pending_returns_unapplied() {
        let pool = InMemoryPool::new();
        let runner = MigrationRunner::new(&pool).add_all(test_migrations());
        assert_eq!(runner.pending().len(), 2);
    }

    #[tokio::test]
    async fn runner_run_pending_applies_all() {
        let pool = InMemoryPool::new();
        let mut runner = MigrationRunner::new(&pool).add_all(test_migrations());
        let applied = runner.run_pending().await.unwrap();
        assert_eq!(applied.len(), 2);
        assert_eq!(runner.pending().len(), 0);
        assert_eq!(runner.applied().len(), 2);
    }

    #[tokio::test]
    async fn runner_rollback_last() {
        let pool = InMemoryPool::new();
        let mut runner = MigrationRunner::new(&pool).add_all(test_migrations());
        runner.run_pending().await.unwrap();
        let rolled = runner.rollback_last().await.unwrap();
        assert_eq!(rolled, Some("002".into()));
        assert_eq!(runner.pending().len(), 1);
    }

    #[tokio::test]
    async fn runner_status_shows_applied() {
        let pool = InMemoryPool::new();
        let mut runner = MigrationRunner::new(&pool).add_all(test_migrations());
        runner.run_pending().await.unwrap();
        let status = runner.status();
        assert!(status[0].applied);
        assert!(status[1].applied);
    }

    #[test]
    fn runner_mark_applied() {
        let pool = InMemoryPool::new();
        let mut runner = MigrationRunner::new(&pool).add_all(test_migrations());
        runner.mark_applied("001");
        assert_eq!(runner.pending().len(), 1);
        assert_eq!(runner.pending()[0].version, "002");
    }
}
