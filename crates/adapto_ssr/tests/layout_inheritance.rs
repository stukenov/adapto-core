use std::collections::HashMap;
use adapto_compiler::compiler::Compiler;
use adapto_ssr::layout::{CompiledLayout, LayoutManager, SlotDefinition};

#[test]
fn single_layout_with_named_slots() {
    let page_src = r#"<route>
        path: "/test"
        layout: "main"
    </route>
    <template>
        {#fill sidebar}
            <nav>Custom sidebar</nav>
        {/fill}
        <h1>Page content</h1>
    </template>"#;

    let page_file = adapto_parser::parse(page_src).unwrap();
    let mut compiler = Compiler::new();
    let page_output = compiler.compile_file(&page_file, "page.adapto").unwrap();

    assert_eq!(page_output.component_ir.fills.len(), 1);
    assert_eq!(page_output.component_ir.fills[0].slot_name, "sidebar");

    let mut mgr = LayoutManager::new();
    mgr.register("main", CompiledLayout {
        name: "main".into(),
        parent: None,
        slots: vec![
            SlotDefinition { name: Some("sidebar".into()), fallback_html: Some("<p>Default</p>".into()) },
            SlotDefinition { name: None, fallback_html: None },
        ],
        template_html: "<div><aside><!-- slot:sidebar --></aside><main><!-- slot:default --></main></div>".into(),
    });

    let mut fills = HashMap::new();
    fills.insert("sidebar".to_string(), "<nav>Custom sidebar</nav>".to_string());

    let result = mgr.compose("main", "<h1>Page content</h1>", &fills).unwrap();
    assert!(result.contains("<nav>Custom sidebar</nav>"));
    assert!(result.contains("<h1>Page content</h1>"));
    assert!(!result.contains("Default"));
}

#[test]
fn multi_level_layout_chain() {
    let mut mgr = LayoutManager::new();

    mgr.register("base", CompiledLayout {
        name: "base".into(),
        parent: None,
        slots: vec![
            SlotDefinition { name: Some("head".into()), fallback_html: Some("<title>App</title>".into()) },
            SlotDefinition { name: None, fallback_html: None },
        ],
        template_html: "<html><head><!-- slot:head --></head><body><!-- slot:default --></body></html>".into(),
    });

    mgr.register("dashboard", CompiledLayout {
        name: "dashboard".into(),
        parent: Some("base".into()),
        slots: vec![
            SlotDefinition { name: None, fallback_html: None },
        ],
        template_html: "<div class=\"dashboard\"><!-- slot:default --></div>".into(),
    });

    let page_fills = HashMap::new();
    let layout_fills = HashMap::from([
        ("dashboard".to_string(), HashMap::from([
            ("head".to_string(), "<title>Dashboard</title>".to_string()),
        ])),
    ]);

    let result = mgr.compose_chain(
        "dashboard",
        "<h1>Stats page</h1>",
        &page_fills,
        &layout_fills,
    ).unwrap();

    assert!(result.contains("<title>Dashboard</title>"));
    assert!(result.contains("<div class=\"dashboard\">"));
    assert!(result.contains("<h1>Stats page</h1>"));
}

#[test]
fn layout_cycle_detected() {
    let mut mgr = LayoutManager::new();
    mgr.register("a", CompiledLayout {
        name: "a".into(),
        parent: Some("b".into()),
        slots: vec![],
        template_html: "<!-- slot:default -->".into(),
    });
    mgr.register("b", CompiledLayout {
        name: "b".into(),
        parent: Some("a".into()),
        slots: vec![],
        template_html: "<!-- slot:default -->".into(),
    });

    let result = mgr.resolve_chain("a");
    assert!(result.is_err());
}

#[test]
fn parse_layout_file_with_slots() {
    let layout_src = r#"<layout name="main">
        auth: public
    </layout>
    <template>
        <header>Nav</header>
        <slot name="sidebar"><p>Default sidebar</p></slot>
        <main><slot/></main>
    </template>"#;

    let file = adapto_parser::parse(layout_src).unwrap();
    assert!(file.layout.is_some());
    assert!(file.template.is_some());

    let layout = file.layout.as_ref().unwrap();
    assert_eq!(layout.name, "main");

    let mut compiler = Compiler::new();
    let output = compiler.compile_file(&file, "layouts/main.adapto").unwrap();
    assert_eq!(output.component_ir.slot_placeholders.len(), 2);
}
