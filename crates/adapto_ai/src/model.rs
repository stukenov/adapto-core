use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for a single AI model endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub name: String,
    pub provider: ModelProvider,
    pub endpoint: Option<String>,
    pub api_key_env: Option<String>,
    pub max_tokens: Option<u32>,
    pub default_temperature: f64,
    pub cost_per_1k_input_tokens: f64,
    pub cost_per_1k_output_tokens: f64,
}

/// Supported model providers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ModelProvider {
    OpenAI,
    Anthropic,
    Custom(String),
    Local,
}

/// Routes model requests to the appropriate configuration,
/// with support for default and fallback models.
#[derive(Debug, Clone)]
pub struct ModelRouter {
    models: HashMap<String, ModelConfig>,
    default_model: Option<String>,
    fallback_model: Option<String>,
}

impl ModelRouter {
    pub fn new() -> Self {
        Self {
            models: HashMap::new(),
            default_model: None,
            fallback_model: None,
        }
    }

    pub fn add_model(&mut self, config: ModelConfig) {
        self.models.insert(config.name.clone(), config);
    }

    pub fn set_default(&mut self, name: &str) {
        self.default_model = Some(name.to_string());
    }

    pub fn set_fallback(&mut self, name: &str) {
        self.fallback_model = Some(name.to_string());
    }

    /// Resolve a model by name. If `"default"` is requested,
    /// returns the configured default model.
    pub fn resolve(&self, requested: &str) -> Option<&ModelConfig> {
        if requested == "default" {
            if let Some(ref default) = self.default_model {
                return self.models.get(default);
            }
            return None;
        }
        self.models.get(requested)
    }

    /// Resolve a model by name, falling back to the configured
    /// fallback model if the requested model is not found.
    pub fn resolve_with_fallback(&self, requested: &str) -> Option<&ModelConfig> {
        self.resolve(requested).or_else(|| {
            self.fallback_model
                .as_ref()
                .and_then(|fb| self.models.get(fb))
        })
    }

    /// Estimate the cost of a request given token counts.
    pub fn estimate_cost(
        &self,
        model_name: &str,
        input_tokens: u32,
        output_tokens: u32,
    ) -> Option<f64> {
        let config = self.models.get(model_name)?;
        let input_cost = (input_tokens as f64 / 1000.0) * config.cost_per_1k_input_tokens;
        let output_cost = (output_tokens as f64 / 1000.0) * config.cost_per_1k_output_tokens;
        Some(input_cost + output_cost)
    }
}

impl Default for ModelRouter {
    fn default() -> Self {
        Self::new()
    }
}
