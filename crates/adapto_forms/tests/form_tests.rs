use adapto_forms::schema::{FieldSchema, FieldType, FormSchema};
use adapto_forms::validation::{validate_email, validate_field, ValidationResult};
use serde_json::{json, Map, Value};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn map_from(pairs: Vec<(&str, Value)>) -> Map<String, Value> {
    let mut m = Map::new();
    for (k, v) in pairs {
        m.insert(k.to_string(), v);
    }
    m
}

// ---------------------------------------------------------------------------
// 1. Schema builder API
// ---------------------------------------------------------------------------

#[test]
fn schema_builder_api() {
    let schema = FormSchema::new("user")
        .field(FieldSchema::new("name", FieldType::String).required().min_length(2))
        .field(FieldSchema::new("age", FieldType::Integer));

    assert_eq!(schema.name, "user");
    assert_eq!(schema.fields.len(), 2);
    assert_eq!(schema.fields[0].name, "name");
    assert!(schema.fields[0].required);
    assert_eq!(schema.fields[1].name, "age");
    assert!(!schema.fields[1].required);
}

// ---------------------------------------------------------------------------
// 2. Validate required field present
// ---------------------------------------------------------------------------

#[test]
fn validate_required_field_present() {
    let field = FieldSchema::new("name", FieldType::String).required();
    let errors = validate_field(&field, Some(&json!("Alice")));
    assert!(errors.is_empty());
}

// ---------------------------------------------------------------------------
// 3. Validate required field missing
// ---------------------------------------------------------------------------

#[test]
fn validate_required_field_missing() {
    let field = FieldSchema::new("name", FieldType::String).required();
    let errors = validate_field(&field, None);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, "required");
}

// ---------------------------------------------------------------------------
// 4. Validate string min_length pass
// ---------------------------------------------------------------------------

#[test]
fn validate_string_min_length_pass() {
    let field = FieldSchema::new("name", FieldType::String).min_length(2);
    let errors = validate_field(&field, Some(&json!("Al")));
    assert!(errors.is_empty());
}

// ---------------------------------------------------------------------------
// 5. Validate string min_length fail
// ---------------------------------------------------------------------------

#[test]
fn validate_string_min_length_fail() {
    let field = FieldSchema::new("name", FieldType::String).min_length(3);
    let errors = validate_field(&field, Some(&json!("Al")));
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, "min_length");
}

// ---------------------------------------------------------------------------
// 6. Validate string max_length pass
// ---------------------------------------------------------------------------

#[test]
fn validate_string_max_length_pass() {
    let field = FieldSchema::new("name", FieldType::String).max_length(10);
    let errors = validate_field(&field, Some(&json!("Alice")));
    assert!(errors.is_empty());
}

// ---------------------------------------------------------------------------
// 7. Validate string max_length fail
// ---------------------------------------------------------------------------

#[test]
fn validate_string_max_length_fail() {
    let field = FieldSchema::new("name", FieldType::String).max_length(3);
    let errors = validate_field(&field, Some(&json!("Alice")));
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, "max_length");
}

// ---------------------------------------------------------------------------
// 8. Validate email valid
// ---------------------------------------------------------------------------

#[test]
fn validate_email_valid() {
    assert!(validate_email("user@example.com"));
    assert!(validate_email("a.b+c@sub.domain.org"));
}

// ---------------------------------------------------------------------------
// 9. Validate email invalid
// ---------------------------------------------------------------------------

#[test]
fn validate_email_invalid() {
    assert!(!validate_email("not-an-email"));
    assert!(!validate_email("@domain.com"));
    assert!(!validate_email("user@"));
    assert!(!validate_email("user@domain"));
    assert!(!validate_email("user @domain.com"));
}

// ---------------------------------------------------------------------------
// 10. Validate integer type
// ---------------------------------------------------------------------------

#[test]
fn validate_integer_type() {
    let field = FieldSchema::new("age", FieldType::Integer);
    assert!(validate_field(&field, Some(&json!(42))).is_empty());
    let errors = validate_field(&field, Some(&json!("not a number")));
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, "invalid_type");
}

// ---------------------------------------------------------------------------
// 11. Validate decimal type
// ---------------------------------------------------------------------------

#[test]
fn validate_decimal_type() {
    let field = FieldSchema::new("price", FieldType::Decimal);
    assert!(validate_field(&field, Some(&json!(19.99))).is_empty());
    assert!(validate_field(&field, Some(&json!(100))).is_empty());
    let errors = validate_field(&field, Some(&json!("cheap")));
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, "invalid_type");
}

// ---------------------------------------------------------------------------
// 12. Validate boolean type
// ---------------------------------------------------------------------------

#[test]
fn validate_boolean_type() {
    let field = FieldSchema::new("active", FieldType::Boolean);
    assert!(validate_field(&field, Some(&json!(true))).is_empty());
    assert!(validate_field(&field, Some(&json!(false))).is_empty());
    let errors = validate_field(&field, Some(&json!("yes")));
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, "invalid_type");
}

// ---------------------------------------------------------------------------
// 13. Validate uuid type
// ---------------------------------------------------------------------------

#[test]
fn validate_uuid_type() {
    let field = FieldSchema::new("id", FieldType::Uuid);
    assert!(
        validate_field(&field, Some(&json!("550e8400-e29b-41d4-a716-446655440000"))).is_empty()
    );
    let errors = validate_field(&field, Some(&json!("not-a-uuid")));
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, "invalid_uuid");
}

// ---------------------------------------------------------------------------
// 14. Validate enum type valid value
// ---------------------------------------------------------------------------

#[test]
fn validate_enum_type_valid() {
    let field = FieldSchema::new(
        "status",
        FieldType::Enum(vec!["active".into(), "inactive".into(), "pending".into()]),
    );
    assert!(validate_field(&field, Some(&json!("active"))).is_empty());
}

// ---------------------------------------------------------------------------
// 15. Validate enum type invalid value
// ---------------------------------------------------------------------------

#[test]
fn validate_enum_type_invalid() {
    let field = FieldSchema::new(
        "status",
        FieldType::Enum(vec!["active".into(), "inactive".into()]),
    );
    let errors = validate_field(&field, Some(&json!("deleted")));
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, "invalid_enum");
}

// ---------------------------------------------------------------------------
// 16. Validate optional field (None is ok)
// ---------------------------------------------------------------------------

#[test]
fn validate_optional_field_none_is_ok() {
    let field = FieldSchema::new("nickname", FieldType::Optional(Box::new(FieldType::String)));
    assert!(validate_field(&field, None).is_empty());
    assert!(validate_field(&field, Some(&Value::Null)).is_empty());
    // Present value should still be validated.
    assert!(validate_field(&field, Some(&json!("nick"))).is_empty());
}

// ---------------------------------------------------------------------------
// 17. Validate multiple fields with multiple errors
// ---------------------------------------------------------------------------

#[test]
fn validate_multiple_fields_with_multiple_errors() {
    let schema = FormSchema::new("signup")
        .field(FieldSchema::new("name", FieldType::String).required().min_length(2))
        .field(FieldSchema::new("email", FieldType::Email).required());

    // Both fields fail.
    let data = map_from(vec![("name", json!("A")), ("email", json!("bad"))]);
    let result = schema.validate(&data);

    assert!(!result.is_valid());
    assert!(!result.field_errors("name").is_empty());
    assert!(!result.field_errors("email").is_empty());
}

// ---------------------------------------------------------------------------
// 18. ValidationResult is_valid true when no errors
// ---------------------------------------------------------------------------

#[test]
fn validation_result_is_valid_true() {
    let result = ValidationResult::default();
    assert!(result.is_valid());
}

// ---------------------------------------------------------------------------
// 19. ValidationResult is_valid false when has errors
// ---------------------------------------------------------------------------

#[test]
fn validation_result_is_valid_false() {
    let mut result = ValidationResult::default();
    result.add_error("field", "code", "message");
    assert!(!result.is_valid());
}

// ---------------------------------------------------------------------------
// 20. Validate complex form — CustomerForm
// ---------------------------------------------------------------------------

#[test]
fn validate_customer_form() {
    let schema = FormSchema::new("CustomerForm")
        .field(
            FieldSchema::new("name", FieldType::String)
                .required()
                .min_length(2)
                .max_length(120)
                .label("Full Name"),
        )
        .field(
            FieldSchema::new("email", FieldType::Email)
                .required()
                .label("Email Address"),
        )
        .field(
            FieldSchema::new("phone", FieldType::Optional(Box::new(FieldType::String)))
                .max_length(32)
                .label("Phone Number"),
        );

    // Valid submission.
    let valid_data = map_from(vec![
        ("name", json!("Saken Tukenov")),
        ("email", json!("saken@example.com")),
        ("phone", json!("+7 701 123 4567")),
    ]);
    let result = schema.validate(&valid_data);
    assert!(result.is_valid());

    // Valid submission without optional phone.
    let no_phone = map_from(vec![
        ("name", json!("Saken Tukenov")),
        ("email", json!("saken@example.com")),
    ]);
    let result = schema.validate(&no_phone);
    assert!(result.is_valid());

    // Invalid: name too short, email invalid, phone too long.
    let invalid_data = map_from(vec![
        ("name", json!("S")),
        ("email", json!("not-email")),
        ("phone", json!("a]".repeat(20))),
    ]);
    let result = schema.validate(&invalid_data);
    assert!(!result.is_valid());
    assert!(!result.field_errors("name").is_empty());
    assert!(!result.field_errors("email").is_empty());
    assert!(!result.field_errors("phone").is_empty());

    // Invalid: required fields missing.
    let empty_data = Map::new();
    let result = schema.validate(&empty_data);
    assert!(!result.is_valid());
    assert!(!result.field_errors("name").is_empty());
    assert!(!result.field_errors("email").is_empty());
    // phone is optional — no error expected.
    assert!(result.field_errors("phone").is_empty());
}
