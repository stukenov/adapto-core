//! # Component HTML Helpers
//!
//! Typed builders that emit accessible HTML strings for Adapto UI components.
//! Each builder ensures correct class names, ARIA attributes, and semantic markup.

// ---------------------------------------------------------------------------
// Button
// ---------------------------------------------------------------------------

/// Button variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonVariant {
    Primary,
    Secondary,
    Destructive,
    Ghost,
    Outline,
}

/// Button size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonSize {
    Small,
    Default,
    Large,
}

/// HTML button type attribute.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonType {
    Button,
    Submit,
    Reset,
}

impl ButtonType {
    fn as_str(&self) -> &'static str {
        match self {
            ButtonType::Button => "button",
            ButtonType::Submit => "submit",
            ButtonType::Reset => "reset",
        }
    }
}

/// A builder for `<button class="au-btn ...">`.
#[derive(Debug, Clone)]
pub struct Button {
    label: String,
    variant: ButtonVariant,
    size: ButtonSize,
    disabled: bool,
    loading: bool,
    icon_html: Option<String>,
    r#type: ButtonType,
}

impl Button {
    /// Create a primary button.
    pub fn primary(label: &str) -> Self {
        Self::new(label, ButtonVariant::Primary)
    }

    /// Create a secondary button.
    pub fn secondary(label: &str) -> Self {
        Self::new(label, ButtonVariant::Secondary)
    }

    /// Create a destructive button.
    pub fn destructive(label: &str) -> Self {
        Self::new(label, ButtonVariant::Destructive)
    }

    /// Create a ghost button.
    pub fn ghost(label: &str) -> Self {
        Self::new(label, ButtonVariant::Ghost)
    }

    /// Create an outline button.
    pub fn outline(label: &str) -> Self {
        Self::new(label, ButtonVariant::Outline)
    }

    fn new(label: &str, variant: ButtonVariant) -> Self {
        Self {
            label: label.to_string(),
            variant,
            size: ButtonSize::Default,
            disabled: false,
            loading: false,
            icon_html: None,
            r#type: ButtonType::Button,
        }
    }

    /// Set size to small.
    pub fn small(mut self) -> Self {
        self.size = ButtonSize::Small;
        self
    }

    /// Set size to large.
    pub fn large(mut self) -> Self {
        self.size = ButtonSize::Large;
        self
    }

    /// Mark as disabled.
    pub fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }

    /// Mark as loading.
    pub fn loading(mut self) -> Self {
        self.loading = true;
        self
    }

    /// Set the button type attribute.
    pub fn button_type(mut self, t: ButtonType) -> Self {
        self.r#type = t;
        self
    }

    /// Prepend an icon (raw HTML string, e.g. an SVG).
    pub fn icon(mut self, html: &str) -> Self {
        self.icon_html = Some(html.to_string());
        self
    }

    /// Render the button to an HTML string.
    pub fn render(&self) -> String {
        let mut classes = vec!["au-btn".to_string()];

        match self.variant {
            ButtonVariant::Primary => classes.push("au-btn--primary".into()),
            ButtonVariant::Secondary => classes.push("au-btn--secondary".into()),
            ButtonVariant::Destructive => classes.push("au-btn--destructive".into()),
            ButtonVariant::Ghost => classes.push("au-btn--ghost".into()),
            ButtonVariant::Outline => classes.push("au-btn--outline".into()),
        }

        match self.size {
            ButtonSize::Small => classes.push("au-btn--sm".into()),
            ButtonSize::Default => {}
            ButtonSize::Large => classes.push("au-btn--lg".into()),
        }

        if self.loading {
            classes.push("au-btn--loading".into());
        }

        let disabled_attr = if self.disabled {
            " disabled aria-disabled=\"true\""
        } else {
            ""
        };

        let icon_part = match &self.icon_html {
            Some(svg) => format!("<span class=\"au-btn-icon\" aria-hidden=\"true\">{}</span>", svg),
            None => String::new(),
        };

        format!(
            "<button type=\"{}\" class=\"{}\"{}>{}{}</button>",
            self.r#type.as_str(),
            classes.join(" "),
            disabled_attr,
            icon_part,
            html_escape(&self.label),
        )
    }
}

// ---------------------------------------------------------------------------
// Input
// ---------------------------------------------------------------------------

/// Input type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputType {
    Text,
    Email,
    Password,
    Number,
    Search,
    Tel,
    Url,
}

impl InputType {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Email => "email",
            Self::Password => "password",
            Self::Number => "number",
            Self::Search => "search",
            Self::Tel => "tel",
            Self::Url => "url",
        }
    }
}

/// A builder for `<input class="au-input ...">`.
#[derive(Debug, Clone)]
pub struct Input {
    input_type: InputType,
    name: String,
    placeholder: Option<String>,
    value: Option<String>,
    error: bool,
    disabled: bool,
    required: bool,
    label: Option<String>,
}

impl Input {
    /// Create a new text input.
    pub fn text(name: &str) -> Self {
        Self::new(name, InputType::Text)
    }

    /// Create an email input.
    pub fn email(name: &str) -> Self {
        Self::new(name, InputType::Email)
    }

    /// Create a password input.
    pub fn password(name: &str) -> Self {
        Self::new(name, InputType::Password)
    }

    fn new(name: &str, input_type: InputType) -> Self {
        Self {
            input_type,
            name: name.to_string(),
            placeholder: None,
            value: None,
            error: false,
            disabled: false,
            required: false,
            label: None,
        }
    }

    /// Set placeholder text.
    pub fn placeholder(mut self, text: &str) -> Self {
        self.placeholder = Some(text.to_string());
        self
    }

    /// Set initial value.
    pub fn value(mut self, val: &str) -> Self {
        self.value = Some(val.to_string());
        self
    }

    /// Mark input as having an error.
    pub fn error(mut self) -> Self {
        self.error = true;
        self
    }

    /// Mark as disabled.
    pub fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }

    /// Mark as required.
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    /// Associate a label (renders an `aria-label` attribute).
    pub fn label(mut self, label: &str) -> Self {
        self.label = Some(label.to_string());
        self
    }

    /// Render the input to an HTML string.
    pub fn render(&self) -> String {
        let mut classes = vec!["au-input".to_string()];
        if self.error {
            classes.push("au-input--error".into());
        }

        let mut attrs = vec![
            format!("type=\"{}\"", self.input_type.as_str()),
            format!("name=\"{}\"", html_escape(&self.name)),
            format!("class=\"{}\"", classes.join(" ")),
        ];

        if let Some(ref ph) = self.placeholder {
            attrs.push(format!("placeholder=\"{}\"", html_escape(ph)));
        }
        if let Some(ref val) = self.value {
            attrs.push(format!("value=\"{}\"", html_escape(val)));
        }
        if let Some(ref lbl) = self.label {
            attrs.push(format!("aria-label=\"{}\"", html_escape(lbl)));
        }
        if self.disabled {
            attrs.push("disabled".into());
            attrs.push("aria-disabled=\"true\"".into());
        }
        if self.required {
            attrs.push("required".into());
            attrs.push("aria-required=\"true\"".into());
        }
        if self.error {
            attrs.push("aria-invalid=\"true\"".into());
        }

        format!("<input {} />", attrs.join(" "))
    }
}

// ---------------------------------------------------------------------------
// Toggle
// ---------------------------------------------------------------------------

/// A builder for the iOS-style toggle switch.
#[derive(Debug, Clone)]
pub struct Toggle {
    name: String,
    checked: bool,
    disabled: bool,
    label: Option<String>,
}

impl Toggle {
    /// Create a new toggle.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            checked: false,
            disabled: false,
            label: None,
        }
    }

    /// Set initial checked state.
    pub fn checked(mut self) -> Self {
        self.checked = true;
        self
    }

    /// Mark as disabled.
    pub fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }

    /// Visible label text.
    pub fn label(mut self, text: &str) -> Self {
        self.label = Some(text.to_string());
        self
    }

    /// Render the toggle to an HTML string.
    pub fn render(&self) -> String {
        let checked_attr = if self.checked { " checked" } else { "" };
        let disabled_attr = if self.disabled { " disabled" } else { "" };

        let aria_label = match &self.label {
            Some(l) => format!(" aria-label=\"{}\"", html_escape(l)),
            None => String::new(),
        };

        let label_span = match &self.label {
            Some(l) => format!("<span class=\"au-toggle__label\">{}</span>", html_escape(l)),
            None => String::new(),
        };

        format!(
            "<label class=\"au-toggle\">\
                <input type=\"checkbox\" class=\"au-toggle__input\" name=\"{}\" role=\"switch\"{}{}{} />\
                <span class=\"au-toggle__track\">\
                    <span class=\"au-toggle__thumb\"></span>\
                </span>\
                {}\
            </label>",
            html_escape(&self.name),
            checked_attr,
            disabled_attr,
            aria_label,
            label_span,
        )
    }
}

// ---------------------------------------------------------------------------
// Card
// ---------------------------------------------------------------------------

/// Card elevation variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardVariant {
    Elevated,
    Flat,
    Grouped,
}

/// A builder for `<div class="au-card ...">`.
///
/// `body`, `header`, and `footer` accept **raw HTML** — the caller is responsible
/// for escaping user-controlled content with `html_escape()` before passing it in.
#[derive(Debug, Clone)]
pub struct Card {
    variant: CardVariant,
    header: Option<String>,
    body: String,
    footer: Option<String>,
    hoverable: bool,
}

impl Card {
    /// Create an elevated card. `body` is raw HTML — escape user content first.
    pub fn elevated(body: &str) -> Self {
        Self::new(body, CardVariant::Elevated)
    }

    /// Create a flat (bordered) card.
    pub fn flat(body: &str) -> Self {
        Self::new(body, CardVariant::Flat)
    }

    fn new(body: &str, variant: CardVariant) -> Self {
        Self {
            variant,
            header: None,
            body: body.to_string(),
            footer: None,
            hoverable: false,
        }
    }

    /// Set header content.
    pub fn header(mut self, text: &str) -> Self {
        self.header = Some(text.to_string());
        self
    }

    /// Set footer content.
    pub fn footer(mut self, text: &str) -> Self {
        self.footer = Some(text.to_string());
        self
    }

    /// Enable hover lift effect.
    pub fn hoverable(mut self) -> Self {
        self.hoverable = true;
        self
    }

    /// Render the card to an HTML string.
    pub fn render(&self) -> String {
        let mut classes = vec!["au-card".to_string()];

        match self.variant {
            CardVariant::Elevated => classes.push("au-card--elevated".into()),
            CardVariant::Flat => classes.push("au-card--flat".into()),
            CardVariant::Grouped => classes.push("au-card--grouped".into()),
        }

        if self.hoverable {
            classes.push("au-card--hoverable".into());
        }

        let header_html = match &self.header {
            Some(h) => format!("<div class=\"au-card__header\">{}</div>", h),
            None => String::new(),
        };

        let footer_html = match &self.footer {
            Some(f) => format!("<div class=\"au-card__footer\">{}</div>", f),
            None => String::new(),
        };

        format!(
            "<div class=\"{}\">{}<div class=\"au-card__body\">{}</div>{}</div>",
            classes.join(" "),
            header_html,
            self.body,
            footer_html,
        )
    }
}

// ---------------------------------------------------------------------------
// Alert
// ---------------------------------------------------------------------------

/// Alert severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertLevel {
    Info,
    Success,
    Warning,
    Error,
}

/// A builder for `<div class="au-alert ..." role="alert">`.
#[derive(Debug, Clone)]
pub struct Alert {
    level: AlertLevel,
    title: Option<String>,
    message: String,
    dismissible: bool,
}

impl Alert {
    /// Create an info alert.
    pub fn info(message: &str) -> Self {
        Self::new(message, AlertLevel::Info)
    }

    /// Create a success alert.
    pub fn success(message: &str) -> Self {
        Self::new(message, AlertLevel::Success)
    }

    /// Create a warning alert.
    pub fn warning(message: &str) -> Self {
        Self::new(message, AlertLevel::Warning)
    }

    /// Create an error alert.
    pub fn error(message: &str) -> Self {
        Self::new(message, AlertLevel::Error)
    }

    fn new(message: &str, level: AlertLevel) -> Self {
        Self {
            level,
            title: None,
            message: message.to_string(),
            dismissible: false,
        }
    }

    /// Set a title line.
    pub fn title(mut self, text: &str) -> Self {
        self.title = Some(text.to_string());
        self
    }

    /// Add a dismiss button.
    pub fn dismissible(mut self) -> Self {
        self.dismissible = true;
        self
    }

    /// Render the alert to an HTML string.
    pub fn render(&self) -> String {
        let variant_class = match self.level {
            AlertLevel::Info => "au-alert--info",
            AlertLevel::Success => "au-alert--success",
            AlertLevel::Warning => "au-alert--warning",
            AlertLevel::Error => "au-alert--error",
        };

        let aria_live = match self.level {
            AlertLevel::Error => "assertive",
            _ => "polite",
        };

        let title_html = match &self.title {
            Some(t) => format!("<div class=\"au-alert__title\">{}</div>", html_escape(t)),
            None => String::new(),
        };

        let dismiss_html = if self.dismissible {
            "<button class=\"au-alert__dismiss\" aria-label=\"Dismiss\">\
                <svg width=\"14\" height=\"14\" viewBox=\"0 0 14 14\" fill=\"none\" aria-hidden=\"true\">\
                    <path d=\"M1 1l12 12M13 1L1 13\" stroke=\"currentColor\" stroke-width=\"1.5\" stroke-linecap=\"round\"/>\
                </svg>\
            </button>"
                .to_string()
        } else {
            String::new()
        };

        format!(
            "<div class=\"au-alert {}\" role=\"alert\" aria-live=\"{}\">\
                <div class=\"au-alert__content\">{}<div class=\"au-alert__message\">{}</div></div>\
                {}\
            </div>",
            variant_class,
            aria_live,
            title_html,
            html_escape(&self.message),
            dismiss_html,
        )
    }
}

// ---------------------------------------------------------------------------
// Badge
// ---------------------------------------------------------------------------

/// Badge variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BadgeVariant {
    Default,
    Info,
    Success,
    Warning,
    Error,
}

/// A builder for `<span class="au-badge ...">`.
#[derive(Debug, Clone)]
pub struct Badge {
    label: String,
    variant: BadgeVariant,
}

impl Badge {
    /// Create a badge.
    pub fn new(label: &str, variant: BadgeVariant) -> Self {
        Self {
            label: label.to_string(),
            variant,
        }
    }

    /// Render the badge to an HTML string.
    pub fn render(&self) -> String {
        let variant_class = match self.variant {
            BadgeVariant::Default => "au-badge--default",
            BadgeVariant::Info => "au-badge--info",
            BadgeVariant::Success => "au-badge--success",
            BadgeVariant::Warning => "au-badge--warning",
            BadgeVariant::Error => "au-badge--error",
        };

        format!(
            "<span class=\"au-badge {}\">{}</span>",
            variant_class,
            html_escape(&self.label),
        )
    }
}

// ---------------------------------------------------------------------------
// Avatar
// ---------------------------------------------------------------------------

/// A builder for avatar components.
#[derive(Debug, Clone)]
pub struct Avatar {
    size_class: Option<String>,
    initials: Option<String>,
    image_src: Option<String>,
    alt: String,
    status: Option<String>,
}

impl Avatar {
    /// Create an avatar with initials.
    pub fn initials(initials: &str, alt: &str) -> Self {
        Self {
            size_class: None,
            initials: Some(initials.to_string()),
            image_src: None,
            alt: alt.to_string(),
            status: None,
        }
    }

    /// Create an avatar with an image.
    pub fn image(src: &str, alt: &str) -> Self {
        Self {
            size_class: None,
            initials: None,
            image_src: Some(src.to_string()),
            alt: alt.to_string(),
            status: None,
        }
    }

    /// Set size class (e.g. "au-avatar--sm", "au-avatar--lg").
    pub fn size(mut self, class: &str) -> Self {
        self.size_class = Some(class.to_string());
        self
    }

    /// Show status dot ("online", "busy", "away").
    pub fn status(mut self, status: &str) -> Self {
        self.status = Some(status.to_string());
        self
    }

    /// Render the avatar to an HTML string.
    pub fn render(&self) -> String {
        let mut classes = vec!["au-avatar".to_string()];
        if let Some(ref sc) = self.size_class {
            classes.push(sc.clone());
        }

        let content = if let Some(ref src) = self.image_src {
            format!(
                "<img class=\"au-avatar__image\" src=\"{}\" alt=\"{}\" />",
                html_escape(src),
                html_escape(&self.alt)
            )
        } else if let Some(ref ini) = self.initials {
            format!(
                "<span class=\"au-avatar__initials\" aria-hidden=\"true\">{}</span>",
                html_escape(ini)
            )
        } else {
            String::new()
        };

        let status_html = match &self.status {
            Some(s) => format!(
                "<span class=\"au-avatar__status au-avatar__status--{}\" aria-label=\"{}\"></span>",
                html_escape(s),
                html_escape(s)
            ),
            None => String::new(),
        };

        format!(
            "<div class=\"{}\" role=\"img\" aria-label=\"{}\">{}{}</div>",
            classes.join(" "),
            html_escape(&self.alt),
            content,
            status_html,
        )
    }
}

// ---------------------------------------------------------------------------
// Progress
// ---------------------------------------------------------------------------

/// A builder for progress bars.
#[derive(Debug, Clone)]
pub struct Progress {
    value: u8,
    max: u8,
    label: Option<String>,
}

impl Progress {
    /// Create a progress bar with a value 0..100.
    pub fn new(value: u8) -> Self {
        Self {
            value: value.min(100),
            max: 100,
            label: None,
        }
    }

    /// Set an accessible label.
    pub fn label(mut self, text: &str) -> Self {
        self.label = Some(text.to_string());
        self
    }

    /// Render the progress bar to an HTML string.
    pub fn render(&self) -> String {
        let aria_label = match &self.label {
            Some(l) => format!(" aria-label=\"{}\"", html_escape(l)),
            None => String::new(),
        };

        format!(
            "<div class=\"au-progress\" role=\"progressbar\" \
                aria-valuenow=\"{}\" aria-valuemin=\"0\" aria-valuemax=\"{}\"{}>\
                <div class=\"au-progress__bar\" style=\"width: {}%\"></div>\
            </div>",
            self.value,
            self.max,
            aria_label,
            self.value,
        )
    }
}

// ---------------------------------------------------------------------------
// Spinner
// ---------------------------------------------------------------------------

/// Renders a loading spinner.
pub struct Spinner;

impl Spinner {
    /// Render a spinner with an optional label.
    pub fn render(label: Option<&str>) -> String {
        match label {
            Some(l) => format!(
                "<div class=\"au-spinner-container\">\
                    <div class=\"au-spinner\" role=\"status\" aria-label=\"{}\">\
                        <span class=\"au-sr-only\">{}</span>\
                    </div>\
                    <span class=\"au-spinner-container__label\">{}</span>\
                </div>",
                html_escape(l),
                html_escape(l),
                html_escape(l),
            ),
            None => "<div class=\"au-spinner\" role=\"status\" aria-label=\"Loading\">\
                        <span class=\"au-sr-only\">Loading</span>\
                    </div>"
                .to_string(),
        }
    }
}

// ---------------------------------------------------------------------------
// FormGroup
// ---------------------------------------------------------------------------

/// A builder for label + input + error message composition.
#[derive(Debug, Clone)]
pub struct FormGroup {
    label: String,
    input_html: String,
    error: Option<String>,
    help: Option<String>,
    required: bool,
}

impl FormGroup {
    /// Create a form group wrapping a rendered input.
    pub fn new(label: &str, input_html: &str) -> Self {
        Self {
            label: label.to_string(),
            input_html: input_html.to_string(),
            error: None,
            help: None,
            required: false,
        }
    }

    /// Add an error message.
    pub fn error(mut self, msg: &str) -> Self {
        self.error = Some(msg.to_string());
        self
    }

    /// Add help text.
    pub fn help(mut self, text: &str) -> Self {
        self.help = Some(text.to_string());
        self
    }

    /// Mark as required (adds asterisk to label).
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    /// Render the form group to an HTML string.
    pub fn render(&self) -> String {
        let required_marker = if self.required {
            "<span class=\"au-form-group__required\" aria-hidden=\"true\">*</span>"
        } else {
            ""
        };

        let help_html = match &self.help {
            Some(h) => format!(
                "<p class=\"au-form-group__help\">{}</p>",
                html_escape(h)
            ),
            None => String::new(),
        };

        let error_html = match &self.error {
            Some(e) => format!(
                "<p class=\"au-form-group__error\" role=\"alert\">{}</p>",
                html_escape(e)
            ),
            None => String::new(),
        };

        format!(
            "<div class=\"au-form-group\">\
                <label class=\"au-form-group__label\">{}{}</label>\
                {}\
                {}\
                {}\
            </div>",
            html_escape(&self.label),
            required_marker,
            self.input_html,
            help_html,
            error_html,
        )
    }
}

// ---------------------------------------------------------------------------
// Breadcrumb
// ---------------------------------------------------------------------------

/// Renders a breadcrumb navigation from a list of (label, href) pairs.
/// The last item is treated as the current page (not a link).
pub struct Breadcrumb;

impl Breadcrumb {
    /// Render a breadcrumb from segments. The last segment has no link.
    pub fn render(segments: &[(&str, Option<&str>)]) -> String {
        if segments.is_empty() {
            return String::new();
        }

        let mut items = Vec::new();

        for (i, (label, href)) in segments.iter().enumerate() {
            let is_last = i == segments.len() - 1;

            if i > 0 {
                items.push("<span class=\"au-breadcrumb__separator\" aria-hidden=\"true\"></span>".to_string());
            }

            if is_last {
                items.push(format!(
                    "<span class=\"au-breadcrumb__item\">\
                        <span class=\"au-breadcrumb__link\" aria-current=\"page\">{}</span>\
                    </span>",
                    html_escape(label),
                ));
            } else if let Some(url) = href {
                items.push(format!(
                    "<span class=\"au-breadcrumb__item\">\
                        <a class=\"au-breadcrumb__link\" href=\"{}\">{}</a>\
                    </span>",
                    html_escape(url),
                    html_escape(label),
                ));
            } else {
                items.push(format!(
                    "<span class=\"au-breadcrumb__item\">\
                        <span class=\"au-breadcrumb__link\">{}</span>\
                    </span>",
                    html_escape(label),
                ));
            }
        }

        format!(
            "<nav class=\"au-breadcrumb\" aria-label=\"Breadcrumb\">{}</nav>",
            items.join(""),
        )
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

use crate::html_escape;

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn button_primary_renders() {
        let html = Button::primary("Save").render();
        assert!(html.contains("au-btn"));
        assert!(html.contains("au-btn--primary"));
        assert!(html.contains("Save"));
        assert!(html.contains("type=\"button\""));
    }

    #[test]
    fn button_disabled_has_aria() {
        let html = Button::secondary("Cancel").disabled().render();
        assert!(html.contains("disabled"));
        assert!(html.contains("aria-disabled=\"true\""));
    }

    #[test]
    fn button_loading_state() {
        let html = Button::primary("Submit").loading().render();
        assert!(html.contains("au-btn--loading"));
    }

    #[test]
    fn button_small_size() {
        let html = Button::primary("OK").small().render();
        assert!(html.contains("au-btn--sm"));
    }

    #[test]
    fn button_large_size() {
        let html = Button::destructive("Delete").large().render();
        assert!(html.contains("au-btn--lg"));
        assert!(html.contains("au-btn--destructive"));
    }

    #[test]
    fn button_with_icon() {
        let html = Button::ghost("Edit").icon("<svg></svg>").render();
        assert!(html.contains("au-btn-icon"));
        assert!(html.contains("aria-hidden=\"true\""));
        assert!(html.contains("<svg></svg>"));
    }

    #[test]
    fn input_text_renders() {
        let html = Input::text("username").placeholder("Enter name").render();
        assert!(html.contains("au-input"));
        assert!(html.contains("type=\"text\""));
        assert!(html.contains("name=\"username\""));
        assert!(html.contains("placeholder=\"Enter name\""));
    }

    #[test]
    fn input_error_has_aria() {
        let html = Input::email("email").error().render();
        assert!(html.contains("au-input--error"));
        assert!(html.contains("aria-invalid=\"true\""));
    }

    #[test]
    fn input_required_has_aria() {
        let html = Input::text("name").required().render();
        assert!(html.contains("required"));
        assert!(html.contains("aria-required=\"true\""));
    }

    #[test]
    fn input_disabled_has_aria() {
        let html = Input::text("name").disabled().render();
        assert!(html.contains("disabled"));
        assert!(html.contains("aria-disabled=\"true\""));
    }

    #[test]
    fn toggle_renders_checkbox() {
        let html = Toggle::new("dark_mode").label("Dark Mode").render();
        assert!(html.contains("au-toggle"));
        assert!(html.contains("type=\"checkbox\""));
        assert!(html.contains("role=\"switch\""));
        assert!(html.contains("Dark Mode"));
    }

    #[test]
    fn toggle_checked() {
        let html = Toggle::new("notify").checked().render();
        assert!(html.contains("checked"));
    }

    #[test]
    fn card_elevated_renders() {
        let html = Card::elevated("Content here").header("Title").render();
        assert!(html.contains("au-card--elevated"));
        assert!(html.contains("au-card__header"));
        assert!(html.contains("au-card__body"));
        assert!(html.contains("Content here"));
    }

    #[test]
    fn card_flat_with_footer() {
        let html = Card::flat("Body").footer("Footer text").render();
        assert!(html.contains("au-card--flat"));
        assert!(html.contains("au-card__footer"));
    }

    #[test]
    fn alert_error_has_role() {
        let html = Alert::error("Something failed").title("Error").render();
        assert!(html.contains("role=\"alert\""));
        assert!(html.contains("au-alert--error"));
        assert!(html.contains("aria-live=\"assertive\""));
        assert!(html.contains("au-alert__title"));
    }

    #[test]
    fn alert_info_polite() {
        let html = Alert::info("FYI").render();
        assert!(html.contains("aria-live=\"polite\""));
    }

    #[test]
    fn alert_dismissible() {
        let html = Alert::warning("Watch out").dismissible().render();
        assert!(html.contains("au-alert__dismiss"));
        assert!(html.contains("aria-label=\"Dismiss\""));
    }

    #[test]
    fn badge_renders() {
        let html = Badge::new("Active", BadgeVariant::Success).render();
        assert!(html.contains("au-badge"));
        assert!(html.contains("au-badge--success"));
        assert!(html.contains("Active"));
    }

    #[test]
    fn avatar_initials_renders() {
        let html = Avatar::initials("JD", "John Doe").status("online").render();
        assert!(html.contains("au-avatar"));
        assert!(html.contains("au-avatar__initials"));
        assert!(html.contains("role=\"img\""));
        assert!(html.contains("aria-label=\"John Doe\""));
        assert!(html.contains("au-avatar__status--online"));
    }

    #[test]
    fn avatar_image_renders() {
        let html = Avatar::image("/photo.jpg", "Jane Doe").size("au-avatar--lg").render();
        assert!(html.contains("au-avatar--lg"));
        assert!(html.contains("src=\"/photo.jpg\""));
        assert!(html.contains("alt=\"Jane Doe\""));
    }

    #[test]
    fn progress_renders() {
        let html = Progress::new(75).label("Upload progress").render();
        assert!(html.contains("role=\"progressbar\""));
        assert!(html.contains("aria-valuenow=\"75\""));
        assert!(html.contains("aria-valuemin=\"0\""));
        assert!(html.contains("aria-valuemax=\"100\""));
        assert!(html.contains("width: 75%"));
    }

    #[test]
    fn progress_clamps_at_100() {
        let p = Progress::new(150);
        assert_eq!(p.value, 100);
    }

    #[test]
    fn spinner_with_label() {
        let html = Spinner::render(Some("Loading data"));
        assert!(html.contains("au-spinner"));
        assert!(html.contains("role=\"status\""));
        assert!(html.contains("au-sr-only"));
        assert!(html.contains("Loading data"));
    }

    #[test]
    fn spinner_without_label() {
        let html = Spinner::render(None);
        assert!(html.contains("au-spinner"));
        assert!(html.contains("Loading"));
    }

    #[test]
    fn form_group_renders() {
        let input = Input::text("name").placeholder("Name").render();
        let html = FormGroup::new("Full Name", &input).required().render();
        assert!(html.contains("au-form-group"));
        assert!(html.contains("au-form-group__label"));
        assert!(html.contains("au-form-group__required"));
        assert!(html.contains("au-input"));
    }

    #[test]
    fn form_group_with_error() {
        let input = Input::email("email").error().render();
        let html = FormGroup::new("Email", &input)
            .error("Invalid email address")
            .render();
        assert!(html.contains("au-form-group__error"));
        assert!(html.contains("role=\"alert\""));
        assert!(html.contains("Invalid email address"));
    }

    #[test]
    fn form_group_with_help() {
        let input = Input::text("bio").render();
        let html = FormGroup::new("Bio", &input)
            .help("Keep it brief")
            .render();
        assert!(html.contains("au-form-group__help"));
        assert!(html.contains("Keep it brief"));
    }

    #[test]
    fn breadcrumb_renders() {
        let html = Breadcrumb::render(&[
            ("Home", Some("/")),
            ("Products", Some("/products")),
            ("Widget", None),
        ]);
        assert!(html.contains("au-breadcrumb"));
        assert!(html.contains("aria-label=\"Breadcrumb\""));
        assert!(html.contains("aria-current=\"page\""));
        assert!(html.contains("href=\"/\""));
        assert!(html.contains("href=\"/products\""));
        assert!(html.contains("Widget"));
    }

    #[test]
    fn breadcrumb_empty() {
        let html = Breadcrumb::render(&[]);
        assert!(html.is_empty());
    }

    #[test]
    fn html_escape_works() {
        assert_eq!(html_escape("<script>"), "&lt;script&gt;");
        assert_eq!(html_escape("a&b"), "a&amp;b");
        assert_eq!(html_escape("x\"y"), "x&quot;y");
    }
}
