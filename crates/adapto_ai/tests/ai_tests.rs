use adapto_ai::action::{AiActionDef, AiActionExecutor, AiRequest, AiResponse, TokenUsage};
use adapto_ai::pii::{PiiPolicy, PiiRedactor};
use adapto_ai::budget::{BudgetTracker, TenantBudget, BudgetError};
use adapto_ai::model::{ModelConfig, ModelProvider, ModelRouter};
use adapto_ai::trace::{TraceCollector, TraceStatus};
use adapto_ai::error::AiError;
use adapto_runtime::types::*;
use serde_json::json;
use uuid::Uuid;

// ===========================================================================
// AiActionDef defaults
// ===========================================================================

#[test]
fn action_def_defaults() {
    let def = AiActionDef::default();
    assert_eq!(def.model, "default");
    assert_eq!(def.temperature, Some(0.7));
    assert!(def.audit);
    assert_eq!(def.max_retries, 2);
    assert_eq!(def.timeout_ms, Some(30_000));
    assert!(def.name.is_empty());
    assert!(def.fallback.is_none());
    assert!(def.pii.is_none());
    assert!(def.permission.is_none());
    assert!(def.max_tokens.is_none());
    assert!(def.input_schema.is_none());
    assert!(def.output_schema.is_none());
    assert!(def.prompt_template.is_none());
}

// ===========================================================================
// AiActionExecutor
// ===========================================================================

#[tokio::test]
async fn executor_register_and_execute() {
    let mut executor = AiActionExecutor::new();
    let mut def = AiActionDef::default();
    def.name = "summarize".to_string();
    def.model = "gpt-4".to_string();
    executor.register_action(def);

    let request = AiRequest {
        action: "summarize".to_string(),
        input: json!({"text": "Hello world"}),
        tenant_id: None,
        user_id: None,
        request_id: RequestId::default(),
    };

    let response = executor.execute(request).await.unwrap();
    assert_eq!(response.model_used, "gpt-4");
    assert_eq!(response.tokens_used.total_tokens, 150);
    assert!(!response.trace_id.is_empty());
}

#[tokio::test]
async fn executor_action_not_found() {
    let executor = AiActionExecutor::new();
    let request = AiRequest {
        action: "nonexistent".to_string(),
        input: json!({}),
        tenant_id: None,
        user_id: None,
        request_id: RequestId::default(),
    };

    let err = executor.execute(request).await.unwrap_err();
    match err {
        AiError::ActionNotFound(name) => assert_eq!(name, "nonexistent"),
        other => panic!("Expected ActionNotFound, got: {other:?}"),
    }
}

#[tokio::test]
async fn executor_multiple_actions() {
    let mut executor = AiActionExecutor::new();

    let mut def1 = AiActionDef::default();
    def1.name = "summarize".to_string();
    def1.model = "gpt-4".to_string();
    executor.register_action(def1);

    let mut def2 = AiActionDef::default();
    def2.name = "translate".to_string();
    def2.model = "claude".to_string();
    executor.register_action(def2);

    let r1 = executor
        .execute(AiRequest {
            action: "summarize".to_string(),
            input: json!({}),
            tenant_id: None,
            user_id: None,
            request_id: RequestId::default(),
        })
        .await
        .unwrap();
    assert_eq!(r1.model_used, "gpt-4");

    let r2 = executor
        .execute(AiRequest {
            action: "translate".to_string(),
            input: json!({}),
            tenant_id: None,
            user_id: None,
            request_id: RequestId::default(),
        })
        .await
        .unwrap();
    assert_eq!(r2.model_used, "claude");
}

// ===========================================================================
// TokenUsage
// ===========================================================================

#[test]
fn token_usage_new() {
    let usage = TokenUsage::new(100, 50);
    assert_eq!(usage.input_tokens, 100);
    assert_eq!(usage.output_tokens, 50);
    assert_eq!(usage.total_tokens, 150);
}

#[test]
fn token_usage_default() {
    let usage = TokenUsage::default();
    assert_eq!(usage.input_tokens, 0);
    assert_eq!(usage.output_tokens, 0);
    assert_eq!(usage.total_tokens, 0);
}

#[test]
fn token_usage_add() {
    let a = TokenUsage::new(100, 50);
    let b = TokenUsage::new(200, 80);
    let sum = a + b;

    assert_eq!(sum.input_tokens, 300);
    assert_eq!(sum.output_tokens, 130);
    assert_eq!(sum.total_tokens, 430);
}

// ===========================================================================
// PiiRedactor
// ===========================================================================

#[test]
fn pii_redact_email() {
    let redactor = PiiRedactor::with_defaults();
    let result = redactor.redact("Contact me at alice@example.com please");
    assert_eq!(result.output, "Contact me at [EMAIL] please");
    assert_eq!(result.redacted_count, 1);
    assert!(result.redacted_types.contains(&"email".to_string()));
}

#[test]
fn pii_redact_phone() {
    let redactor = PiiRedactor::with_defaults();
    let result = redactor.redact("Call me at 555-123-4567");
    assert_eq!(result.output, "Call me at [PHONE]");
    assert_eq!(result.redacted_count, 1);
    assert!(result.redacted_types.contains(&"phone".to_string()));
}

#[test]
fn pii_redact_ssn() {
    let redactor = PiiRedactor::with_defaults();
    let result = redactor.redact("SSN: 123-45-6789");
    assert!(result.output.contains("[SSN]"));
    assert!(result.redacted_count >= 1);
}

#[test]
fn pii_redact_credit_card() {
    let redactor = PiiRedactor::with_defaults();
    let result = redactor.redact("CC: 4111 1111 1111 1111");
    assert!(result.output.contains("[CREDIT_CARD]"));
    assert!(result.redacted_count >= 1);
}

#[test]
fn pii_redact_all_types() {
    let redactor = PiiRedactor::with_defaults();
    let input = "SSN: 123-45-6789, CC: 4111 1111 1111 1111, \
                 email: bob@test.org, phone: 555.867.5309";
    let result = redactor.redact(input);

    assert!(result.output.contains("[SSN]"));
    assert!(result.output.contains("[CREDIT_CARD]"));
    assert!(result.output.contains("[EMAIL]"));
    assert!(result.output.contains("[PHONE]"));
    assert!(result.redacted_count >= 4);
}

#[test]
fn pii_redact_multiple_same_type() {
    let redactor = PiiRedactor::with_defaults();
    let result = redactor.redact("a@b.com and c@d.com and e@f.com");
    assert_eq!(result.redacted_count, 3);
    assert_eq!(result.redacted_types, vec!["email"]);
}

#[test]
fn pii_redact_no_matches() {
    let redactor = PiiRedactor::with_defaults();
    let result = redactor.redact("nothing sensitive here");
    assert_eq!(result.output, "nothing sensitive here");
    assert_eq!(result.redacted_count, 0);
    assert!(result.redacted_types.is_empty());
}

#[test]
fn pii_mask() {
    let redactor = PiiRedactor::with_defaults();
    let result = redactor.mask("Email: test@example.com");
    assert!(result.output.contains("****************"));
    assert_eq!(result.redacted_count, 1);
    assert!(!result.output.contains("test@example.com"));
}

#[test]
fn pii_mask_no_matches() {
    let redactor = PiiRedactor::with_defaults();
    let result = redactor.mask("plain text");
    assert_eq!(result.output, "plain text");
    assert_eq!(result.redacted_count, 0);
}

#[test]
fn pii_custom_pattern() {
    let mut redactor = PiiRedactor::new();
    redactor.add_pattern("api_key", r"sk-[a-zA-Z0-9]{32}", "[API_KEY]");
    let result = redactor.redact("key: sk-abcdefghijklmnopqrstuvwxyz012345");
    assert_eq!(result.output, "key: [API_KEY]");
    assert_eq!(result.redacted_count, 1);
    assert!(result.redacted_types.contains(&"api_key".to_string()));
}

// ===========================================================================
// BudgetTracker
// ===========================================================================

#[test]
fn budget_within_limits() {
    let tracker = BudgetTracker::new();
    let tenant = TenantId(Uuid::new_v4());

    tracker.set_budget(
        &tenant,
        TenantBudget {
            total_tokens: 10_000,
            used_tokens: 0,
            total_cost: 1.0,
            used_cost: 0.0,
            max_tokens_per_request: None,
        },
    );

    assert!(tracker.check_budget(&tenant, 500).is_ok());
}

#[test]
fn budget_token_exceeded() {
    let tracker = BudgetTracker::new();
    let tenant = TenantId(Uuid::new_v4());

    tracker.set_budget(
        &tenant,
        TenantBudget {
            total_tokens: 1_000,
            used_tokens: 900,
            total_cost: 10.0,
            used_cost: 0.0,
            max_tokens_per_request: None,
        },
    );

    let err = tracker.check_budget(&tenant, 200).unwrap_err();
    match err {
        BudgetError::TokenBudgetExceeded { used, total } => {
            assert_eq!(used, 900);
            assert_eq!(total, 1_000);
        }
        other => panic!("Expected TokenBudgetExceeded, got: {other:?}"),
    }
}

#[test]
fn budget_cost_exceeded() {
    let tracker = BudgetTracker::new();
    let tenant = TenantId(Uuid::new_v4());

    tracker.set_budget(
        &tenant,
        TenantBudget {
            total_tokens: 100_000,
            used_tokens: 0,
            total_cost: 1.0,
            used_cost: 1.0,
            max_tokens_per_request: None,
        },
    );

    let err = tracker.check_budget(&tenant, 10).unwrap_err();
    match err {
        BudgetError::CostBudgetExceeded { .. } => {}
        other => panic!("Expected CostBudgetExceeded, got: {other:?}"),
    }
}

#[test]
fn budget_no_budget_set() {
    let tracker = BudgetTracker::new();
    let tenant = TenantId(Uuid::new_v4());

    let err = tracker.check_budget(&tenant, 100).unwrap_err();
    match err {
        BudgetError::NoBudget => {}
        other => panic!("Expected NoBudget, got: {other:?}"),
    }
}

#[test]
fn budget_record_usage() {
    let tracker = BudgetTracker::new();
    let tenant = TenantId(Uuid::new_v4());

    tracker.set_budget(
        &tenant,
        TenantBudget {
            total_tokens: 10_000,
            used_tokens: 0,
            total_cost: 5.0,
            used_cost: 0.0,
            max_tokens_per_request: None,
        },
    );

    tracker.record_usage(&tenant, 500, 0.05);
    let usage = tracker.get_usage(&tenant).unwrap();
    assert_eq!(usage.used_tokens, 500);
    assert!((usage.used_cost - 0.05).abs() < 1e-10);
}

#[test]
fn budget_record_usage_accumulates() {
    let tracker = BudgetTracker::new();
    let tenant = TenantId(Uuid::new_v4());

    tracker.set_budget(
        &tenant,
        TenantBudget {
            total_tokens: 10_000,
            used_tokens: 0,
            total_cost: 5.0,
            used_cost: 0.0,
            max_tokens_per_request: None,
        },
    );

    tracker.record_usage(&tenant, 100, 0.01);
    tracker.record_usage(&tenant, 200, 0.02);
    tracker.record_usage(&tenant, 300, 0.03);

    let usage = tracker.get_usage(&tenant).unwrap();
    assert_eq!(usage.used_tokens, 600);
    assert!((usage.used_cost - 0.06).abs() < 1e-10);
}

#[test]
fn budget_reset() {
    let tracker = BudgetTracker::new();
    let tenant = TenantId(Uuid::new_v4());

    tracker.set_budget(
        &tenant,
        TenantBudget {
            total_tokens: 10_000,
            used_tokens: 5_000,
            total_cost: 5.0,
            used_cost: 2.5,
            max_tokens_per_request: None,
        },
    );

    tracker.reset(&tenant);
    let usage = tracker.get_usage(&tenant).unwrap();
    assert_eq!(usage.used_tokens, 0);
    assert!(usage.used_cost.abs() < 1e-10);
    // Limits preserved
    assert_eq!(usage.total_tokens, 10_000);
    assert!((usage.total_cost - 5.0).abs() < 1e-10);
}

#[test]
fn budget_get_usage_no_tenant() {
    let tracker = BudgetTracker::new();
    let tenant = TenantId(Uuid::new_v4());
    assert!(tracker.get_usage(&tenant).is_none());
}

// ===========================================================================
// ModelRouter
// ===========================================================================

fn gpt4_config() -> ModelConfig {
    ModelConfig {
        name: "gpt-4".to_string(),
        provider: ModelProvider::OpenAI,
        endpoint: None,
        api_key_env: Some("OPENAI_API_KEY".to_string()),
        max_tokens: Some(4096),
        default_temperature: 0.7,
        cost_per_1k_input_tokens: 0.03,
        cost_per_1k_output_tokens: 0.06,
    }
}

fn gpt35_config() -> ModelConfig {
    ModelConfig {
        name: "gpt-3.5".to_string(),
        provider: ModelProvider::OpenAI,
        endpoint: None,
        api_key_env: None,
        max_tokens: None,
        default_temperature: 0.7,
        cost_per_1k_input_tokens: 0.001,
        cost_per_1k_output_tokens: 0.002,
    }
}

#[test]
fn model_router_add_and_resolve() {
    let mut router = ModelRouter::new();
    router.add_model(gpt4_config());

    let resolved = router.resolve("gpt-4").unwrap();
    assert_eq!(resolved.name, "gpt-4");
    assert_eq!(resolved.provider, ModelProvider::OpenAI);
}

#[test]
fn model_router_resolve_missing() {
    let router = ModelRouter::new();
    assert!(router.resolve("nonexistent").is_none());
}

#[test]
fn model_router_resolve_default() {
    let mut router = ModelRouter::new();
    router.add_model(gpt4_config());
    router.set_default("gpt-4");

    let resolved = router.resolve("default").unwrap();
    assert_eq!(resolved.name, "gpt-4");
}

#[test]
fn model_router_resolve_default_not_set() {
    let router = ModelRouter::new();
    assert!(router.resolve("default").is_none());
}

#[test]
fn model_router_fallback() {
    let mut router = ModelRouter::new();
    router.add_model(gpt4_config());
    router.add_model(gpt35_config());
    router.set_fallback("gpt-3.5");

    // Missing model falls back
    let resolved = router.resolve_with_fallback("claude-opus").unwrap();
    assert_eq!(resolved.name, "gpt-3.5");

    // Existing model returned directly
    let resolved = router.resolve_with_fallback("gpt-4").unwrap();
    assert_eq!(resolved.name, "gpt-4");
}

#[test]
fn model_router_fallback_not_set() {
    let router = ModelRouter::new();
    assert!(router.resolve_with_fallback("missing").is_none());
}

#[test]
fn model_router_estimate_cost() {
    let mut router = ModelRouter::new();
    router.add_model(gpt4_config());

    // 1000 input * $0.03/1k + 500 output * $0.06/1k = $0.03 + $0.03 = $0.06
    let cost = router.estimate_cost("gpt-4", 1000, 500).unwrap();
    assert!((cost - 0.06).abs() < 1e-10);
}

#[test]
fn model_router_estimate_cost_missing() {
    let router = ModelRouter::new();
    assert!(router.estimate_cost("nonexistent", 100, 100).is_none());
}

// ===========================================================================
// TraceCollector
// ===========================================================================

fn make_request(action: &str, tenant_id: Option<TenantId>) -> AiRequest {
    AiRequest {
        action: action.to_string(),
        input: json!({}),
        tenant_id,
        user_id: None,
        request_id: RequestId::default(),
    }
}

#[test]
fn trace_start_returns_started() {
    let collector = TraceCollector::new();
    let trace = collector.start_trace(&make_request("summarize", None), "gpt-4");
    assert_eq!(trace.status, TraceStatus::Started);
    assert_eq!(trace.action, "summarize");
    assert_eq!(trace.model, "gpt-4");
    assert!(trace.finished_at.is_none());
    assert!(trace.latency_ms.is_none());
    assert!(trace.tokens.is_none());
}

#[test]
fn trace_start_and_complete() {
    let collector = TraceCollector::new();
    let mut trace = collector.start_trace(&make_request("summarize", None), "gpt-4");

    trace.status = TraceStatus::Completed;
    trace.finished_at = Some(chrono::Utc::now());
    trace.latency_ms = Some(150);
    trace.tokens = Some(TokenUsage::new(100, 50));
    collector.complete_trace(trace);

    let traces = collector.get_traces();
    assert_eq!(traces.len(), 1);
    assert_eq!(traces[0].status, TraceStatus::Completed);
    assert!(traces[0].finished_at.is_some());
    assert_eq!(traces[0].latency_ms, Some(150));
}

#[test]
fn trace_get_multiple() {
    let collector = TraceCollector::new();
    for action in &["summarize", "translate", "classify"] {
        collector.start_trace(&make_request(action, None), "gpt-4");
    }
    assert_eq!(collector.get_traces().len(), 3);
}

#[test]
fn trace_filter_by_tenant() {
    let collector = TraceCollector::new();
    let tenant_a = TenantId(Uuid::new_v4());
    let tenant_b = TenantId(Uuid::new_v4());

    collector.start_trace(&make_request("summarize", Some(tenant_a.clone())), "gpt-4");
    collector.start_trace(&make_request("translate", Some(tenant_b.clone())), "gpt-4");
    collector.start_trace(&make_request("classify", None), "gpt-4");

    assert_eq!(collector.get_traces_for_tenant(&tenant_a).len(), 1);
    assert_eq!(collector.get_traces_for_tenant(&tenant_b).len(), 1);
    assert_eq!(collector.get_traces().len(), 3);
}

#[test]
fn trace_filter_by_tenant_empty() {
    let collector = TraceCollector::new();
    let tenant = TenantId(Uuid::new_v4());
    assert!(collector.get_traces_for_tenant(&tenant).is_empty());
}

// ===========================================================================
// Serialization roundtrips
// ===========================================================================

#[test]
fn ai_request_roundtrip() {
    let request = AiRequest {
        action: "summarize".to_string(),
        input: json!({"text": "hello"}),
        tenant_id: Some(TenantId(Uuid::nil())),
        user_id: Some(UserId(Uuid::nil())),
        request_id: RequestId(Uuid::nil()),
    };

    let json_str = serde_json::to_string(&request).unwrap();
    let deserialized: AiRequest = serde_json::from_str(&json_str).unwrap();
    assert_eq!(deserialized.action, "summarize");
    assert_eq!(deserialized.tenant_id, Some(TenantId(Uuid::nil())));
}

#[test]
fn ai_response_roundtrip() {
    let response = AiResponse {
        output: json!({"summary": "short"}),
        model_used: "gpt-4".to_string(),
        tokens_used: TokenUsage::new(100, 50),
        latency_ms: 200,
        trace_id: "abc-123".to_string(),
    };

    let json_str = serde_json::to_string(&response).unwrap();
    let deserialized: AiResponse = serde_json::from_str(&json_str).unwrap();
    assert_eq!(deserialized.model_used, "gpt-4");
    assert_eq!(deserialized.tokens_used.total_tokens, 150);
    assert_eq!(deserialized.latency_ms, 200);
}

// ===========================================================================
// Error display
// ===========================================================================

#[test]
fn ai_error_display() {
    assert_eq!(
        AiError::ActionNotFound("test".into()).to_string(),
        "AI action not found: test"
    );
    assert_eq!(
        AiError::ModelNotFound("gpt-5".into()).to_string(),
        "Model not found: gpt-5"
    );
    assert_eq!(
        AiError::Timeout(5000).to_string(),
        "AI action timed out after 5000ms"
    );
    assert_eq!(
        AiError::RetriesExhausted.to_string(),
        "All retries exhausted"
    );
}
