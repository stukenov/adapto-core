use adapto_forms::error::FormError;
use adapto_forms::schema::{Constraint, FieldSchema, FieldType, FormSchema};
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

// ---------------------------------------------------------------------------
// 21. Pattern constraint — pass
// ---------------------------------------------------------------------------

#[test]
fn validate_pattern_contains_pass() {
    let field = FieldSchema::new("code", FieldType::String).pattern("ABC");
    let errors = validate_field(&field, Some(&json!("xyzABCdef")));
    assert!(errors.is_empty());
}

// ---------------------------------------------------------------------------
// 22. Pattern constraint — fail
// ---------------------------------------------------------------------------

#[test]
fn validate_pattern_contains_fail() {
    let field = FieldSchema::new("code", FieldType::String).pattern("ABC");
    let errors = validate_field(&field, Some(&json!("xyz")));
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, "pattern");
}

// ---------------------------------------------------------------------------
// 23. Pattern constraint — anchored exact match
// ---------------------------------------------------------------------------

#[test]
fn validate_pattern_anchored_exact() {
    let field = FieldSchema::new("code", FieldType::String).pattern("^hello$");
    assert!(validate_field(&field, Some(&json!("hello"))).is_empty());
    assert_eq!(validate_field(&field, Some(&json!("hello world"))).len(), 1);
}

// ---------------------------------------------------------------------------
// 24. Integer min/max constraints
// ---------------------------------------------------------------------------

#[test]
fn validate_integer_min_max() {
    let field = FieldSchema::new("age", FieldType::Integer).min(0).max(150);

    assert!(validate_field(&field, Some(&json!(25))).is_empty());
    assert!(validate_field(&field, Some(&json!(0))).is_empty());
    assert!(validate_field(&field, Some(&json!(150))).is_empty());

    let errors = validate_field(&field, Some(&json!(-1)));
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, "min");

    let errors = validate_field(&field, Some(&json!(151)));
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, "max");
}

// ---------------------------------------------------------------------------
// 25. DateTime validation
// ---------------------------------------------------------------------------

#[test]
fn validate_datetime_valid() {
    let field = FieldSchema::new("created_at", FieldType::DateTime);
    assert!(validate_field(&field, Some(&json!("2024-01-15T10:30:00Z"))).is_empty());
    assert!(validate_field(&field, Some(&json!("2024-01-15T10:30:00+05:00"))).is_empty());
}

#[test]
fn validate_datetime_invalid() {
    let field = FieldSchema::new("created_at", FieldType::DateTime);
    let errors = validate_field(&field, Some(&json!("not-a-date")));
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, "invalid_datetime");

    let errors = validate_field(&field, Some(&json!(12345)));
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, "invalid_type");
}

// ---------------------------------------------------------------------------
// 26. Required field with empty string
// ---------------------------------------------------------------------------

#[test]
fn validate_required_field_empty_string_passes_type_check() {
    // Empty string is still a present string — passes required + type check.
    // Use min_length constraint to reject empty strings.
    let field = FieldSchema::new("name", FieldType::String).required();
    let errors = validate_field(&field, Some(&json!("")));
    assert!(errors.is_empty());
}

#[test]
fn validate_required_field_empty_string_fails_min_length() {
    let field = FieldSchema::new("name", FieldType::String).required().min_length(1);
    let errors = validate_field(&field, Some(&json!("")));
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, "min_length");
}

// ---------------------------------------------------------------------------
// 27. Required field with null value
// ---------------------------------------------------------------------------

#[test]
fn validate_required_field_null_value() {
    let field = FieldSchema::new("name", FieldType::String).required();
    let errors = validate_field(&field, Some(&Value::Null));
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, "required");
}

// ---------------------------------------------------------------------------
// 28. Special characters in string fields
// ---------------------------------------------------------------------------

#[test]
fn validate_string_special_characters() {
    let field = FieldSchema::new("bio", FieldType::String);
    let specials = vec![
        json!("<script>alert('xss')</script>"),
        json!("line1\nline2"),
        json!("tab\there"),
        json!("\u{0000}null byte"),
        json!("emoji \u{1F600}"),
    ];
    for val in &specials {
        let errors = validate_field(&field, Some(val));
        assert!(errors.is_empty(), "should accept special chars: {}", val);
    }
}

// ---------------------------------------------------------------------------
// 29. Constraint builder method
// ---------------------------------------------------------------------------

#[test]
fn field_schema_constraint_builder() {
    let field = FieldSchema::new("token", FieldType::String)
        .constraint(Constraint::MinLength(8))
        .constraint(Constraint::MaxLength(64))
        .constraint(Constraint::Unique);

    assert_eq!(field.constraints.len(), 3);
}

// ---------------------------------------------------------------------------
// 30. Email field type validation (via validate_field)
// ---------------------------------------------------------------------------

#[test]
fn validate_email_field_type() {
    let field = FieldSchema::new("email", FieldType::Email).required();
    assert!(validate_field(&field, Some(&json!("user@example.com"))).is_empty());

    let errors = validate_field(&field, Some(&json!("bad-email")));
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, "invalid_email");

    // Non-string value
    let errors = validate_field(&field, Some(&json!(42)));
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, "invalid_type");
}

// ---------------------------------------------------------------------------
// 31. UUID non-string value
// ---------------------------------------------------------------------------

#[test]
fn validate_uuid_non_string() {
    let field = FieldSchema::new("id", FieldType::Uuid);
    let errors = validate_field(&field, Some(&json!(12345)));
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, "invalid_type");
}

// ---------------------------------------------------------------------------
// 32. Enum non-string value
// ---------------------------------------------------------------------------

#[test]
fn validate_enum_non_string() {
    let field = FieldSchema::new("role", FieldType::Enum(vec!["admin".into(), "user".into()]));
    let errors = validate_field(&field, Some(&json!(1)));
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, "invalid_type");
}

// ---------------------------------------------------------------------------
// 33. FormError display
// ---------------------------------------------------------------------------

#[test]
fn form_error_display() {
    assert_eq!(
        FormError::InvalidSchema("missing name".into()).to_string(),
        "Invalid schema: missing name"
    );
    assert_eq!(
        FormError::ValidationFailed(3).to_string(),
        "Validation failed: 3 error(s)"
    );
    assert_eq!(
        FormError::UnknownField("foo".into()).to_string(),
        "Unknown field: foo"
    );
    assert_eq!(
        FormError::SerializationError("bad json".into()).to_string(),
        "Serialization error: bad json"
    );
}

// ---------------------------------------------------------------------------
// 34. ValidationResult all_errors aggregation
// ---------------------------------------------------------------------------

#[test]
fn validation_result_all_errors() {
    let mut result = ValidationResult::default();
    result.add_error("a", "c1", "m1");
    result.add_error("b", "c2", "m2");
    result.add_error("a", "c3", "m3");
    assert_eq!(result.all_errors().len(), 3);
    assert_eq!(result.field_errors("a").len(), 2);
    assert_eq!(result.field_errors("b").len(), 1);
    assert!(result.field_errors("nonexistent").is_empty());
}

// ---------------------------------------------------------------------------
// 35. Optional field with required flag — null still ok
// ---------------------------------------------------------------------------

#[test]
fn validate_optional_type_with_required_flag() {
    let field = FieldSchema::new("nick", FieldType::Optional(Box::new(FieldType::String))).required();
    // Optional type wrapper overrides required — null should be accepted.
    assert!(validate_field(&field, None).is_empty());
    assert!(validate_field(&field, Some(&Value::Null)).is_empty());
    // But present non-string should fail type check.
    let errors = validate_field(&field, Some(&json!(42)));
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, "invalid_type");
}
