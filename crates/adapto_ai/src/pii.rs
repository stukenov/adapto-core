use serde::{Deserialize, Serialize};

/// Policy governing how PII is handled before sending to an AI model.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PiiPolicy {
    #[serde(rename = "none")]
    None,
    #[serde(rename = "redact")]
    Redact,
    #[serde(rename = "mask")]
    Mask,
    #[serde(rename = "hash")]
    Hash,
}

/// A named regex pattern used to detect PII.
#[derive(Debug, Clone)]
struct PiiPattern {
    name: String,
    regex: regex_lite::Regex,
    replacement: String,
}

/// Detects and removes PII from text using configurable regex patterns.
#[derive(Debug, Clone)]
pub struct PiiRedactor {
    patterns: Vec<PiiPattern>,
}

/// The outcome of a redaction or masking operation.
#[derive(Debug, Clone)]
pub struct RedactionResult {
    pub output: String,
    pub redacted_count: usize,
    pub redacted_types: Vec<String>,
}

impl PiiRedactor {
    pub fn new() -> Self {
        Self {
            patterns: Vec::new(),
        }
    }

    /// Create a redactor pre-loaded with common PII patterns:
    /// email, phone (US), SSN, and credit card numbers.
    pub fn with_defaults() -> Self {
        let mut redactor = Self::new();
        redactor.add_pattern(
            "email",
            r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}",
            "[EMAIL]",
        );
        redactor.add_pattern(
            "phone",
            r"\b\d{3}[-.]?\d{3}[-.]?\d{4}\b",
            "[PHONE]",
        );
        redactor.add_pattern("ssn", r"\b\d{3}-\d{2}-\d{4}\b", "[SSN]");
        redactor.add_pattern(
            "credit_card",
            r"\b\d{4}[-\s]?\d{4}[-\s]?\d{4}[-\s]?\d{4}\b",
            "[CREDIT_CARD]",
        );
        redactor
    }

    pub fn add_pattern(&mut self, name: &str, regex: &str, replacement: &str) {
        if let Ok(re) = regex_lite::Regex::new(regex) {
            self.patterns.push(PiiPattern {
                name: name.to_string(),
                regex: re,
                replacement: replacement.to_string(),
            });
        }
    }

    /// Replace all PII matches with their configured replacement tokens.
    pub fn redact(&self, input: &str) -> RedactionResult {
        let mut output = input.to_string();
        let mut redacted_count = 0usize;
        let mut redacted_types = Vec::new();

        for pattern in &self.patterns {
            let matches: Vec<_> = pattern.regex.find_iter(&output).collect();
            let count = matches.len();
            if count > 0 {
                redacted_count += count;
                redacted_types.push(pattern.name.clone());
                output = pattern.regex.replace_all(&output, &*pattern.replacement).to_string();
            }
        }

        RedactionResult {
            output,
            redacted_count,
            redacted_types,
        }
    }

    /// Replace all PII matches with asterisks of equal length.
    pub fn mask(&self, input: &str) -> RedactionResult {
        let mut output = input.to_string();
        let mut redacted_count = 0usize;
        let mut redacted_types = Vec::new();

        for pattern in &self.patterns {
            let matches: Vec<_> = pattern.regex.find_iter(&output).collect();
            let count = matches.len();
            if count > 0 {
                redacted_count += count;
                redacted_types.push(pattern.name.clone());
                // Replace each match with asterisks of the same length
                output = pattern
                    .regex
                    .replace_all(&output, |caps: &regex_lite::Captures| {
                        "*".repeat(caps[0].len())
                    })
                    .to_string();
            }
        }

        RedactionResult {
            output,
            redacted_count,
            redacted_types,
        }
    }
}

impl Default for PiiRedactor {
    fn default() -> Self {
        Self::new()
    }
}
