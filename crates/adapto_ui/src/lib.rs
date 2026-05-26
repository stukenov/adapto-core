//! # Adapto UI
//!
//! A handwritten CSS component library inspired by Apple Human Interface Guidelines.
//! Every token, every component, every utility is built from scratch — no frameworks.
//!
//! ## Usage
//!
//! Embed the full CSS bundle as a string constant:
//!
//! ```rust
//! use adapto_ui::TOKENS_CSS;
//! assert!(!TOKENS_CSS.is_empty());
//! ```
//!
//! Or use individual component CSS files for tree-shaking at the build level.

pub mod components;

// ---------------------------------------------------------------------------
// Base
// ---------------------------------------------------------------------------

/// Modern CSS reset — predictable baseline across all browsers.
pub const RESET_CSS: &str = include_str!("../css/base/reset.css");

/// Design tokens — colors, spacing, typography, radii, shadows, motion.
pub const TOKENS_CSS: &str = include_str!("../css/base/tokens.css");

/// Typographic scale, heading styles, prose formatting.
pub const TYPOGRAPHY_CSS: &str = include_str!("../css/base/typography.css");

// ---------------------------------------------------------------------------
// Components
// ---------------------------------------------------------------------------

/// Button — primary, secondary, destructive, ghost, outline variants.
pub const BUTTON_CSS: &str = include_str!("../css/components/button.css");

/// Input, textarea, select — form controls with focus rings and error states.
pub const INPUT_CSS: &str = include_str!("../css/components/input.css");

/// Card — elevated, flat, and grouped content containers.
pub const CARD_CSS: &str = include_str!("../css/components/card.css");

/// Badge — status indicators and count labels.
pub const BADGE_CSS: &str = include_str!("../css/components/badge.css");

/// Alert — info, success, warning, error contextual messages.
pub const ALERT_CSS: &str = include_str!("../css/components/alert.css");

/// Modal — overlay dialog with backdrop blur.
pub const MODAL_CSS: &str = include_str!("../css/components/modal.css");

/// Table — clean data tables with sorting, striping, hover.
pub const TABLE_CSS: &str = include_str!("../css/components/table.css");

/// Nav — top bar, sidebar, and tab components.
pub const NAV_CSS: &str = include_str!("../css/components/nav.css");

/// Toggle — pure CSS iOS-style switch.
pub const TOGGLE_CSS: &str = include_str!("../css/components/toggle.css");

/// Avatar — profile images with initials and status dot.
pub const AVATAR_CSS: &str = include_str!("../css/components/avatar.css");

/// Tooltip — contextual hints with directional positioning.
pub const TOOLTIP_CSS: &str = include_str!("../css/components/tooltip.css");

/// Breadcrumb — navigational path indicator.
pub const BREADCRUMB_CSS: &str = include_str!("../css/components/breadcrumb.css");

/// Progress bar and spinner.
pub const PROGRESS_CSS: &str = include_str!("../css/components/progress.css");

/// Dropdown — menu panel with items and dividers.
pub const DROPDOWN_CSS: &str = include_str!("../css/components/dropdown.css");

/// Form group — label + input + help/error composition.
pub const FORM_GROUP_CSS: &str = include_str!("../css/components/form-group.css");

// ---------------------------------------------------------------------------
// Utilities
// ---------------------------------------------------------------------------

/// Flex, grid, container, stack, and layout helpers.
pub const LAYOUT_CSS: &str = include_str!("../css/utilities/layout.css");

/// Margin and padding utilities on the spacing scale.
pub const SPACING_CSS: &str = include_str!("../css/utilities/spacing.css");

/// Show/hide, screen-reader only, display helpers.
pub const VISIBILITY_CSS: &str = include_str!("../css/utilities/visibility.css");

// ---------------------------------------------------------------------------
// Bundle
// ---------------------------------------------------------------------------

/// All CSS files in the correct cascade order. This is the primary way to
/// embed the full Adapto UI system into a page.
///
/// Note: This concatenates individual files rather than using the `@import`
/// entry point, so it works as a single inline `<style>` block without
/// needing a CSS file resolver.
pub fn bundle_css() -> String {
    [
        // Base
        RESET_CSS,
        TOKENS_CSS,
        TYPOGRAPHY_CSS,
        // Components
        BUTTON_CSS,
        INPUT_CSS,
        CARD_CSS,
        BADGE_CSS,
        ALERT_CSS,
        MODAL_CSS,
        TABLE_CSS,
        NAV_CSS,
        TOGGLE_CSS,
        AVATAR_CSS,
        TOOLTIP_CSS,
        BREADCRUMB_CSS,
        PROGRESS_CSS,
        DROPDOWN_CSS,
        FORM_GROUP_CSS,
        // Utilities
        LAYOUT_CSS,
        SPACING_CSS,
        VISIBILITY_CSS,
    ]
    .join("\n")
}

/// Returns a `<style>` element containing the full Adapto UI CSS bundle.
pub fn style_tag() -> String {
    format!("<style>{}</style>", bundle_css())
}

/// Escape a string for safe inclusion in HTML content or attribute values.
pub fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

/// All CSS constant names and their content, useful for iteration.
pub fn all_css_files() -> Vec<(&'static str, &'static str)> {
    vec![
        ("reset", RESET_CSS),
        ("tokens", TOKENS_CSS),
        ("typography", TYPOGRAPHY_CSS),
        ("button", BUTTON_CSS),
        ("input", INPUT_CSS),
        ("card", CARD_CSS),
        ("badge", BADGE_CSS),
        ("alert", ALERT_CSS),
        ("modal", MODAL_CSS),
        ("table", TABLE_CSS),
        ("nav", NAV_CSS),
        ("toggle", TOGGLE_CSS),
        ("avatar", AVATAR_CSS),
        ("tooltip", TOOLTIP_CSS),
        ("breadcrumb", BREADCRUMB_CSS),
        ("progress", PROGRESS_CSS),
        ("dropdown", DROPDOWN_CSS),
        ("form-group", FORM_GROUP_CSS),
        ("layout", LAYOUT_CSS),
        ("spacing", SPACING_CSS),
        ("visibility", VISIBILITY_CSS),
    ]
}
