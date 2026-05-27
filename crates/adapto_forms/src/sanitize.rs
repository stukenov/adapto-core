use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum Sanitizer {
    Trim,
    Lowercase,
    Uppercase,
    StripHtml,
    TruncateTo(usize),
}

impl Sanitizer {
    pub fn apply(&self, value: &Value) -> Value {
        match value {
            Value::String(s) => Value::String(self.apply_str(s)),
            other => other.clone(),
        }
    }

    fn apply_str(&self, s: &str) -> String {
        match self {
            Sanitizer::Trim => s.trim().to_string(),
            Sanitizer::Lowercase => s.to_lowercase(),
            Sanitizer::Uppercase => s.to_uppercase(),
            Sanitizer::StripHtml => strip_html_tags(s),
            Sanitizer::TruncateTo(max) => {
                if s.len() > *max {
                    s[..*max].to_string()
                } else {
                    s.to_string()
                }
            }
        }
    }
}

fn strip_html_tags(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut in_tag = false;
    for ch in input.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(ch),
            _ => {}
        }
    }
    result
}

pub struct SanitizerPipeline {
    field_sanitizers: HashMap<String, Vec<Sanitizer>>,
}

impl SanitizerPipeline {
    pub fn new() -> Self {
        Self {
            field_sanitizers: HashMap::new(),
        }
    }

    pub fn field(mut self, name: &str, sanitizers: Vec<Sanitizer>) -> Self {
        self.field_sanitizers.insert(name.into(), sanitizers);
        self
    }

    pub fn apply(&self, data: &mut serde_json::Map<String, Value>) {
        for (field, sanitizers) in &self.field_sanitizers {
            if let Some(value) = data.get(field).cloned() {
                let mut result = value;
                for sanitizer in sanitizers {
                    result = sanitizer.apply(&result);
                }
                data.insert(field.clone(), result);
            }
        }
    }

    pub fn apply_value(&self, data: &mut Value) {
        if let Some(map) = data.as_object_mut() {
            self.apply(map);
        }
    }
}

impl Default for SanitizerPipeline {
    fn default() -> Self {
        Self::new()
    }
}
