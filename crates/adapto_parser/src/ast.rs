use serde::{Deserialize, Serialize};

/// Root AST node representing a parsed `.adapto` file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptoFile {
    pub route: Option<RouteBlock>,
    pub script: Option<ScriptBlock>,
    pub template: Option<TemplateBlock>,
    pub style: Option<StyleBlock>,
    pub resource: Option<ResourceBlock>,
    pub layout: Option<LayoutBlock>,
}

// ---------------------------------------------------------------------------
// Route
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteBlock {
    pub path: Option<String>,
    pub method: Option<String>,
    pub layout: Option<String>,
    pub page_title: Option<String>,
    pub auth: Option<AuthLevel>,
    pub role: Option<String>,
    pub permission: Option<String>,
    pub tenant: Option<TenantLevel>,
    pub cache: Option<CachePolicy>,
    pub error: Option<String>,
    pub not_found: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthLevel {
    Public,
    Optional,
    Required,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TenantLevel {
    None,
    Optional,
    Required,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CachePolicy {
    NoStore,
    Private,
    Public,
    Static,
}

// ---------------------------------------------------------------------------
// Script
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptBlock {
    pub uses: Vec<UseStatement>,
    pub props: Vec<PropDecl>,
    pub states: Vec<StateDecl>,
    pub memos: Vec<MemoDecl>,
    pub loaders: Vec<LoaderDecl>,
    pub actions: Vec<ActionDecl>,
    pub server_fns: Vec<ServerFnDecl>,
    pub forms: Vec<FormDecl>,
    pub ai_actions: Vec<AiActionDecl>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UseStatement {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropDecl {
    pub name: String,
    pub ty: String,
    pub default: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateDecl {
    pub name: String,
    pub ty: String,
    pub default: Option<String>,
    pub secret: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoDecl {
    pub name: String,
    pub ty: String,
    pub expr: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoaderDecl {
    pub name: String,
    pub is_async: bool,
    pub params: Vec<ParamDecl>,
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionDecl {
    pub name: String,
    pub is_async: bool,
    pub params: Vec<ParamDecl>,
    pub permission: Option<String>,
    pub audit: Option<String>,
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamDecl {
    pub name: String,
    pub ty: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerFnDecl {
    pub name: String,
    pub is_async: bool,
    pub params: Vec<ParamDecl>,
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormDecl {
    pub name: String,
    pub fields: Vec<FormFieldDecl>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormFieldDecl {
    pub name: String,
    pub ty: String,
    pub constraints: Vec<FieldConstraint>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FieldConstraint {
    Required,
    Min(usize),
    Max(usize),
    Unique,
    Optional,
    Searchable,
    Readonly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiActionDecl {
    pub name: String,
    pub input_param: String,
    pub input_type: String,
    pub return_type: String,
    pub model: String,
    pub fallback: Option<String>,
    pub temperature: Option<f64>,
    pub audit: bool,
    pub pii: Option<String>,
    pub permission: Option<String>,
}

// ---------------------------------------------------------------------------
// Template
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateBlock {
    pub children: Vec<TemplateNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TemplateNode {
    Element(ElementNode),
    Text(String),
    Expression(ExprNode),
    UnsafeHtml(String),
    If(IfNode),
    Each(EachNode),
    Match(MatchNode),
    Can(CanNode),
    Slot(SlotNode),
    Component(ComponentNode),
    ErrorBoundary(ErrorBoundaryNode),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementNode {
    pub tag: String,
    pub attributes: Vec<Attribute>,
    pub events: Vec<EventBinding>,
    pub bindings: Vec<BindingDecl>,
    pub children: Vec<TemplateNode>,
    pub self_closing: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attribute {
    pub name: String,
    pub value: AttributeValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AttributeValue {
    Static(String),
    Dynamic(String),
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventBinding {
    pub event: String,
    pub handler: String,
    pub modifiers: Vec<EventModifier>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventModifier {
    Prevent,
    Stop,
    Debounce(u32),
    Throttle(u32),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BindingDecl {
    pub kind: String,
    pub target: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExprNode {
    pub expr: String,
    pub id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IfNode {
    pub condition: String,
    pub then_branch: Vec<TemplateNode>,
    pub else_if_branches: Vec<(String, Vec<TemplateNode>)>,
    pub else_branch: Option<Vec<TemplateNode>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EachNode {
    pub iterable: String,
    pub item: String,
    pub index: Option<String>,
    pub children: Vec<TemplateNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchNode {
    pub expr: String,
    pub arms: Vec<(String, Vec<TemplateNode>)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanNode {
    pub permission: String,
    pub children: Vec<TemplateNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotNode {
    pub name: Option<String>,
    pub fallback: Vec<TemplateNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentNode {
    pub name: String,
    pub props: Vec<Attribute>,
    pub events: Vec<EventBinding>,
    pub bindings: Vec<BindingDecl>,
    pub children: Vec<TemplateNode>,
    pub is_island: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorBoundaryNode {
    pub error_template: Option<Vec<TemplateNode>>,
    pub children: Vec<TemplateNode>,
}

// ---------------------------------------------------------------------------
// Style
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleBlock {
    pub scoped: bool,
    pub content: String,
}

// ---------------------------------------------------------------------------
// Resource
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceBlock {
    pub name: String,
    pub table: String,
    pub tenant: TenantLevel,
    pub primary_key: String,
    pub fields: Vec<ResourceField>,
    pub permissions: Vec<ResourcePermission>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceField {
    pub name: String,
    pub ty: String,
    pub constraints: Vec<FieldConstraint>,
    pub searchable: bool,
    pub readonly: bool,
    pub default: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcePermission {
    pub action: String,
    pub permission: String,
}

// ---------------------------------------------------------------------------
// Layout
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutBlock {
    pub name: String,
    pub auth: Option<AuthLevel>,
    pub tenant: Option<TenantLevel>,
}
