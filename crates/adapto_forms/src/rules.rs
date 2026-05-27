use crate::validation::{ValidationError, ValidationResult};
use serde_json::Value;

pub enum FormRule {
    FieldsMatch {
        field_a: String,
        field_b: String,
        message: String,
    },
    RequiredIf {
        field: String,
        condition_field: String,
        condition_value: Value,
    },
    RequiredUnless {
        field: String,
        condition_field: String,
        condition_value: Value,
    },
    MutuallyExclusive {
        fields: Vec<String>,
    },
    AtLeastOneOf {
        fields: Vec<String>,
        message: String,
    },
    Custom {
        name: String,
        validator: Box<dyn Fn(&serde_json::Map<String, Value>) -> Option<ValidationError> + Send + Sync>,
    },
}

impl FormRule {
    pub fn fields_match(field_a: &str, field_b: &str) -> Self {
        Self::FieldsMatch {
            field_a: field_a.into(),
            field_b: field_b.into(),
            message: format!("{} must match {}", field_a, field_b),
        }
    }

    pub fn fields_match_with_message(field_a: &str, field_b: &str, message: &str) -> Self {
        Self::FieldsMatch {
            field_a: field_a.into(),
            field_b: field_b.into(),
            message: message.into(),
        }
    }

    pub fn required_if(field: &str, condition_field: &str, condition_value: Value) -> Self {
        Self::RequiredIf {
            field: field.into(),
            condition_field: condition_field.into(),
            condition_value,
        }
    }

    pub fn required_unless(field: &str, condition_field: &str, condition_value: Value) -> Self {
        Self::RequiredUnless {
            field: field.into(),
            condition_field: condition_field.into(),
            condition_value,
        }
    }

    pub fn mutually_exclusive(fields: &[&str]) -> Self {
        Self::MutuallyExclusive {
            fields: fields.iter().map(|s| s.to_string()).collect(),
        }
    }

    pub fn at_least_one_of(fields: &[&str]) -> Self {
        let names = fields.join(", ");
        Self::AtLeastOneOf {
            fields: fields.iter().map(|s| s.to_string()).collect(),
            message: format!("at least one of {} is required", names),
        }
    }

    pub fn validate(&self, data: &serde_json::Map<String, Value>, result: &mut ValidationResult) {
        match self {
            FormRule::FieldsMatch {
                field_a,
                field_b,
                message,
            } => {
                let a = data.get(field_a);
                let b = data.get(field_b);
                if a != b {
                    result.add_error(field_b, "fields_match", message);
                }
            }
            FormRule::RequiredIf {
                field,
                condition_field,
                condition_value,
            } => {
                if data.get(condition_field) == Some(condition_value) {
                    let val = data.get(field);
                    if val.is_none() || val == Some(&Value::Null) || val == Some(&Value::String(String::new())) {
                        result.add_error(
                            field,
                            "required_if",
                            &format!("{} is required when {} is {}", field, condition_field, condition_value),
                        );
                    }
                }
            }
            FormRule::RequiredUnless {
                field,
                condition_field,
                condition_value,
            } => {
                if data.get(condition_field) != Some(condition_value) {
                    let val = data.get(field);
                    if val.is_none() || val == Some(&Value::Null) || val == Some(&Value::String(String::new())) {
                        result.add_error(
                            field,
                            "required_unless",
                            &format!("{} is required unless {} is {}", field, condition_field, condition_value),
                        );
                    }
                }
            }
            FormRule::MutuallyExclusive { fields } => {
                let present: Vec<&String> = fields
                    .iter()
                    .filter(|f| {
                        let v = data.get(f.as_str());
                        v.is_some() && v != Some(&Value::Null)
                    })
                    .collect();
                if present.len() > 1 {
                    for f in &present[1..] {
                        result.add_error(
                            f,
                            "mutually_exclusive",
                            &format!("only one of {} can be set", fields.join(", ")),
                        );
                    }
                }
            }
            FormRule::AtLeastOneOf { fields, message } => {
                let any_present = fields.iter().any(|f| {
                    let v = data.get(f.as_str());
                    v.is_some() && v != Some(&Value::Null) && v != Some(&Value::String(String::new()))
                });
                if !any_present {
                    let field = fields.first().map_or("_form", |f| f.as_str());
                    result.add_error(field, "at_least_one", message);
                }
            }
            FormRule::Custom { name: _, validator } => {
                if let Some(error) = validator(data) {
                    result.add_error(&error.field, &error.code, &error.message);
                }
            }
        }
    }
}

impl std::fmt::Debug for FormRule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FormRule::FieldsMatch { field_a, field_b, .. } => {
                write!(f, "FieldsMatch({}, {})", field_a, field_b)
            }
            FormRule::RequiredIf { field, condition_field, .. } => {
                write!(f, "RequiredIf({} when {})", field, condition_field)
            }
            FormRule::RequiredUnless { field, condition_field, .. } => {
                write!(f, "RequiredUnless({} unless {})", field, condition_field)
            }
            FormRule::MutuallyExclusive { fields } => {
                write!(f, "MutuallyExclusive({:?})", fields)
            }
            FormRule::AtLeastOneOf { fields, .. } => {
                write!(f, "AtLeastOneOf({:?})", fields)
            }
            FormRule::Custom { name, .. } => {
                write!(f, "Custom({})", name)
            }
        }
    }
}
