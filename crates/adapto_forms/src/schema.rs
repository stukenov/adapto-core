use serde::{Deserialize, Serialize};

use crate::rules::FormRule;
use crate::sanitize::SanitizerPipeline;
use crate::validation::{validate_field, ValidationResult};

// ---------------------------------------------------------------------------
// FormSchema
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormSchema {
    pub name: String,
    pub fields: Vec<FieldSchema>,
    #[serde(skip)]
    rules: Vec<FormRuleHolder>,
}

#[derive(Clone)]
struct FormRuleHolder(std::sync::Arc<FormRule>);

impl std::fmt::Debug for FormRuleHolder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl serde::Serialize for FormRuleHolder {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&format!("{:?}", self.0))
    }
}

impl<'de> serde::Deserialize<'de> for FormRuleHolder {
    fn deserialize<D: serde::Deserializer<'de>>(_deserializer: D) -> Result<Self, D::Error> {
        Err(serde::de::Error::custom("FormRuleHolder cannot be deserialized"))
    }
}

impl FormSchema {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            fields: Vec::new(),
            rules: Vec::new(),
        }
    }

    pub fn field(mut self, field: FieldSchema) -> Self {
        self.fields.push(field);
        self
    }

    pub fn rule(mut self, rule: FormRule) -> Self {
        self.rules.push(FormRuleHolder(std::sync::Arc::new(rule)));
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
        for rule in &self.rules {
            rule.0.validate(data, &mut result);
        }
        result
    }

    pub fn validate_and_sanitize(
        &self,
        data: &mut serde_json::Map<String, serde_json::Value>,
        pipeline: &SanitizerPipeline,
    ) -> ValidationResult {
        pipeline.apply(data);
        self.validate(data)
    }

    pub fn field_names(&self) -> Vec<&str> {
        self.fields.iter().map(|f| f.name.as_str()).collect()
    }

    pub fn get_field(&self, name: &str) -> Option<&FieldSchema> {
        self.fields.iter().find(|f| f.name == name)
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
