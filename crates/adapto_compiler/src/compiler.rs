use std::collections::HashSet;

use adapto_parser::ast::*;

use crate::codegen::CodeGenerator;
use crate::dependency::DependencyGraph;
use crate::error::CompileError;
use crate::ir::*;
use crate::manifest::*;

/// The output of compiling a single `.adapto` file.
#[derive(Debug)]
pub struct CompileOutput {
    pub component_ir: ComponentIR,
    pub dependency_graph: DependencyGraph,
    pub generated_rust: String,
    pub route_entry: Option<RouteEntry>,
}

/// The compiler orchestrator.
///
/// Transforms parsed `AdaptoFile` ASTs into fully analyzed component IRs,
/// route manifests, dependency graphs, and generated Rust code.
pub struct Compiler {
    route_manifest: RouteManifest,
    component_manifest: ComponentManifest,
    dyn_counter: usize,
    event_counter: usize,
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            route_manifest: RouteManifest::new(),
            component_manifest: ComponentManifest::new(),
            dyn_counter: 0,
            event_counter: 0,
        }
    }

    /// Access the accumulated route manifest.
    pub fn route_manifest(&self) -> &RouteManifest {
        &self.route_manifest
    }

    /// Access the accumulated component manifest.
    pub fn component_manifest(&self) -> &ComponentManifest {
        &self.component_manifest
    }

    /// Compile a single `.adapto` file to IR.
    pub fn compile_file(
        &mut self,
        file: &AdaptoFile,
        source_path: &str,
    ) -> Result<CompileOutput, CompileError> {
        // Reset per-file counters
        self.dyn_counter = 0;
        self.event_counter = 0;

        // Derive component name from source path
        let component_name = derive_component_name(source_path);
        let component_id = format!("comp_{}", component_name.to_lowercase());

        // Compile script block -> actions, state fields, form schemas
        let (actions, state_fields, form_schemas) = if let Some(ref script) = file.script {
            self.compile_script(script)?
        } else {
            (Vec::new(), Vec::new(), Vec::new())
        };

        // Compile template block -> static/dynamic segments, events
        let (static_segments, dynamic_segments, events) =
            if let Some(ref template) = file.template {
                self.compile_template(template, &component_id, source_path)?
            } else {
                (Vec::new(), Vec::new(), Vec::new())
            };

        // Compile route block
        let route_ir = file
            .route
            .as_ref()
            .map(|r| self.compile_route_ir(r));

        let route_entry = file
            .route
            .as_ref()
            .map(|r| self.compile_route(r, source_path));

        // Compile style block
        let style = file.style.as_ref().map(|s| {
            let scope_id = if s.scoped {
                Some(format!("sc_{}", component_name.to_lowercase()))
            } else {
                None
            };
            CompiledStyle {
                css: s.content.clone(),
                scoped: s.scoped,
                scope_id,
            }
        });

        // Collect permissions from actions and route
        let mut permissions: Vec<String> = actions
            .iter()
            .filter_map(|a| a.permission.clone())
            .collect();
        if let Some(ref route) = route_ir {
            if let Some(ref perm) = route.permission {
                if !permissions.contains(perm) {
                    permissions.push(perm.clone());
                }
            }
        }

        // Collect child component references from template
        let children = if let Some(ref template) = file.template {
            collect_child_components(&template.children)
        } else {
            Vec::new()
        };

        let ir = ComponentIR {
            id: component_id.clone(),
            name: component_name.clone(),
            route: route_ir,
            static_segments,
            dynamic_segments,
            events,
            actions,
            state_fields,
            form_schemas,
            permissions,
            children,
            is_island: false,
            style,
        };

        // Security checks
        self.check_security(&ir, source_path)?;

        // Build dependency graph
        let dependency_graph = self.build_dependency_graph(&ir);

        // Generate Rust code
        let mut codegen = CodeGenerator::new();
        let generated_rust = codegen.generate_component(&ir);

        // Register in route manifest
        if let Some(entry) = route_entry.clone() {
            self.route_manifest.add(entry);
        }

        // Register in component manifest
        let deps: Vec<String> = dependency_graph.all_state_fields().into_iter().collect();
        self.component_manifest.add(ComponentEntry {
            id: component_id,
            name: component_name,
            file: source_path.to_string(),
            is_island: ir.is_island,
            dependencies: deps,
        });

        Ok(CompileOutput {
            component_ir: ir,
            dependency_graph,
            generated_rust,
            route_entry,
        })
    }

    /// Compile template AST to static/dynamic segments and events.
    fn compile_template(
        &mut self,
        template: &TemplateBlock,
        component_id: &str,
        source_path: &str,
    ) -> Result<(Vec<String>, Vec<DynamicSegment>, Vec<EventIR>), CompileError> {
        let mut static_parts = Vec::new();
        let mut dynamic_parts = Vec::new();
        let mut events = Vec::new();

        for node in &template.children {
            self.compile_node(
                node,
                &mut static_parts,
                &mut dynamic_parts,
                &mut events,
                component_id,
                source_path,
            )?;
        }

        Ok((static_parts, dynamic_parts, events))
    }

    fn append_static(
        static_parts: &mut Vec<String>,
        dynamic_parts: &[DynamicSegment],
        text: String,
    ) {
        if static_parts.len() > dynamic_parts.len() {
            static_parts.last_mut().unwrap().push_str(&text);
        } else {
            static_parts.push(text);
        }
    }

    /// Compile a single template node recursively.
    fn compile_node(
        &mut self,
        node: &TemplateNode,
        static_parts: &mut Vec<String>,
        dynamic_parts: &mut Vec<DynamicSegment>,
        events: &mut Vec<EventIR>,
        component_id: &str,
        source_path: &str,
    ) -> Result<(), CompileError> {
        match node {
            TemplateNode::Text(text) => {
                if !text.is_empty() {
                    Self::append_static(static_parts, dynamic_parts, text.clone());
                }
            }

            TemplateNode::Expression(expr_node) => {
                if static_parts.len() <= dynamic_parts.len() {
                    static_parts.push(String::new());
                }
                let dyn_id = self.next_dyn_id();
                let deps = extract_deps_from_expr(&expr_node.expr);
                dynamic_parts.push(DynamicSegment::new(
                    dyn_id,
                    format!("state.{}", expr_node.expr),
                    deps,
                    SegmentType::Text,
                ));
            }

            TemplateNode::UnsafeHtml(expr) => {
                if static_parts.len() <= dynamic_parts.len() {
                    static_parts.push(String::new());
                }
                let dyn_id = self.next_dyn_id();
                let deps = extract_deps_from_expr(expr);
                dynamic_parts.push(DynamicSegment::new(
                    dyn_id,
                    format!("state.{}", expr),
                    deps,
                    SegmentType::Html,
                ));
            }

            TemplateNode::Element(elem) => {
                let element_id = format!("el_{}_{}", elem.tag, static_parts.len());

                // Opening tag
                let mut open_tag = format!("<{}", elem.tag);

                // Static attributes
                for attr in &elem.attributes {
                    match &attr.value {
                        AttributeValue::Static(val) => {
                            open_tag.push_str(&format!(" {}=\"{}\"", attr.name, val));
                        }
                        AttributeValue::Dynamic(expr) => {
                            let dyn_id = self.next_dyn_id();
                            let deps = extract_deps_from_expr(expr);
                            dynamic_parts.push(DynamicSegment::new(
                                dyn_id,
                                format!("state.{}", expr),
                                deps,
                                SegmentType::Attribute {
                                    element_id: element_id.clone(),
                                    attr_name: attr.name.clone(),
                                },
                            ));
                            open_tag.push_str(&format!(
                                " {}=\"\"",
                                attr.name
                            ));
                        }
                        AttributeValue::None => {
                            open_tag.push_str(&format!(" {}", attr.name));
                        }
                    }
                }

                // Event bindings as data attributes
                for event in &elem.events {
                    let event_id = self.next_event_id();
                    open_tag.push_str(&format!(
                        " data-ar-{}=\"{}\"",
                        event.event, event.handler
                    ));

                    let modifiers: Vec<String> = event
                        .modifiers
                        .iter()
                        .map(|m| match m {
                            EventModifier::Prevent => "prevent".to_string(),
                            EventModifier::Stop => "stop".to_string(),
                            EventModifier::Debounce(ms) => format!("debounce:{}", ms),
                            EventModifier::Throttle(ms) => format!("throttle:{}", ms),
                        })
                        .collect();

                    events.push(EventIR {
                        id: event_id,
                        event_type: event.event.clone(),
                        handler: event.handler.clone(),
                        component_id: component_id.to_string(),
                        modifiers,
                        element_id: element_id.clone(),
                    });
                }

                // Bindings as data attributes
                for binding in &elem.bindings {
                    open_tag.push_str(&format!(
                        " data-ar-bind-{}=\"{}\"",
                        binding.kind, binding.target
                    ));
                }

                if elem.self_closing {
                    open_tag.push_str(" />");
                    Self::append_static(static_parts, dynamic_parts, open_tag);
                } else {
                    open_tag.push('>');
                    Self::append_static(static_parts, dynamic_parts, open_tag);

                    // Children
                    for child in &elem.children {
                        self.compile_node(
                            child,
                            static_parts,
                            dynamic_parts,
                            events,
                            component_id,
                            source_path,
                        )?;
                    }

                    // Closing tag
                    Self::append_static(static_parts, dynamic_parts, format!("</{}>", elem.tag));
                }
            }

            TemplateNode::If(if_node) => {
                if static_parts.len() <= dynamic_parts.len() {
                    static_parts.push(String::new());
                }
                let dyn_id = self.next_dyn_id();
                let deps = extract_deps_from_expr(&if_node.condition);

                let then_body = self.compile_body(&if_node.then_branch, component_id, source_path, events)?;

                let mut else_if_bodies = Vec::new();
                for (cond, branch) in &if_node.else_if_branches {
                    let body = self.compile_body(branch, component_id, source_path, events)?;
                    else_if_bodies.push((cond.clone(), body));
                }

                let else_body = if let Some(ref else_branch) = if_node.else_branch {
                    Some(self.compile_body(else_branch, component_id, source_path, events)?)
                } else {
                    None
                };

                let mut seg = DynamicSegment::new(
                    dyn_id,
                    if_node.condition.clone(),
                    deps,
                    SegmentType::Conditional,
                );
                seg.then_body = Some(then_body);
                seg.else_if_bodies = else_if_bodies;
                seg.else_body = else_body;
                dynamic_parts.push(seg);
            }

            TemplateNode::Each(each_node) => {
                if static_parts.len() <= dynamic_parts.len() {
                    static_parts.push(String::new());
                }
                let dyn_id = self.next_dyn_id();
                let deps = extract_deps_from_expr(&each_node.iterable);

                let body = self.compile_body(&each_node.children, component_id, source_path, events)?;

                let mut seg = DynamicSegment::new(
                    dyn_id,
                    each_node.iterable.clone(),
                    deps,
                    SegmentType::Loop,
                );
                seg.loop_body = Some(LoopBody {
                    item_var: each_node.item.clone(),
                    index_var: each_node.index.clone(),
                    body,
                });
                dynamic_parts.push(seg);
            }

            TemplateNode::Match(match_node) => {
                if static_parts.len() <= dynamic_parts.len() {
                    static_parts.push(String::new());
                }
                let dyn_id = self.next_dyn_id();
                let deps = extract_deps_from_expr(&match_node.expr);

                let mut else_if_bodies = Vec::new();
                for (pattern, children) in &match_node.arms {
                    let body = self.compile_body(children, component_id, source_path, events)?;
                    else_if_bodies.push((pattern.clone(), body));
                }

                let mut seg = DynamicSegment::new(
                    dyn_id,
                    match_node.expr.clone(),
                    deps,
                    SegmentType::Conditional,
                );
                seg.else_if_bodies = else_if_bodies;
                dynamic_parts.push(seg);
            }

            TemplateNode::Can(can_node) => {
                if static_parts.len() <= dynamic_parts.len() {
                    static_parts.push(String::new());
                }
                let dyn_id = self.next_dyn_id();

                let body = self.compile_body(&can_node.children, component_id, source_path, events)?;

                let mut seg = DynamicSegment::new(
                    dyn_id,
                    can_node.permission.clone(),
                    vec![format!("permission:{}", can_node.permission)],
                    SegmentType::Permission,
                );
                seg.permission_body = Some(body);
                dynamic_parts.push(seg);
            }

            TemplateNode::Slot(_slot) => {
                static_parts.push("<!-- slot -->".to_string());
            }

            TemplateNode::Component(comp) => {
                let element_id = format!("comp_{}", comp.name.to_lowercase());
                let mut tag = format!("<{}", comp.name);

                // Props as attributes
                for prop in &comp.props {
                    match &prop.value {
                        AttributeValue::Static(val) => {
                            tag.push_str(&format!(" {}=\"{}\"", prop.name, val));
                        }
                        AttributeValue::Dynamic(expr) => {
                            let dyn_id = self.next_dyn_id();
                            let deps = extract_deps_from_expr(expr);
                            dynamic_parts.push(DynamicSegment::new(
                                dyn_id,
                                format!("state.{}", expr),
                                deps,
                                SegmentType::Attribute {
                                    element_id: element_id.clone(),
                                    attr_name: prop.name.clone(),
                                },
                            ));
                        }
                        AttributeValue::None => {
                            tag.push_str(&format!(" {}", prop.name));
                        }
                    }
                }

                // Event bindings on component
                for event in &comp.events {
                    let event_id = self.next_event_id();
                    let modifiers: Vec<String> = event
                        .modifiers
                        .iter()
                        .map(|m| match m {
                            EventModifier::Prevent => "prevent".to_string(),
                            EventModifier::Stop => "stop".to_string(),
                            EventModifier::Debounce(ms) => format!("debounce:{}", ms),
                            EventModifier::Throttle(ms) => format!("throttle:{}", ms),
                        })
                        .collect();

                    events.push(EventIR {
                        id: event_id,
                        event_type: event.event.clone(),
                        handler: event.handler.clone(),
                        component_id: component_id.to_string(),
                        modifiers,
                        element_id: element_id.clone(),
                    });
                }

                if comp.is_island {
                    tag.push_str(" data-ar-island");
                }

                tag.push_str(" />");
                static_parts.push(tag);

                // Compile children of the component
                for child in &comp.children {
                    self.compile_node(
                        child,
                        static_parts,
                        dynamic_parts,
                        events,
                        component_id,
                        source_path,
                    )?;
                }
            }

            TemplateNode::ErrorBoundary(eb) => {
                static_parts.push("<!-- error-boundary -->".to_string());
                for child in &eb.children {
                    self.compile_node(
                        child,
                        static_parts,
                        dynamic_parts,
                        events,
                        component_id,
                        source_path,
                    )?;
                }
                if let Some(ref error_template) = eb.error_template {
                    for child in error_template {
                        self.compile_node(
                            child,
                            static_parts,
                            dynamic_parts,
                            events,
                            component_id,
                            source_path,
                        )?;
                    }
                }
                static_parts.push("<!-- /error-boundary -->".to_string());
            }
        }

        Ok(())
    }

    /// Compile a list of template nodes into an isolated SegmentBody.
    fn compile_body(
        &mut self,
        nodes: &[TemplateNode],
        component_id: &str,
        source_path: &str,
        events: &mut Vec<EventIR>,
    ) -> Result<SegmentBody, CompileError> {
        let mut statics = Vec::new();
        let mut dynamics = Vec::new();

        for node in nodes {
            self.compile_node(
                node,
                &mut statics,
                &mut dynamics,
                events,
                component_id,
                source_path,
            )?;
        }

        Ok(SegmentBody {
            static_segments: statics,
            dynamic_segments: dynamics,
        })
    }

    /// Compile script block to actions, state fields, and form schemas.
    fn compile_script(
        &mut self,
        script: &ScriptBlock,
    ) -> Result<(Vec<ActionIR>, Vec<StateFieldIR>, Vec<FormSchemaIR>), CompileError> {
        // Check for duplicate state names
        let mut seen_states: HashSet<String> = HashSet::new();
        for state in &script.states {
            if !seen_states.insert(state.name.clone()) {
                return Err(CompileError::DuplicateState {
                    name: state.name.clone(),
                });
            }
        }

        // Compile state declarations
        let state_fields: Vec<StateFieldIR> = script
            .states
            .iter()
            .map(|s| StateFieldIR {
                name: s.name.clone(),
                ty: s.ty.clone(),
                default: s.default.clone(),
                secret: s.secret,
            })
            .collect();

        // Compile actions
        let actions: Vec<ActionIR> = script
            .actions
            .iter()
            .map(|a| ActionIR {
                name: a.name.clone(),
                is_async: a.is_async,
                params: a
                    .params
                    .iter()
                    .map(|p| ParamIR {
                        name: p.name.clone(),
                        ty: p.ty.clone(),
                    })
                    .collect(),
                permission: a.permission.clone(),
                audit: a.audit.clone(),
                body: a.body.clone(),
            })
            .collect();

        // Compile form schemas
        let form_schemas: Vec<FormSchemaIR> = script
            .forms
            .iter()
            .map(|f| {
                let fields = f
                    .fields
                    .iter()
                    .map(|field| {
                        let required = field.constraints.contains(&FieldConstraint::Required);
                        let min = field.constraints.iter().find_map(|c| {
                            if let FieldConstraint::Min(n) = c {
                                Some(*n)
                            } else {
                                None
                            }
                        });
                        let max = field.constraints.iter().find_map(|c| {
                            if let FieldConstraint::Max(n) = c {
                                Some(*n)
                            } else {
                                None
                            }
                        });
                        FormFieldIR {
                            name: field.name.clone(),
                            ty: field.ty.clone(),
                            required,
                            min,
                            max,
                        }
                    })
                    .collect();
                FormSchemaIR {
                    name: f.name.clone(),
                    fields,
                }
            })
            .collect();

        Ok((actions, state_fields, form_schemas))
    }

    /// Check security rules.
    ///
    /// - Secret state fields must not appear in template expressions.
    /// - Tenant-required routes should not use unscoped queries (heuristic).
    fn check_security(
        &self,
        ir: &ComponentIR,
        source_path: &str,
    ) -> Result<(), CompileError> {
        let secret_fields: HashSet<&str> = ir
            .state_fields
            .iter()
            .filter(|f| f.secret)
            .map(|f| f.name.as_str())
            .collect();

        if !secret_fields.is_empty() {
            for seg in &ir.dynamic_segments {
                for dep in &seg.deps {
                    // Check the root field name (before any dot)
                    let root = dep.split('.').next().unwrap_or(dep);
                    if secret_fields.contains(root) {
                        return Err(CompileError::SecretStateInTemplate {
                            field: dep.clone(),
                            file: source_path.to_string(),
                            line: 0,
                            col: 0,
                        });
                    }
                }
            }
        }

        Ok(())
    }

    /// Build a dependency graph from the compiled IR.
    fn build_dependency_graph(&self, ir: &ComponentIR) -> DependencyGraph {
        let mut graph = DependencyGraph::new();

        for seg in &ir.dynamic_segments {
            for dep in &seg.deps {
                graph.add_dependency(&seg.id, dep);
            }
        }

        graph
    }

    /// Compile a route block to a RouteIR (attached to the component).
    fn compile_route_ir(&self, route: &RouteBlock) -> RouteIR {
        RouteIR {
            path: route.path.clone().unwrap_or_else(|| "/".to_string()),
            method: route.method.clone().unwrap_or_else(|| "GET".to_string()),
            layout: route.layout.clone(),
            auth: route
                .auth
                .as_ref()
                .map(|a| format!("{:?}", a).to_lowercase())
                .unwrap_or_else(|| "public".to_string()),
            tenant: route
                .tenant
                .as_ref()
                .map(|t| format!("{:?}", t).to_lowercase())
                .unwrap_or_else(|| "none".to_string()),
            permission: route.permission.clone(),
            cache: route
                .cache
                .as_ref()
                .map(|c| match c {
                    CachePolicy::NoStore => "no-store".to_string(),
                    CachePolicy::Private => "private".to_string(),
                    CachePolicy::Public => "public".to_string(),
                    CachePolicy::Static => "static".to_string(),
                })
                .unwrap_or_else(|| "no-store".to_string()),
        }
    }

    /// Compile a route block to a RouteEntry (for the manifest).
    fn compile_route(&self, route: &RouteBlock, source_path: &str) -> RouteEntry {
        let path = route.path.clone().unwrap_or_else(|| "/".to_string());
        let id = format!(
            "route_{}",
            path.trim_start_matches('/')
                .replace('/', "_")
                .replace(':', "_")
        );

        RouteEntry {
            id,
            path,
            file: source_path.to_string(),
            method: route.method.clone().unwrap_or_else(|| "GET".to_string()),
            auth: route
                .auth
                .as_ref()
                .map(|a| format!("{:?}", a).to_lowercase())
                .unwrap_or_else(|| "public".to_string()),
            tenant: route
                .tenant
                .as_ref()
                .map(|t| format!("{:?}", t).to_lowercase())
                .unwrap_or_else(|| "none".to_string()),
            permission: route.permission.clone(),
            layout: route.layout.clone(),
            cache: route
                .cache
                .as_ref()
                .map(|c| match c {
                    CachePolicy::NoStore => "no-store".to_string(),
                    CachePolicy::Private => "private".to_string(),
                    CachePolicy::Public => "public".to_string(),
                    CachePolicy::Static => "static".to_string(),
                })
                .unwrap_or_else(|| "no-store".to_string()),
        }
    }

    /// Generate the next dynamic segment ID.
    fn next_dyn_id(&mut self) -> String {
        let id = format!("dyn_{}", self.dyn_counter);
        self.dyn_counter += 1;
        id
    }

    /// Generate the next event ID.
    fn next_event_id(&mut self) -> String {
        let id = format!("evt_{}", self.event_counter);
        self.event_counter += 1;
        id
    }
}

impl Default for Compiler {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Derive a PascalCase component name from a file path.
///
/// `"pages/customer_list.adapto"` -> `"CustomerList"`
fn derive_component_name(path: &str) -> String {
    let stem = std::path::Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Unknown");

    stem.split(|c: char| c == '_' || c == '-')
        .map(|segment| {
            let mut chars = segment.chars();
            match chars.next() {
                Some(first) => {
                    let upper: String = first.to_uppercase().collect();
                    format!("{}{}", upper, chars.collect::<String>())
                }
                None => String::new(),
            }
        })
        .collect()
}

/// Extract dependency field names from an expression string.
///
/// Uses a simple heuristic: any identifier-like token (letters, digits,
/// underscores, dots) that does not start with a digit is treated as a
/// potential state field reference. The root name (before the first dot)
/// is returned as the dependency.
fn extract_deps_from_expr(expr: &str) -> Vec<String> {
    let mut deps = Vec::new();

    // Tokenize: split on non-identifier characters
    let mut current = String::new();
    for ch in expr.chars() {
        if ch.is_alphanumeric() || ch == '_' || ch == '.' {
            current.push(ch);
        } else {
            if !current.is_empty() {
                maybe_add_dep(&current, &mut deps);
                current.clear();
            }
        }
    }
    if !current.is_empty() {
        maybe_add_dep(&current, &mut deps);
    }

    deps
}

/// Add a dependency if the token looks like a state field reference.
fn maybe_add_dep(token: &str, deps: &mut Vec<String>) {
    // Skip numeric literals, keywords, and common non-state tokens
    let first = token.chars().next().unwrap_or('0');
    if first.is_ascii_digit() {
        return;
    }

    let keywords = [
        "true", "false", "null", "none", "if", "else", "for", "in", "as", "let", "mut",
        "fn", "return", "match", "self", "Self", "struct", "enum", "impl", "pub", "use",
        "mod", "crate", "super", "const", "static", "type", "where", "while", "loop",
        "break", "continue", "ref", "move", "async", "await", "dyn", "trait",
    ];
    if keywords.contains(&token) {
        return;
    }

    // Use the root field (before first dot) as the dependency
    let root = token.split('.').next().unwrap_or(token);
    let dep = root.to_string();
    if !deps.contains(&dep) {
        deps.push(dep);
    }
}

/// Collect child component names from template nodes.
fn collect_child_components(nodes: &[TemplateNode]) -> Vec<String> {
    let mut children = Vec::new();

    for node in nodes {
        match node {
            TemplateNode::Component(comp) => {
                if !children.contains(&comp.name) {
                    children.push(comp.name.clone());
                }
                let nested = collect_child_components(&comp.children);
                for name in nested {
                    if !children.contains(&name) {
                        children.push(name);
                    }
                }
            }
            TemplateNode::Element(elem) => {
                let nested = collect_child_components(&elem.children);
                for name in nested {
                    if !children.contains(&name) {
                        children.push(name);
                    }
                }
            }
            TemplateNode::If(if_node) => {
                let mut all_children = Vec::new();
                all_children.extend(collect_child_components(&if_node.then_branch));
                for (_, branch) in &if_node.else_if_branches {
                    all_children.extend(collect_child_components(branch));
                }
                if let Some(ref else_branch) = if_node.else_branch {
                    all_children.extend(collect_child_components(else_branch));
                }
                for name in all_children {
                    if !children.contains(&name) {
                        children.push(name);
                    }
                }
            }
            TemplateNode::Each(each_node) => {
                let nested = collect_child_components(&each_node.children);
                for name in nested {
                    if !children.contains(&name) {
                        children.push(name);
                    }
                }
            }
            TemplateNode::Can(can_node) => {
                let nested = collect_child_components(&can_node.children);
                for name in nested {
                    if !children.contains(&name) {
                        children.push(name);
                    }
                }
            }
            TemplateNode::ErrorBoundary(eb) => {
                let nested = collect_child_components(&eb.children);
                for name in nested {
                    if !children.contains(&name) {
                        children.push(name);
                    }
                }
            }
            TemplateNode::Match(match_node) => {
                for (_, arm_children) in &match_node.arms {
                    let nested = collect_child_components(arm_children);
                    for name in nested {
                        if !children.contains(&name) {
                            children.push(name);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    children
}
