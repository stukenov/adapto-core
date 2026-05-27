use thiserror::Error;

#[derive(Debug, Error)]
pub enum SsrError {
    #[error("Route not found: {0}")]
    RouteNotFound(String),

    #[error("Layout not found: {0}")]
    LayoutNotFound(String),

    #[error("Component not found: {0}")]
    ComponentNotFound(String),

    #[error("Render error: {0}")]
    RenderError(String),

    #[error("Auth required")]
    AuthRequired,

    #[error("Tenant required")]
    TenantRequired,

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("State evaluation error: {0}")]
    StateError(String),

    #[error("Project error: {0}")]
    ProjectError(String),
}
