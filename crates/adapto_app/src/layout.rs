//! Page layout system for Adapto apps.
//!
//! Provides the standard page chrome — HTML head with Adapto UI CSS,
//! navigation bar, breadcrumb trail, content area, stats area, and
//! the embedded live.js client runtime.

use adapto_ui::html_escape;

/// The built-in client-side JavaScript that powers live WebSocket updates.
/// Embedded at compile time from `live.js`.
pub const LIVE_JS: &str = include_str!("live.js");

/// Configuration for page layout rendering.
pub struct LayoutConfig<'a> {
    /// Application title displayed in the nav bar.
    pub title: &'a str,
    /// Navigation items: `(label, href, is_active)`.
    pub nav_items: &'a [(&'a str, &'a str, bool)],
    /// Breadcrumb segments: `(label, optional_href)`.
    pub breadcrumbs: &'a [(&'a str, Option<&'a str>)],
    /// HTML content for the stats area (above main content).
    pub stats_html: &'a str,
    /// Main page content HTML.
    pub content_html: &'a str,
    /// Optional extra CSS to inject into the `<head>`.
    pub extra_css: &'a str,
}

/// Render a navigation item.
fn render_nav_item(label: &str, href: &str, active: bool) -> String {
    let cls = if active {
        "au-nav__item au-nav__item--active"
    } else {
        "au-nav__item"
    };
    format!(
        r#"<a href="{href}" class="{cls}" data-route="{href}">{label}</a>"#,
        href = html_escape(href),
        cls = cls,
        label = html_escape(label),
    )
}

/// Render the breadcrumb bar.
fn render_breadcrumbs(breadcrumbs: &[(&str, Option<&str>)]) -> String {
    if breadcrumbs.is_empty() {
        return String::new();
    }

    let segments: Vec<String> = breadcrumbs
        .iter()
        .enumerate()
        .map(|(i, (label, href))| {
            let is_last = i == breadcrumbs.len() - 1;
            match href {
                Some(h) if !is_last => format!(
                    r#"<a href="{}" data-route="{}">{}</a>"#,
                    html_escape(h),
                    html_escape(h),
                    html_escape(label),
                ),
                _ => format!(
                    r#"<span class="adapto-breadcrumb-current">{}</span>"#,
                    html_escape(label),
                ),
            }
        })
        .collect();

    let sep = r#" <span class="adapto-breadcrumb-sep">/</span> "#;
    segments.join(sep)
}

/// Render a full HTML page with the standard Adapto layout.
///
/// This produces a complete `<!DOCTYPE html>` document with:
/// - Adapto UI CSS bundle inlined in a `<style>` tag
/// - Navigation bar with the app title and nav items
/// - Breadcrumb trail
/// - Stats area (`#app-stats`)
/// - Content area (`#app-content`)
/// - Embedded `live.js` for WebSocket reactivity
pub fn render_layout(config: &LayoutConfig<'_>) -> String {
    let css = adapto_ui::bundle_css();

    let nav_items_html: String = config
        .nav_items
        .iter()
        .map(|(label, href, active)| render_nav_item(label, href, *active))
        .collect();

    let breadcrumb_html = render_breadcrumbs(config.breadcrumbs);
    let title = html_escape(config.title);

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>{title}</title>
  <style>{css}</style>
  <style>
    body {{
      background-color: var(--au-color-bg-secondary);
      margin: 0;
    }}
    .adapto-main {{
      padding: var(--au-space-6);
    }}
    .adapto-breadcrumbs {{
      display: flex;
      gap: var(--au-space-2);
      align-items: center;
      padding: var(--au-space-2) var(--au-space-4);
      background: var(--au-color-bg);
      border-bottom: 1px solid var(--au-color-border);
      font-size: var(--au-text-sm);
      color: var(--au-color-text-secondary);
      font-family: var(--au-font-family);
    }}
    .adapto-breadcrumbs a {{
      color: var(--au-color-blue);
      text-decoration: none;
      font-weight: var(--au-weight-medium);
    }}
    .adapto-breadcrumbs a:hover {{
      text-decoration: underline;
    }}
    .adapto-breadcrumb-current {{
      color: var(--au-color-text);
      font-weight: var(--au-weight-semibold);
    }}
    .adapto-breadcrumb-sep {{
      color: var(--au-color-text-tertiary);
    }}
    {extra_css}
  </style>
</head>
<body>
  <nav class="au-nav au-nav--sticky au-nav--blur">
    <span class="au-nav__brand">{title}</span>
    <div class="au-nav__items">{nav_items_html}</div>
    <div class="au-nav__end">
      <div class="au-avatar au-avatar--sm au-avatar--blue" role="img" aria-label="User">
        <span class="au-avatar__initials" aria-hidden="true">U</span>
      </div>
    </div>
  </nav>
  <div class="adapto-breadcrumbs" id="app-breadcrumb">{breadcrumb_html}</div>
  <main class="adapto-main">
    <div class="au-container">
      <div id="app-stats">{stats_html}</div>
      <div id="app-content">{content_html}</div>
    </div>
  </main>
  <script>{live_js}</script>
</body>
</html>"#,
        title = title,
        css = css,
        extra_css = config.extra_css,
        nav_items_html = nav_items_html,
        breadcrumb_html = breadcrumb_html,
        stats_html = config.stats_html,
        content_html = config.content_html,
        live_js = LIVE_JS,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_contains_essential_structure() {
        let config = LayoutConfig {
            title: "Test App",
            nav_items: &[("Home", "/", true)],
            breadcrumbs: &[("home", Some("/")), ("page", None)],
            stats_html: "<p>stats</p>",
            content_html: "<p>content</p>",
            extra_css: "",
        };
        let html = render_layout(&config);

        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("<title>Test App</title>"));
        assert!(html.contains("au-nav__brand"));
        assert!(html.contains("id=\"app-content\""));
        assert!(html.contains("id=\"app-stats\""));
        assert!(html.contains("id=\"app-breadcrumb\""));
        assert!(html.contains("<p>content</p>"));
        assert!(html.contains("<p>stats</p>"));
        assert!(html.contains("WebSocket")); // live.js
    }

    #[test]
    fn breadcrumbs_render_links_and_current() {
        let crumbs = &[("Home", Some("/")), ("Users", Some("/users")), ("Detail", None)];
        let html = render_breadcrumbs(crumbs);

        assert!(html.contains(r#"<a href="/"#));
        assert!(html.contains(r#"data-route="/users""#));
        assert!(html.contains("adapto-breadcrumb-current"));
        assert!(html.contains("Detail"));
    }

    #[test]
    fn empty_breadcrumbs_produce_no_output() {
        assert!(render_breadcrumbs(&[]).is_empty());
    }

    #[test]
    fn nav_item_active_class() {
        let active = render_nav_item("Home", "/", true);
        let inactive = render_nav_item("About", "/about", false);

        assert!(active.contains("au-nav__item--active"));
        assert!(!inactive.contains("au-nav__item--active"));
    }
}
