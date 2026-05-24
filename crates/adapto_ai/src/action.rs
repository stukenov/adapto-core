use crate::error::AiError;
use crate::model::ModelConfig;
use crate::pii::PiiPolicy;
use adapto_runtime::types::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Defines a single AI-powered action: which model to call,
/// how to handle PII, retry policy, and schema constraints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiActionDef {
    pub name: String,
    pub model: String,
    pub fallback: Option<String>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u32>,
    pub audit: bool,
    pub pii: Option<PiiPolicy>,
    pub permission: Option<String>,
    pub input_schema: Option<String>,
    pub output_schema: Option<String>,
    pub prompt_template: Option<String>,
    pub timeout_ms: Option<u64>,
    pub max_retries: u32,
}

impl Default for AiActionDef {
    fn default() -> Self {
        Self {
            name: String::new(),
            model: "default".to_string(),
            fallback: None,
            temperature: Some(0.7),
            max_tokens: None,
            audit: true,
            pii: None,
            permission: None,
            input_schema: None,
            output_schema: None,
            prompt_template: None,
            timeout_ms: Some(30_000),
            max_retries: 2,
        }
    }
}

/// Inbound request to execute an AI action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiRequest {
    pub action: String,
    pub input: serde_json::Value,
    pub tenant_id: Option<TenantId>,
    pub user_id: Option<UserId>,
    pub request_id: RequestId,
}

/// Result of a successful AI action execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiResponse {
    pub output: serde_json::Value,
    pub model_used: String,
    pub tokens_used: TokenUsage,
    pub latency_ms: u64,
    pub trace_id: String,
}

/// Token consumption for a single request.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
}

impl TokenUsage {
    pub fn new(input_tokens: u32, output_tokens: u32) -> Self {
        Self {
            input_tokens,
            output_tokens,
            total_tokens: input_tokens + output_tokens,
        }
    }
}

impl std::ops::Add for TokenUsage {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            input_tokens: self.input_tokens + rhs.input_tokens,
            output_tokens: self.output_tokens + rhs.output_tokens,
            total_tokens: self.total_tokens + rhs.total_tokens,
        }
    }
}

/// Orchestrates the full AI action lifecycle: permission checks,
/// PII redaction, model routing, retry/fallback, budget tracking,
/// and audit logging.
pub struct AiActionExecutor {
    actions: HashMap<String, AiActionDef>,
    model_configs: HashMap<String, ModelConfig>,
}

impl AiActionExecutor {
    pub fn new() -> Self {
        Self {
            actions: HashMap::new(),
            model_configs: HashMap::new(),
        }
    }

    pub fn register_action(&mut self, def: AiActionDef) {
        self.actions.insert(def.name.clone(), def);
    }

    pub fn register_model(&mut self, config: ModelConfig) {
        self.model_configs.insert(config.name.clone(), config);
    }

    /// Execute an AI action by name.
    ///
    /// The execution pipeline:
    /// 1. Look up the action definition
    /// 2. Check permissions (when configured)
    /// 3. Apply PII redaction policy
    /// 4. Route to the configured model
    /// 5. Execute with retry and fallback
    /// 6. Validate the output schema
    /// 7. Track budget usage
    /// 8. Write audit trail
    ///
    /// Returns a mock response for now — the architecture and
    /// extension points are the deliverable, not a live LLM client.
    pub async fn execute(&self, request: AiRequest) -> Result<AiResponse, AiError> {
        let action_def = self
            .actions
            .get(&request.action)
            .ok_or_else(|| AiError::ActionNotFound(request.action.clone()))?;

        let model_name = &action_def.model;

        Ok(AiResponse {
            output: serde_json::json!({"result": "mock response"}),
            model_used: model_name.clone(),
            tokens_used: TokenUsage::new(100, 50),
            latency_ms: 200,
            trace_id: uuid::Uuid::new_v4().to_string(),
        })
    }
}

impl Default for AiActionExecutor {
    fn default() -> Self {
        Self::new()
    }
}
