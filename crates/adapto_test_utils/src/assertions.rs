use adapto_client_protocol::patch::*;
use adapto_forms::validation::ValidationResult;
use adapto_runtime::state::StateStore;

// ---------------------------------------------------------------------------
// Patch assertions
// ---------------------------------------------------------------------------

/// Assert that the server message contains a `ReplaceText` operation
/// targeting `target` with the expected `value`.
///
/// Panics with a descriptive message listing all ops if no match is found.
pub fn assert_patch_contains_text(msg: &ServerMessage, target: &str, value: &str) {
    let ops = extract_ops(msg);
    let found = ops.iter().any(|op| matches!(
        op,
        PatchOp::ReplaceText { target: t, value: v } if t == target && v == value
    ));
    assert!(
        found,
        "Expected a ReplaceText op for target {:?} with value {:?}, but none matched.\nOps: {:#?}",
        target, value, ops,
    );
}

/// Assert that the server message contains a `ReplaceHtml` operation
/// targeting `target`.
///
/// Panics with a descriptive message listing all ops if no match is found.
pub fn assert_patch_contains_html(msg: &ServerMessage, target: &str) {
    let ops = extract_ops(msg);
    let found = ops.iter().any(|op| matches!(
        op,
        PatchOp::ReplaceHtml { target: t, .. } if t == target
    ));
    assert!(
        found,
        "Expected a ReplaceHtml op for target {:?}, but none matched.\nOps: {:#?}",
        target, ops,
    );
}

/// Assert the total number of patch operations in the server message.
pub fn assert_patch_op_count(msg: &ServerMessage, expected: usize) {
    let ops = extract_ops(msg);
    assert_eq!(
        ops.len(),
        expected,
        "Expected {} patch ops, got {}.\nOps: {:#?}",
        expected,
        ops.len(),
        ops,
    );
}

/// Extract the ops vector from a `ServerMessage`, panicking if the
/// payload is not a `Patch`.
fn extract_ops(msg: &ServerMessage) -> &[PatchOp] {
    match &msg.payload {
        ServerPayload::Patch(patch) => &patch.ops,
        other => panic!(
            "Expected ServerPayload::Patch, got {:#?}",
            other,
        ),
    }
}

// ---------------------------------------------------------------------------
// State assertions
// ---------------------------------------------------------------------------

/// Assert that the state store contains `key` with a value equal to
/// `expected`.
pub fn assert_state_eq(store: &StateStore, key: &str, expected: &serde_json::Value) {
    let actual = store.get(key);
    assert_eq!(
        actual,
        Some(expected),
        "State key {:?}: expected {:?}, got {:?}",
        key, expected, actual,
    );
}

/// Assert that `key` is marked dirty in the state store.
pub fn assert_state_dirty(store: &StateStore, key: &str) {
    assert!(
        store.is_dirty(key),
        "Expected state key {:?} to be dirty, but it was clean",
        key,
    );
}

/// Assert that `key` is NOT marked dirty in the state store.
pub fn assert_state_clean(store: &StateStore, key: &str) {
    assert!(
        !store.is_dirty(key),
        "Expected state key {:?} to be clean, but it was dirty",
        key,
    );
}

// ---------------------------------------------------------------------------
// Validation assertions
// ---------------------------------------------------------------------------

/// Assert that the validation result contains no errors.
pub fn assert_validation_valid(result: &ValidationResult) {
    assert!(
        result.is_valid(),
        "Expected validation to pass, but got errors: {:#?}",
        result.errors,
    );
}

/// Assert that the validation result contains at least one error.
pub fn assert_validation_invalid(result: &ValidationResult) {
    assert!(
        !result.is_valid(),
        "Expected validation to fail, but it passed with no errors",
    );
}

/// Assert that a specific field has a specific error code.
pub fn assert_validation_error(result: &ValidationResult, field: &str, code: &str) {
    let field_errors = result.field_errors(field);
    let found = field_errors.iter().any(|e| e.code == code);
    assert!(
        found,
        "Expected field {:?} to have error code {:?}, but its errors were: {:#?}",
        field, code, field_errors,
    );
}

/// Assert that a specific field has no validation errors.
pub fn assert_no_validation_error(result: &ValidationResult, field: &str) {
    let field_errors = result.field_errors(field);
    assert!(
        field_errors.is_empty(),
        "Expected field {:?} to have no errors, but found: {:#?}",
        field, field_errors,
    );
}
