use serde::{Deserialize, Serialize};

use crate::validation::{validate_field, ValidationResult};

// ---------------------------------------------------------------------------
// FormSchema
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormSchema {
    pub name: String,
    pub fields: Vec<FieldSchema>,
}

impl FormSchema {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            fields: Vec::new(),
        }
    }

    pub fn field(mut self, field: FieldSchema) -> Self {
        self.fields.push(field);
        self
    }

    pub fn validate(
        &self,
        data: &serde_json::Map<String, serde_json::Value>,
    ) -> ValidationResult {
        let mut result = ValidationResult::default();
        for field in &self.fields {
            let value = data.get(&field.name);
            let errors = validate_field(field, value);
            for error in errors {
                result.add_error(&error.field, &error.code, &error.message);
            }
        }
        result
    }
}

// ---------------------------------------------------------------------------
// FieldSchema
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldSchema {
    pub name: String,
    pub field_type: FieldType,
    pub required: bool,
    pub constraints: Vec<Constraint>,
    pub label: Option<String>,
}

impl FieldSchema {
    pub fn new(name: &str, field_type: FieldType) -> Self {
        Self {
            name: name.to_string(),
            field_type,
            required: false,
            constraints: Vec::new(),
            label: None,
        }
    }

    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    pub fn optional(mut self) -> Self {
        self.required = false;
        self
    }

    pub fn min_length(mut self, min: usize) -> Self {
        self.constraints.push(Constraint::MinLength(min));
        self
    }

    pub fn max_length(mut self, max: usize) -> Self {
        self.constraints.push(Constraint::MaxLength(max));
        self
    }

    pub fn min(mut self, min: i64) -> Self {
        self.constraints.push(Constraint::Min(min));
        self
    }

    pub fn max(mut self, max: i64) -> Self {
        self.constraints.push(Constraint::Max(max));
        self
    }

    pub fn pattern(mut self, pattern: &str) -> Self {
        self.constraints.push(Constraint::Pattern(pattern.to_string()));
        self
    }

    pub fn label(mut self, label: &str) -> Self {
        self.label = Some(label.to_string());
        self
    }

    pub fn constraint(mut self, constraint: Constraint) -> Self {
        self.constraints.push(constraint);
        self
    }
}

// ---------------------------------------------------------------------------
// FieldType
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FieldType {
    String,
    Email,
    Integer,
    Decimal,
    Boolean,
    Uuid,
    DateTime,
    Enum(Vec<std::string::String>),
    Optional(Box<FieldType>),
}

// ---------------------------------------------------------------------------
// Constraint
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Constraint {
    MinLength(usize),
    MaxLength(usize),
    Min(i64),
    Max(i64),
    Pattern(std::string::String),
    Unique,
    Custom(std::string::String),
}
