use serde_json::Value;
use std::collections::BTreeMap;

pub fn insert_sql(table: &str, data: &BTreeMap<String, Value>) -> (String, Vec<Value>) {
    let columns: Vec<&String> = data.keys().collect();
    let params: Vec<Value> = data.values().cloned().collect();
    let placeholders: Vec<String> = (1..=columns.len()).map(|i| format!("${}", i)).collect();

    let sql = format!(
        "INSERT INTO {} ({}) VALUES ({}) RETURNING *",
        table,
        columns.iter().map(|c| c.as_str()).collect::<Vec<_>>().join(", "),
        placeholders.join(", ")
    );

    (sql, params)
}

pub fn update_sql(
    table: &str,
    id_column: &str,
    id_value: &Value,
    data: &BTreeMap<String, Value>,
) -> (String, Vec<Value>) {
    let mut params: Vec<Value> = Vec::new();
    let set_clauses: Vec<String> = data
        .iter()
        .map(|(col, val)| {
            params.push(val.clone());
            format!("{} = ${}", col, params.len())
        })
        .collect();

    params.push(id_value.clone());
    let sql = format!(
        "UPDATE {} SET {} WHERE {} = ${} RETURNING *",
        table,
        set_clauses.join(", "),
        id_column,
        params.len()
    );

    (sql, params)
}

pub fn delete_sql(table: &str, id_column: &str, id_value: &Value) -> (String, Vec<Value>) {
    let sql = format!("DELETE FROM {} WHERE {} = $1", table, id_column);
    (sql, vec![id_value.clone()])
}

pub fn select_by_id_sql(table: &str, id_column: &str, id_value: &Value) -> (String, Vec<Value>) {
    let sql = format!("SELECT * FROM {} WHERE {} = $1", table, id_column);
    (sql, vec![id_value.clone()])
}

pub fn count_sql(table: &str) -> String {
    format!("SELECT COUNT(*) as count FROM {}", table)
}

pub fn upsert_sql(
    table: &str,
    conflict_column: &str,
    data: &BTreeMap<String, Value>,
) -> (String, Vec<Value>) {
    let columns: Vec<&String> = data.keys().collect();
    let params: Vec<Value> = data.values().cloned().collect();
    let placeholders: Vec<String> = (1..=columns.len()).map(|i| format!("${}", i)).collect();

    let update_clauses: Vec<String> = columns
        .iter()
        .filter(|c| c.as_str() != conflict_column)
        .enumerate()
        .map(|(_, col)| format!("{} = EXCLUDED.{}", col, col))
        .collect();

    let sql = format!(
        "INSERT INTO {} ({}) VALUES ({}) ON CONFLICT ({}) DO UPDATE SET {} RETURNING *",
        table,
        columns.iter().map(|c| c.as_str()).collect::<Vec<_>>().join(", "),
        placeholders.join(", "),
        conflict_column,
        update_clauses.join(", ")
    );

    (sql, params)
}

pub fn truncate_sql(table: &str) -> String {
    format!("TRUNCATE TABLE {} CASCADE", table)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn test_data() -> BTreeMap<String, Value> {
        let mut data = BTreeMap::new();
        data.insert("name".into(), json!("Alice"));
        data.insert("email".into(), json!("alice@example.com"));
        data
    }

    #[test]
    fn insert_generates_correct_sql() {
        let (sql, params) = insert_sql("users", &test_data());
        assert!(sql.starts_with("INSERT INTO users"));
        assert!(sql.contains("$1"));
        assert!(sql.contains("$2"));
        assert!(sql.contains("RETURNING *"));
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn update_generates_correct_sql() {
        let (sql, params) = update_sql("users", "id", &json!("uuid-1"), &test_data());
        assert!(sql.starts_with("UPDATE users SET"));
        assert!(sql.contains("WHERE id = $3"));
        assert!(sql.contains("RETURNING *"));
        assert_eq!(params.len(), 3);
    }

    #[test]
    fn delete_generates_correct_sql() {
        let (sql, params) = delete_sql("users", "id", &json!("uuid-1"));
        assert_eq!(sql, "DELETE FROM users WHERE id = $1");
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn select_by_id_generates_correct_sql() {
        let (sql, params) = select_by_id_sql("users", "id", &json!(42));
        assert_eq!(sql, "SELECT * FROM users WHERE id = $1");
        assert_eq!(params[0], json!(42));
    }

    #[test]
    fn count_sql_correct() {
        assert_eq!(count_sql("users"), "SELECT COUNT(*) as count FROM users");
    }

    #[test]
    fn upsert_generates_correct_sql() {
        let (sql, params) = upsert_sql("users", "email", &test_data());
        assert!(sql.contains("ON CONFLICT (email)"));
        assert!(sql.contains("DO UPDATE SET"));
        assert!(sql.contains("RETURNING *"));
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn truncate_generates_correct_sql() {
        assert_eq!(truncate_sql("users"), "TRUNCATE TABLE users CASCADE");
    }
}
