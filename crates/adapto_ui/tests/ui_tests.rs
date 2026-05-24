//! Integration tests for Adapto UI.
//!
//! Verifies CSS constants load correctly and component HTML helpers
//! produce valid, accessible output.

use adapto_ui::components::*;

// ===========================================================================
// CSS constants — non-empty verification
// ===========================================================================

#[test]
fn all_css_constants_are_non_empty() {
    for (name, css) in adapto_ui::all_css_files() {
        assert!(
            !css.is_empty(),
            "CSS file '{}' is empty — it should contain styles",
            name
        );
    }
}

#[test]
fn css_constants_count() {
    // 3 base + 15 components + 3 utilities = 21
    assert_eq!(adapto_ui::all_css_files().len(), 21);
}

#[test]
fn bundle_css_is_non_empty() {
    let bundle = adapto_ui::bundle_css();
    assert!(!bundle.is_empty());
    // Should contain tokens from multiple files
    assert!(bundle.contains("--au-color-blue"));
    assert!(bundle.contains(".au-btn"));
    assert!(bundle.contains(".au-input"));
    assert!(bundle.contains(".au-card"));
}

#[test]
fn style_tag_wraps_bundle() {
    let tag = adapto_ui::style_tag();
    assert!(tag.starts_with("<style>"));
    assert!(tag.ends_with("</style>"));
    assert!(tag.contains("--au-color-blue"));
}

// ===========================================================================
// CSS content quality checks
// ===========================================================================

#[test]
fn reset_css_has_box_sizing() {
    assert!(adapto_ui::RESET_CSS.contains("box-sizing: border-box"));
}

#[test]
fn tokens_css_has_design_tokens() {
    let css = adapto_ui::TOKENS_CSS;
    assert!(css.contains("--au-color-blue"));
    assert!(css.contains("--au-color-green"));
    assert!(css.contains("--au-color-red"));
    assert!(css.contains("--au-space-1"));
    assert!(css.contains("--au-radius-md"));
    assert!(css.contains("--au-shadow-md"));
    assert!(css.contains("--au-font-family"));
    assert!(css.contains("--au-text-base"));
    assert!(css.contains("--au-duration-fast"));
    assert!(css.contains("--au-ease-default"));
}

#[test]
fn tokens_css_has_dark_mode() {
    assert!(adapto_ui::TOKENS_CSS.contains("prefers-color-scheme: dark"));
}

#[test]
fn tokens_css_has_high_contrast() {
    assert!(adapto_ui::TOKENS_CSS.contains("forced-colors: active"));
}

#[test]
fn button_css_has_all_variants() {
    let css = adapto_ui::BUTTON_CSS;
    assert!(css.contains(".au-btn--primary"));
    assert!(css.contains(".au-btn--secondary"));
    assert!(css.contains(".au-btn--destructive"));
    assert!(css.contains(".au-btn--ghost"));
    assert!(css.contains(".au-btn--outline"));
    assert!(css.contains(".au-btn--sm"));
    assert!(css.contains(".au-btn--lg"));
    assert!(css.contains(".au-btn--loading"));
}

#[test]
fn input_css_has_states() {
    let css = adapto_ui::INPUT_CSS;
    assert!(css.contains(".au-input"));
    assert!(css.contains(".au-input--error"));
    assert!(css.contains(".au-textarea"));
    assert!(css.contains(".au-select"));
}

#[test]
fn card_css_has_variants() {
    let css = adapto_ui::CARD_CSS;
    assert!(css.contains(".au-card--elevated"));
    assert!(css.contains(".au-card--flat"));
    assert!(css.contains(".au-card--grouped"));
    assert!(css.contains(".au-card__header"));
    assert!(css.contains(".au-card__body"));
    assert!(css.contains(".au-card__footer"));
}

#[test]
fn toggle_css_has_ios_switch() {
    let css = adapto_ui::TOGGLE_CSS;
    assert!(css.contains(".au-toggle"));
    assert!(css.contains(".au-toggle__track"));
    assert!(css.contains(".au-toggle__thumb"));
    assert!(css.contains(":checked"));
}

#[test]
fn nav_css_has_all_patterns() {
    let css = adapto_ui::NAV_CSS;
    assert!(css.contains(".au-nav"));
    assert!(css.contains(".au-sidebar"));
    assert!(css.contains(".au-tabs"));
    assert!(css.contains(".au-tabs--pill"));
}

#[test]
fn modal_css_has_overlay_and_animation() {
    let css = adapto_ui::MODAL_CSS;
    assert!(css.contains(".au-modal-overlay"));
    assert!(css.contains(".au-modal"));
    assert!(css.contains("backdrop-filter"));
    assert!(css.contains(".au-modal__header"));
    assert!(css.contains(".au-modal__body"));
    assert!(css.contains(".au-modal__footer"));
}

#[test]
fn table_css_has_variants() {
    let css = adapto_ui::TABLE_CSS;
    assert!(css.contains(".au-table"));
    assert!(css.contains(".au-table--striped"));
    assert!(css.contains(".au-table--hoverable"));
    assert!(css.contains(".au-table--sticky"));
}

#[test]
fn progress_css_has_bar_and_spinner() {
    let css = adapto_ui::PROGRESS_CSS;
    assert!(css.contains(".au-progress"));
    assert!(css.contains(".au-progress__bar"));
    assert!(css.contains(".au-spinner"));
}

// ===========================================================================
// Reduced motion support in components
// ===========================================================================

#[test]
fn components_with_animation_have_reduced_motion() {
    let animated = [
        ("button", adapto_ui::BUTTON_CSS),
        ("card", adapto_ui::CARD_CSS),
        ("modal", adapto_ui::MODAL_CSS),
        ("toggle", adapto_ui::TOGGLE_CSS),
        ("nav", adapto_ui::NAV_CSS),
        ("table", adapto_ui::TABLE_CSS),
        ("progress", adapto_ui::PROGRESS_CSS),
        ("tooltip", adapto_ui::TOOLTIP_CSS),
    ];

    for (name, css) in animated {
        assert!(
            css.contains("prefers-reduced-motion"),
            "Component '{}' has animations but no prefers-reduced-motion media query",
            name
        );
    }
}

// ===========================================================================
// Focus styles
// ===========================================================================

#[test]
fn reset_has_focus_visible() {
    assert!(adapto_ui::RESET_CSS.contains(":focus-visible"));
}

#[test]
fn button_has_focus_visible() {
    assert!(adapto_ui::BUTTON_CSS.contains(":focus-visible"));
}

#[test]
fn input_has_focus_styles() {
    assert!(adapto_ui::INPUT_CSS.contains(":focus"));
}

#[test]
fn nav_items_have_focus_visible() {
    assert!(adapto_ui::NAV_CSS.contains(":focus-visible"));
}

// ===========================================================================
// CSS concatenation integrity
// ===========================================================================

#[test]
fn bundle_has_no_malformed_markers() {
    let bundle = adapto_ui::bundle_css();
    // Should not have stray markers from include_str
    assert!(!bundle.contains('\0'), "Bundle contains null bytes");
    // All opening braces should have matching closing braces
    let opens = bundle.matches('{').count();
    let closes = bundle.matches('}').count();
    assert_eq!(
        opens, closes,
        "Mismatched braces: {} opens vs {} closes",
        opens, closes
    );
}

#[test]
fn bundle_preserves_css_custom_properties() {
    let bundle = adapto_ui::bundle_css();
    // Verify tokens are present and intact
    assert!(bundle.contains(":root"));
    assert!(bundle.contains("--au-color-blue"));
    assert!(bundle.contains("--au-space-4"));
}

#[test]
fn individual_files_have_balanced_braces() {
    for (name, css) in adapto_ui::all_css_files() {
        let opens = css.matches('{').count();
        let closes = css.matches('}').count();
        assert_eq!(
            opens, closes,
            "File '{}' has mismatched braces: {} opens vs {} closes",
            name, opens, closes
        );
    }
}

// ===========================================================================
// Component HTML helpers — structural tests
// ===========================================================================

#[test]
fn button_variants_produce_correct_classes() {
    let cases = vec![
        (Button::primary("A").render(), "au-btn--primary"),
        (Button::secondary("B").render(), "au-btn--secondary"),
        (Button::destructive("C").render(), "au-btn--destructive"),
        (Button::ghost("D").render(), "au-btn--ghost"),
        (Button::outline("E").render(), "au-btn--outline"),
    ];

    for (html, expected_class) in cases {
        assert!(
            html.contains(expected_class),
            "Expected class '{}' in: {}",
            expected_class,
            html
        );
        assert!(html.contains("<button"), "Missing <button tag in: {}", html);
        assert!(html.contains("</button>"), "Missing </button> in: {}", html);
    }
}

#[test]
fn input_renders_self_closing() {
    let html = Input::text("name").render();
    assert!(html.contains("<input "));
    assert!(html.contains("/>"));
    assert!(!html.contains("</input>"));
}

#[test]
fn toggle_wraps_in_label_for_accessibility() {
    let html = Toggle::new("test").render();
    assert!(html.contains("<label"));
    assert!(html.contains("</label>"));
    assert!(html.contains("role=\"switch\""));
}

#[test]
fn card_has_proper_nesting() {
    let html = Card::elevated("Body text").header("Header").footer("Footer").render();
    assert!(html.contains("<div class=\"au-card au-card--elevated\">"));
    assert!(html.contains("<div class=\"au-card__header\">Header</div>"));
    assert!(html.contains("<div class=\"au-card__body\">Body text</div>"));
    assert!(html.contains("<div class=\"au-card__footer\">Footer</div>"));
}

#[test]
fn alert_all_levels() {
    let levels = vec![
        (Alert::info("A"), "au-alert--info"),
        (Alert::success("B"), "au-alert--success"),
        (Alert::warning("C"), "au-alert--warning"),
        (Alert::error("D"), "au-alert--error"),
    ];

    for (alert, expected) in levels {
        let html = alert.render();
        assert!(html.contains(expected));
        assert!(html.contains("role=\"alert\""));
    }
}

#[test]
fn avatar_group_scenario() {
    let a1 = Avatar::initials("AB", "Alice Brown").render();
    let a2 = Avatar::image("/bob.jpg", "Bob").size("au-avatar--sm").render();
    assert!(a1.contains("au-avatar__initials"));
    assert!(a2.contains("au-avatar--sm"));
    assert!(a2.contains("src=\"/bob.jpg\""));
}

#[test]
fn progress_value_in_range() {
    let p = Progress::new(50);
    let html = p.render();
    assert!(html.contains("aria-valuenow=\"50\""));
    assert!(html.contains("width: 50%"));
}

#[test]
fn breadcrumb_single_item() {
    let html = Breadcrumb::render(&[("Home", None)]);
    assert!(html.contains("aria-current=\"page\""));
    // Single item should not have separators
    assert!(!html.contains("au-breadcrumb__separator"));
}

#[test]
fn breadcrumb_multiple_items_have_separators() {
    let html = Breadcrumb::render(&[
        ("A", Some("/a")),
        ("B", Some("/b")),
        ("C", None),
    ]);
    // Two separators between three items
    let sep_count = html.matches("au-breadcrumb__separator").count();
    assert_eq!(sep_count, 2, "Expected 2 separators, got {}", sep_count);
}

// ===========================================================================
// HTML escaping in component output
// ===========================================================================

#[test]
fn button_escapes_label() {
    let html = Button::primary("<b>XSS</b>").render();
    assert!(!html.contains("<b>XSS</b>"));
    assert!(html.contains("&lt;b&gt;XSS&lt;/b&gt;"));
}

#[test]
fn input_escapes_placeholder() {
    let html = Input::text("x").placeholder("a\"b").render();
    assert!(html.contains("placeholder=\"a&quot;b\""));
}

#[test]
fn alert_escapes_message() {
    let html = Alert::error("<img onerror=alert(1)>").render();
    assert!(!html.contains("<img"));
    assert!(html.contains("&lt;img"));
}

// ===========================================================================
// Utility CSS checks
// ===========================================================================

#[test]
fn layout_css_has_container_and_flex() {
    let css = adapto_ui::LAYOUT_CSS;
    assert!(css.contains(".au-container"));
    assert!(css.contains(".au-flex"));
    assert!(css.contains(".au-grid"));
    assert!(css.contains(".au-stack"));
}

#[test]
fn spacing_css_has_padding_and_margin() {
    let css = adapto_ui::SPACING_CSS;
    assert!(css.contains(".au-p-4"));
    assert!(css.contains(".au-m-4"));
    assert!(css.contains("padding-inline"));
    assert!(css.contains("margin-block"));
}

#[test]
fn visibility_css_has_sr_only() {
    let css = adapto_ui::VISIBILITY_CSS;
    assert!(css.contains(".au-sr-only"));
    assert!(css.contains(".au-hidden"));
}

// ===========================================================================
// au- prefix consistency
// ===========================================================================

#[test]
fn all_component_css_uses_au_prefix() {
    let component_files = [
        ("button", adapto_ui::BUTTON_CSS),
        ("input", adapto_ui::INPUT_CSS),
        ("card", adapto_ui::CARD_CSS),
        ("badge", adapto_ui::BADGE_CSS),
        ("alert", adapto_ui::ALERT_CSS),
        ("modal", adapto_ui::MODAL_CSS),
        ("table", adapto_ui::TABLE_CSS),
        ("nav", adapto_ui::NAV_CSS),
        ("toggle", adapto_ui::TOGGLE_CSS),
        ("avatar", adapto_ui::AVATAR_CSS),
        ("tooltip", adapto_ui::TOOLTIP_CSS),
        ("breadcrumb", adapto_ui::BREADCRUMB_CSS),
        ("progress", adapto_ui::PROGRESS_CSS),
        ("dropdown", adapto_ui::DROPDOWN_CSS),
        ("form-group", adapto_ui::FORM_GROUP_CSS),
    ];

    for (name, css) in component_files {
        // Each component file should contain at least one .au- class
        assert!(
            css.contains(".au-"),
            "Component '{}' does not use the au- prefix",
            name
        );
    }
}

// ===========================================================================
// Print styles
// ===========================================================================

#[test]
fn print_styles_exist_where_needed() {
    let printable = [
        ("button", adapto_ui::BUTTON_CSS),
        ("card", adapto_ui::CARD_CSS),
        ("table", adapto_ui::TABLE_CSS),
        ("modal", adapto_ui::MODAL_CSS),
        ("nav", adapto_ui::NAV_CSS),
        ("alert", adapto_ui::ALERT_CSS),
    ];

    for (name, css) in printable {
        assert!(
            css.contains("@media print"),
            "Component '{}' should have print styles",
            name
        );
    }
}
