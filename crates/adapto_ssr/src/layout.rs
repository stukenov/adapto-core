use std::collections::HashMap;

use crate::error::SsrError;

/// Manages layout templates and composes page content into them.
///
/// A layout is an HTML template with a `{slot}` placeholder where page
/// content is injected. This follows the same mental model as a UIKit
/// container view controller — the layout provides the chrome, the
/// page provides the content.
pub struct LayoutManager {
    /// Registered layouts keyed by name. Each value is an HTML template
    /// string containing exactly one `{slot}` marker.
    layouts: HashMap<String, String>,
}

impl LayoutManager {
    pub fn new() -> Self {
        Self {
            layouts: HashMap::new(),
        }
    }

    /// Register a layout template. The `html` string must contain
    /// `{slot}` where page content will be inserted.
    pub fn register(&mut self, name: &str, html: String) {
        self.layouts.insert(name.to_string(), html);
    }

    /// Compose page content into a named layout by replacing `{slot}`
    /// with the rendered page HTML.
    pub fn compose(&self, layout_name: &str, page_content: &str) -> Result<String, SsrError> {
        let template = self
            .layouts
            .get(layout_name)
            .ok_or_else(|| SsrError::LayoutNotFound(layout_name.to_string()))?;

        Ok(template.replace("{slot}", page_content))
    }

    /// Check whether a layout with the given name has been registered.
    pub fn has_layout(&self, name: &str) -> bool {
        self.layouts.contains_key(name)
    }
}

impl Default for LayoutManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_and_compose() {
        let mut mgr = LayoutManager::new();
        mgr.register(
            "main",
            "<html><body><nav>Menu</nav><main>{slot}</main></body></html>".to_string(),
        );

        let result = mgr.compose("main", "<h1>Hello</h1>").unwrap();
        assert_eq!(
            result,
            "<html><body><nav>Menu</nav><main><h1>Hello</h1></main></body></html>"
        );
    }

    #[test]
    fn unknown_layout_error() {
        let mgr = LayoutManager::new();
        let result = mgr.compose("missing", "<p>content</p>");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, SsrError::LayoutNotFound(_)));
    }

    #[test]
    fn has_layout_check() {
        let mut mgr = LayoutManager::new();
        assert!(!mgr.has_layout("dashboard"));
        mgr.register("dashboard", "<div>{slot}</div>".into());
        assert!(mgr.has_layout("dashboard"));
    }
}
