use crate::schema::{Constraint, FieldSchema, FieldType};
use serde_json::Value;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// ValidationResult
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct ValidationResult {
    pub errors: HashMap<String, Vec<ValidationError>>,
}

impl ValidationResult {
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn add_error(&mut self, field: &str, code: &str, message: &str) {
        self.errors
            .entry(field.to_string())
            .or_default()
            .push(ValidationError {
                field: field.to_string(),
                message: message.to_string(),
                code: code.to_string(),
            });
    }

    pub fn field_errors(&self, field: &str) -> &[ValidationError] {
        static EMPTY: Vec<ValidationError> = Vec::new();
        self.errors.get(field).map_or(&EMPTY, |v| v.as_slice())
    }

    pub fn all_errors(&self) -> Vec<&ValidationError> {
        self.errors.values().flat_map(|v| v.iter()).collect()
    }
}

// ---------------------------------------------------------------------------
// ValidationError
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
    pub code: String,
}

// ---------------------------------------------------------------------------
// Field validation
// ---------------------------------------------------------------------------

pub fn validate_field(field: &FieldSchema, value: Option<&Value>) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    let field_name = &field.name;

    // Handle Optional wrapper: if the inner type is Optional and value is absent or null, skip.
    let (inner_type, is_optional_type) = match &field.field_type {
        FieldType::Optional(inner) => (inner.as_ref(), true),
        other => (other, false),
    };

    // Check for missing / null value.
    match value {
        None | Some(Value::Null) => {
            if field.required && !is_optional_type {
                errors.push(ValidationError {
                    field: field_name.clone(),
                    message: format!("{} is required", field_name),
                    code: "required".to_string(),
                });
            }
            // Nothing more to validate when the value is absent.
            return errors;
        }
        _ => {}
    }

    let value = value.unwrap();

    // Type-specific validation.
    validate_type(field_name, inner_type, value, &mut errors);

    // Constraint validation.
    for constraint in &field.constraints {
        validate_constraint(field_name, constraint, value, &mut errors);
    }

    errors
}

/// Validate that `value` conforms to the expected `FieldType`.
fn validate_type(
    field_name: &str,
    field_type: &FieldType,
    value: &Value,
    errors: &mut Vec<ValidationError>,
) {
    match field_type {
        FieldType::String => {
            if !value.is_string() {
                errors.push(ValidationError {
                    field: field_name.to_string(),
                    message: format!("{} must be a string", field_name),
                    code: "invalid_type".to_string(),
                });
            }
        }
        FieldType::Email => {
            match value.as_str() {
                Some(s) => {
                    if !validate_email(s) {
                        errors.push(ValidationError {
                            field: field_name.to_string(),
                            message: format!("{} must be a valid email address", field_name),
                            code: "invalid_email".to_string(),
                        });
                    }
                }
                None => {
                    errors.push(ValidationError {
                        field: field_name.to_string(),
                        message: format!("{} must be a string", field_name),
                        code: "invalid_type".to_string(),
                    });
                }
            }
        }
        FieldType::Integer => {
            if !value.is_i64() && !value.is_u64() {
                errors.push(ValidationError {
                    field: field_name.to_string(),
                    message: format!("{} must be an integer", field_name),
                    code: "invalid_type".to_string(),
                });
            }
        }
        FieldType::Decimal => {
            if !value.is_f64() && !value.is_i64() && !value.is_u64() {
                errors.push(ValidationError {
                    field: field_name.to_string(),
                    message: format!("{} must be a number", field_name),
                    code: "invalid_type".to_string(),
                });
            }
        }
        FieldType::Boolean => {
            if !value.is_boolean() {
                errors.push(ValidationError {
                    field: field_name.to_string(),
                    message: format!("{} must be a boolean", field_name),
                    code: "invalid_type".to_string(),
                });
            }
        }
        FieldType::Uuid => {
            match value.as_str() {
                Some(s) => {
                    if uuid::Uuid::parse_str(s).is_err() {
                        errors.push(ValidationError {
                            field: field_name.to_string(),
                            message: format!("{} must be a valid UUID", field_name),
                            code: "invalid_uuid".to_string(),
                        });
                    }
                }
                None => {
                    errors.push(ValidationError {
                        field: field_name.to_string(),
                        message: format!("{} must be a string", field_name),
                        code: "invalid_type".to_string(),
                    });
                }
            }
        }
        FieldType::DateTime => {
            match value.as_str() {
                Some(s) => {
                    // Accept ISO 8601 / RFC 3339 format.
                    if chrono::DateTime::parse_from_rfc3339(s).is_err() {
                        errors.push(ValidationError {
                            field: field_name.to_string(),
                            message: format!("{} must be a valid ISO 8601 datetime", field_name),
                            code: "invalid_datetime".to_string(),
                        });
                    }
                }
                None => {
                    errors.push(ValidationError {
                        field: field_name.to_string(),
                        message: format!("{} must be a string", field_name),
                        code: "invalid_type".to_string(),
                    });
                }
            }
        }
        FieldType::Enum(variants) => {
            match value.as_str() {
                Some(s) => {
                    if !variants.contains(&s.to_string()) {
                        errors.push(ValidationError {
                            field: field_name.to_string(),
                            message: format!(
                                "{} must be one of: {}",
                                field_name,
                                variants.join(", ")
                            ),
                            code: "invalid_enum".to_string(),
                        });
                    }
                }
                None => {
                    errors.push(ValidationError {
                        field: field_name.to_string(),
                        message: format!("{} must be a string", field_name),
                        code: "invalid_type".to_string(),
                    });
                }
            }
        }
        FieldType::Optional(inner) => {
            // If we reach here the value is non-null, so validate against inner type.
            validate_type(field_name, inner, value, errors);
        }
    }
}

/// Validate a single constraint against a value.
fn validate_constraint(
    field_name: &str,
    constraint: &Constraint,
    value: &Value,
    errors: &mut Vec<ValidationError>,
) {
    match constraint {
        Constraint::MinLength(min) => {
            if let Some(s) = value.as_str() {
                if s.len() < *min {
                    errors.push(ValidationError {
                        field: field_name.to_string(),
                        message: format!(
                            "{} must be at least {} characters",
                            field_name, min
                        ),
                        code: "min_length".to_string(),
                    });
                }
            }
        }
        Constraint::MaxLength(max) => {
            if let Some(s) = value.as_str() {
                if s.len() > *max {
                    errors.push(ValidationError {
                        field: field_name.to_string(),
                        message: format!(
                            "{} must be at most {} characters",
                            field_name, max
                        ),
                        code: "max_length".to_string(),
                    });
                }
            }
        }
        Constraint::Min(min) => {
            if let Some(n) = value.as_i64() {
                if n < *min {
                    errors.push(ValidationError {
                        field: field_name.to_string(),
                        message: format!("{} must be at least {}", field_name, min),
                        code: "min".to_string(),
                    });
                }
            } else if let Some(n) = value.as_f64() {
                if n < *min as f64 {
                    errors.push(ValidationError {
                        field: field_name.to_string(),
                        message: format!("{} must be at least {}", field_name, min),
                        code: "min".to_string(),
                    });
                }
            }
        }
        Constraint::Max(max) => {
            if let Some(n) = value.as_i64() {
                if n > *max {
                    errors.push(ValidationError {
                        field: field_name.to_string(),
                        message: format!("{} must be at most {}", field_name, max),
                        code: "max".to_string(),
                    });
                }
            } else if let Some(n) = value.as_f64() {
                if n > *max as f64 {
                    errors.push(ValidationError {
                        field: field_name.to_string(),
                        message: format!("{} must be at most {}", field_name, max),
                        code: "max".to_string(),
                    });
                }
            }
        }
        Constraint::Pattern(pattern) => {
            if let Some(s) = value.as_str() {
                // Simple pattern matching: we check if the pattern appears as a substring.
                // For production use this would be a proper regex, but we avoid the regex
                // crate dependency for now and treat the pattern as a literal contains check
                // unless it looks like a basic anchored regex (^...$).
                let matches = if pattern.starts_with('^') && pattern.ends_with('$') {
                    // Exact match: strip anchors.
                    let inner = &pattern[1..pattern.len() - 1];
                    s == inner
                } else {
                    s.contains(pattern.as_str())
                };
                if !matches {
                    errors.push(ValidationError {
                        field: field_name.to_string(),
                        message: format!("{} does not match pattern {}", field_name, pattern),
                        code: "pattern".to_string(),
                    });
                }
            }
        }
        Constraint::Unique | Constraint::Custom(_) => {
            // These constraints require external context (database, custom fn) and cannot
            // be validated at the schema level alone. They are recorded for upstream use.
        }
    }
}

// ---------------------------------------------------------------------------
// Email validation
// ---------------------------------------------------------------------------

pub fn validate_email(email: &str) -> bool {
    // A practical email check: non-empty local part, single @, non-empty domain with a dot.
    let parts: Vec<&str> = email.splitn(2, '@').collect();
    if parts.len() != 2 {
        return false;
    }
    let local = parts[0];
    let domain = parts[1];

    if local.is_empty() || domain.is_empty() {
        return false;
    }
    if !domain.contains('.') {
        return false;
    }
    // Domain must not start or end with a dot.
    if domain.starts_with('.') || domain.ends_with('.') {
        return false;
    }
    // Local part must not contain spaces.
    if local.contains(' ') {
        return false;
    }
    true
}
