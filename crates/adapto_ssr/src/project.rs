use std::collections::HashMap;
use std::path::Path;

use adapto_compiler::compiler::Compiler;
use adapto_compiler::dependency::DependencyGraph;
use adapto_compiler::ir::ComponentIR;
use adapto_compiler::manifest::RouteManifest;
use adapto_runtime::resource::ResourceManager;

use crate::error::SsrError;
use crate::layout::LayoutManager;
use crate::page::PageRenderer;
use crate::router::Router;

pub struct CompiledProject {
    pub page_renderer: PageRenderer,
    pub layout_manager: LayoutManager,
    pub route_manifest: RouteManifest,
    pub resource_managers: HashMap<String, ResourceManager>,
    pub dependency_graphs: HashMap<String, DependencyGraph>,
    pub component_irs: HashMap<String, ComponentIR>,
    pub file_count: usize,
}

pub struct ProjectLoader;

impl ProjectLoader {
    pub fn load_project(path: &str, secret: &[u8]) -> Result<CompiledProject, SsrError> {
        let files = find_adapto_files(Path::new(path));
        if files.is_empty() {
            return Err(SsrError::ProjectError("no .adapto files found".into()));
        }

        let mut compiler = Compiler::new();
        let mut page_renderer = PageRenderer::new(secret);
        let mut layout_manager = LayoutManager::new();
        let mut resource_managers = HashMap::new();
        let mut dependency_graphs = HashMap::new();
        let mut component_irs = HashMap::new();
        let file_count = files.len();

        for file_path in &files {
            let source = std::fs::read_to_string(file_path)
                .map_err(|e| SsrError::ProjectError(format!("read {}: {}", file_path, e)))?;

            let ast = adapto_parser::parse(&source)
                .map_err(|e| SsrError::ProjectError(format!("parse {}: {}", file_path, e)))?;

            // Register layout if present
            if let Some(ref layout) = ast.layout {
                let template_html = ast.template
                    .as_ref()
                    .map(|t| template_to_raw_html(t))
                    .unwrap_or_default();
                layout_manager.register(&layout.name, template_html);
            }

            let output = compiler.compile_file(&ast, file_path)
                .map_err(|e| SsrError::ProjectError(format!("compile {}: {}", file_path, e)))?;

            let component_id = output.component_ir.id.clone();

            // Register component for page rendering
            if let Some(ref route_entry) = output.route_entry {
                page_renderer.register_component(&route_entry.id, output.component_ir.clone());
            }

            // Register resource manager if present
            if let Some(resource_ir) = output.resource_ir {
                let runtime_ir = compiler_to_runtime_resource(&resource_ir);
                let mgr = ResourceManager::new(runtime_ir);
                resource_managers.insert(resource_ir.name.clone(), mgr);
            }

            dependency_graphs.insert(component_id.clone(), output.dependency_graph);
            component_irs.insert(component_id, output.component_ir);
        }

        // Set up router from manifest
        let route_manifest = compiler.route_manifest().clone();
        let router = Router::new(route_manifest.clone());
        page_renderer.set_router(router);

        Ok(CompiledProject {
            page_renderer,
            layout_manager,
            route_manifest,
            resource_managers,
            dependency_graphs,
            component_irs,
            file_count,
        })
    }
}

fn find_adapto_files(dir: &Path) -> Vec<String> {
    let mut files = Vec::new();
    walk_dir(dir, &mut files);
    files.sort();
    files
}

fn walk_dir(dir: &Path, files: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name.starts_with('.') || name == "target" || name == "node_modules" {
                continue;
            }
            walk_dir(&path, files);
        } else if path.extension().and_then(|e| e.to_str()) == Some("adapto") {
            files.push(path.to_string_lossy().to_string());
        }
    }
}

fn template_to_raw_html(template: &adapto_parser::ast::TemplateBlock) -> String {
    let mut html = String::new();
    for node in &template.children {
        node_to_html(node, &mut html);
    }
    html
}

fn node_to_html(node: &adapto_parser::ast::TemplateNode, out: &mut String) {
    use adapto_parser::ast::{TemplateNode, AttributeValue};
    match node {
        TemplateNode::Text(t) => out.push_str(t),
        TemplateNode::Expression(e) => {
            out.push('{');
            out.push_str(&e.expr);
            out.push('}');
        }
        TemplateNode::Element(el) => {
            out.push('<');
            out.push_str(&el.tag);
            for attr in &el.attributes {
                out.push(' ');
                out.push_str(&attr.name);
                match &attr.value {
                    AttributeValue::Static(val) => {
                        out.push_str("=\"");
                        out.push_str(val);
                        out.push('"');
                    }
                    AttributeValue::Dynamic(val) => {
                        out.push_str("=\"{");
                        out.push_str(val);
                        out.push_str("}\"");
                    }
                    AttributeValue::None => {}
                }
            }
            if el.self_closing {
                out.push_str(" />");
            } else {
                out.push('>');
                for child in &el.children {
                    node_to_html(child, out);
                }
                out.push_str("</");
                out.push_str(&el.tag);
                out.push('>');
            }
        }
        TemplateNode::If(if_node) => {
            for child in &if_node.then_branch { node_to_html(child, out); }
        }
        TemplateNode::Each(each_node) => {
            for child in &each_node.children { node_to_html(child, out); }
        }
        TemplateNode::Match(_) => {}
        TemplateNode::Can(can_node) => {
            for child in &can_node.children { node_to_html(child, out); }
        }
        TemplateNode::UnsafeHtml(h) => out.push_str(h),
        TemplateNode::Slot(_) => { out.push_str("{slot}"); }
        TemplateNode::Component(comp) => {
            out.push('<');
            out.push_str(&comp.name);
            out.push('>');
            for child in &comp.children { node_to_html(child, out); }
            out.push_str("</");
            out.push_str(&comp.name);
            out.push('>');
        }
        TemplateNode::ErrorBoundary(eb) => {
            for child in &eb.children { node_to_html(child, out); }
        }
    }
}

fn compiler_to_runtime_resource(ir: &adapto_compiler::ir::ResourceIR) -> adapto_runtime::resource::ResourceIR {
    adapto_runtime::resource::ResourceIR {
        name: ir.name.clone(),
        collection_name: ir.collection_name.clone(),
        tenant_scoped: ir.tenant_scoped,
        primary_key: ir.primary_key.clone(),
        fields: ir.fields.iter().map(|f| adapto_runtime::resource::ResourceFieldIR {
            name: f.name.clone(),
            ty: f.ty.clone(),
            required: f.required,
            unique: f.unique,
            searchable: f.searchable,
            readonly: f.readonly,
            default: f.default.clone(),
            min: f.min,
            max: f.max,
        }).collect(),
        indexes: ir.indexes.iter().map(|i| adapto_runtime::resource::ResourceIndexIR {
            field: i.field.clone(),
            unique: i.unique,
        }).collect(),
        permissions: ir.permissions.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn create_test_project(dir: &Path) {
        let pages_dir = dir.join("pages");
        fs::create_dir_all(&pages_dir).unwrap();

        fs::write(pages_dir.join("home.adapto"), r#"
<route>
  path: "/"
  method: GET
  auth: public
</route>
<script lang="rust">
  state title: String = "Home"
</script>
<template>
  <h1>{title}</h1>
</template>
"#).unwrap();

        fs::write(pages_dir.join("about.adapto"), r#"
<route>
  path: "/about"
  method: GET
  auth: public
</route>
<script lang="rust">
  state page: String = "About"
</script>
<template>
  <h1>{page}</h1>
  <p>About this site</p>
</template>
"#).unwrap();

        fs::write(pages_dir.join("customers.adapto"), r#"
<route>
  path: "/customers"
  method: GET
  auth: required
</route>
<script lang="rust">
  state customers: Vec<Customer> = []
</script>
<template>
  {#each customers as customer}
    <div>{customer.name}</div>
  {/each}
</template>
<resource name="Customer" table="customers">
  tenant: none
  primary_key: id
  field id: Uuid readonly
  field name: String required max=100 searchable
  field email: Email required unique
  permission read: "customers.read"
</resource>
"#).unwrap();
    }

    #[test]
    fn load_project_discovers_files() {
        let tmp = tempdir();
        create_test_project(&tmp);

        let project = ProjectLoader::load_project(
            tmp.to_str().unwrap(),
            b"test-secret",
        ).unwrap();

        assert_eq!(project.file_count, 3);
    }

    #[test]
    fn load_project_registers_routes() {
        let tmp = tempdir();
        create_test_project(&tmp);

        let project = ProjectLoader::load_project(
            tmp.to_str().unwrap(),
            b"test-secret",
        ).unwrap();

        assert!(!project.route_manifest.routes.is_empty());
        assert!(project.route_manifest.find_by_path("/").is_some());
        assert!(project.route_manifest.find_by_path("/about").is_some());
        assert!(project.route_manifest.find_by_path("/customers").is_some());
    }

    #[test]
    fn load_project_registers_resources() {
        let tmp = tempdir();
        create_test_project(&tmp);

        let project = ProjectLoader::load_project(
            tmp.to_str().unwrap(),
            b"test-secret",
        ).unwrap();

        assert!(project.resource_managers.contains_key("Customer"));
    }

    #[test]
    fn load_project_registers_components() {
        let tmp = tempdir();
        create_test_project(&tmp);

        let project = ProjectLoader::load_project(
            tmp.to_str().unwrap(),
            b"test-secret",
        ).unwrap();

        assert_eq!(project.component_irs.len(), 3);
        assert_eq!(project.dependency_graphs.len(), 3);
    }

    #[test]
    fn load_project_empty_dir() {
        let tmp = tempdir();
        let result = ProjectLoader::load_project(
            tmp.to_str().unwrap(),
            b"test-secret",
        );
        assert!(result.is_err());
    }

    fn tempdir() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("adapto_test_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }
}
