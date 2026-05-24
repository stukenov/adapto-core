//! # Adapto School AI Example
//!
//! Demonstrates the full AI-native workflow within the Adapto Live Runtime:
//!
//! - Parse and compile `.adapto` DSL for a lesson tracker
//! - Configure AI model routing with primary + fallback
//! - Apply PII redaction before sending data to models
//! - Track per-tenant token budgets
//! - Execute AI actions (mock) and record traces
//! - Validate lesson data through form schemas
//! - Enforce RBAC permissions for AI features
//! - Write structured audit logs for every AI action

use adapto_ai::action::*;
use adapto_ai::budget::*;
use adapto_ai::model::*;
use adapto_ai::pii::*;
use adapto_ai::trace::*;
use adapto_audit::event::*;
use adapto_audit::sink::{AuditSink, InMemoryAuditSink};
use adapto_auth::rbac::{RbacStore, Role};
use adapto_compiler::compiler::Compiler;
use adapto_db::repository::InMemoryRepository;
use adapto_forms::schema::*;
use adapto_runtime::context::Ctx;
use adapto_runtime::state::StateStore;
use adapto_runtime::types::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashSet;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Lesson {
    id: Uuid,
    title: String,
    status: String,
    transcript: String,
}

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

#[tokio::main]
async fn main() {
    println!("=== Adapto School AI Example ===\n");

    // -----------------------------------------------------------------------
    // 1. Parse lesson tracker DSL
    // -----------------------------------------------------------------------
    println!("1. Parsing lesson tracker DSL...");
    let ast = adapto_parser::parse(LESSON_DSL).expect("Parse failed");
    println!(
        "   Route: {:?}",
        ast.route.as_ref().and_then(|r| r.path.as_ref())
    );
    println!(
        "   Props: {}",
        ast.script.as_ref().map(|s| s.props.len()).unwrap_or(0)
    );
    println!(
        "   States: {}",
        ast.script.as_ref().map(|s| s.states.len()).unwrap_or(0)
    );

    // -----------------------------------------------------------------------
    // 2. Compile
    // -----------------------------------------------------------------------
    println!("\n2. Compiling...");
    let mut compiler = Compiler::new();
    let output = compiler
        .compile_file(&ast, "lessons/[id]/page.adapto")
        .expect("Compile failed");
    println!(
        "   Dynamic segments: {}",
        output.component_ir.dynamic_segments.len()
    );

    // -----------------------------------------------------------------------
    // 3. AI model routing
    // -----------------------------------------------------------------------
    println!("\n3. Setting up AI model routing...");
    let mut model_router = ModelRouter::new();
    model_router.add_model(ModelConfig {
        name: "soz-kz-600m".into(),
        provider: ModelProvider::Custom("soz-kz".into()),
        endpoint: Some("https://api.soz.kz/v1".into()),
        api_key_env: Some("SOZ_API_KEY".into()),
        max_tokens: Some(4096),
        default_temperature: 0.2,
        cost_per_1k_input_tokens: 0.001,
        cost_per_1k_output_tokens: 0.002,
    });
    model_router.add_model(ModelConfig {
        name: "gpt-5.5-thinking".into(),
        provider: ModelProvider::OpenAI,
        endpoint: None,
        api_key_env: Some("OPENAI_API_KEY".into()),
        max_tokens: Some(8192),
        default_temperature: 0.7,
        cost_per_1k_input_tokens: 0.01,
        cost_per_1k_output_tokens: 0.03,
    });
    model_router.set_default("soz-kz-600m");
    model_router.set_fallback("gpt-5.5-thinking");
    println!("   Default model: soz-kz-600m");
    println!("   Fallback model: gpt-5.5-thinking");

    // -----------------------------------------------------------------------
    // 4. PII redaction
    // -----------------------------------------------------------------------
    println!("\n4. PII Redaction...");
    let redactor = PiiRedactor::with_defaults();
    let raw_transcript = "The student John Smith (john@school.kz, phone: +7 701 555 1234) \
                          performed well in class today. His SSN is 123-45-6789.";
    let redacted = redactor.redact(raw_transcript);
    println!("   Original: {raw_transcript}");
    println!("   Redacted: {}", redacted.output);
    println!("   PII items found: {}", redacted.redacted_count);
    println!("   Types: {:?}", redacted.redacted_types);

    // -----------------------------------------------------------------------
    // 5. Tenant budget
    // -----------------------------------------------------------------------
    println!("\n5. Tenant AI budget...");
    let budget_tracker = BudgetTracker::new();
    let school_tenant = TenantId(Uuid::new_v4());
    budget_tracker.set_budget(
        &school_tenant,
        TenantBudget {
            total_tokens: 100_000,
            used_tokens: 0,
            total_cost: 10.0,
            used_cost: 0.0,
            max_tokens_per_request: Some(4096),
        },
    );
    println!("   Budget set: 100,000 tokens / $10.00");

    let budget_ok = budget_tracker.check_budget(&school_tenant, 500);
    println!("   Budget check (500 tokens): {:?}", budget_ok);

    // -----------------------------------------------------------------------
    // 6. Execute AI action (mock)
    // -----------------------------------------------------------------------
    println!("\n6. Executing AI action...");
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

    let ai_request = AiRequest {
        action: "summarize_lesson".into(),
        input: json!({ "transcript": redacted.output }),
        tenant_id: Some(school_tenant.clone()),
        user_id: Some(UserId(Uuid::new_v4())),
        request_id: RequestId::default(),
    };

    let response = executor.execute(ai_request).await.expect("AI execution failed");
    println!("   Model used: {}", response.model_used);
    println!(
        "   Tokens: {} input + {} output = {} total",
        response.tokens_used.input_tokens,
        response.tokens_used.output_tokens,
        response.tokens_used.total_tokens
    );
    println!("   Latency: {}ms", response.latency_ms);

    // Record usage against budget
    budget_tracker.record_usage(
        &school_tenant,
        response.tokens_used.total_tokens as u64,
        model_router
            .estimate_cost(
                &response.model_used,
                response.tokens_used.input_tokens,
                response.tokens_used.output_tokens,
            )
            .unwrap_or(0.0),
    );
    let usage = budget_tracker.get_usage(&school_tenant).unwrap();
    println!(
        "   Budget remaining: {}/{} tokens",
        usage.total_tokens - usage.used_tokens,
        usage.total_tokens
    );

    // -----------------------------------------------------------------------
    // 7. AI trace
    // -----------------------------------------------------------------------
    println!("\n7. AI Trace logging...");
    let trace_collector = TraceCollector::new();
    let trace = trace_collector.start_trace(
        &AiRequest {
            action: "summarize_lesson".into(),
            input: json!({}),
            tenant_id: Some(school_tenant.clone()),
            user_id: None,
            request_id: RequestId::default(),
        },
        "soz-kz-600m",
    );
    trace_collector.complete_trace(trace);
    let traces = trace_collector.get_traces_for_tenant(&school_tenant);
    println!("   Traces for school tenant: {}", traces.len());

    // -----------------------------------------------------------------------
    // 8. RBAC and permissions
    // -----------------------------------------------------------------------
    println!("\n8. RBAC permissions...");
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
    let student_perms = rbac.get_permissions(&student_id);
    println!(
        "   Teacher can summarize: {}",
        teacher_perms.has("lessons.ai.summarize")
    );
    println!(
        "   Student can summarize: {}",
        student_perms.has("lessons.ai.summarize")
    );

    // -----------------------------------------------------------------------
    // 9. Form validation
    // -----------------------------------------------------------------------
    println!("\n9. Form validation...");
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
    let valid_data = serde_json::from_value::<serde_json::Map<String, serde_json::Value>>(json!({
        "title": "Introduction to Algebra",
        "status": "active",
        "transcript": "Today we covered basic algebraic expressions..."
    }))
    .unwrap();
    let result = lesson_form.validate(&valid_data);
    println!("   Valid lesson data: {}", result.is_valid());

    // Invalid data
    let invalid_data = serde_json::from_value::<serde_json::Map<String, serde_json::Value>>(json!({
        "title": "Hi",
        "status": "unknown",
    }))
    .unwrap();
    let result = lesson_form.validate(&invalid_data);
    println!("   Invalid lesson data: {} errors", result.all_errors().len());

    // -----------------------------------------------------------------------
    // 10. State dirty tracking
    // -----------------------------------------------------------------------
    println!("\n10. State dirty tracking...");
    let mut state = StateStore::new();
    state.set("lesson", json!({"id": "abc", "title": "Algebra", "status": "active"}));
    state.set("ai_summary", json!(null));
    println!("   Dirty after init: {:?}", state.get_dirty());
    state.clear_dirty();

    // Simulate AI summary arriving
    state.set("ai_summary", json!("The lesson covered quadratic equations."));
    println!("   Dirty after AI summary: {:?}", state.get_dirty());
    assert!(state.is_dirty("ai_summary"));
    assert!(!state.is_dirty("lesson"));

    // -----------------------------------------------------------------------
    // 11. Audit logging
    // -----------------------------------------------------------------------
    println!("\n11. Audit logging...");
    let audit = InMemoryAuditSink::new();
    let teacher_ctx = Ctx {
        user_id: Some(teacher_id.clone()),
        tenant_id: Some(school_tenant.clone()),
        request_id: RequestId::default(),
        permissions: teacher_perms,
        route: RouteId("/lessons/123".into()),
        session_id: SessionId("sess-teacher".into()),
    };

    let audit_event = AuditEvent::new("lesson.ai.summarized", &teacher_ctx, "summarize_lesson")
        .with_metadata("model", json!("soz-kz-600m"))
        .with_metadata("tokens", json!(150))
        .success();
    audit.write(audit_event);
    println!("   Audit events: {}", audit.len());

    // -----------------------------------------------------------------------
    // 12. InMemoryRepository for lessons
    // -----------------------------------------------------------------------
    println!("\n12. Lesson repository...");
    let repo: InMemoryRepository<Lesson> = InMemoryRepository::new();
    let lesson_id = Uuid::new_v4();
    repo.create(
        &school_tenant,
        lesson_id,
        Lesson {
            id: lesson_id,
            title: "Introduction to Algebra".into(),
            status: "active".into(),
            transcript: "Today we covered basic algebraic expressions...".into(),
        },
    );
    let found = repo.find(&school_tenant, &lesson_id);
    println!(
        "   Found lesson: {}",
        found.as_ref().map(|l| l.title.as_str()).unwrap_or("none")
    );
    println!("   Tenant lesson count: {}", repo.count(&school_tenant));

    println!("\n=== School AI example complete ===");
}
