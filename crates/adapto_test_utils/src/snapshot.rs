use serde_json::Value;

pub fn assert_json_eq(actual: &Value, expected: &Value) {
    assert_eq!(
        actual, expected,
        "JSON mismatch:\n  actual:   {}\n  expected: {}",
        serde_json::to_string_pretty(actual).unwrap(),
        serde_json::to_string_pretty(expected).unwrap(),
    );
}

pub fn assert_json_includes(actual: &Value, subset: &Value) {
    match (actual, subset) {
        (Value::Object(a), Value::Object(s)) => {
            for (key, expected_val) in s {
                let actual_val = a.get(key).unwrap_or_else(|| {
                    panic!(
                        "Expected key {:?} not found in actual object.\nActual keys: {:?}",
                        key,
                        a.keys().collect::<Vec<_>>(),
                    )
                });
                assert_json_includes(actual_val, expected_val);
            }
        }
        (Value::Array(a), Value::Array(s)) => {
            for (i, expected_item) in s.iter().enumerate() {
                let actual_item = a.get(i).unwrap_or_else(|| {
                    panic!(
                        "Expected array index {} not found. Actual array has {} items.",
                        i,
                        a.len(),
                    )
                });
                assert_json_includes(actual_item, expected_item);
            }
        }
        _ => assert_eq!(
            actual, subset,
            "Value mismatch:\n  actual:   {}\n  expected: {}",
            actual, subset,
        ),
    }
}

pub fn assert_json_shape(value: &Value, shape: &[&str]) {
    let obj = value
        .as_object()
        .expect("assert_json_shape requires a JSON object");
    for field in shape {
        assert!(
            obj.contains_key(*field),
            "Expected field {:?} not found. Available: {:?}",
            field,
            obj.keys().collect::<Vec<_>>(),
        );
    }
}

pub fn assert_json_array_len(value: &Value, expected: usize) {
    let arr = value
        .as_array()
        .expect("assert_json_array_len requires a JSON array");
    assert_eq!(
        arr.len(),
        expected,
        "Expected array length {}, got {}",
        expected,
        arr.len(),
    );
}

pub fn json_diff(a: &Value, b: &Value) -> Vec<String> {
    let mut diffs = Vec::new();
    json_diff_inner(a, b, "", &mut diffs);
    diffs
}

fn json_diff_inner(a: &Value, b: &Value, path: &str, diffs: &mut Vec<String>) {
    match (a, b) {
        (Value::Object(ma), Value::Object(mb)) => {
            for key in ma.keys().chain(mb.keys()).collect::<std::collections::BTreeSet<_>>() {
                let child_path = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{}.{}", path, key)
                };
                match (ma.get(key), mb.get(key)) {
                    (Some(va), Some(vb)) => json_diff_inner(va, vb, &child_path, diffs),
                    (Some(va), None) => diffs.push(format!("{}: only in left = {}", child_path, va)),
                    (None, Some(vb)) => diffs.push(format!("{}: only in right = {}", child_path, vb)),
                    (None, None) => {}
                }
            }
        }
        (Value::Array(aa), Value::Array(ab)) => {
            let max_len = aa.len().max(ab.len());
            for i in 0..max_len {
                let child_path = format!("{}[{}]", path, i);
                match (aa.get(i), ab.get(i)) {
                    (Some(va), Some(vb)) => json_diff_inner(va, vb, &child_path, diffs),
                    (Some(va), None) => diffs.push(format!("{}: only in left = {}", child_path, va)),
                    (None, Some(vb)) => diffs.push(format!("{}: only in right = {}", child_path, vb)),
                    (None, None) => {}
                }
            }
        }
        _ => {
            if a != b {
                diffs.push(format!("{}: {} != {}", path, a, b));
            }
        }
    }
}
