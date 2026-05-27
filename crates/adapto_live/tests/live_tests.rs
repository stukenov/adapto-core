use adapto_live::session::{ActionHandler, LiveSession};
use adapto_live::handler::EventDispatcher;
use adapto_live::event::{validate_client_event, validate_form_event};
use adapto_live::patch::PatchGenerator;
use adapto_live::manager::SessionManager;
use adapto_live::error::LiveError;
use adapto_runtime::types::*;
use adapto_runtime::state::StateStore;
use adapto_runtime::context::PermissionSet;
use adapto_compiler::ir::*;
use adapto_compiler::dependency::DependencyGraph;
use adapto_client_protocol::event::*;
use adapto_client_protocol::patch::*;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_ir() -> ComponentIR {
    ComponentIR {
        id: "test_component".to_string(),
        name: "TestComponent".to_string(),
        route: None,
        static_segments: vec!["<div>".to_string(), "</div>".to_string()],
        dynamic_segments: vec![
            DynamicSegment::new(
                "dyn_0".to_string(),
                "counter".to_string(),
                vec!["counter".to_string()],
                SegmentType::Text,
            ),
            DynamicSegment::new(
                "dyn_1".to_string(),
                "label".to_string(),
                vec!["label".to_string()],
                SegmentType::Text,
            ),
        ],
        events: vec![],
        actions: vec![],
        state_fields: vec![],
        form_schemas: vec![],
        permissions: vec![],
        children: vec![],
        is_island: false,
        style: None,
    }
}

fn make_dep_graph() -> DependencyGraph {
    let mut g = DependencyGraph::new();
    g.add_dependency("dyn_0", "counter");
    g.add_dependency("dyn_1", "label");
    g
}

fn make_permissions() -> PermissionSet {
    let mut p = PermissionSet::new();
    p.add("read");
    p.add("write");
    p
}

fn make_session() -> LiveSession {
    LiveSession::new(
        SessionId::from("sess_1"),
        Some(UserId::default()),
        Some(TenantId::default()),
        RouteId::from("/dashboard"),
        make_ir(),
        make_dep_graph(),
        make_permissions(),
    )
}

fn make_client_event(handler: &str) -> ClientEvent {
    ClientEvent {
        session: "sess_1".to_string(),
        component: "test_component".to_string(),
        event: "click".to_string(),
        handler: handler.to_string(),
        payload: HashMap::new(),
        seq: 1,
    }
}

fn make_form_event(handler: &str) -> FormSubmitEvent {
    FormSubmitEvent {
        session: "sess_1".to_string(),
        component: "test_component".to_string(),
        handler: handler.to_string(),
        form: {
            let mut m = HashMap::new();
            m.insert("name".to_string(), serde_json::json!("Alice"));
            m
        },
        seq: 1,
    }
}

fn increment_handler() -> ActionHandler {
    Box::new(|state, _ctx, _args| {
        let current = state
            .get("counter")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        state.set("counter", serde_json::json!(current + 1));
        Ok(())
    })
}

// ===========================================================================
// 1. LiveSession creation
// ===========================================================================

#[test]
fn test_session_creation() {
    let session = make_session();
    assert_eq!(session.id, SessionId::from("sess_1"));
    assert_eq!(session.route, RouteId::from("/dashboard"));
    assert_eq!(session.seq, 0);
    assert!(session.user_id.is_some());
    assert!(session.tenant_id.is_some());
}

// ===========================================================================
// 2. LiveSession handle_event with registered handler
// ===========================================================================

#[test]
fn test_handle_event_with_registered_handler() {
    let mut session = make_session();
    session.state.set("counter", serde_json::json!(0));
    session.state.clear_dirty();
    session.register_handler("increment", increment_handler());

    let event = make_client_event("increment");
    let patch = session.handle_event(&event).unwrap();

    // Should have produced a patch replacing the counter text.
    assert!(!patch.ops.is_empty());
    assert_eq!(patch.seq, 1);
}

// ===========================================================================
// 3. LiveSession handle_event unknown handler error
// ===========================================================================

#[test]
fn test_handle_event_unknown_handler() {
    let mut session = make_session();
    let event = make_client_event("nonexistent");
    let err = session.handle_event(&event).unwrap_err();

    match err {
        LiveError::UnknownHandler(name) => assert_eq!(name, "nonexistent"),
        other => panic!("Expected UnknownHandler, got: {:?}", other),
    }
}

// ===========================================================================
// 4. LiveSession generate_patches for dirty state
// ===========================================================================

#[test]
fn test_generate_patches_dirty_state() {
    let mut session = make_session();
    session.state.set("counter", serde_json::json!(42));

    let patch = session.generate_patches();
    assert!(!patch.ops.is_empty());

    // After generate_patches, dirty set should be cleared.
    assert!(session.state.get_dirty().is_empty());
}

// ===========================================================================
// 5. LiveSession generate_patches empty when clean
// ===========================================================================

#[test]
fn test_generate_patches_clean_state() {
    let mut session = make_session();
    // No state mutations — nothing is dirty.
    let patch = session.generate_patches();
    assert!(patch.ops.is_empty());
}

// ===========================================================================
// 6. LiveSession ctx() returns correct context
// ===========================================================================

#[test]
fn test_session_ctx() {
    let session = make_session();
    let ctx = session.ctx();

    assert!(ctx.user_id.is_some());
    assert!(ctx.tenant_id.is_some());
    assert_eq!(ctx.route, RouteId::from("/dashboard"));
    assert_eq!(ctx.session_id, SessionId::from("sess_1"));
    assert!(ctx.permissions.has("read"));
    assert!(ctx.permissions.has("write"));
}

// ===========================================================================
// 7. LiveSession is_expired
// ===========================================================================

#[test]
fn test_session_is_expired() {
    let mut session = make_session();

    // Just created — should not be expired with a generous timeout.
    assert!(!session.is_expired(std::time::Duration::from_secs(60)));

    // Force last_event_at far into the past.
    session.last_event_at = chrono::Utc::now() - chrono::Duration::seconds(120);
    assert!(session.is_expired(std::time::Duration::from_secs(60)));
}

// ===========================================================================
// 8. LiveSession touch updates last_event_at
// ===========================================================================

#[test]
fn test_session_touch_via_handle_event() {
    let mut session = make_session();
    session.register_handler("increment", increment_handler());
    session.state.set("counter", serde_json::json!(0));
    session.state.clear_dirty();

    // Force last_event_at to the past.
    let past = chrono::Utc::now() - chrono::Duration::seconds(300);
    session.last_event_at = past;

    let event = make_client_event("increment");
    session.handle_event(&event).unwrap();

    // last_event_at should have been refreshed.
    let elapsed = chrono::Utc::now()
        .signed_duration_since(session.last_event_at)
        .num_seconds();
    assert!(elapsed < 2, "Expected touch to refresh timestamp");
}

// ===========================================================================
// 9. EventDispatcher: dispatch click event
// ===========================================================================

#[test]
fn test_dispatcher_click_event() {
    let mut session = make_session();
    session.state.set("counter", serde_json::json!(0));
    session.state.clear_dirty();
    session.register_handler("increment", increment_handler());

    let mut dispatcher = EventDispatcher::new(100);
    let event = make_client_event("increment");
    let payload = ClientPayload::Event(event);

    let result = dispatcher.dispatch(&mut session, &payload).unwrap();
    match result {
        ServerPayload::Patch(patch) => {
            assert!(!patch.ops.is_empty());
        }
        other => panic!("Expected Patch, got: {:?}", other),
    }
}

// ===========================================================================
// 10. EventDispatcher: rate limit exceeded
// ===========================================================================

#[test]
fn test_dispatcher_rate_limit() {
    let mut session = make_session();
    session.register_handler("increment", increment_handler());
    session.state.set("counter", serde_json::json!(0));
    session.state.clear_dirty();

    // Rate limit of 1 event per second.
    let mut dispatcher = EventDispatcher::new(1);
    let event = make_client_event("increment");
    let payload = ClientPayload::Event(event);

    // First dispatch should succeed.
    dispatcher.dispatch(&mut session, &payload).unwrap();

    // Second dispatch immediately should be rate-limited.
    let err = dispatcher.dispatch(&mut session, &payload).unwrap_err();
    match err {
        LiveError::RateLimitExceeded => {}
        other => panic!("Expected RateLimitExceeded, got: {:?}", other),
    }
}

// ===========================================================================
// 11. Event validation: empty session rejected
// ===========================================================================

#[test]
fn test_validate_event_empty_session() {
    let event = ClientEvent {
        session: String::new(),
        component: "c".to_string(),
        event: "click".to_string(),
        handler: "h".to_string(),
        payload: HashMap::new(),
        seq: 1,
    };
    let err = validate_client_event(&event).unwrap_err();
    match err {
        LiveError::InvalidEvent(msg) => assert_eq!(msg, "empty session"),
        other => panic!("Expected InvalidEvent, got: {:?}", other),
    }
}

// ===========================================================================
// 12. Event validation: empty handler rejected
// ===========================================================================

#[test]
fn test_validate_event_empty_handler() {
    let event = ClientEvent {
        session: "sess_1".to_string(),
        component: "c".to_string(),
        event: "click".to_string(),
        handler: String::new(),
        payload: HashMap::new(),
        seq: 1,
    };
    let err = validate_client_event(&event).unwrap_err();
    match err {
        LiveError::InvalidEvent(msg) => assert_eq!(msg, "empty handler"),
        other => panic!("Expected InvalidEvent, got: {:?}", other),
    }
}

// ===========================================================================
// 13. Form event validation
// ===========================================================================

#[test]
fn test_validate_form_event_valid() {
    let event = make_form_event("submit");
    assert!(validate_form_event(&event).is_ok());
}

#[test]
fn test_validate_form_event_empty_session() {
    let event = FormSubmitEvent {
        session: String::new(),
        component: "c".to_string(),
        handler: "submit".to_string(),
        form: HashMap::new(),
        seq: 1,
    };
    let err = validate_form_event(&event).unwrap_err();
    match err {
        LiveError::InvalidEvent(msg) => assert_eq!(msg, "empty session"),
        other => panic!("Expected InvalidEvent, got: {:?}", other),
    }
}

#[test]
fn test_validate_form_event_empty_handler() {
    let event = FormSubmitEvent {
        session: "sess_1".to_string(),
        component: "c".to_string(),
        handler: String::new(),
        form: HashMap::new(),
        seq: 1,
    };
    let err = validate_form_event(&event).unwrap_err();
    match err {
        LiveError::InvalidEvent(msg) => assert_eq!(msg, "empty handler"),
        other => panic!("Expected InvalidEvent, got: {:?}", other),
    }
}

// ===========================================================================
// 14. PatchGenerator: generate patches for dirty text
// ===========================================================================

#[test]
fn test_patch_generator_dirty_text() {
    let mut state = StateStore::new();
    state.set("counter", serde_json::json!(7));

    let dep_graph = make_dep_graph();
    let segments = vec![DynamicSegment::new(
        "dyn_0".to_string(),
        "counter".to_string(),
        vec!["counter".to_string()],
        SegmentType::Text,
    )];

    let ops = PatchGenerator::generate(&state, &dep_graph, &segments);
    assert_eq!(ops.len(), 1);
    match &ops[0] {
        PatchOp::ReplaceText { target, value } => {
            assert_eq!(target, "dyn_0");
            assert_eq!(value, "7");
        }
        other => panic!("Expected ReplaceText, got: {:?}", other),
    }
}

// ===========================================================================
// 15. PatchGenerator: no patches when clean
// ===========================================================================

#[test]
fn test_patch_generator_no_patches_when_clean() {
    let state = StateStore::new();
    let dep_graph = make_dep_graph();
    let segments = vec![DynamicSegment::new(
        "dyn_0".to_string(),
        "counter".to_string(),
        vec!["counter".to_string()],
        SegmentType::Text,
    )];

    let ops = PatchGenerator::generate(&state, &dep_graph, &segments);
    assert!(ops.is_empty());
}

// ===========================================================================
// 16. PatchGenerator: multiple dirty fields
// ===========================================================================

#[test]
fn test_patch_generator_multiple_dirty_fields() {
    let mut state = StateStore::new();
    state.set("counter", serde_json::json!(10));
    state.set("label", serde_json::json!("hello"));

    let dep_graph = make_dep_graph();
    let segments = vec![
        DynamicSegment::new(
            "dyn_0".to_string(),
            "counter".to_string(),
            vec!["counter".to_string()],
            SegmentType::Text,
        ),
        DynamicSegment::new(
            "dyn_1".to_string(),
            "label".to_string(),
            vec!["label".to_string()],
            SegmentType::Text,
        ),
    ];

    let ops = PatchGenerator::generate(&state, &dep_graph, &segments);
    assert_eq!(ops.len(), 2);
}

// ===========================================================================
// 17. PatchGenerator: eval_expr simple value
// ===========================================================================

#[test]
fn test_patch_generator_eval_simple() {
    let mut state = StateStore::new();
    state.set("name", serde_json::json!("Alice"));

    let mut dep_graph = DependencyGraph::new();
    dep_graph.add_dependency("dyn_name", "name");

    let segments = vec![DynamicSegment::new(
        "dyn_name".to_string(),
        "name".to_string(),
        vec!["name".to_string()],
        SegmentType::Text,
    )];

    let ops = PatchGenerator::generate(&state, &dep_graph, &segments);
    assert_eq!(ops.len(), 1);
    match &ops[0] {
        PatchOp::ReplaceText { value, .. } => assert_eq!(value, "Alice"),
        other => panic!("Expected ReplaceText, got: {:?}", other),
    }
}

// ===========================================================================
// 18. PatchGenerator: eval_expr dotted path
// ===========================================================================

#[test]
fn test_patch_generator_eval_dotted_path() {
    let mut state = StateStore::new();
    state.set(
        "customer",
        serde_json::json!({ "name": "Bob", "age": 30 }),
    );

    let mut dep_graph = DependencyGraph::new();
    dep_graph.add_dependency("dyn_cname", "customer");

    let segments = vec![DynamicSegment::new(
        "dyn_cname".to_string(),
        "customer.name".to_string(),
        vec!["customer".to_string()],
        SegmentType::Text,
    )];

    let ops = PatchGenerator::generate(&state, &dep_graph, &segments);
    assert_eq!(ops.len(), 1);
    match &ops[0] {
        PatchOp::ReplaceText { value, .. } => assert_eq!(value, "Bob"),
        other => panic!("Expected ReplaceText, got: {:?}", other),
    }
}

// ===========================================================================
// 19. SessionManager: add and count
// ===========================================================================

#[test]
fn test_manager_add_and_count() {
    let manager = SessionManager::new(10);
    assert_eq!(manager.count(), 0);

    manager.add(make_session()).unwrap();
    assert_eq!(manager.count(), 1);
}

// ===========================================================================
// 20. SessionManager: with_session
// ===========================================================================

#[test]
fn test_manager_with_session() {
    let manager = SessionManager::new(10);
    let session = make_session();
    let sid = session.id.clone();
    manager.add(session).unwrap();

    let route = manager
        .with_session(&sid, |s| s.route.clone())
        .unwrap();
    assert_eq!(route, RouteId::from("/dashboard"));
}

// ===========================================================================
// 21. SessionManager: remove
// ===========================================================================

#[test]
fn test_manager_remove() {
    let manager = SessionManager::new(10);
    let session = make_session();
    let sid = session.id.clone();
    manager.add(session).unwrap();
    assert_eq!(manager.count(), 1);

    let removed = manager.remove(&sid);
    assert!(removed.is_some());
    assert_eq!(manager.count(), 0);
}

// ===========================================================================
// 22. SessionManager: has
// ===========================================================================

#[test]
fn test_manager_has() {
    let manager = SessionManager::new(10);
    let session = make_session();
    let sid = session.id.clone();

    assert!(!manager.has(&sid));
    manager.add(session).unwrap();
    assert!(manager.has(&sid));
}

// ===========================================================================
// 23. SessionManager: count_for_user
// ===========================================================================

#[test]
fn test_manager_count_for_user() {
    let manager = SessionManager::new(10);

    let user_id = UserId::default();

    let s1 = LiveSession::new(
        SessionId::from("s1"),
        Some(user_id.clone()),
        None,
        RouteId::from("/a"),
        make_ir(),
        make_dep_graph(),
        PermissionSet::new(),
    );
    let s2 = LiveSession::new(
        SessionId::from("s2"),
        Some(user_id.clone()),
        None,
        RouteId::from("/b"),
        make_ir(),
        make_dep_graph(),
        PermissionSet::new(),
    );
    let s3 = LiveSession::new(
        SessionId::from("s3"),
        Some(UserId::default()), // different user
        None,
        RouteId::from("/c"),
        make_ir(),
        make_dep_graph(),
        PermissionSet::new(),
    );

    manager.add(s1).unwrap();
    manager.add(s2).unwrap();
    manager.add(s3).unwrap();

    assert_eq!(manager.count_for_user(&user_id), 2);
    assert_eq!(manager.count(), 3);
}

// ===========================================================================
// 24. SessionManager: max sessions exceeded
// ===========================================================================

#[test]
fn test_manager_max_sessions_exceeded() {
    let user_id = UserId::default();
    let manager = SessionManager::new(1);

    let s1 = LiveSession::new(
        SessionId::from("s1"),
        Some(user_id.clone()),
        None,
        RouteId::from("/a"),
        make_ir(),
        make_dep_graph(),
        PermissionSet::new(),
    );
    manager.add(s1).unwrap();

    let s2 = LiveSession::new(
        SessionId::from("s2"),
        Some(user_id.clone()),
        None,
        RouteId::from("/b"),
        make_ir(),
        make_dep_graph(),
        PermissionSet::new(),
    );
    let err = manager.add(s2).unwrap_err();
    match err {
        LiveError::MaxSessionsExceeded => {}
        other => panic!("Expected MaxSessionsExceeded, got: {:?}", other),
    }
}

// ===========================================================================
// 25. SessionManager: cleanup_expired
// ===========================================================================

#[test]
fn test_manager_cleanup_expired() {
    let manager = SessionManager::new(10);

    let mut s1 = LiveSession::new(
        SessionId::from("s_old"),
        None,
        None,
        RouteId::from("/old"),
        make_ir(),
        make_dep_graph(),
        PermissionSet::new(),
    );
    // Force this session to look expired.
    s1.last_event_at = chrono::Utc::now() - chrono::Duration::seconds(600);

    let s2 = LiveSession::new(
        SessionId::from("s_new"),
        None,
        None,
        RouteId::from("/new"),
        make_ir(),
        make_dep_graph(),
        PermissionSet::new(),
    );

    manager.add(s1).unwrap();
    manager.add(s2).unwrap();
    assert_eq!(manager.count(), 2);

    let removed = manager.cleanup_expired(std::time::Duration::from_secs(300));
    assert_eq!(removed, 1);
    assert_eq!(manager.count(), 1);
    assert!(manager.has(&SessionId::from("s_new")));
    assert!(!manager.has(&SessionId::from("s_old")));
}

// ===========================================================================
// 26. SessionManager: add anonymous sessions bypass user limit
// ===========================================================================

#[test]
fn test_manager_anonymous_sessions_bypass_limit() {
    let manager = SessionManager::new(1);

    let s1 = LiveSession::new(
        SessionId::from("anon_1"),
        None,
        None,
        RouteId::from("/a"),
        make_ir(),
        make_dep_graph(),
        PermissionSet::new(),
    );
    let s2 = LiveSession::new(
        SessionId::from("anon_2"),
        None,
        None,
        RouteId::from("/b"),
        make_ir(),
        make_dep_graph(),
        PermissionSet::new(),
    );

    manager.add(s1).unwrap();
    manager.add(s2).unwrap();
    assert_eq!(manager.count(), 2);
}

// ===========================================================================
// 27. SessionManager: count_for_user returns 0 for unknown user
// ===========================================================================

#[test]
fn test_manager_count_for_unknown_user() {
    let manager = SessionManager::new(10);
    manager.add(make_session()).unwrap();
    let unknown = UserId(uuid::Uuid::new_v4());
    assert_eq!(manager.count_for_user(&unknown), 0);
}

// ===========================================================================
// 28. SessionManager: remove nonexistent returns None
// ===========================================================================

#[test]
fn test_manager_remove_nonexistent() {
    let manager = SessionManager::new(10);
    assert!(manager.remove(&SessionId::from("ghost")).is_none());
}

// ===========================================================================
// 29. SessionManager: cleanup_expired with no expired sessions
// ===========================================================================

#[test]
fn test_manager_cleanup_none_expired() {
    let manager = SessionManager::new(10);
    manager.add(make_session()).unwrap();
    let removed = manager.cleanup_expired(std::time::Duration::from_secs(3600));
    assert_eq!(removed, 0);
    assert_eq!(manager.count(), 1);
}

// ===========================================================================
// 30. Event validation: valid client event passes
// ===========================================================================

#[test]
fn test_validate_client_event_valid() {
    let event = make_client_event("do_stuff");
    assert!(validate_client_event(&event).is_ok());
}

// ===========================================================================
// 31. Event validation: empty component still passes
// ===========================================================================

#[test]
fn test_validate_event_empty_component_passes() {
    let event = ClientEvent {
        session: "s".to_string(),
        component: String::new(),
        event: "click".to_string(),
        handler: "h".to_string(),
        payload: HashMap::new(),
        seq: 1,
    };
    assert!(validate_client_event(&event).is_ok());
}

// ===========================================================================
// 32. extract_action_args: empty payload returns empty object
// ===========================================================================

#[test]
fn test_extract_action_args_empty() {
    use adapto_live::event::extract_action_args;
    let event = make_client_event("handler");
    let args = extract_action_args(&event);
    assert!(args.is_object());
    assert_eq!(args.as_object().unwrap().len(), 0);
}

// ===========================================================================
// 33. extract_action_args: with payload fields
// ===========================================================================

#[test]
fn test_extract_action_args_with_fields() {
    use adapto_live::event::extract_action_args;
    let mut event = make_client_event("handler");
    event.payload.insert("idx".to_string(), serde_json::json!(5));
    event.payload.insert("name".to_string(), serde_json::json!("Alice"));

    let args = extract_action_args(&event);
    let obj = args.as_object().unwrap();
    assert_eq!(obj.get("idx"), Some(&serde_json::json!(5)));
    assert_eq!(obj.get("name"), Some(&serde_json::json!("Alice")));
}

// ===========================================================================
// 34. PatchGenerator: attribute segment type
// ===========================================================================

#[test]
fn test_patch_generator_attribute_segment() {
    let mut state = StateStore::new();
    state.set("color", serde_json::json!("red"));

    let mut dep_graph = DependencyGraph::new();
    dep_graph.add_dependency("dyn_attr", "color");

    let segments = vec![DynamicSegment::new(
        "dyn_attr".to_string(),
        "color".to_string(),
        vec!["color".to_string()],
        SegmentType::Attribute {
            element_id: "el_1".to_string(),
            attr_name: "style".to_string(),
        },
    )];

    let ops = PatchGenerator::generate(&state, &dep_graph, &segments);
    assert_eq!(ops.len(), 1);
    match &ops[0] {
        PatchOp::SetAttr { target, name, value } => {
            assert_eq!(target, "el_1");
            assert_eq!(name, "style");
            assert_eq!(value, "red");
        }
        other => panic!("Expected SetAttr, got: {:?}", other),
    }
}

// ===========================================================================
// 35. PatchGenerator: html segment type
// ===========================================================================

#[test]
fn test_patch_generator_html_segment() {
    let mut state = StateStore::new();
    state.set("content", serde_json::json!("<b>bold</b>"));

    let mut dep_graph = DependencyGraph::new();
    dep_graph.add_dependency("dyn_html", "content");

    let segments = vec![DynamicSegment::new(
        "dyn_html".to_string(),
        "content".to_string(),
        vec!["content".to_string()],
        SegmentType::Html,
    )];

    let ops = PatchGenerator::generate(&state, &dep_graph, &segments);
    assert_eq!(ops.len(), 1);
    match &ops[0] {
        PatchOp::ReplaceHtml { target, html } => {
            assert_eq!(target, "dyn_html");
            assert_eq!(html, "<b>bold</b>");
        }
        other => panic!("Expected ReplaceHtml, got: {:?}", other),
    }
}

// ===========================================================================
// 36. Form validation: empty component still passes
// ===========================================================================

#[test]
fn test_validate_form_event_empty_component_passes() {
    let event = FormSubmitEvent {
        session: "s".to_string(),
        component: String::new(),
        handler: "submit".to_string(),
        form: HashMap::new(),
        seq: 1,
    };
    assert!(validate_form_event(&event).is_ok());
}

// ===========================================================================
// Phase 6: WS Session Lifecycle
// ===========================================================================

fn make_ir_with_defaults() -> ComponentIR {
    ComponentIR {
        id: "counter_comp".to_string(),
        name: "Counter".to_string(),
        route: None,
        static_segments: vec!["<div>".to_string(), "</div>".to_string()],
        dynamic_segments: vec![
            DynamicSegment::new(
                "dyn_0".to_string(),
                "count".to_string(),
                vec!["count".to_string()],
                SegmentType::Text,
            ),
            DynamicSegment::new(
                "dyn_1".to_string(),
                "name".to_string(),
                vec!["name".to_string()],
                SegmentType::Text,
            ),
        ],
        events: vec![],
        actions: vec![ActionIR {
            name: "increment".into(),
            body: "count += 1".into(),
            permission: None,
            is_async: false,
            params: vec![],
            audit: None,
        }],
        state_fields: vec![
            StateFieldIR {
                name: "count".into(),
                ty: "i64".into(),
                default: Some("0".into()),
                secret: false,
            },
            StateFieldIR {
                name: "name".into(),
                ty: "String".into(),
                default: Some("\"World\"".into()),
                secret: false,
            },
        ],
        form_schemas: vec![],
        permissions: vec![],
        children: vec![],
        is_island: false,
        style: None,
    }
}

#[test]
fn phase6_init_state_from_defaults() {
    let ir = make_ir_with_defaults();
    let mut session = make_session_from_ir(ir);
    session.init_state_from_defaults();

    assert_eq!(session.state.get("count"), Some(&serde_json::json!(0)));
    assert_eq!(session.state.get("name"), Some(&serde_json::json!("World")));
    assert!(!session.state.is_dirty("count"), "dirty cleared after init");
}

#[test]
fn phase6_init_state_partial_defaults() {
    let mut ir = make_ir_with_defaults();
    ir.state_fields[1].default = None;
    let mut session = make_session_from_ir(ir);
    session.init_state_from_defaults();

    assert_eq!(session.state.get("count"), Some(&serde_json::json!(0)));
    assert_eq!(session.state.get("name"), None);
}

#[test]
fn phase6_event_after_init_generates_patches() {
    let ir = make_ir_with_defaults();
    let graph = build_graph_for_ir(&ir);
    let mut session = LiveSession::new(
        SessionId::from("s1"),
        None,
        None,
        RouteId::from("/test"),
        ir,
        graph,
        PermissionSet::new(),
    );
    session.init_state_from_defaults();

    let event = ClientEvent {
        session: "s1".into(),
        component: "counter_comp".into(),
        event: "click".into(),
        handler: "increment".into(),
        payload: HashMap::new(),
        seq: 1,
    };

    let patches = session.handle_event(&event).unwrap();
    assert!(!patches.ops.is_empty(), "should generate patches");
    let text_patch = patches.ops.iter().find(|op| matches!(op, PatchOp::ReplaceText { target, .. } if target == "dyn_0"));
    assert!(text_patch.is_some(), "should have replace_text for count");
    if let Some(PatchOp::ReplaceText { value, .. }) = text_patch {
        assert_eq!(value, "1");
    }
}

#[test]
fn phase6_multiple_events_increment_state() {
    let ir = make_ir_with_defaults();
    let graph = build_graph_for_ir(&ir);
    let mut session = LiveSession::new(
        SessionId::from("s2"),
        None,
        None,
        RouteId::from("/test"),
        ir,
        graph,
        PermissionSet::new(),
    );
    session.init_state_from_defaults();

    let event = ClientEvent {
        session: "s2".into(),
        component: "counter_comp".into(),
        event: "click".into(),
        handler: "increment".into(),
        payload: HashMap::new(),
        seq: 1,
    };

    session.handle_event(&event).unwrap();
    session.handle_event(&event).unwrap();
    let patches = session.handle_event(&event).unwrap();

    let text_patch = patches.ops.iter().find(|op| matches!(op, PatchOp::ReplaceText { target, .. } if target == "dyn_0"));
    if let Some(PatchOp::ReplaceText { value, .. }) = text_patch {
        assert_eq!(value, "3");
    }
}

#[test]
fn phase6_session_expiry() {
    let ir = make_ir_with_defaults();
    let mut session = make_session_from_ir(ir);
    session.last_event_at = chrono::Utc::now() - chrono::Duration::seconds(400);

    assert!(session.is_expired(std::time::Duration::from_secs(300)));
    assert!(!session.is_expired(std::time::Duration::from_secs(500)));
}

#[test]
fn phase6_session_touch_prevents_expiry() {
    let ir = make_ir_with_defaults();
    let mut session = make_session_from_ir(ir);
    session.last_event_at = chrono::Utc::now() - chrono::Duration::seconds(400);
    assert!(session.is_expired(std::time::Duration::from_secs(300)));

    session.handle_event(&ClientEvent {
        session: "s".into(),
        component: "c".into(),
        event: "click".into(),
        handler: "increment".into(),
        payload: HashMap::new(),
        seq: 1,
    }).unwrap();

    assert!(!session.is_expired(std::time::Duration::from_secs(300)));
}

#[test]
fn phase6_session_manager_add_and_dispatch() {
    let ir = make_ir_with_defaults();
    let graph = build_graph_for_ir(&ir);
    let mut session = LiveSession::new(
        SessionId::from("ws-1"),
        None,
        None,
        RouteId::from("/"),
        ir,
        graph,
        PermissionSet::new(),
    );
    session.init_state_from_defaults();

    let mgr = SessionManager::new(10);
    mgr.add(session).unwrap();
    assert!(mgr.has(&SessionId::from("ws-1")));

    let result = mgr.with_session(&SessionId::from("ws-1"), |s| {
        s.state.get("count").cloned()
    });
    assert_eq!(result.unwrap(), Some(serde_json::json!(0)));
}

#[test]
fn phase6_cleanup_removes_expired() {
    let ir = make_ir_with_defaults();
    let mut session = make_session_from_ir(ir);
    session.id = SessionId::from("expired-1");
    session.last_event_at = chrono::Utc::now() - chrono::Duration::seconds(600);

    let mgr = SessionManager::new(10);
    mgr.add(session).unwrap();
    assert_eq!(mgr.count(), 1);

    let removed = mgr.cleanup_expired(std::time::Duration::from_secs(300));
    assert_eq!(removed, 1);
    assert_eq!(mgr.count(), 0);
}

#[test]
fn phase6_dispatcher_heartbeat_ack() {
    let ir = make_ir_with_defaults();
    let mut session = make_session_from_ir(ir);
    session.init_state_from_defaults();

    let mut dispatcher = EventDispatcher::new(100);
    let payload = ClientPayload::Heartbeat(HeartbeatEvent {
        session: "s".into(),
        seq: 42,
    });

    let result = dispatcher.dispatch(&mut session, &payload).unwrap();
    match result {
        ServerPayload::HeartbeatAck(ack) => assert_eq!(ack.seq, 42),
        _ => panic!("expected HeartbeatAck"),
    }
}

#[test]
fn phase6_dispatcher_event_returns_patch() {
    let ir = make_ir_with_defaults();
    let graph = build_graph_for_ir(&ir);
    let mut session = LiveSession::new(
        SessionId::from("d1"),
        None,
        None,
        RouteId::from("/"),
        ir,
        graph,
        PermissionSet::new(),
    );
    session.init_state_from_defaults();

    let mut dispatcher = EventDispatcher::new(100);
    let payload = ClientPayload::Event(ClientEvent {
        session: "d1".into(),
        component: "counter_comp".into(),
        event: "click".into(),
        handler: "increment".into(),
        payload: HashMap::new(),
        seq: 1,
    });

    let result = dispatcher.dispatch(&mut session, &payload).unwrap();
    match result {
        ServerPayload::Patch(p) => {
            assert!(!p.ops.is_empty());
        }
        _ => panic!("expected Patch"),
    }
}

#[test]
fn phase6_seq_monotonically_increases() {
    let ir = make_ir_with_defaults();
    let graph = build_graph_for_ir(&ir);
    let mut session = LiveSession::new(
        SessionId::from("seq"),
        None,
        None,
        RouteId::from("/"),
        ir,
        graph,
        PermissionSet::new(),
    );
    session.init_state_from_defaults();

    let event = ClientEvent {
        session: "seq".into(),
        component: "c".into(),
        event: "click".into(),
        handler: "increment".into(),
        payload: HashMap::new(),
        seq: 1,
    };

    let p1 = session.handle_event(&event).unwrap();
    let p2 = session.handle_event(&event).unwrap();
    let p3 = session.handle_event(&event).unwrap();

    assert_eq!(p1.seq, 1);
    assert_eq!(p2.seq, 2);
    assert_eq!(p3.seq, 3);
}

fn make_session_from_ir(ir: ComponentIR) -> LiveSession {
    let graph = build_graph_for_ir(&ir);
    LiveSession::new(
        SessionId::from("test-session"),
        None,
        None,
        RouteId::from("/test"),
        ir,
        graph,
        PermissionSet::new(),
    )
}

fn build_graph_for_ir(ir: &ComponentIR) -> DependencyGraph {
    let mut graph = DependencyGraph::default();
    for seg in &ir.dynamic_segments {
        for dep in &seg.deps {
            graph.add_dependency(&seg.id, dep);
        }
    }
    graph
}
