use adapto_ai::action::*;
use adapto_ai::budget::*;
use adapto_ai::error::AiError;
use adapto_ai::model::*;
use adapto_ai::pii::*;
use adapto_ai::trace::*;
use adapto_runtime::types::*;
use uuid::Uuid;

// -----------------------------------------------------------------------
// 1. AiActionDef default values
// -----------------------------------------------------------------------

#[test]
fn action_def_defaults() {
    let def = AiActionDef::default();
    assert_eq!(def.model, "default");
    assert_eq!(def.temperature, Some(0.7));
    assert!(def.audit);
    assert_eq!(def.max_retries, 2);
    assert_eq!(def.timeout_ms, Some(30_000));
    assert!(def.fallback.is_none());
    assert!(def.pii.is_none());
    assert!(def.permission.is_none());
}

// -----------------------------------------------------------------------
// 2. Register and execute action (mock)
// -----------------------------------------------------------------------

#[tokio::test]
async fn register_and_execute_action() {
    let mut executor = AiActionExecutor::new();
    let mut def = AiActionDef::default();
    def.name = "summarize".to_string();
    def.model = "gpt-4".to_string();
    executor.register_action(def);

    let request = AiRequest {
        action: "summarize".to_string(),
        input: serde_json::json!({"text": "Hello world"}),
        tenant_id: None,
        user_id: None,
        request_id: RequestId::default(),
    };

    let response = executor.execute(request).await.unwrap();
    assert_eq!(response.model_used, "gpt-4");
    assert_eq!(response.tokens_used.total_tokens, 150);
    assert!(!response.trace_id.is_empty());
}

// -----------------------------------------------------------------------
// 3. Action not found error
// -----------------------------------------------------------------------

#[tokio::test]
async fn action_not_found() {
    let executor = AiActionExecutor::new();
    let request = AiRequest {
        action: "nonexistent".to_string(),
        input: serde_json::json!({}),
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

// -----------------------------------------------------------------------
// 4. ModelRouter resolve default
// -----------------------------------------------------------------------

#[test]
fn model_router_resolve_default() {
    let mut router = ModelRouter::new();
    router.add_model(ModelConfig {
        name: "gpt-4".to_string(),
        provider: ModelProvider::OpenAI,
        endpoint: None,
        api_key_env: Some("OPENAI_API_KEY".to_string()),
        max_tokens: Some(4096),
        default_temperature: 0.7,
        cost_per_1k_input_tokens: 0.03,
        cost_per_1k_output_tokens: 0.06,
    });
    router.set_default("gpt-4");

    let resolved = router.resolve("default").unwrap();
    assert_eq!(resolved.name, "gpt-4");
}

// -----------------------------------------------------------------------
// 5. ModelRouter resolve fallback
// -----------------------------------------------------------------------

#[test]
fn model_router_resolve_fallback() {
    let mut router = ModelRouter::new();
    router.add_model(ModelConfig {
        name: "gpt-4".to_string(),
        provider: ModelProvider::OpenAI,
        endpoint: None,
        api_key_env: None,
        max_tokens: None,
        default_temperature: 0.7,
        cost_per_1k_input_tokens: 0.03,
        cost_per_1k_output_tokens: 0.06,
    });
    router.add_model(ModelConfig {
        name: "gpt-3.5".to_string(),
        provider: ModelProvider::OpenAI,
        endpoint: None,
        api_key_env: None,
        max_tokens: None,
        default_temperature: 0.7,
        cost_per_1k_input_tokens: 0.001,
        cost_per_1k_output_tokens: 0.002,
    });
    router.set_fallback("gpt-3.5");

    // Request a model that doesn't exist — should fall back
    let resolved = router.resolve_with_fallback("claude-opus").unwrap();
    assert_eq!(resolved.name, "gpt-3.5");

    // Request a model that exists — should return it directly
    let resolved = router.resolve_with_fallback("gpt-4").unwrap();
    assert_eq!(resolved.name, "gpt-4");
}

// -----------------------------------------------------------------------
// 6. ModelRouter estimate_cost
// -----------------------------------------------------------------------

#[test]
fn model_router_estimate_cost() {
    let mut router = ModelRouter::new();
    router.add_model(ModelConfig {
        name: "gpt-4".to_string(),
        provider: ModelProvider::OpenAI,
        endpoint: None,
        api_key_env: None,
        max_tokens: None,
        default_temperature: 0.7,
        cost_per_1k_input_tokens: 0.03,
        cost_per_1k_output_tokens: 0.06,
    });

    // 1000 input tokens * $0.03/1k + 500 output tokens * $0.06/1k
    // = $0.03 + $0.03 = $0.06
    let cost = router.estimate_cost("gpt-4", 1000, 500).unwrap();
    assert!((cost - 0.06).abs() < 1e-10);

    assert!(router.estimate_cost("nonexistent", 100, 100).is_none());
}

// -----------------------------------------------------------------------
// 7. PiiRedactor redact email
// -----------------------------------------------------------------------

#[test]
fn pii_redact_email() {
    let redactor = PiiRedactor::with_defaults();
    let result = redactor.redact("Contact me at alice@example.com please");
    assert_eq!(result.output, "Contact me at [EMAIL] please");
    assert_eq!(result.redacted_count, 1);
    assert!(result.redacted_types.contains(&"email".to_string()));
}

// -----------------------------------------------------------------------
// 8. PiiRedactor redact phone
// -----------------------------------------------------------------------

#[test]
fn pii_redact_phone() {
    let redactor = PiiRedactor::with_defaults();
    let result = redactor.redact("Call me at 555-123-4567");
    assert_eq!(result.output, "Call me at [PHONE]");
    assert_eq!(result.redacted_count, 1);
    assert!(result.redacted_types.contains(&"phone".to_string()));
}

// -----------------------------------------------------------------------
// 9. PiiRedactor mask
// -----------------------------------------------------------------------

#[test]
fn pii_mask() {
    let redactor = PiiRedactor::with_defaults();
    let result = redactor.mask("Email: test@example.com");
    // "test@example.com" is 16 chars
    assert!(result.output.contains("****************"));
    assert_eq!(result.redacted_count, 1);
}

// -----------------------------------------------------------------------
// 10. PiiRedactor with_defaults
// -----------------------------------------------------------------------

#[test]
fn pii_with_defaults_handles_all_types() {
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

// -----------------------------------------------------------------------
// 11. RedactionResult count
// -----------------------------------------------------------------------

#[test]
fn pii_redaction_count_multiple() {
    let redactor = PiiRedactor::with_defaults();
    let result = redactor.redact("a@b.com and c@d.com and e@f.com");
    assert_eq!(result.redacted_count, 3);
    assert_eq!(result.redacted_types, vec!["email"]);
}

// -----------------------------------------------------------------------
// 12. BudgetTracker set and check (within budget)
// -----------------------------------------------------------------------

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

// -----------------------------------------------------------------------
// 13. BudgetTracker check exceeds token budget
// -----------------------------------------------------------------------

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

// -----------------------------------------------------------------------
// 14. BudgetTracker check exceeds cost budget
// -----------------------------------------------------------------------

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
            used_cost: 1.0, // already at limit
            max_tokens_per_request: None,
        },
    );

    let err = tracker.check_budget(&tenant, 10).unwrap_err();
    match err {
        BudgetError::CostBudgetExceeded { .. } => {}
        other => panic!("Expected CostBudgetExceeded, got: {other:?}"),
    }
}

// -----------------------------------------------------------------------
// 15. BudgetTracker record_usage
// -----------------------------------------------------------------------

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

// -----------------------------------------------------------------------
// 16. BudgetTracker reset
// -----------------------------------------------------------------------

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
    assert!((usage.used_cost).abs() < 1e-10);
    // Limits should remain unchanged
    assert_eq!(usage.total_tokens, 10_000);
}

// -----------------------------------------------------------------------
// 17. TraceCollector start and complete trace
// -----------------------------------------------------------------------

#[test]
fn trace_start_and_complete() {
    let collector = TraceCollector::new();
    let request = AiRequest {
        action: "summarize".to_string(),
        input: serde_json::json!({}),
        tenant_id: None,
        user_id: None,
        request_id: RequestId::default(),
    };

    let mut trace = collector.start_trace(&request, "gpt-4");
    assert_eq!(trace.status, TraceStatus::Started);
    assert!(trace.finished_at.is_none());

    trace.status = TraceStatus::Completed;
    trace.finished_at = Some(chrono::Utc::now());
    trace.latency_ms = Some(150);
    trace.tokens = Some(TokenUsage::new(100, 50));
    collector.complete_trace(trace);

    let traces = collector.get_traces();
    assert_eq!(traces.len(), 1);
    assert_eq!(traces[0].status, TraceStatus::Completed);
    assert!(traces[0].finished_at.is_some());
}

// -----------------------------------------------------------------------
// 18. TraceCollector get_traces
// -----------------------------------------------------------------------

#[test]
fn trace_get_multiple() {
    let collector = TraceCollector::new();

    for action_name in &["summarize", "translate", "classify"] {
        let request = AiRequest {
            action: action_name.to_string(),
            input: serde_json::json!({}),
            tenant_id: None,
            user_id: None,
            request_id: RequestId::default(),
        };
        collector.start_trace(&request, "gpt-4");
    }

    assert_eq!(collector.get_traces().len(), 3);
}

// -----------------------------------------------------------------------
// 19. TraceCollector get_traces_for_tenant
// -----------------------------------------------------------------------

#[test]
fn trace_filter_by_tenant() {
    let collector = TraceCollector::new();
    let tenant_a = TenantId(Uuid::new_v4());
    let tenant_b = TenantId(Uuid::new_v4());

    let req_a = AiRequest {
        action: "summarize".to_string(),
        input: serde_json::json!({}),
        tenant_id: Some(tenant_a.clone()),
        user_id: None,
        request_id: RequestId::default(),
    };
    let req_b = AiRequest {
        action: "translate".to_string(),
        input: serde_json::json!({}),
        tenant_id: Some(tenant_b.clone()),
        user_id: None,
        request_id: RequestId::default(),
    };
    let req_none = AiRequest {
        action: "classify".to_string(),
        input: serde_json::json!({}),
        tenant_id: None,
        user_id: None,
        request_id: RequestId::default(),
    };

    collector.start_trace(&req_a, "gpt-4");
    collector.start_trace(&req_b, "gpt-4");
    collector.start_trace(&req_none, "gpt-4");

    assert_eq!(collector.get_traces_for_tenant(&tenant_a).len(), 1);
    assert_eq!(collector.get_traces_for_tenant(&tenant_b).len(), 1);
    assert_eq!(collector.get_traces().len(), 3);
}

// -----------------------------------------------------------------------
// 20. AiRequest serialization
// -----------------------------------------------------------------------

#[test]
fn ai_request_roundtrip() {
    let request = AiRequest {
        action: "summarize".to_string(),
        input: serde_json::json!({"text": "hello"}),
        tenant_id: Some(TenantId(Uuid::nil())),
        user_id: Some(UserId(Uuid::nil())),
        request_id: RequestId(Uuid::nil()),
    };

    let json = serde_json::to_string(&request).unwrap();
    let deserialized: AiRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.action, "summarize");
    assert_eq!(deserialized.tenant_id, Some(TenantId(Uuid::nil())));
}

// -----------------------------------------------------------------------
// 21. AiResponse serialization
// -----------------------------------------------------------------------

#[test]
fn ai_response_roundtrip() {
    let response = AiResponse {
        output: serde_json::json!({"summary": "short"}),
        model_used: "gpt-4".to_string(),
        tokens_used: TokenUsage::new(100, 50),
        latency_ms: 200,
        trace_id: "abc-123".to_string(),
    };

    let json = serde_json::to_string(&response).unwrap();
    let deserialized: AiResponse = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.model_used, "gpt-4");
    assert_eq!(deserialized.tokens_used.total_tokens, 150);
    assert_eq!(deserialized.latency_ms, 200);
}

// -----------------------------------------------------------------------
// 22. TokenUsage arithmetic
// -----------------------------------------------------------------------

#[test]
fn token_usage_add() {
    let a = TokenUsage::new(100, 50);
    let b = TokenUsage::new(200, 80);
    let sum = a + b;

    assert_eq!(sum.input_tokens, 300);
    assert_eq!(sum.output_tokens, 130);
    assert_eq!(sum.total_tokens, 430);
}

#[test]
fn token_usage_default() {
    let usage = TokenUsage::default();
    assert_eq!(usage.input_tokens, 0);
    assert_eq!(usage.output_tokens, 0);
    assert_eq!(usage.total_tokens, 0);
}
