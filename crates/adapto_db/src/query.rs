use serde_json::Value;

// ---------------------------------------------------------------------------
// Query builder
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Query {
    pub table: String,
    pub conditions: Vec<Condition>,
    pub order_by: Vec<OrderBy>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Clone)]
pub enum Condition {
    Eq(String, Value),
    Ne(String, Value),
    Gt(String, Value),
    Lt(String, Value),
    Gte(String, Value),
    Lte(String, Value),
    Like(String, String),
    In(String, Vec<Value>),
    IsNull(String),
    IsNotNull(String),
    And(Vec<Condition>),
    Or(Vec<Condition>),
}

#[derive(Debug, Clone)]
pub struct OrderBy {
    pub column: String,
    pub direction: Direction,
}

#[derive(Debug, Clone)]
pub enum Direction {
    Asc,
    Desc,
}

impl Query {
    pub fn table(name: &str) -> Self {
        Self {
            table: name.to_string(),
            conditions: Vec::new(),
            order_by: Vec::new(),
            limit: None,
            offset: None,
        }
    }

    pub fn where_eq(mut self, column: &str, value: Value) -> Self {
        self.conditions.push(Condition::Eq(column.to_string(), value));
        self
    }

    pub fn where_ne(mut self, column: &str, value: Value) -> Self {
        self.conditions.push(Condition::Ne(column.to_string(), value));
        self
    }

    pub fn where_gt(mut self, column: &str, value: Value) -> Self {
        self.conditions.push(Condition::Gt(column.to_string(), value));
        self
    }

    pub fn where_lt(mut self, column: &str, value: Value) -> Self {
        self.conditions.push(Condition::Lt(column.to_string(), value));
        self
    }

    pub fn where_gte(mut self, column: &str, value: Value) -> Self {
        self.conditions
            .push(Condition::Gte(column.to_string(), value));
        self
    }

    pub fn where_lte(mut self, column: &str, value: Value) -> Self {
        self.conditions
            .push(Condition::Lte(column.to_string(), value));
        self
    }

    pub fn where_like(mut self, column: &str, pattern: &str) -> Self {
        self.conditions
            .push(Condition::Like(column.to_string(), pattern.to_string()));
        self
    }

    pub fn where_in(mut self, column: &str, values: Vec<Value>) -> Self {
        self.conditions
            .push(Condition::In(column.to_string(), values));
        self
    }

    pub fn where_null(mut self, column: &str) -> Self {
        self.conditions
            .push(Condition::IsNull(column.to_string()));
        self
    }

    pub fn where_not_null(mut self, column: &str) -> Self {
        self.conditions
            .push(Condition::IsNotNull(column.to_string()));
        self
    }

    pub fn and(mut self, conditions: Vec<Condition>) -> Self {
        self.conditions.push(Condition::And(conditions));
        self
    }

    pub fn or(mut self, conditions: Vec<Condition>) -> Self {
        self.conditions.push(Condition::Or(conditions));
        self
    }

    pub fn order_by(mut self, column: &str, direction: Direction) -> Self {
        self.order_by.push(OrderBy {
            column: column.to_string(),
            direction,
        });
        self
    }

    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }

    /// Generate a parameterised SQL query.
    ///
    /// Returns `(sql_string, ordered_params)`. Parameters use `$1`, `$2`, ...
    /// placeholders (PostgreSQL style).
    pub fn to_sql(&self) -> (String, Vec<Value>) {
        let mut params: Vec<Value> = Vec::new();
        let mut sql = format!("SELECT * FROM {}", self.table);

        if !self.conditions.is_empty() {
            let mut where_clauses = Vec::new();
            for cond in &self.conditions {
                let clause = condition_to_sql(cond, &mut params);
                where_clauses.push(clause);
            }
            sql.push_str(" WHERE ");
            sql.push_str(&where_clauses.join(" AND "));
        }

        if !self.order_by.is_empty() {
            let order_parts: Vec<String> = self
                .order_by
                .iter()
                .map(|o| {
                    let dir = match o.direction {
                        Direction::Asc => "ASC",
                        Direction::Desc => "DESC",
                    };
                    format!("{} {}", o.column, dir)
                })
                .collect();
            sql.push_str(" ORDER BY ");
            sql.push_str(&order_parts.join(", "));
        }

        if let Some(limit) = self.limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }

        if let Some(offset) = self.offset {
            sql.push_str(&format!(" OFFSET {}", offset));
        }

        (sql, params)
    }
}

/// Recursively convert a `Condition` tree into SQL, appending parameter values.
fn condition_to_sql(condition: &Condition, params: &mut Vec<Value>) -> String {
    match condition {
        Condition::Eq(col, val) => {
            params.push(val.clone());
            format!("{} = ${}", col, params.len())
        }
        Condition::Ne(col, val) => {
            params.push(val.clone());
            format!("{} != ${}", col, params.len())
        }
        Condition::Gt(col, val) => {
            params.push(val.clone());
            format!("{} > ${}", col, params.len())
        }
        Condition::Lt(col, val) => {
            params.push(val.clone());
            format!("{} < ${}", col, params.len())
        }
        Condition::Gte(col, val) => {
            params.push(val.clone());
            format!("{} >= ${}", col, params.len())
        }
        Condition::Lte(col, val) => {
            params.push(val.clone());
            format!("{} <= ${}", col, params.len())
        }
        Condition::Like(col, pattern) => {
            params.push(Value::String(pattern.clone()));
            format!("{} LIKE ${}", col, params.len())
        }
        Condition::In(col, values) => {
            let placeholders: Vec<String> = values
                .iter()
                .map(|v| {
                    params.push(v.clone());
                    format!("${}", params.len())
                })
                .collect();
            format!("{} IN ({})", col, placeholders.join(", "))
        }
        Condition::IsNull(col) => {
            format!("{} IS NULL", col)
        }
        Condition::IsNotNull(col) => {
            format!("{} IS NOT NULL", col)
        }
        Condition::And(conditions) => {
            let parts: Vec<String> = conditions
                .iter()
                .map(|c| condition_to_sql(c, params))
                .collect();
            format!("({})", parts.join(" AND "))
        }
        Condition::Or(conditions) => {
            let parts: Vec<String> = conditions
                .iter()
                .map(|c| condition_to_sql(c, params))
                .collect();
            format!("({})", parts.join(" OR "))
        }
    }
}
