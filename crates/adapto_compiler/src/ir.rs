use serde::{Deserialize, Serialize};

/// A compiled component — the central artifact produced by the compiler.
///
/// Contains the fully analyzed template (split into static and dynamic segments),
/// extracted events, actions, state fields, form schemas, permission requirements,
/// and optional route and style information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentIR {
    pub id: String,
    pub name: String,
    pub route: Option<RouteIR>,
    pub static_segments: Vec<String>,
    pub dynamic_segments: Vec<DynamicSegment>,
    pub events: Vec<EventIR>,
    pub actions: Vec<ActionIR>,
    pub state_fields: Vec<StateFieldIR>,
    pub form_schemas: Vec<FormSchemaIR>,
    pub permissions: Vec<String>,
    pub children: Vec<String>,
    pub is_island: bool,
    pub style: Option<CompiledStyle>,
}

/// A dynamic segment within a compiled template.
///
/// Each dynamic segment has a unique ID, the expression that produces its value,
/// a list of state fields it depends on (for fine-grained re-rendering), and
/// a type describing where it appears in the template.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicSegment {
    /// Unique identifier, e.g. "dyn_0", "dyn_1"
    pub id: String,
    /// The expression to evaluate, e.g. "customer.name"
    pub expr: String,
    /// State fields this segment depends on, e.g. ["customer.name"]
    pub deps: Vec<String>,
    /// Where this segment appears in the template
    pub segment_type: SegmentType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SegmentType {
    Text,
    Html,
    Attribute { element_id: String, attr_name: String },
    Conditional,
    Loop,
    Permission,
}

/// A compiled event binding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventIR {
    pub id: String,
    pub event_type: String,
    pub handler: String,
    pub component_id: String,
    pub modifiers: Vec<String>,
    pub element_id: String,
}

/// A compiled action extracted from the script block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionIR {
    pub name: String,
    pub is_async: bool,
    pub params: Vec<ParamIR>,
    pub permission: Option<String>,
    pub audit: Option<String>,
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamIR {
    pub name: String,
    pub ty: String,
}

/// A compiled state field declaration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateFieldIR {
    pub name: String,
    pub ty: String,
    pub default: Option<String>,
    pub secret: bool,
}

/// A compiled form schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormSchemaIR {
    pub name: String,
    pub fields: Vec<FormFieldIR>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormFieldIR {
    pub name: String,
    pub ty: String,
    pub required: bool,
    pub min: Option<usize>,
    pub max: Option<usize>,
}

/// A compiled route extracted from the route block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteIR {
    pub path: String,
    pub method: String,
    pub layout: Option<String>,
    pub auth: String,
    pub tenant: String,
    pub permission: Option<String>,
    pub cache: String,
}

/// Compiled style block — CSS with optional scoping.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledStyle {
    pub css: String,
    pub scoped: bool,
    pub scope_id: Option<String>,
}
