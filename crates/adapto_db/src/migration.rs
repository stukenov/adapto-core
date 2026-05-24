use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Migration
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Migration {
    pub version: String,
    pub name: String,
    pub up: String,
    pub down: String,
}

// ---------------------------------------------------------------------------
// MigrationPlan
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct MigrationPlan {
    pub migrations: Vec<Migration>,
}

impl MigrationPlan {
    pub fn new() -> Self {
        Self {
            migrations: Vec::new(),
        }
    }

    pub fn add(mut self, migration: Migration) -> Self {
        self.migrations.push(migration);
        self
    }

    /// Generate a `CREATE TABLE` migration from a list of column definitions.
    pub fn create_table(name: &str, columns: Vec<ColumnDef>) -> Migration {
        let col_defs: Vec<String> = columns.iter().map(|c| c.to_sql()).collect();
        let up = format!("CREATE TABLE {} (\n  {}\n);", name, col_defs.join(",\n  "));
        let down = format!("DROP TABLE IF EXISTS {};", name);

        Migration {
            version: String::new(),
            name: format!("create_{}", name),
            up,
            down,
        }
    }
}

impl Default for MigrationPlan {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// ColumnDef
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ColumnDef {
    pub name: String,
    pub sql_type: String,
    pub nullable: bool,
    pub primary_key: bool,
    pub unique: bool,
    pub default: Option<String>,
}

impl ColumnDef {
    pub fn new(name: &str, sql_type: &str) -> Self {
        Self {
            name: name.to_string(),
            sql_type: sql_type.to_string(),
            nullable: true,
            primary_key: false,
            unique: false,
            default: None,
        }
    }

    pub fn primary_key(mut self) -> Self {
        self.primary_key = true;
        self.nullable = false;
        self
    }

    pub fn not_null(mut self) -> Self {
        self.nullable = false;
        self
    }

    pub fn unique(mut self) -> Self {
        self.unique = true;
        self
    }

    pub fn default_value(mut self, value: &str) -> Self {
        self.default = Some(value.to_string());
        self
    }

    /// Render this column definition as a SQL fragment.
    fn to_sql(&self) -> String {
        let mut parts = vec![self.name.clone(), self.sql_type.clone()];

        if self.primary_key {
            parts.push("PRIMARY KEY".to_string());
        }

        if !self.nullable && !self.primary_key {
            parts.push("NOT NULL".to_string());
        }

        if self.unique {
            parts.push("UNIQUE".to_string());
        }

        if let Some(ref default) = self.default {
            parts.push(format!("DEFAULT {}", default));
        }

        parts.join(" ")
    }
}
