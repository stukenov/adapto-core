# adapto_forms

Form validation for Adapto — schema-based validation, typed fields, constraints, cross-field rules, sanitizer pipeline.

Part of the [Adapto](https://github.com/stukenov/adapto-core) web framework.

## Features

- **Schema-based validation** — declarative form schemas with typed fields
- **Field types** — String, Email, Integer, Boolean, UUID, Float, Date, and more
- **Constraints** — required, min/max length, min/max value, pattern (regex)
- **Cross-field rules** — FieldsMatch, RequiredIf, MutuallyExclusive
- **Sanitizer pipeline** — trim, lowercase, strip HTML before validation
- **Error reporting** — per-field error messages with i18n support

## Quick Start

```toml
[dependencies]
adapto_forms = "0.2"
```

```rust
use adapto_forms::{FormSchema, FieldType, Constraint, Rule};
use serde_json::json;

let schema = FormSchema::new("registration")
    .field("email", FieldType::Email, &[Constraint::Required])
    .field("password", FieldType::String, &[
        Constraint::Required,
        Constraint::MinLength(8),
    ])
    .field("password_confirm", FieldType::String, &[Constraint::Required])
    .rule(Rule::FieldsMatch("password", "password_confirm"));

let data = json!({"email": "a@b.com", "password": "secret12", "password_confirm": "secret12"});
let result = schema.validate(&data)?;
```

## License

MIT — [Saken Tukenov](https://github.com/stukenov)
