# Template Inheritance Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add Svelte/Astro-style template inheritance with `{#fill name}...{/fill}`, named `<slot/>`, multi-level layout chains, and full .adapto layout files.

**Architecture:** Layouts become full `.adapto` files with `<slot/>` placeholders. Pages/child layouts fill slots via `{#fill name}...{/fill}`. SSR composes bottom-up through layout chains (page → child layout → parent layout). Parser gets FillNode + extended LayoutBlock, compiler gets slot/fill IR, SSR LayoutManager is rewritten for chain-based composition.

**Tech Stack:** Rust, adapto_parser (hand-written recursive descent), adapto_compiler (AST→IR), adapto_ssr (HTML rendering)

---

### Task 1: AST — Add FillNode and extend LayoutBlock

**Files:**
- Modify: `crates/adapto_parser/src/ast.rs`
- Test: `crates/adapto_parser/src/ast.rs` (derives verify via compile)

- [ ] **Step 1: Write test — verify FillNode and extended LayoutBlock compile**

Add to bottom of `crates/adapto_parser/src/ast.rs`, inside a new `#[cfg(test)] mod tests`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fill_node_construction() {
        let fill = FillNode {
            slot_name: "sidebar".to_string(),
            children: vec![TemplateNode::Text("nav content".to_string())],
        };
        assert_eq!(fill.slot_name, "sidebar");
        assert_eq!(fill.children.len(), 1);
    }

    #[test]
    fn layout_block_with_parent() {
        let layout = LayoutBlock {
            name: "dashboard".to_string(),
            parent_layout: Some("base".to_string()),
            auth: None,
            tenant: None,
        };
        assert_eq!(layout.parent_layout, Some("base".to_string()));
    }

    #[test]
    fn layout_block_without_parent() {
        let layout = LayoutBlock {
            name: "base".to_string(),
            parent_layout: None,
            auth: Some(AuthLevel::Public),
            tenant: None,
        };
        assert!(layout.parent_layout.is_none());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p adapto_parser -- ast::tests --no-run 2>&1 | head -20`
Expected: FAIL — `FillNode` not defined, `parent_layout` not a field on `LayoutBlock`

- [ ] **Step 3: Add FillNode struct and Fill variant to TemplateNode**

In `crates/adapto_parser/src/ast.rs`, add `FillNode` struct after `SlotNode`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FillNode {
    pub slot_name: String,
    pub children: Vec<TemplateNode>,
}
```

Add variant to `TemplateNode` enum:

```rust
pub enum TemplateNode {
    // ... existing variants
    Fill(FillNode),
}
```

- [ ] **Step 4: Add `parent_layout` field to LayoutBlock**

In `crates/adapto_parser/src/ast.rs`, change `LayoutBlock`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutBlock {
    pub name: String,
    pub parent_layout: Option<String>,
    pub auth: Option<AuthLevel>,
    pub tenant: Option<TenantLevel>,
}
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test -p adapto_parser -- ast::tests -v`
Expected: 3 tests PASS

- [ ] **Step 6: Fix compilation in downstream crates**

The `LayoutBlock` struct change will break `parse_layout_block` in `parser.rs` (missing `parent_layout` field). Fix by adding `parent_layout: None` to the struct literal in `parse_layout_block` at line ~2096.

Also fix any pattern matches on `TemplateNode` in `adapto_compiler/src/compiler.rs` — the `Fill` variant needs a match arm. Add a temporary placeholder:

In `compiler.rs` `compile_node` method, add arm before the final `}`:

```rust
TemplateNode::Fill(_fill) => {
    // Will be implemented in Task 5
}
```

Run: `cargo check --workspace`
Expected: PASS (no errors)

- [ ] **Step 7: Commit**

```bash
git add crates/adapto_parser/src/ast.rs crates/adapto_parser/src/parser.rs crates/adapto_compiler/src/compiler.rs
git commit -m "feat(parser): add FillNode AST type and extend LayoutBlock with parent_layout"
```

---

### Task 2: Parser — Parse `{#fill name}...{/fill}` blocks

**Files:**
- Modify: `crates/adapto_parser/src/parser.rs`

- [ ] **Step 1: Write failing tests for fill parsing**

Add at the bottom of `crates/adapto_parser/src/parser.rs` in the existing `#[cfg(test)] mod tests` block (find it with `grep -n "mod tests" crates/adapto_parser/src/parser.rs`):

```rust
#[test]
fn parse_fill_block() {
    let input = r#"<template>
        {#fill sidebar}
            <nav>Side nav</nav>
        {/fill}
        <h1>Main content</h1>
    </template>"#;
    let file = parse(input).unwrap();
    let template = file.template.unwrap();
    // Should have Fill node and Text/Element nodes
    let has_fill = template.children.iter().any(|n| matches!(n, TemplateNode::Fill(_)));
    assert!(has_fill, "Expected a Fill node in template children");

    if let Some(TemplateNode::Fill(fill)) = template.children.iter().find(|n| matches!(n, TemplateNode::Fill(_))) {
        assert_eq!(fill.slot_name, "sidebar");
        assert!(!fill.children.is_empty());
    }
}

#[test]
fn parse_multiple_fills() {
    let input = r#"<template>
        {#fill head}<title>Page</title>{/fill}
        <h1>Content</h1>
        {#fill sidebar}<nav>Nav</nav>{/fill}
    </template>"#;
    let file = parse(input).unwrap();
    let template = file.template.unwrap();
    let fills: Vec<_> = template.children.iter().filter(|n| matches!(n, TemplateNode::Fill(_))).collect();
    assert_eq!(fills.len(), 2);
}

#[test]
fn parse_fill_with_nested_control_flow() {
    let input = r#"<template>
        {#fill panel}
            {#if show}
                <div>Visible</div>
            {/if}
        {/fill}
    </template>"#;
    let file = parse(input).unwrap();
    let template = file.template.unwrap();
    if let Some(TemplateNode::Fill(fill)) = template.children.iter().find(|n| matches!(n, TemplateNode::Fill(_))) {
        assert_eq!(fill.slot_name, "panel");
        let has_if = fill.children.iter().any(|n| matches!(n, TemplateNode::If(_)));
        assert!(has_if, "Fill should contain an If node");
    } else {
        panic!("Expected Fill node");
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p adapto_parser -- parse_fill --no-run 2>&1 | head -20`
Expected: compile passes but tests FAIL (fill blocks not parsed — treated as expressions or text)

- [ ] **Step 3: Implement fill block parsing in TemplateParser**

In `crates/adapto_parser/src/parser.rs`, in `TemplateParser::parse_node` method (around line 1126), add fill detection. In the block that handles `{` tokens, add these lines **before** the plain expression fallback:

```rust
if after_brace.starts_with("#fill ") || after_brace.starts_with("#fill\t") {
    return self.parse_fill_block().map(Some);
}
```

Also add `"/fill"` to the stop-marker check (around line 1146):

```rust
if after_brace.starts_with(":else")
    || after_brace.starts_with("/if")
    || after_brace.starts_with("/each")
    || after_brace.starts_with("/match")
    || after_brace.starts_with("/can")
    || after_brace.starts_with("/fill")
{
    return Ok(None);
}
```

Then add the `parse_fill_block` method to `TemplateParser` impl (after `parse_can_block`):

```rust
fn parse_fill_block(&mut self) -> ParseResult<TemplateNode> {
    // {#fill slot_name}
    self.advance(1); // skip `{`
    let tag_content = self.consume_until_balanced_brace()?;
    let slot_name = tag_content
        .trim_start_matches("#fill")
        .trim()
        .to_string();

    if slot_name.is_empty() {
        return Err(ParseError::Syntax {
            line: 0,
            col: 0,
            message: "{#fill} requires a slot name".into(),
        });
    }

    let children = self.parse_children(&["{/fill}"])?;

    self.skip_whitespace();
    if self.remaining().starts_with("{/fill}") {
        self.advance(7);
    }

    Ok(TemplateNode::Fill(FillNode {
        slot_name,
        children,
    }))
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p adapto_parser -- parse_fill -v`
Expected: 3 tests PASS

- [ ] **Step 5: Commit**

```bash
git add crates/adapto_parser/src/parser.rs
git commit -m "feat(parser): parse {#fill name}...{/fill} template blocks"
```

---

### Task 3: Parser — Parse `layout:` field in layout block

**Files:**
- Modify: `crates/adapto_parser/src/parser.rs`

- [ ] **Step 1: Write failing test for layout parent**

Add to parser tests:

```rust
#[test]
fn parse_layout_block_with_parent() {
    let input = r#"<layout name="dashboard">
        layout: "base"
        auth: required
    </layout>"#;
    let file = parse(input).unwrap();
    let layout = file.layout.unwrap();
    assert_eq!(layout.name, "dashboard");
    assert_eq!(layout.parent_layout, Some("base".to_string()));
    assert_eq!(layout.auth, Some(AuthLevel::Required));
}

#[test]
fn parse_layout_block_without_parent() {
    let input = r#"<layout name="base">
        auth: public
    </layout>"#;
    let file = parse(input).unwrap();
    let layout = file.layout.unwrap();
    assert_eq!(layout.name, "base");
    assert!(layout.parent_layout.is_none());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p adapto_parser -- parse_layout_block -v`
Expected: `parse_layout_block_with_parent` FAILS — `parent_layout` is always `None`

- [ ] **Step 3: Add layout field parsing to parse_layout_block**

In `crates/adapto_parser/src/parser.rs`, in `parse_layout_block` function (around line 2049), add `parent_layout` variable and handle the `"layout"` key:

Change the function to:

```rust
fn parse_layout_block(
    attrs: &str,
    content: &str,
    _base_line: usize,
) -> ParseResult<LayoutBlock> {
    let attr_pairs = parse_tag_attrs(attrs);
    let name = attr_pairs
        .iter()
        .find(|(k, _)| k == "name")
        .map(|(_, v)| v.clone())
        .unwrap_or_default();

    let mut auth = None;
    let mut tenant = None;
    let mut parent_layout = None;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("//") {
            continue;
        }
        if let Some((key, value)) = parse_kv_line(line) {
            let v = unquote(&value);
            match key.as_str() {
                "auth" => {
                    auth = Some(parse_auth_level(&v).map_err(|reason| {
                        ParseError::InvalidValue {
                            field: "auth".into(),
                            value: v.clone(),
                            reason,
                        }
                    })?);
                }
                "tenant" => {
                    tenant = Some(parse_tenant_level(&v).map_err(|reason| {
                        ParseError::InvalidValue {
                            field: "tenant".into(),
                            value: v.clone(),
                            reason,
                        }
                    })?);
                }
                "layout" => {
                    parent_layout = Some(v);
                }
                _ => {}
            }
        }
    }

    Ok(LayoutBlock {
        name,
        parent_layout,
        auth,
        tenant,
    })
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p adapto_parser -- parse_layout_block -v`
Expected: PASS

- [ ] **Step 5: Run full parser test suite**

Run: `cargo test -p adapto_parser -v`
Expected: All tests PASS

- [ ] **Step 6: Commit**

```bash
git add crates/adapto_parser/src/parser.rs
git commit -m "feat(parser): parse layout: parent field in layout blocks"
```

---

### Task 4: Compiler IR — Add slot placeholder and fill segment types

**Files:**
- Modify: `crates/adapto_compiler/src/ir.rs`

- [ ] **Step 1: Write test verifying new IR types compile**

Add to bottom of `crates/adapto_compiler/src/ir.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_placeholder_default() {
        let slot = SlotPlaceholderIR {
            name: None,
            fallback: None,
        };
        assert!(slot.name.is_none());
    }

    #[test]
    fn slot_placeholder_named_with_fallback() {
        let slot = SlotPlaceholderIR {
            name: Some("sidebar".to_string()),
            fallback: Some(SegmentBody {
                static_segments: vec!["<p>Default sidebar</p>".into()],
                dynamic_segments: vec![],
            }),
        };
        assert_eq!(slot.name, Some("sidebar".to_string()));
        assert!(slot.fallback.is_some());
    }

    #[test]
    fn fill_segment_construction() {
        let fill = FillSegmentIR {
            slot_name: "head".to_string(),
            body: SegmentBody {
                static_segments: vec!["<title>My Page</title>".into()],
                dynamic_segments: vec![],
            },
        };
        assert_eq!(fill.slot_name, "head");
    }

    #[test]
    fn component_ir_with_slots_and_fills() {
        let ir = ComponentIR {
            id: "test".into(),
            name: "Test".into(),
            route: None,
            static_segments: vec![],
            dynamic_segments: vec![],
            events: vec![],
            actions: vec![],
            state_fields: vec![],
            form_schemas: vec![],
            permissions: vec![],
            children: vec![],
            is_island: false,
            style: None,
            slot_placeholders: vec![SlotPlaceholderIR { name: None, fallback: None }],
            fills: vec![FillSegmentIR {
                slot_name: "head".into(),
                body: SegmentBody {
                    static_segments: vec![],
                    dynamic_segments: vec![],
                },
            }],
        };
        assert_eq!(ir.slot_placeholders.len(), 1);
        assert_eq!(ir.fills.len(), 1);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p adapto_compiler -- ir::tests --no-run 2>&1 | head -20`
Expected: FAIL — types not defined

- [ ] **Step 3: Add new IR types and extend ComponentIR**

In `crates/adapto_compiler/src/ir.rs`, add after `LoopBody`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotPlaceholderIR {
    pub name: Option<String>,
    pub fallback: Option<SegmentBody>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FillSegmentIR {
    pub slot_name: String,
    pub body: SegmentBody,
}
```

Add two fields to `ComponentIR`:

```rust
pub struct ComponentIR {
    // ... existing fields ...
    pub style: Option<CompiledStyle>,
    pub slot_placeholders: Vec<SlotPlaceholderIR>,
    pub fills: Vec<FillSegmentIR>,
}
```

- [ ] **Step 4: Fix all ComponentIR construction sites**

Every place that constructs a `ComponentIR` needs the new fields. Search and add `slot_placeholders: vec![], fills: vec![]` to each:

1. `crates/adapto_compiler/src/compiler.rs` — in `compile_file` (around line 126)
2. `crates/adapto_ssr/src/renderer.rs` — in test helpers `minimal_ir()`, `ir_with_dynamics()`, `ir_with_events()`
3. `crates/adapto_ssr/src/page.rs` — in test helper `test_ir()`

Run: `cargo check --workspace`
Expected: PASS

- [ ] **Step 5: Run tests**

Run: `cargo test -p adapto_compiler -- ir::tests -v`
Expected: 4 tests PASS

- [ ] **Step 6: Commit**

```bash
git add crates/adapto_compiler/src/ir.rs crates/adapto_compiler/src/compiler.rs crates/adapto_ssr/src/renderer.rs crates/adapto_ssr/src/page.rs
git commit -m "feat(compiler): add SlotPlaceholderIR and FillSegmentIR to component IR"
```

---

### Task 5: Compiler — Compile Fill and Slot nodes into IR

**Files:**
- Modify: `crates/adapto_compiler/src/compiler.rs`
- Modify: `crates/adapto_compiler/src/error.rs`

- [ ] **Step 1: Write failing tests**

Add to `crates/adapto_compiler/src/compiler.rs` test module:

```rust
#[test]
fn compile_fill_node() {
    let file = adapto_parser::parse(r#"<route>
        path: "/test"
        layout: "main"
    </route>
    <template>
        {#fill sidebar}
            <nav>Side</nav>
        {/fill}
        <h1>Main</h1>
    </template>"#).unwrap();

    let mut compiler = Compiler::new();
    let output = compiler.compile_file(&file, "test.adapto").unwrap();
    assert_eq!(output.component_ir.fills.len(), 1);
    assert_eq!(output.component_ir.fills[0].slot_name, "sidebar");
}

#[test]
fn compile_slot_with_fallback() {
    let file = adapto_parser::parse(r#"<layout name="base">
    </layout>
    <template>
        <slot name="sidebar"><p>Default</p></slot>
        <slot/>
    </template>"#).unwrap();

    let mut compiler = Compiler::new();
    let output = compiler.compile_file(&file, "layouts/base.adapto").unwrap();
    assert_eq!(output.component_ir.slot_placeholders.len(), 2);
    assert_eq!(output.component_ir.slot_placeholders[0].name, Some("sidebar".to_string()));
    assert!(output.component_ir.slot_placeholders[0].fallback.is_some());
    assert!(output.component_ir.slot_placeholders[1].name.is_none());
}

#[test]
fn compile_duplicate_fill_error() {
    let file = adapto_parser::parse(r#"<route>
        path: "/test"
        layout: "main"
    </route>
    <template>
        {#fill sidebar}<p>A</p>{/fill}
        {#fill sidebar}<p>B</p>{/fill}
    </template>"#).unwrap();

    let mut compiler = Compiler::new();
    let result = compiler.compile_file(&file, "test.adapto");
    assert!(result.is_err());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p adapto_compiler -- compile_fill compile_slot compile_duplicate --no-run 2>&1 | head -20`
Expected: compile succeeds but tests FAIL

- [ ] **Step 3: Add DuplicateFill error variant**

In `crates/adapto_compiler/src/error.rs`, add:

```rust
#[error("E0601: Duplicate fill for slot `{slot_name}` in {file}")]
DuplicateFill { slot_name: String, file: String },
```

- [ ] **Step 4: Implement Fill compilation in compile_node**

In `crates/adapto_compiler/src/compiler.rs`, replace the placeholder `TemplateNode::Fill` arm in `compile_node`:

```rust
TemplateNode::Fill(fill) => {
    let body = self.compile_body(&fill.children, component_id, source_path, events)?;
    // Fills are collected separately — they don't produce static/dynamic segments
    // Store in a thread-local or pass through. We use a separate collection pass.
    // For now, emit a comment marker so the slot can be identified.
    Self::append_static(
        static_parts,
        dynamic_parts,
        format!("<!-- fill:{} -->", fill.slot_name),
    );
    // Actual fill bodies are collected in compile_file after template compilation.
    // We store a marker that compile_file will process.
}
```

Actually, a cleaner approach — collect fills in `compile_file` by walking the AST before template compilation:

In `compile_file`, after deriving component name and before compiling template, add a fill extraction pass:

```rust
// Extract fill blocks from template
let fills = if let Some(ref template) = file.template {
    self.extract_fills(&template.children, source_path)?
} else {
    Vec::new()
};
```

Add method `extract_fills`:

```rust
fn extract_fills(
    &mut self,
    nodes: &[TemplateNode],
    source_path: &str,
) -> Result<Vec<FillSegmentIR>, CompileError> {
    let mut fills = Vec::new();
    let mut seen_names: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut events = Vec::new();

    for node in nodes {
        if let TemplateNode::Fill(fill) = node {
            if !seen_names.insert(fill.slot_name.clone()) {
                return Err(CompileError::DuplicateFill {
                    slot_name: fill.slot_name.clone(),
                    file: source_path.to_string(),
                });
            }
            let body = self.compile_body(
                &fill.children,
                &format!("fill_{}", fill.slot_name),
                source_path,
                &mut events,
            )?;
            fills.push(FillSegmentIR {
                slot_name: fill.slot_name.clone(),
                body,
            });
        }
    }
    Ok(fills)
}
```

Then in `compile_node`, the `Fill` arm should skip the node (fills are already extracted):

```rust
TemplateNode::Fill(_fill) => {
    // Fills extracted separately in extract_fills — skip in template output
}
```

- [ ] **Step 5: Implement Slot compilation with fallback**

In `compile_node`, replace the existing `TemplateNode::Slot` arm:

```rust
TemplateNode::Slot(slot) => {
    let fallback = if slot.fallback.is_empty() {
        None
    } else {
        let mut slot_events = Vec::new();
        Some(self.compile_body(
            &slot.fallback,
            component_id,
            source_path,
            &mut slot_events,
        )?)
    };
    // We don't collect slot_placeholders here — we collect them in compile_file
    // For static output, emit a slot marker
    Self::append_static(
        static_parts,
        dynamic_parts,
        format!("<!-- slot:{} -->", slot.name.as_deref().unwrap_or("default")),
    );
}
```

Add slot extraction in `compile_file`, similar to fills:

```rust
// Extract slot placeholders from template (for layout files)
let slot_placeholders = if let Some(ref template) = file.template {
    self.extract_slots(&template.children, &component_id, source_path)?
} else {
    Vec::new()
};
```

Add method:

```rust
fn extract_slots(
    &mut self,
    nodes: &[TemplateNode],
    component_id: &str,
    source_path: &str,
) -> Result<Vec<SlotPlaceholderIR>, CompileError> {
    let mut slots = Vec::new();
    self.collect_slots(nodes, component_id, source_path, &mut slots)?;
    Ok(slots)
}

fn collect_slots(
    &mut self,
    nodes: &[TemplateNode],
    component_id: &str,
    source_path: &str,
    slots: &mut Vec<SlotPlaceholderIR>,
) -> Result<(), CompileError> {
    for node in nodes {
        match node {
            TemplateNode::Slot(slot) => {
                let fallback = if slot.fallback.is_empty() {
                    None
                } else {
                    let mut events = Vec::new();
                    Some(self.compile_body(&slot.fallback, component_id, source_path, &mut events)?)
                };
                slots.push(SlotPlaceholderIR {
                    name: slot.name.clone(),
                    fallback,
                });
            }
            TemplateNode::Element(el) => {
                self.collect_slots(&el.children, component_id, source_path, slots)?;
            }
            TemplateNode::If(if_node) => {
                self.collect_slots(&if_node.then_branch, component_id, source_path, slots)?;
                for (_, branch) in &if_node.else_if_branches {
                    self.collect_slots(branch, component_id, source_path, slots)?;
                }
                if let Some(ref branch) = if_node.else_branch {
                    self.collect_slots(branch, component_id, source_path, slots)?;
                }
            }
            TemplateNode::Each(each) => {
                self.collect_slots(&each.children, component_id, source_path, slots)?;
            }
            _ => {}
        }
    }
    Ok(())
}
```

Then wire both into the `ComponentIR` construction in `compile_file`:

```rust
let ir = ComponentIR {
    // ... existing fields ...
    slot_placeholders,
    fills,
};
```

- [ ] **Step 6: Run tests**

Run: `cargo test -p adapto_compiler -- compile_fill compile_slot compile_duplicate -v`
Expected: 3 tests PASS

- [ ] **Step 7: Run full workspace tests**

Run: `cargo test --workspace`
Expected: All tests PASS

- [ ] **Step 8: Commit**

```bash
git add crates/adapto_compiler/src/compiler.rs crates/adapto_compiler/src/error.rs crates/adapto_compiler/src/ir.rs
git commit -m "feat(compiler): compile Fill and Slot nodes into IR with duplicate detection"
```

---

### Task 6: SSR — Add new error variants

**Files:**
- Modify: `crates/adapto_ssr/src/error.rs`

- [ ] **Step 1: Add error variants**

In `crates/adapto_ssr/src/error.rs`, add to `SsrError` enum:

```rust
#[error("Layout cycle detected: {0}")]
LayoutCycle(String),

#[error("Layout chain exceeds maximum depth of {0}")]
LayoutDepthExceeded(usize),
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check -p adapto_ssr`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add crates/adapto_ssr/src/error.rs
git commit -m "feat(ssr): add LayoutCycle and LayoutDepthExceeded error variants"
```

---

### Task 7: SSR — Rewrite LayoutManager for chain-based composition

**Files:**
- Modify: `crates/adapto_ssr/src/layout.rs`

- [ ] **Step 1: Write failing tests for new LayoutManager**

Replace the entire test module in `crates/adapto_ssr/src/layout.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use adapto_compiler::ir::*;

    fn make_segment_body(html: &str) -> SegmentBody {
        SegmentBody {
            static_segments: vec![html.to_string()],
            dynamic_segments: vec![],
        }
    }

    #[test]
    fn register_and_compose_simple() {
        let mut mgr = LayoutManager::new();
        mgr.register("main", CompiledLayout {
            name: "main".into(),
            parent: None,
            slots: vec![
                SlotDefinition { name: None, fallback_html: None },
            ],
            template_html: "<html><body><nav>Menu</nav><main><!-- slot:default --></main></body></html>".into(),
        });

        let mut fills = HashMap::new();
        let result = mgr.compose("main", "<h1>Hello</h1>", &fills).unwrap();
        assert!(result.contains("<nav>Menu</nav>"));
        assert!(result.contains("<h1>Hello</h1>"));
    }

    #[test]
    fn named_slot_with_fill() {
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
        let result = mgr.compose("main", "<h1>Content</h1>", &fills).unwrap();
        assert!(result.contains("<nav>Custom sidebar</nav>"));
        assert!(result.contains("<h1>Content</h1>"));
        assert!(!result.contains("Default"));
    }

    #[test]
    fn named_slot_fallback_when_no_fill() {
        let mut mgr = LayoutManager::new();
        mgr.register("main", CompiledLayout {
            name: "main".into(),
            parent: None,
            slots: vec![
                SlotDefinition { name: Some("sidebar".into()), fallback_html: Some("<p>Default sidebar</p>".into()) },
                SlotDefinition { name: None, fallback_html: None },
            ],
            template_html: "<div><aside><!-- slot:sidebar --></aside><main><!-- slot:default --></main></div>".into(),
        });

        let fills = HashMap::new();
        let result = mgr.compose("main", "<h1>Content</h1>", &fills).unwrap();
        assert!(result.contains("<p>Default sidebar</p>"));
    }

    #[test]
    fn multi_level_chain() {
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
            template_html: "<div class=\"dash\"><!-- slot:default --></div>".into(),
            // dashboard fills base's head slot
        });

        // Page fills dashboard's default slot
        let mut page_fills = HashMap::new();
        let result = mgr.compose_chain("dashboard", "<h1>Stats</h1>", &page_fills, &HashMap::from([
            ("dashboard".to_string(), HashMap::from([
                ("head".to_string(), "<title>Dashboard</title>".to_string()),
            ])),
        ])).unwrap();

        assert!(result.contains("<title>Dashboard</title>"));
        assert!(result.contains("<div class=\"dash\">"));
        assert!(result.contains("<h1>Stats</h1>"));
    }

    #[test]
    fn cycle_detection() {
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
    fn missing_layout_error() {
        let mgr = LayoutManager::new();
        let result = mgr.compose("nonexistent", "<p>test</p>", &HashMap::new());
        assert!(result.is_err());
    }

    #[test]
    fn has_layout_check() {
        let mut mgr = LayoutManager::new();
        assert!(!mgr.has_layout("main"));
        mgr.register("main", CompiledLayout {
            name: "main".into(),
            parent: None,
            slots: vec![],
            template_html: "<!-- slot:default -->".into(),
        });
        assert!(mgr.has_layout("main"));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p adapto_ssr -- layout::tests --no-run 2>&1 | head -30`
Expected: FAIL — `CompiledLayout`, `SlotDefinition`, `compose_chain`, `resolve_chain` don't exist

- [ ] **Step 3: Rewrite LayoutManager**

Replace the contents of `crates/adapto_ssr/src/layout.rs` (keep the test module):

```rust
use std::collections::{HashMap, HashSet};

use crate::error::SsrError;

const MAX_LAYOUT_DEPTH: usize = 10;

pub struct CompiledLayout {
    pub name: String,
    pub parent: Option<String>,
    pub slots: Vec<SlotDefinition>,
    pub template_html: String,
}

pub struct SlotDefinition {
    pub name: Option<String>,
    pub fallback_html: Option<String>,
}

pub struct LayoutManager {
    layouts: HashMap<String, CompiledLayout>,
}

impl LayoutManager {
    pub fn new() -> Self {
        Self {
            layouts: HashMap::new(),
        }
    }

    pub fn register(&mut self, name: &str, layout: CompiledLayout) {
        self.layouts.insert(name.to_string(), layout);
    }

    pub fn has_layout(&self, name: &str) -> bool {
        self.layouts.contains_key(name)
    }

    /// Resolve the layout chain from innermost to outermost.
    /// Returns layout names in order: [innermost, ..., outermost].
    pub fn resolve_chain(&self, layout_name: &str) -> Result<Vec<&str>, SsrError> {
        let mut chain = Vec::new();
        let mut visited = HashSet::new();
        let mut current = layout_name;

        loop {
            if !visited.insert(current) {
                return Err(SsrError::LayoutCycle(current.to_string()));
            }
            if chain.len() >= MAX_LAYOUT_DEPTH {
                return Err(SsrError::LayoutDepthExceeded(MAX_LAYOUT_DEPTH));
            }

            let layout = self.layouts.get(current)
                .ok_or_else(|| SsrError::LayoutNotFound(current.to_string()))?;
            chain.push(current);

            match &layout.parent {
                Some(parent) => current = parent.as_str(),
                None => break,
            }
        }

        Ok(chain)
    }

    /// Simple single-layout composition (no chain).
    /// Default slot receives `page_default`, named slots use `fills`.
    pub fn compose(
        &self,
        layout_name: &str,
        page_default: &str,
        fills: &HashMap<String, String>,
    ) -> Result<String, SsrError> {
        let layout = self.layouts.get(layout_name)
            .ok_or_else(|| SsrError::LayoutNotFound(layout_name.to_string()))?;

        Ok(self.apply_slots(&layout.template_html, &layout.slots, page_default, fills))
    }

    /// Multi-level chain composition.
    /// `layout_fills` maps layout name → (slot_name → rendered HTML).
    pub fn compose_chain(
        &self,
        layout_name: &str,
        page_default: &str,
        page_fills: &HashMap<String, String>,
        layout_fills: &HashMap<String, HashMap<String, String>>,
    ) -> Result<String, SsrError> {
        let chain = self.resolve_chain(layout_name)?;

        // Start with page content
        let mut current_default = page_default.to_string();
        let mut current_fills = page_fills.clone();

        // Compose bottom-up: innermost layout first
        for &name in &chain {
            let layout = self.layouts.get(name).unwrap();
            let composed = self.apply_slots(
                &layout.template_html,
                &layout.slots,
                &current_default,
                &current_fills,
            );

            // The composed result becomes the default content for the parent
            current_default = composed;

            // Switch to this layout's fills for its parent
            current_fills = layout_fills
                .get(name)
                .cloned()
                .unwrap_or_default();
        }

        Ok(current_default)
    }

    /// Replace slot markers in template HTML with fills or fallbacks.
    fn apply_slots(
        &self,
        template: &str,
        slots: &[SlotDefinition],
        default_content: &str,
        fills: &HashMap<String, String>,
    ) -> String {
        let mut result = template.to_string();

        for slot in slots {
            let marker = match &slot.name {
                Some(name) => format!("<!-- slot:{} -->", name),
                None => "<!-- slot:default -->".to_string(),
            };

            let replacement = match &slot.name {
                Some(name) => {
                    fills.get(name.as_str())
                        .cloned()
                        .or_else(|| slot.fallback_html.clone())
                        .unwrap_or_default()
                }
                None => default_content.to_string(),
            };

            result = result.replace(&marker, &replacement);
        }

        // Also handle {slot} for backwards compatibility
        result = result.replace("{slot}", default_content);

        result
    }
}

impl Default for LayoutManager {
    fn default() -> Self {
        Self::new()
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p adapto_ssr -- layout::tests -v`
Expected: All 7 tests PASS

- [ ] **Step 5: Commit**

```bash
git add crates/adapto_ssr/src/layout.rs
git commit -m "feat(ssr): rewrite LayoutManager with named slots, fallbacks, and chain composition"
```

---

### Task 8: SSR — Update PageRenderer and Renderer for new layout system

**Files:**
- Modify: `crates/adapto_ssr/src/page.rs`
- Modify: `crates/adapto_ssr/src/renderer.rs`

- [ ] **Step 1: Update PageRenderer to use new LayoutManager API**

In `crates/adapto_ssr/src/page.rs`, the `render_request` method currently calls `self.get_layout_template()` which returns an HTML string with `{slot}`. Update the layout composition section (around line 133-154) to use the new `compose` method:

Replace lines 133-154 with:

```rust
// 4 + 5. Render component, then compose with layout.
let (content_html, session_id) = self.renderer.render_page(
    ir,
    &initial_state,
    None, // No layout wrapping in render_page — we do it ourselves
)?;

let final_html = if let Some(ref layout_name) = route_match.layout {
    if self.layouts.has_layout(layout_name) {
        let fills = HashMap::new(); // Pages don't have fills yet in basic flow
        self.layouts.compose(layout_name, &content_html, &fills)?
    } else {
        content_html
    }
} else {
    content_html
};
```

Wait — `render_page` wraps in `<!DOCTYPE html>`. We need it to NOT wrap when there's a layout, since the layout provides the HTML shell. Actually, looking at the code more carefully, `render_page` always calls `wrap_page` which adds `<!DOCTYPE html><html>...`. The layout template should provide the outer HTML, not `wrap_page`.

This is a bigger change. For now, keep the existing approach: `render_page` renders the component fragment + bootstrap, and the layout wraps it. The `Renderer::render_page` already handles `layout_html: Option<&str>` parameter. We just need `PageRenderer` to get the template string from the new `LayoutManager`.

Simpler approach — add a `get_template` method to `LayoutManager`:

In `crates/adapto_ssr/src/layout.rs`, add:

```rust
pub fn get_template(&self, name: &str) -> Option<&str> {
    self.layouts.get(name).map(|l| l.template_html.as_str())
}
```

Then in `crates/adapto_ssr/src/page.rs`, replace the layout template retrieval (lines 133-148):

```rust
let layout_template = route_match
    .layout
    .as_ref()
    .and_then(|name| self.layouts.get_template(name));
```

And remove the `get_layout_template` helper method (lines 174-177).

- [ ] **Step 2: Fix PageRenderer tests**

Update `build_page_renderer` in page.rs tests to use the new `LayoutManager::register` API:

```rust
fn build_page_renderer() -> PageRenderer {
    let mut pr = PageRenderer::new(b"test-secret-key");
    pr.set_router(Router::new(test_manifest()));

    let mut layouts = LayoutManager::new();
    layouts.register(
        "main",
        crate::layout::CompiledLayout {
            name: "main".into(),
            parent: None,
            slots: vec![],
            template_html: "<html><body><nav>Nav</nav><main>{slot}</main></body></html>".into(),
        },
    );
    pr.set_layouts(layouts);

    pr.register_component("home", test_ir("home", "Home"));
    pr.register_component("dashboard", test_ir("dashboard", "Dashboard"));
    pr.register_component("admin", test_ir("admin", "Admin"));

    pr
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p adapto_ssr -v`
Expected: All existing tests PASS

- [ ] **Step 4: Commit**

```bash
git add crates/adapto_ssr/src/page.rs crates/adapto_ssr/src/layout.rs crates/adapto_ssr/src/renderer.rs
git commit -m "feat(ssr): update PageRenderer to use new LayoutManager API"
```

---

### Task 9: Update grammar documentation

**Files:**
- Modify: `crates/adapto_parser/src/grammar.pest`

- [ ] **Step 1: Update PEG grammar docs**

In `crates/adapto_parser/src/grammar.pest`, add `fill_block` to `template_node`:

```pest
template_node = {
    fill_block
  | if_block | each_block | match_block | can_block
  | unsafe_html | expression | element | component
  | slot_element | text_node
}
```

Add the fill_block rule after `can_block`:

```pest
fill_block  = { "{#fill" ~ ident ~ "}" ~ template_body ~ "{/fill}" }
```

Add `"layout"` to `layout_key`:

```pest
layout_key = { "auth" | "tenant" | "layout" }
```

- [ ] **Step 2: Commit**

```bash
git add crates/adapto_parser/src/grammar.pest
git commit -m "docs(parser): update PEG grammar with fill_block and layout parent field"
```

---

### Task 10: Integration test — Full parse→compile→compose pipeline

**Files:**
- Create: `crates/adapto_ssr/tests/layout_inheritance.rs`

- [ ] **Step 1: Write integration test**

```rust
//! Integration test: parse → compile → SSR compose with layout inheritance.

use std::collections::HashMap;
use adapto_compiler::compiler::Compiler;
use adapto_ssr::layout::{CompiledLayout, LayoutManager, SlotDefinition};

#[test]
fn single_layout_with_named_slots() {
    // Parse and compile a page with fills
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

    // Verify fills were extracted
    assert_eq!(page_output.component_ir.fills.len(), 1);
    assert_eq!(page_output.component_ir.fills[0].slot_name, "sidebar");

    // Set up layout manager with a layout that has named slots
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

    // Compose
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

    // Base layout: html shell with head + default slots
    mgr.register("base", CompiledLayout {
        name: "base".into(),
        parent: None,
        slots: vec![
            SlotDefinition { name: Some("head".into()), fallback_html: Some("<title>App</title>".into()) },
            SlotDefinition { name: None, fallback_html: None },
        ],
        template_html: "<html><head><!-- slot:head --></head><body><!-- slot:default --></body></html>".into(),
    });

    // Dashboard layout: wraps content in dashboard grid, fills base's head
    mgr.register("dashboard", CompiledLayout {
        name: "dashboard".into(),
        parent: Some("base".into()),
        slots: vec![
            SlotDefinition { name: None, fallback_html: None },
        ],
        template_html: "<div class=\"dashboard\"><!-- slot:default --></div>".into(),
    });

    // Compose: page → dashboard → base
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

    let layout = file.layout.unwrap();
    assert_eq!(layout.name, "main");

    let mut compiler = Compiler::new();
    let output = compiler.compile_file(&file, "layouts/main.adapto").unwrap();
    assert_eq!(output.component_ir.slot_placeholders.len(), 2);
}
```

- [ ] **Step 2: Run integration tests**

Run: `cargo test -p adapto_ssr --test layout_inheritance -v`
Expected: 4 tests PASS

- [ ] **Step 3: Run full workspace test suite**

Run: `cargo test --workspace`
Expected: All tests PASS

- [ ] **Step 4: Commit**

```bash
git add crates/adapto_ssr/tests/layout_inheritance.rs
git commit -m "test: add integration tests for template inheritance pipeline"
```

---

### Task 11: Update CHANGELOG.md

**Files:**
- Modify: `CHANGELOG.md`

- [ ] **Step 1: Add changelog entry**

Add under the latest unreleased version at the top of `CHANGELOG.md`:

```markdown
### Added
- **Template Inheritance** — layouts are now full `.adapto` files with `<slot/>` and `<slot name="..."/>` placeholders
  - Pages fill named slots via `{#fill name}...{/fill}` syntax
  - Multi-level layout chains: page → child layout → parent layout
  - Named slots with fallback content
  - Cycle detection and max depth (10) enforcement
  - `adapto_parser`: `FillNode` AST type, `{#fill}` parsing, `layout:` field in `<layout>` block
  - `adapto_compiler`: `SlotPlaceholderIR`, `FillSegmentIR`, duplicate fill detection
  - `adapto_ssr`: rewritten `LayoutManager` with chain resolution and slot-based composition
```

- [ ] **Step 2: Commit**

```bash
git add CHANGELOG.md
git commit -m "docs: add template inheritance to CHANGELOG"
```
