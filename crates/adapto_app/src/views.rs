//! Auto-generated views from ResourceMeta.
//!
//! Provides default list, detail, and form views that apps can use
//! out of the box or override with custom renderers. Full auto-CRUD
//! generation will be enabled once the `#[derive(Resource)]` macro
//! is in place.

use adapto_ui::html_escape;

/// Render a placeholder view for a resource collection.
/// Used when no custom view is registered — shows a simple message
/// indicating the resource is available but has no custom UI yet.
pub fn render_resource_placeholder(label_plural: &str, route_prefix: &str) -> String {
    let label = html_escape(label_plural);
    let route = html_escape(route_prefix);

    format!(
        r#"<div class="au-card au-card--flat" style="text-align:center;padding:var(--au-space-10)">
  <h2 style="font-size:var(--au-text-2xl);font-weight:var(--au-weight-bold);margin:0 0 var(--au-space-2)">{label}</h2>
  <p style="font-size:var(--au-text-sm);color:var(--au-color-text-secondary);margin:0">
    This resource is registered at <code>{route}</code>.
    Add a custom view or wait for the <code>#[derive(Resource)]</code> macro to auto-generate CRUD views.
  </p>
</div>"#,
    )
}

/// Render a simple empty-state card when a collection has no documents.
pub fn render_empty_state(label_plural: &str) -> String {
    let label = html_escape(label_plural);

    format!(
        r#"<div class="au-card au-card--flat" style="text-align:center;padding:var(--au-space-10)">
  <p style="font-size:var(--au-text-lg);color:var(--au-color-text-secondary);margin:0">
    No {label} yet.
  </p>
</div>"#,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholder_contains_label_and_route() {
        let html = render_resource_placeholder("Customers", "/customers");
        assert!(html.contains("Customers"));
        assert!(html.contains("/customers"));
        assert!(html.contains("au-card"));
    }

    #[test]
    fn empty_state_shows_label() {
        let html = render_empty_state("Orders");
        assert!(html.contains("No Orders yet."));
    }

    #[test]
    fn html_escaping_in_views() {
        let html = render_resource_placeholder("<script>alert(1)</script>", "/x&y");
        assert!(!html.contains("<script>"));
        assert!(html.contains("&lt;script&gt;"));
        assert!(html.contains("/x&amp;y"));
    }
}
