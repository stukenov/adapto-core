//! Integration tests for the School AI example.
//!
//! Covers parsing, compilation, AI actions, model routing, PII redaction,
//! budget tracking, tracing, permissions, audit, state, and form validation.

use adapto_ai::action::*;
use adapto_ai::budget::*;
use adapto_ai::model::*;
use adapto_ai::pii::*;
use adapto_ai::trace::*;
use adapto_audit::event::*;
use adapto_audit::sink::{AuditSink, InMemoryAuditSink};
use adapto_auth::rbac::{RbacStore, Role};
use adapto_compiler::compiler::Compiler;
use adapto_forms::schema::*;
use adapto_runtime::context::{Ctx, PermissionSet};
use adapto_runtime::state::StateStore;
use adapto_runtime::types::*;
use serde_json::json;
use std::collections::HashSet;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// DSL shared across tests
// ---------------------------------------------------------------------------

const LESSON_DSL: &str = r#"
<route>
  path: "/lessons/[id]"
  layout: "school"
  auth: required
  tenant: required
  permission: "lessons.read"
</route>

<script lang="rust">
  prop id: Uuid

  state lesson: Lesson
  state transcript: String = ""
  state ai_summary: Option<String> = None

  load async fn load(ctx: Ctx) {
    lesson = LessonRepo::find(ctx.tenant_id, id).await?;
    transcript = lesson.transcript.clone();
  }

  #[permission("lessons.update")]
  #[audit("lesson.status.changed")]
  action async fn set_status(status: String, ctx: Ctx) {
    lesson.status = status;
  }
</script>

<template>
  <div>
    <h1>{lesson.title}</h1>
    <p>Status: {lesson.status}</p>
    <button on:click="set_status('done')">Mark Done</button>

    {#if ai_summary}
      <div>
        <h2>AI Summary</h2>
        <p>{ai_summary}</p>
      </div>
    {/if}
  </div>
</template>
"#;

// ---------------------------------------------------------------------------
// 1. Parse lesson tracker DSL
// ---------------------------------------------------------------------------

#[test]
fn test_parse_lesson_dsl() {
    let ast = adapto_parser::parse(LESSON_DSL).expect("Parse failed");

    let route = ast.route.as_ref().expect("Route block missing");
    assert_eq!(route.path.as_deref(), Some("/lessons/[id]"));
    assert_eq!(route.layout.as_deref(), Some("school"));
    assert_eq!(route.permission.as_deref(), Some("lessons.read"));

    let script = ast.script.as_ref().expect("Script block missing");
    assert_eq!(script.props.len(), 1);
    assert_eq!(script.props[0].name, "id");
    assert_eq!(script.states.len(), 3);
    assert_eq!(script.actions.len(), 1);
    assert_eq!(script.actions[0].name, "set_status");

    assert!(ast.template.is_some());
}

// ---------------------------------------------------------------------------
// 2. Compile lesson tracker
// ---------------------------------------------------------------------------

#[test]
fn test_compile_lesson_tracker() {
    let ast = adapto_parser::parse(LESSON_DSL).expect("Parse failed");
    let mut compiler = Compiler::new();
    let output = compiler
        .compile_file(&ast, "lessons/[id]/page.adapto")
        .expect("Compile failed");

    // The compiled component should have dynamic segments for template expressions
    assert!(
        !output.component_ir.dynamic_segments.is_empty(),
        "Expected dynamic segments from template expressions"
    );

    // State fields from the script block
    assert_eq!(output.component_ir.state_fields.len(), 3);

    // Actions from the script block
    assert_eq!(output.component_ir.actions.len(), 1);
    assert_eq!(output.component_ir.actions[0].name, "set_status");
    assert_eq!(
        output.component_ir.actions[0].permission.as_deref(),
        Some("lessons.update")
    );

    // Route IR
    let route = output.component_ir.route.as_ref().expect("Route IR missing");
    assert_eq!(route.path, "/lessons/[id]");

    // Route entry in the manifest
    assert!(output.route_entry.is_some());

    // Generated Rust code should be non-empty
    assert!(!output.generated_rust.is_empty());
}

// ---------------------------------------------------------------------------
// 3. AI action registration and execution
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_ai_action_execution() {
    let mut executor = AiActionExecutor::new();
    executor.register_action(AiActionDef {
        name: "summarize_lesson".into(),
        model: "soz-kz-600m".into(),
        fallback: Some("gpt-5.5-thinking".into()),
        temperature: Some(0.2),
        audit: true,
        pii: Some(PiiPolicy::Redact),
        permission: Some("lessons.ai.summarize".into()),
        ..Default::default()
    });
    executor.register_model(ModelConfig {
        name: "soz-kz-600m".into(),
        provider: ModelProvider::Custom("soz-kz".into()),
        endpoint: None,
        api_key_env: None,
        max_tokens: Some(4096),
        default_temperature: 0.2,
        cost_per_1k_input_tokens: 0.001,
        cost_per_1k_output_tokens: 0.002,
    });

    let request = AiRequest {
        action: "summarize_lesson".into(),
        input: json!({"transcript": "Today we learned about fractions."}),
        tenant_id: Some(TenantId(Uuid::new_v4())),
        user_id: Some(UserId(Uuid::new_v4())),
        request_id: RequestId::default(),
    };

    let response = executor.execute(request).await.expect("AI execution failed");
    assert_eq!(response.model_used, "soz-kz-600m");
    assert!(response.tokens_used.total_tokens > 0);
    assert!(response.latency_ms > 0);
    assert!(!response.trace_id.is_empty());
}

// ---------------------------------------------------------------------------
// 4. AI action not found
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_ai_action_not_found() {
    let executor = AiActionExecutor::new();
    let request = AiRequest {
        action: "nonexistent".into(),
        input: json!({}),
        tenant_id: None,
        user_id: None,
        request_id: RequestId::default(),
    };

    let result = executor.execute(request).await;
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// 5. Model router: default resolution
// ---------------------------------------------------------------------------

#[test]
fn test_model_router_default() {
    let mut router = ModelRouter::new();
    router.add_model(ModelConfig {
        name: "soz-kz-600m".into(),
        provider: ModelProvider::Custom("soz-kz".into()),
        endpoint: None,
        api_key_env: None,
        max_tokens: Some(4096),
        default_temperature: 0.2,
        cost_per_1k_input_tokens: 0.001,
        cost_per_1k_output_tokens: 0.002,
    });
    router.set_default("soz-kz-600m");

    let resolved = router.resolve("default").expect("Default model not found");
    assert_eq!(resolved.name, "soz-kz-600m");
    assert_eq!(
        resolved.provider,
        ModelProvider::Custom("soz-kz".into())
    );
}

// ---------------------------------------------------------------------------
// 6. Model router: fallback
// ---------------------------------------------------------------------------

#[test]
fn test_model_router_fallback() {
    let mut router = ModelRouter::new();
    router.add_model(ModelConfig {
        name: "gpt-5.5-thinking".into(),
        provider: ModelProvider::OpenAI,
        endpoint: None,
        api_key_env: Some("OPENAI_API_KEY".into()),
        max_tokens: Some(8192),
        default_temperature: 0.7,
        cost_per_1k_input_tokens: 0.01,
        cost_per_1k_output_tokens: 0.03,
    });
    router.set_fallback("gpt-5.5-thinking");

    // Request a model that doesn't exist -- should fall back
    let resolved = router
        .resolve_with_fallback("nonexistent-model")
        .expect("Fallback model not found");
    assert_eq!(resolved.name, "gpt-5.5-thinking");
    assert_eq!(resolved.provider, ModelProvider::OpenAI);
}

// ---------------------------------------------------------------------------
// 7. PII redaction: email
// ---------------------------------------------------------------------------

#[test]
fn test_pii_redaction_email() {
    let redactor = PiiRedactor::with_defaults();
    let result = redactor.redact("Contact john@school.kz for details.");
    assert!(result.output.contains("[EMAIL]"));
    assert!(!result.output.contains("john@school.kz"));
    assert!(result.redacted_types.contains(&"email".to_string()));
}

// ---------------------------------------------------------------------------
// 8. PII redaction: phone
// ---------------------------------------------------------------------------

#[test]
fn test_pii_redaction_phone() {
    let redactor = PiiRedactor::with_defaults();
    let result = redactor.redact("Call me at 555-123-4567 today.");
    assert!(result.output.contains("[PHONE]"));
    assert!(!result.output.contains("555-123-4567"));
    assert!(result.redacted_types.contains(&"phone".to_string()));
}

// ---------------------------------------------------------------------------
// 9. PII redaction: multiple items
// ---------------------------------------------------------------------------

#[test]
fn test_pii_redaction_multiple() {
    let redactor = PiiRedactor::with_defaults();
    let input = "Email: a@b.com, SSN: 123-45-6789, Phone: 555-000-1234";
    let result = redactor.redact(input);

    assert!(result.output.contains("[EMAIL]"));
    assert!(result.output.contains("[SSN]"));
    assert!(result.output.contains("[PHONE]"));
    assert!(result.redacted_count >= 3);
}

// ---------------------------------------------------------------------------
// 10. Budget tracker: within budget
// ---------------------------------------------------------------------------

#[test]
fn test_budget_within_limits() {
    let tracker = BudgetTracker::new();
    let tenant = TenantId(Uuid::new_v4());
    tracker.set_budget(
        &tenant,
        TenantBudget {
            total_tokens: 10_000,
            used_tokens: 0,
            total_cost: 5.0,
            used_cost: 0.0,
            max_tokens_per_request: Some(4096),
        },
    );

    assert!(tracker.check_budget(&tenant, 500).is_ok());
}

// ---------------------------------------------------------------------------
// 11. Budget tracker: exceeds budget
// ---------------------------------------------------------------------------

#[test]
fn test_budget_exceeded() {
    let tracker = BudgetTracker::new();
    let tenant = TenantId(Uuid::new_v4());
    tracker.set_budget(
        &tenant,
        TenantBudget {
            total_tokens: 1_000,
            used_tokens: 900,
            total_cost: 5.0,
            used_cost: 0.0,
            max_tokens_per_request: Some(4096),
        },
    );

    // 200 tokens would push past the 1000 limit
    let result = tracker.check_budget(&tenant, 200);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// 12. Budget tracking after usage
// ---------------------------------------------------------------------------

#[test]
fn test_budget_tracking_after_usage() {
    let tracker = BudgetTracker::new();
    let tenant = TenantId(Uuid::new_v4());
    tracker.set_budget(
        &tenant,
        TenantBudget {
            total_tokens: 10_000,
            used_tokens: 0,
            total_cost: 5.0,
            used_cost: 0.0,
            max_tokens_per_request: Some(4096),
        },
    );

    tracker.record_usage(&tenant, 150, 0.05);
    let usage = tracker.get_usage(&tenant).unwrap();
    assert_eq!(usage.used_tokens, 150);
    assert!((usage.used_cost - 0.05).abs() < f64::EPSILON);
    assert_eq!(usage.total_tokens - usage.used_tokens, 9_850);
}

// ---------------------------------------------------------------------------
// 13. Trace collector: start and complete
// ---------------------------------------------------------------------------

#[test]
fn test_trace_start_and_complete() {
    let collector = TraceCollector::new();
    let request = AiRequest {
        action: "summarize_lesson".into(),
        input: json!({}),
        tenant_id: Some(TenantId(Uuid::new_v4())),
        user_id: None,
        request_id: RequestId::default(),
    };

    let trace = collector.start_trace(&request, "soz-kz-600m");
    assert_eq!(trace.status, TraceStatus::Started);
    assert_eq!(trace.model, "soz-kz-600m");
    assert_eq!(trace.action, "summarize_lesson");

    // Complete the trace
    let mut completed = trace.clone();
    completed.status = TraceStatus::Completed;
    completed.latency_ms = Some(200);
    completed.tokens = Some(TokenUsage::new(100, 50));
    collector.complete_trace(completed);

    let all_traces = collector.get_traces();
    assert_eq!(all_traces.len(), 1);
    assert_eq!(all_traces[0].status, TraceStatus::Completed);
    assert_eq!(all_traces[0].latency_ms, Some(200));
}

// ---------------------------------------------------------------------------
// 14. Trace collector: filter by tenant
// ---------------------------------------------------------------------------

#[test]
fn test_trace_filter_by_tenant() {
    let collector = TraceCollector::new();
    let tenant_a = TenantId(Uuid::new_v4());
    let tenant_b = TenantId(Uuid::new_v4());

    collector.start_trace(
        &AiRequest {
            action: "summarize".into(),
            input: json!({}),
            tenant_id: Some(tenant_a.clone()),
            user_id: None,
            request_id: RequestId::default(),
        },
        "model-a",
    );
    collector.start_trace(
        &AiRequest {
            action: "translate".into(),
            input: json!({}),
            tenant_id: Some(tenant_b.clone()),
            user_id: None,
            request_id: RequestId::default(),
        },
        "model-b",
    );
    collector.start_trace(
        &AiRequest {
            action: "classify".into(),
            input: json!({}),
            tenant_id: Some(tenant_a.clone()),
            user_id: None,
            request_id: RequestId::default(),
        },
        "model-a",
    );

    let tenant_a_traces = collector.get_traces_for_tenant(&tenant_a);
    assert_eq!(tenant_a_traces.len(), 2);

    let tenant_b_traces = collector.get_traces_for_tenant(&tenant_b);
    assert_eq!(tenant_b_traces.len(), 1);
}

// ---------------------------------------------------------------------------
// 15. Permission check for AI action
// ---------------------------------------------------------------------------

#[test]
fn test_permission_check_for_ai_action() {
    let teacher_id = UserId(Uuid::new_v4());
    let student_id = UserId(Uuid::new_v4());

    let mut rbac = RbacStore::new();
    rbac.add_role(Role {
        name: "teacher".into(),
        permissions: HashSet::from([
            "lessons.read".into(),
            "lessons.update".into(),
            "lessons.ai.summarize".into(),
        ]),
    });
    rbac.add_role(Role {
        name: "student".into(),
        permissions: HashSet::from(["lessons.read".into()]),
    });
    rbac.assign_role(&teacher_id, "teacher");
    rbac.assign_role(&student_id, "student");

    let teacher_perms = rbac.get_permissions(&teacher_id);
    assert!(teacher_perms.has("lessons.ai.summarize"));
    assert!(teacher_perms.has("lessons.read"));
    assert!(teacher_perms.has("lessons.update"));

    let student_perms = rbac.get_permissions(&student_id);
    assert!(!student_perms.has("lessons.ai.summarize"));
    assert!(student_perms.has("lessons.read"));

    // Ctx-level permission check
    let teacher_ctx = Ctx {
        user_id: Some(teacher_id),
        tenant_id: Some(TenantId(Uuid::new_v4())),
        request_id: RequestId::default(),
        permissions: teacher_perms,
        route: RouteId("/lessons/123".into()),
        session_id: SessionId("sess-teacher".into()),
    };
    assert!(teacher_ctx.require("lessons.ai.summarize").is_ok());

    let student_ctx = Ctx {
        user_id: Some(student_id),
        tenant_id: Some(TenantId(Uuid::new_v4())),
        request_id: RequestId::default(),
        permissions: student_perms,
        route: RouteId("/lessons/123".into()),
        session_id: SessionId("sess-student".into()),
    };
    assert!(student_ctx.require("lessons.ai.summarize").is_err());
}

// ---------------------------------------------------------------------------
// 16. Audit event for AI action
// ---------------------------------------------------------------------------

#[test]
fn test_audit_event_for_ai_action() {
    let audit = InMemoryAuditSink::new();
    let ctx = Ctx {
        user_id: Some(UserId(Uuid::new_v4())),
        tenant_id: Some(TenantId(Uuid::new_v4())),
        request_id: RequestId::default(),
        permissions: PermissionSet::new(),
        route: RouteId("/lessons/456".into()),
        session_id: SessionId("sess-test".into()),
    };

    let event = AuditEvent::new("lesson.ai.summarized", &ctx, "summarize_lesson")
        .with_metadata("model", json!("soz-kz-600m"))
        .with_metadata("tokens", json!(150))
        .success();

    audit.write(event);
    assert_eq!(audit.len(), 1);

    let events = audit.events();
    assert_eq!(events[0].event, "lesson.ai.summarized");
    assert_eq!(events[0].action, "summarize_lesson");
    assert_eq!(events[0].status, AuditStatus::Success);
    assert_eq!(events[0].metadata.get("model"), Some(&json!("soz-kz-600m")));
    assert_eq!(events[0].metadata.get("tokens"), Some(&json!(150)));

    // Denied event
    let denied_event = AuditEvent::new("lesson.ai.summarized", &ctx, "summarize_lesson").denied();
    audit.write(denied_event);
    assert_eq!(audit.len(), 2);
    let events = audit.events();
    assert_eq!(events[1].status, AuditStatus::Denied);
}

// ---------------------------------------------------------------------------
// 17. State dirty tracking for ai_summary
// ---------------------------------------------------------------------------

#[test]
fn test_state_dirty_tracking() {
    let mut state = StateStore::new();
    state.set("lesson", json!({"title": "Algebra", "status": "active"}));
    state.set("ai_summary", json!(null));

    // Both keys should be dirty after initial set
    assert!(state.is_dirty("lesson"));
    assert!(state.is_dirty("ai_summary"));

    state.clear_dirty();
    assert!(!state.is_dirty("lesson"));
    assert!(!state.is_dirty("ai_summary"));

    // Simulate AI summary arriving
    state.set(
        "ai_summary",
        json!("The lesson covered quadratic equations."),
    );
    assert!(state.is_dirty("ai_summary"));
    assert!(!state.is_dirty("lesson"));

    // Value should be readable
    assert_eq!(
        state.get("ai_summary"),
        Some(&json!("The lesson covered quadratic equations."))
    );
}

// ---------------------------------------------------------------------------
// 18. Form validation for lesson data
// ---------------------------------------------------------------------------

#[test]
fn test_form_validation_lesson_data() {
    let lesson_form = FormSchema::new("lesson_form")
        .field(
            FieldSchema::new("title", FieldType::String)
                .required()
                .min_length(3)
                .max_length(200),
        )
        .field(
            FieldSchema::new(
                "status",
                FieldType::Enum(vec![
                    "draft".into(),
                    "active".into(),
                    "done".into(),
                    "archived".into(),
                ]),
            )
            .required(),
        )
        .field(FieldSchema::new("transcript", FieldType::String).required());

    // Valid data
    let valid = serde_json::from_value::<serde_json::Map<String, serde_json::Value>>(json!({
        "title": "Introduction to Algebra",
        "status": "active",
        "transcript": "Today we covered basic algebraic expressions..."
    }))
    .unwrap();
    let result = lesson_form.validate(&valid);
    assert!(result.is_valid());

    // Invalid: title too short
    let short_title = serde_json::from_value::<serde_json::Map<String, serde_json::Value>>(json!({
        "title": "Hi",
        "status": "active",
        "transcript": "Some content"
    }))
    .unwrap();
    let result = lesson_form.validate(&short_title);
    assert!(!result.is_valid());
    assert!(!result.field_errors("title").is_empty());

    // Invalid: unknown status enum value
    let bad_status = serde_json::from_value::<serde_json::Map<String, serde_json::Value>>(json!({
        "title": "Valid Title",
        "status": "unknown",
        "transcript": "Some content"
    }))
    .unwrap();
    let result = lesson_form.validate(&bad_status);
    assert!(!result.is_valid());
    assert!(!result.field_errors("status").is_empty());

    // Invalid: missing required field
    let missing = serde_json::from_value::<serde_json::Map<String, serde_json::Value>>(json!({
        "title": "Valid Title",
        "status": "active"
    }))
    .unwrap();
    let result = lesson_form.validate(&missing);
    assert!(!result.is_valid());
    assert!(!result.field_errors("transcript").is_empty());
}

// ---------------------------------------------------------------------------
// 19. Model cost estimation
// ---------------------------------------------------------------------------

#[test]
fn test_model_cost_estimation() {
    let mut router = ModelRouter::new();
    router.add_model(ModelConfig {
        name: "soz-kz-600m".into(),
        provider: ModelProvider::Custom("soz-kz".into()),
        endpoint: None,
        api_key_env: None,
        max_tokens: Some(4096),
        default_temperature: 0.2,
        cost_per_1k_input_tokens: 0.001,
        cost_per_1k_output_tokens: 0.002,
    });

    // 1000 input tokens * 0.001/1k + 500 output tokens * 0.002/1k
    // = 0.001 + 0.001 = 0.002
    let cost = router.estimate_cost("soz-kz-600m", 1000, 500).unwrap();
    assert!((cost - 0.002).abs() < 1e-9);

    // Unknown model returns None
    assert!(router.estimate_cost("nonexistent", 100, 50).is_none());
}
