use crate::event::AuditEvent;

pub struct PiiRedactor {
    fields: Vec<String>,
    replacement: String,
}

impl PiiRedactor {
    pub fn new() -> Self {
        Self {
            fields: vec![
                "email".into(),
                "password".into(),
                "ssn".into(),
                "phone".into(),
                "credit_card".into(),
                "token".into(),
                "secret".into(),
                "api_key".into(),
            ],
            replacement: "[REDACTED]".into(),
        }
    }

    pub fn with_fields(mut self, fields: &[&str]) -> Self {
        self.fields = fields.iter().map(|s| s.to_string()).collect();
        self
    }

    pub fn add_field(mut self, field: &str) -> Self {
        self.fields.push(field.into());
        self
    }

    pub fn replacement(mut self, replacement: &str) -> Self {
        self.replacement = replacement.into();
        self
    }

    pub fn redact(&self, event: &mut AuditEvent) {
        let replacement = serde_json::Value::String(self.replacement.clone());
        for field in &self.fields {
            if event.metadata.contains_key(field) {
                event.metadata.insert(field.clone(), replacement.clone());
            }
        }
    }

    pub fn redact_clone(&self, event: &AuditEvent) -> AuditEvent {
        let mut cloned = event.clone();
        self.redact(&mut cloned);
        cloned
    }

    pub fn is_sensitive(&self, key: &str) -> bool {
        let lower = key.to_lowercase();
        self.fields.iter().any(|f| lower.contains(f))
    }
}

impl Default for PiiRedactor {
    fn default() -> Self {
        Self::new()
    }
}

pub fn redact_value(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::String(s) => {
            if s.len() <= 4 {
                serde_json::Value::String("****".into())
            } else {
                let masked = format!("{}***", &s[..2]);
                serde_json::Value::String(masked)
            }
        }
        other => other.clone(),
    }
}
