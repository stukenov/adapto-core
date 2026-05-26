//! # Component HTML Helpers
//!
//! Typed builders that emit accessible HTML strings for Adapto UI components.
//! Each builder ensures correct class names, ARIA attributes, and semantic markup.
//!
//! All components support `.id()`, `.class()`, and `.attr()` for customization.

use crate::html_escape;

// ---------------------------------------------------------------------------
// Common: extra attributes rendering
// ---------------------------------------------------------------------------

fn render_extra(
    id: &Option<String>,
    classes: &[String],
    attrs: &[(String, String)],
) -> String {
    let mut parts = Vec::new();
    if let Some(ref id) = id {
        parts.push(format!("id=\"{}\"", html_escape(id)));
    }
    for (k, v) in attrs {
        parts.push(format!("{}=\"{}\"", html_escape(k), html_escape(v)));
    }
    let _ = classes; // classes handled inline in each component
    if parts.is_empty() {
        String::new()
    } else {
        format!(" {}", parts.join(" "))
    }
}

// ---------------------------------------------------------------------------
// Button
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonVariant {
    Primary,
    Secondary,
    Destructive,
    Ghost,
    Outline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonSize {
    Small,
    Default,
    Large,
}

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

#[derive(Debug, Clone)]
pub struct Button {
    label: String,
    variant: ButtonVariant,
    size: ButtonSize,
    disabled: bool,
    loading: bool,
    icon_html: Option<String>,
    r#type: ButtonType,
    href: Option<String>,
    id: Option<String>,
    extra_classes: Vec<String>,
    extra_attrs: Vec<(String, String)>,
}

impl Button {
    pub fn primary(label: &str) -> Self { Self::new(label, ButtonVariant::Primary) }
    pub fn secondary(label: &str) -> Self { Self::new(label, ButtonVariant::Secondary) }
    pub fn destructive(label: &str) -> Self { Self::new(label, ButtonVariant::Destructive) }
    pub fn ghost(label: &str) -> Self { Self::new(label, ButtonVariant::Ghost) }
    pub fn outline(label: &str) -> Self { Self::new(label, ButtonVariant::Outline) }

    fn new(label: &str, variant: ButtonVariant) -> Self {
        Self {
            label: label.to_string(),
            variant,
            size: ButtonSize::Default,
            disabled: false,
            loading: false,
            icon_html: None,
            r#type: ButtonType::Button,
            href: None,
            id: None,
            extra_classes: Vec::new(),
            extra_attrs: Vec::new(),
        }
    }

    pub fn small(mut self) -> Self { self.size = ButtonSize::Small; self }
    pub fn large(mut self) -> Self { self.size = ButtonSize::Large; self }
    pub fn disabled(mut self) -> Self { self.disabled = true; self }
    pub fn loading(mut self) -> Self { self.loading = true; self }
    pub fn button_type(mut self, t: ButtonType) -> Self { self.r#type = t; self }
    pub fn icon(mut self, html: &str) -> Self { self.icon_html = Some(html.to_string()); self }

    /// Render as `<a>` instead of `<button>`. Visually identical, semantically a link.
    pub fn href(mut self, url: &str) -> Self { self.href = Some(url.to_string()); self }

    /// Set `data-action` for live.js event handling.
    pub fn action(mut self, name: &str) -> Self {
        self.extra_attrs.push(("data-action".to_string(), name.to_string()));
        self
    }

    /// Set `data-id` for live.js payload.
    pub fn data_id(mut self, id: &str) -> Self {
        self.extra_attrs.push(("data-id".to_string(), id.to_string()));
        self
    }

    pub fn id(mut self, id: &str) -> Self { self.id = Some(id.to_string()); self }
    pub fn class(mut self, cls: &str) -> Self { self.extra_classes.push(cls.to_string()); self }
    pub fn attr(mut self, key: &str, value: &str) -> Self {
        self.extra_attrs.push((key.to_string(), value.to_string())); self
    }

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
        if self.loading { classes.push("au-btn--loading".into()); }
        classes.extend(self.extra_classes.iter().cloned());

        let icon_part = match &self.icon_html {
            Some(svg) => format!("<span class=\"au-btn-icon\" aria-hidden=\"true\">{}</span>", svg),
            None => String::new(),
        };

        let extra = render_extra(&self.id, &self.extra_classes, &self.extra_attrs);

        if let Some(ref url) = self.href {
            return format!(
                "<a href=\"{}\" class=\"{}\"{}>{}{}</a>",
                html_escape(url),
                classes.join(" "),
                extra,
                icon_part,
                html_escape(&self.label),
            );
        }

        let disabled_attr = if self.disabled {
            " disabled aria-disabled=\"true\""
        } else {
            ""
        };

        format!(
            "<button type=\"{}\" class=\"{}\"{}{}>{}{}</button>",
            self.r#type.as_str(),
            classes.join(" "),
            disabled_attr,
            extra,
            icon_part,
            html_escape(&self.label),
        )
    }
}

// ---------------------------------------------------------------------------
// Input
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputType {
    Text, Email, Password, Number, Search, Tel, Url, Date, Hidden,
}

impl InputType {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Text => "text", Self::Email => "email", Self::Password => "password",
            Self::Number => "number", Self::Search => "search", Self::Tel => "tel",
            Self::Url => "url", Self::Date => "date", Self::Hidden => "hidden",
        }
    }
}

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
    id: Option<String>,
    extra_classes: Vec<String>,
    extra_attrs: Vec<(String, String)>,
}

impl Input {
    pub fn text(name: &str) -> Self { Self::new(name, InputType::Text) }
    pub fn email(name: &str) -> Self { Self::new(name, InputType::Email) }
    pub fn password(name: &str) -> Self { Self::new(name, InputType::Password) }
    pub fn number(name: &str) -> Self { Self::new(name, InputType::Number) }
    pub fn search(name: &str) -> Self { Self::new(name, InputType::Search) }
    pub fn tel(name: &str) -> Self { Self::new(name, InputType::Tel) }
    pub fn date(name: &str) -> Self { Self::new(name, InputType::Date) }
    pub fn hidden(name: &str) -> Self { Self::new(name, InputType::Hidden) }

    fn new(name: &str, input_type: InputType) -> Self {
        Self {
            input_type, name: name.to_string(),
            placeholder: None, value: None, error: false,
            disabled: false, required: false, label: None,
            id: None, extra_classes: Vec::new(), extra_attrs: Vec::new(),
        }
    }

    pub fn placeholder(mut self, text: &str) -> Self { self.placeholder = Some(text.to_string()); self }
    pub fn value(mut self, val: &str) -> Self { self.value = Some(val.to_string()); self }
    pub fn error(mut self) -> Self { self.error = true; self }
    pub fn disabled(mut self) -> Self { self.disabled = true; self }
    pub fn required(mut self) -> Self { self.required = true; self }
    pub fn label(mut self, label: &str) -> Self { self.label = Some(label.to_string()); self }
    pub fn id(mut self, id: &str) -> Self { self.id = Some(id.to_string()); self }
    pub fn class(mut self, cls: &str) -> Self { self.extra_classes.push(cls.to_string()); self }
    pub fn attr(mut self, key: &str, value: &str) -> Self {
        self.extra_attrs.push((key.to_string(), value.to_string())); self
    }

    pub fn render(&self) -> String {
        let mut classes = vec!["au-input".to_string()];
        if self.error { classes.push("au-input--error".into()); }
        classes.extend(self.extra_classes.iter().cloned());

        let mut attrs = vec![
            format!("type=\"{}\"", self.input_type.as_str()),
            format!("name=\"{}\"", html_escape(&self.name)),
            format!("class=\"{}\"", classes.join(" ")),
        ];
        if let Some(ref id) = self.id { attrs.push(format!("id=\"{}\"", html_escape(id))); }
        if let Some(ref ph) = self.placeholder { attrs.push(format!("placeholder=\"{}\"", html_escape(ph))); }
        if let Some(ref val) = self.value { attrs.push(format!("value=\"{}\"", html_escape(val))); }
        if let Some(ref lbl) = self.label { attrs.push(format!("aria-label=\"{}\"", html_escape(lbl))); }
        if self.disabled { attrs.push("disabled".into()); attrs.push("aria-disabled=\"true\"".into()); }
        if self.required { attrs.push("required".into()); attrs.push("aria-required=\"true\"".into()); }
        if self.error { attrs.push("aria-invalid=\"true\"".into()); }
        for (k, v) in &self.extra_attrs {
            attrs.push(format!("{}=\"{}\"", html_escape(k), html_escape(v)));
        }

        format!("<input {} />", attrs.join(" "))
    }
}

// ---------------------------------------------------------------------------
// Textarea (NEW)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Textarea {
    name: String,
    placeholder: Option<String>,
    value: Option<String>,
    rows: u32,
    error: bool,
    disabled: bool,
    required: bool,
    id: Option<String>,
    extra_classes: Vec<String>,
    extra_attrs: Vec<(String, String)>,
}

impl Textarea {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(), placeholder: None, value: None,
            rows: 4, error: false, disabled: false, required: false,
            id: None, extra_classes: Vec::new(), extra_attrs: Vec::new(),
        }
    }

    pub fn placeholder(mut self, text: &str) -> Self { self.placeholder = Some(text.to_string()); self }
    pub fn value(mut self, val: &str) -> Self { self.value = Some(val.to_string()); self }
    pub fn rows(mut self, n: u32) -> Self { self.rows = n; self }
    pub fn error(mut self) -> Self { self.error = true; self }
    pub fn disabled(mut self) -> Self { self.disabled = true; self }
    pub fn required(mut self) -> Self { self.required = true; self }
    pub fn id(mut self, id: &str) -> Self { self.id = Some(id.to_string()); self }
    pub fn class(mut self, cls: &str) -> Self { self.extra_classes.push(cls.to_string()); self }
    pub fn attr(mut self, key: &str, value: &str) -> Self {
        self.extra_attrs.push((key.to_string(), value.to_string())); self
    }

    pub fn render(&self) -> String {
        let mut classes = vec!["au-textarea".to_string()];
        if self.error { classes.push("au-textarea--error".into()); }
        classes.extend(self.extra_classes.iter().cloned());

        let mut attrs = vec![
            format!("name=\"{}\"", html_escape(&self.name)),
            format!("class=\"{}\"", classes.join(" ")),
            format!("rows=\"{}\"", self.rows),
        ];
        if let Some(ref id) = self.id { attrs.push(format!("id=\"{}\"", html_escape(id))); }
        if let Some(ref ph) = self.placeholder { attrs.push(format!("placeholder=\"{}\"", html_escape(ph))); }
        if self.disabled { attrs.push("disabled".into()); attrs.push("aria-disabled=\"true\"".into()); }
        if self.required { attrs.push("required".into()); attrs.push("aria-required=\"true\"".into()); }
        if self.error { attrs.push("aria-invalid=\"true\"".into()); }
        for (k, v) in &self.extra_attrs {
            attrs.push(format!("{}=\"{}\"", html_escape(k), html_escape(v)));
        }

        let content = self.value.as_deref().map(html_escape).unwrap_or_default();
        format!("<textarea {}>{}</textarea>", attrs.join(" "), content)
    }
}

// ---------------------------------------------------------------------------
// Select (NEW)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Select {
    name: String,
    options: Vec<(String, String)>,
    selected: Option<String>,
    placeholder: Option<String>,
    error: bool,
    disabled: bool,
    required: bool,
    id: Option<String>,
    extra_classes: Vec<String>,
    extra_attrs: Vec<(String, String)>,
}

impl Select {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(), options: Vec::new(), selected: None,
            placeholder: None, error: false, disabled: false, required: false,
            id: None, extra_classes: Vec::new(), extra_attrs: Vec::new(),
        }
    }

    /// Add an option (value, label).
    pub fn option(mut self, value: &str, label: &str) -> Self {
        self.options.push((value.to_string(), label.to_string())); self
    }

    /// Add multiple options from a slice of (value, label).
    pub fn options(mut self, opts: &[(&str, &str)]) -> Self {
        for (v, l) in opts { self.options.push((v.to_string(), l.to_string())); }
        self
    }

    pub fn selected(mut self, value: &str) -> Self { self.selected = Some(value.to_string()); self }
    pub fn placeholder(mut self, text: &str) -> Self { self.placeholder = Some(text.to_string()); self }
    pub fn error(mut self) -> Self { self.error = true; self }
    pub fn disabled(mut self) -> Self { self.disabled = true; self }
    pub fn required(mut self) -> Self { self.required = true; self }
    pub fn id(mut self, id: &str) -> Self { self.id = Some(id.to_string()); self }
    pub fn class(mut self, cls: &str) -> Self { self.extra_classes.push(cls.to_string()); self }
    pub fn attr(mut self, key: &str, value: &str) -> Self {
        self.extra_attrs.push((key.to_string(), value.to_string())); self
    }

    pub fn render(&self) -> String {
        let mut classes = vec!["au-select".to_string()];
        if self.error { classes.push("au-select--error".into()); }
        classes.extend(self.extra_classes.iter().cloned());

        let mut attrs = vec![
            format!("name=\"{}\"", html_escape(&self.name)),
            format!("class=\"{}\"", classes.join(" ")),
        ];
        if let Some(ref id) = self.id { attrs.push(format!("id=\"{}\"", html_escape(id))); }
        if self.disabled { attrs.push("disabled".into()); attrs.push("aria-disabled=\"true\"".into()); }
        if self.required { attrs.push("required".into()); attrs.push("aria-required=\"true\"".into()); }
        if self.error { attrs.push("aria-invalid=\"true\"".into()); }
        for (k, v) in &self.extra_attrs {
            attrs.push(format!("{}=\"{}\"", html_escape(k), html_escape(v)));
        }

        let mut options_html = String::new();
        if let Some(ref ph) = self.placeholder {
            options_html.push_str(&format!(
                "<option value=\"\" disabled selected>{}</option>",
                html_escape(ph)
            ));
        }
        for (value, label) in &self.options {
            let sel = if self.selected.as_deref() == Some(value) { " selected" } else { "" };
            options_html.push_str(&format!(
                "<option value=\"{}\"{}>{}</option>",
                html_escape(value), sel, html_escape(label)
            ));
        }

        format!("<select {}>{}</select>", attrs.join(" "), options_html)
    }
}

// ---------------------------------------------------------------------------
// Toggle
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Toggle {
    name: String,
    checked: bool,
    disabled: bool,
    label: Option<String>,
    id: Option<String>,
    extra_classes: Vec<String>,
    extra_attrs: Vec<(String, String)>,
}

impl Toggle {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(), checked: false, disabled: false,
            label: None, id: None, extra_classes: Vec::new(), extra_attrs: Vec::new(),
        }
    }

    pub fn checked(mut self) -> Self { self.checked = true; self }
    pub fn disabled(mut self) -> Self { self.disabled = true; self }
    pub fn label(mut self, text: &str) -> Self { self.label = Some(text.to_string()); self }
    pub fn id(mut self, id: &str) -> Self { self.id = Some(id.to_string()); self }
    pub fn class(mut self, cls: &str) -> Self { self.extra_classes.push(cls.to_string()); self }
    pub fn attr(mut self, key: &str, value: &str) -> Self {
        self.extra_attrs.push((key.to_string(), value.to_string())); self
    }

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

        let mut classes = vec!["au-toggle".to_string()];
        classes.extend(self.extra_classes.iter().cloned());
        let extra = render_extra(&self.id, &self.extra_classes, &self.extra_attrs);

        format!(
            "<label class=\"{}\"{}>\
                <input type=\"checkbox\" class=\"au-toggle__input\" name=\"{}\" role=\"switch\"{}{}{} />\
                <span class=\"au-toggle__track\">\
                    <span class=\"au-toggle__thumb\"></span>\
                </span>\
                {}\
            </label>",
            classes.join(" "),
            extra,
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
    id: Option<String>,
    extra_classes: Vec<String>,
    extra_attrs: Vec<(String, String)>,
}

impl Card {
    pub fn elevated(body: &str) -> Self { Self::new(body, CardVariant::Elevated) }
    pub fn flat(body: &str) -> Self { Self::new(body, CardVariant::Flat) }

    fn new(body: &str, variant: CardVariant) -> Self {
        Self {
            variant, header: None, body: body.to_string(), footer: None,
            hoverable: false, id: None, extra_classes: Vec::new(), extra_attrs: Vec::new(),
        }
    }

    pub fn header(mut self, text: &str) -> Self { self.header = Some(text.to_string()); self }
    pub fn footer(mut self, text: &str) -> Self { self.footer = Some(text.to_string()); self }
    pub fn hoverable(mut self) -> Self { self.hoverable = true; self }
    pub fn id(mut self, id: &str) -> Self { self.id = Some(id.to_string()); self }
    pub fn class(mut self, cls: &str) -> Self { self.extra_classes.push(cls.to_string()); self }
    pub fn attr(mut self, key: &str, value: &str) -> Self {
        self.extra_attrs.push((key.to_string(), value.to_string())); self
    }

    pub fn render(&self) -> String {
        let mut classes = vec!["au-card".to_string()];
        match self.variant {
            CardVariant::Elevated => classes.push("au-card--elevated".into()),
            CardVariant::Flat => classes.push("au-card--flat".into()),
            CardVariant::Grouped => classes.push("au-card--grouped".into()),
        }
        if self.hoverable { classes.push("au-card--hoverable".into()); }
        classes.extend(self.extra_classes.iter().cloned());

        let header_html = match &self.header {
            Some(h) => format!("<div class=\"au-card__header\">{}</div>", h),
            None => String::new(),
        };
        let footer_html = match &self.footer {
            Some(f) => format!("<div class=\"au-card__footer\">{}</div>", f),
            None => String::new(),
        };
        let extra = render_extra(&self.id, &self.extra_classes, &self.extra_attrs);

        format!(
            "<div class=\"{}\"{}>{}<div class=\"au-card__body\">{}</div>{}</div>",
            classes.join(" "), extra, header_html, self.body, footer_html,
        )
    }
}

// ---------------------------------------------------------------------------
// Alert
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertLevel { Info, Success, Warning, Error }

#[derive(Debug, Clone)]
pub struct Alert {
    level: AlertLevel,
    title: Option<String>,
    message: String,
    dismissible: bool,
    id: Option<String>,
    extra_classes: Vec<String>,
    extra_attrs: Vec<(String, String)>,
}

impl Alert {
    pub fn info(message: &str) -> Self { Self::new(message, AlertLevel::Info) }
    pub fn success(message: &str) -> Self { Self::new(message, AlertLevel::Success) }
    pub fn warning(message: &str) -> Self { Self::new(message, AlertLevel::Warning) }
    pub fn error(message: &str) -> Self { Self::new(message, AlertLevel::Error) }

    fn new(message: &str, level: AlertLevel) -> Self {
        Self {
            level, title: None, message: message.to_string(), dismissible: false,
            id: None, extra_classes: Vec::new(), extra_attrs: Vec::new(),
        }
    }

    pub fn title(mut self, text: &str) -> Self { self.title = Some(text.to_string()); self }
    pub fn dismissible(mut self) -> Self { self.dismissible = true; self }
    pub fn id(mut self, id: &str) -> Self { self.id = Some(id.to_string()); self }
    pub fn class(mut self, cls: &str) -> Self { self.extra_classes.push(cls.to_string()); self }
    pub fn attr(mut self, key: &str, value: &str) -> Self {
        self.extra_attrs.push((key.to_string(), value.to_string())); self
    }

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

        let mut classes = vec!["au-alert".to_string(), variant_class.to_string()];
        classes.extend(self.extra_classes.iter().cloned());

        let title_html = match &self.title {
            Some(t) => format!("<div class=\"au-alert__title\">{}</div>", html_escape(t)),
            None => String::new(),
        };

        let dismiss_html = if self.dismissible {
            "<button class=\"au-alert__dismiss\" aria-label=\"Dismiss\" \
                onclick=\"this.closest('.au-alert').remove()\">\
                <svg width=\"14\" height=\"14\" viewBox=\"0 0 14 14\" fill=\"none\" aria-hidden=\"true\">\
                    <path d=\"M1 1l12 12M13 1L1 13\" stroke=\"currentColor\" stroke-width=\"1.5\" stroke-linecap=\"round\"/>\
                </svg>\
            </button>".to_string()
        } else {
            String::new()
        };

        let extra = render_extra(&self.id, &self.extra_classes, &self.extra_attrs);

        format!(
            "<div class=\"{}\" role=\"alert\" aria-live=\"{}\"{}>\
                <div class=\"au-alert__content\">{}<div class=\"au-alert__message\">{}</div></div>\
                {}\
            </div>",
            classes.join(" "), aria_live, extra, title_html,
            html_escape(&self.message), dismiss_html,
        )
    }
}

// ---------------------------------------------------------------------------
// Badge
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BadgeVariant { Default, Info, Success, Warning, Error }

#[derive(Debug, Clone)]
pub struct Badge {
    label: String,
    variant: BadgeVariant,
    id: Option<String>,
    extra_classes: Vec<String>,
    extra_attrs: Vec<(String, String)>,
}

impl Badge {
    pub fn new(label: &str, variant: BadgeVariant) -> Self {
        Self {
            label: label.to_string(), variant,
            id: None, extra_classes: Vec::new(), extra_attrs: Vec::new(),
        }
    }

    pub fn id(mut self, id: &str) -> Self { self.id = Some(id.to_string()); self }
    pub fn class(mut self, cls: &str) -> Self { self.extra_classes.push(cls.to_string()); self }
    pub fn attr(mut self, key: &str, value: &str) -> Self {
        self.extra_attrs.push((key.to_string(), value.to_string())); self
    }

    pub fn render(&self) -> String {
        let variant_class = match self.variant {
            BadgeVariant::Default => "au-badge--default",
            BadgeVariant::Info => "au-badge--info",
            BadgeVariant::Success => "au-badge--success",
            BadgeVariant::Warning => "au-badge--warning",
            BadgeVariant::Error => "au-badge--error",
        };
        let mut classes = vec!["au-badge".to_string(), variant_class.to_string()];
        classes.extend(self.extra_classes.iter().cloned());
        let extra = render_extra(&self.id, &self.extra_classes, &self.extra_attrs);

        format!("<span class=\"{}\"{}>{}</span>", classes.join(" "), extra, html_escape(&self.label))
    }
}

// ---------------------------------------------------------------------------
// Avatar
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AvatarSize { Small, Default, Large }

#[derive(Debug, Clone)]
pub struct Avatar {
    size: AvatarSize,
    initials: Option<String>,
    image_src: Option<String>,
    alt: String,
    status: Option<String>,
    id: Option<String>,
    extra_classes: Vec<String>,
    extra_attrs: Vec<(String, String)>,
}

impl Avatar {
    pub fn initials(initials: &str, alt: &str) -> Self {
        Self {
            size: AvatarSize::Default, initials: Some(initials.to_string()),
            image_src: None, alt: alt.to_string(), status: None,
            id: None, extra_classes: Vec::new(), extra_attrs: Vec::new(),
        }
    }

    pub fn image(src: &str, alt: &str) -> Self {
        Self {
            size: AvatarSize::Default, initials: None,
            image_src: Some(src.to_string()), alt: alt.to_string(), status: None,
            id: None, extra_classes: Vec::new(), extra_attrs: Vec::new(),
        }
    }

    pub fn small(mut self) -> Self { self.size = AvatarSize::Small; self }
    pub fn large(mut self) -> Self { self.size = AvatarSize::Large; self }

    /// Set size class (deprecated — use `.small()` or `.large()` instead).
    pub fn size(mut self, class: &str) -> Self {
        match class {
            "au-avatar--sm" => self.size = AvatarSize::Small,
            "au-avatar--lg" => self.size = AvatarSize::Large,
            _ => self.extra_classes.push(class.to_string()),
        }
        self
    }

    pub fn status(mut self, status: &str) -> Self { self.status = Some(status.to_string()); self }
    pub fn id(mut self, id: &str) -> Self { self.id = Some(id.to_string()); self }
    pub fn class(mut self, cls: &str) -> Self { self.extra_classes.push(cls.to_string()); self }
    pub fn attr(mut self, key: &str, value: &str) -> Self {
        self.extra_attrs.push((key.to_string(), value.to_string())); self
    }

    pub fn render(&self) -> String {
        let mut classes = vec!["au-avatar".to_string()];
        match self.size {
            AvatarSize::Small => classes.push("au-avatar--sm".into()),
            AvatarSize::Default => {}
            AvatarSize::Large => classes.push("au-avatar--lg".into()),
        }
        classes.extend(self.extra_classes.iter().cloned());

        let content = if let Some(ref src) = self.image_src {
            format!("<img class=\"au-avatar__image\" src=\"{}\" alt=\"{}\" />",
                html_escape(src), html_escape(&self.alt))
        } else if let Some(ref ini) = self.initials {
            format!("<span class=\"au-avatar__initials\" aria-hidden=\"true\">{}</span>",
                html_escape(ini))
        } else {
            String::new()
        };

        let status_html = match &self.status {
            Some(s) => format!(
                "<span class=\"au-avatar__status au-avatar__status--{}\" aria-label=\"{}\"></span>",
                html_escape(s), html_escape(s)),
            None => String::new(),
        };

        let extra = render_extra(&self.id, &self.extra_classes, &self.extra_attrs);

        format!(
            "<div class=\"{}\" role=\"img\" aria-label=\"{}\"{}>{}{}</div>",
            classes.join(" "), html_escape(&self.alt), extra, content, status_html,
        )
    }
}

// ---------------------------------------------------------------------------
// Progress
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Progress {
    pub value: u8,
    max: u8,
    label: Option<String>,
    indeterminate: bool,
    id: Option<String>,
    extra_classes: Vec<String>,
    extra_attrs: Vec<(String, String)>,
}

impl Progress {
    pub fn new(value: u8) -> Self {
        Self {
            value: value.min(100), max: 100, label: None, indeterminate: false,
            id: None, extra_classes: Vec::new(), extra_attrs: Vec::new(),
        }
    }

    /// Create an indeterminate progress bar (unknown duration).
    pub fn indeterminate() -> Self {
        Self {
            value: 0, max: 100, label: None, indeterminate: true,
            id: None, extra_classes: Vec::new(), extra_attrs: Vec::new(),
        }
    }

    pub fn label(mut self, text: &str) -> Self { self.label = Some(text.to_string()); self }
    pub fn id(mut self, id: &str) -> Self { self.id = Some(id.to_string()); self }
    pub fn class(mut self, cls: &str) -> Self { self.extra_classes.push(cls.to_string()); self }
    pub fn attr(mut self, key: &str, value: &str) -> Self {
        self.extra_attrs.push((key.to_string(), value.to_string())); self
    }

    pub fn render(&self) -> String {
        let aria_label = match &self.label {
            Some(l) => format!(" aria-label=\"{}\"", html_escape(l)),
            None => String::new(),
        };

        let mut classes = vec!["au-progress".to_string()];
        if self.indeterminate { classes.push("au-progress--indeterminate".into()); }
        classes.extend(self.extra_classes.iter().cloned());
        let extra = render_extra(&self.id, &self.extra_classes, &self.extra_attrs);

        if self.indeterminate {
            return format!(
                "<div class=\"{}\" role=\"progressbar\"{}{}>\
                    <div class=\"au-progress__bar\"></div>\
                </div>",
                classes.join(" "), aria_label, extra,
            );
        }

        format!(
            "<div class=\"{}\" role=\"progressbar\" \
                aria-valuenow=\"{}\" aria-valuemin=\"0\" aria-valuemax=\"{}\"{}{}>\
                <div class=\"au-progress__bar\" style=\"width: {}%\"></div>\
            </div>",
            classes.join(" "), self.value, self.max, aria_label, extra, self.value,
        )
    }
}

// ---------------------------------------------------------------------------
// Spinner
// ---------------------------------------------------------------------------

pub struct Spinner;

impl Spinner {
    pub fn render(label: Option<&str>) -> String {
        match label {
            Some(l) => format!(
                "<div class=\"au-spinner-container\">\
                    <div class=\"au-spinner\" role=\"status\" aria-label=\"{}\">\
                        <span class=\"au-sr-only\">{}</span>\
                    </div>\
                    <span class=\"au-spinner-container__label\">{}</span>\
                </div>",
                html_escape(l), html_escape(l), html_escape(l),
            ),
            None => "<div class=\"au-spinner\" role=\"status\" aria-label=\"Loading\">\
                        <span class=\"au-sr-only\">Loading</span>\
                    </div>".to_string(),
        }
    }
}

// ---------------------------------------------------------------------------
// FormGroup (enhanced with label↔input association)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct FormGroup {
    label: String,
    input_html: String,
    error: Option<String>,
    help: Option<String>,
    required: bool,
    input_id: Option<String>,
    id: Option<String>,
    extra_classes: Vec<String>,
    extra_attrs: Vec<(String, String)>,
}

impl FormGroup {
    /// Create a form group wrapping a rendered input.
    pub fn new(label: &str, input_html: &str) -> Self {
        Self {
            label: label.to_string(), input_html: input_html.to_string(),
            error: None, help: None, required: false, input_id: None,
            id: None, extra_classes: Vec::new(), extra_attrs: Vec::new(),
        }
    }

    /// Set the input's `id` to link `<label for>` and `aria-describedby`.
    pub fn input_id(mut self, id: &str) -> Self { self.input_id = Some(id.to_string()); self }
    pub fn error(mut self, msg: &str) -> Self { self.error = Some(msg.to_string()); self }
    pub fn help(mut self, text: &str) -> Self { self.help = Some(text.to_string()); self }
    pub fn required(mut self) -> Self { self.required = true; self }
    pub fn id(mut self, id: &str) -> Self { self.id = Some(id.to_string()); self }
    pub fn class(mut self, cls: &str) -> Self { self.extra_classes.push(cls.to_string()); self }
    pub fn attr(mut self, key: &str, value: &str) -> Self {
        self.extra_attrs.push((key.to_string(), value.to_string())); self
    }

    pub fn render(&self) -> String {
        let required_marker = if self.required {
            "<span class=\"au-form-group__required\" aria-hidden=\"true\">*</span>"
        } else {
            ""
        };

        let label_for = match &self.input_id {
            Some(id) => format!(" for=\"{}\"", html_escape(id)),
            None => String::new(),
        };

        let help_html = match &self.help {
            Some(h) => {
                let help_id = self.input_id.as_ref().map(|id| format!(" id=\"{}-help\"", html_escape(id))).unwrap_or_default();
                format!("<p class=\"au-form-group__help\"{}>{}</p>", help_id, html_escape(h))
            }
            None => String::new(),
        };

        let error_html = match &self.error {
            Some(e) => {
                let err_id = self.input_id.as_ref().map(|id| format!(" id=\"{}-error\"", html_escape(id))).unwrap_or_default();
                format!("<p class=\"au-form-group__error\" role=\"alert\"{}>{}</p>", err_id, html_escape(e))
            }
            None => String::new(),
        };

        let mut classes = vec!["au-form-group".to_string()];
        classes.extend(self.extra_classes.iter().cloned());
        let extra = render_extra(&self.id, &self.extra_classes, &self.extra_attrs);

        format!(
            "<div class=\"{}\"{}>\
                <label class=\"au-form-group__label\"{}>{}{}</label>\
                {}\
                {}\
                {}\
            </div>",
            classes.join(" "), extra, label_for,
            html_escape(&self.label), required_marker,
            self.input_html, help_html, error_html,
        )
    }
}

// ---------------------------------------------------------------------------
// Form (NEW)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Form {
    action: Option<String>,
    method: String,
    children: Vec<String>,
    id: Option<String>,
    extra_classes: Vec<String>,
    extra_attrs: Vec<(String, String)>,
}

impl Form {
    pub fn new() -> Self {
        Self {
            action: None, method: "POST".to_string(), children: Vec::new(),
            id: None, extra_classes: Vec::new(), extra_attrs: Vec::new(),
        }
    }

    pub fn action(mut self, url: &str) -> Self { self.action = Some(url.to_string()); self }
    pub fn method(mut self, m: &str) -> Self { self.method = m.to_uppercase(); self }
    pub fn child(mut self, html: &str) -> Self { self.children.push(html.to_string()); self }
    pub fn id(mut self, id: &str) -> Self { self.id = Some(id.to_string()); self }
    pub fn class(mut self, cls: &str) -> Self { self.extra_classes.push(cls.to_string()); self }
    pub fn attr(mut self, key: &str, value: &str) -> Self {
        self.extra_attrs.push((key.to_string(), value.to_string())); self
    }

    pub fn render(&self) -> String {
        let mut classes = vec!["au-form".to_string()];
        classes.extend(self.extra_classes.iter().cloned());

        let action_attr = match &self.action {
            Some(a) => format!(" action=\"{}\"", html_escape(a)),
            None => String::new(),
        };

        let extra = render_extra(&self.id, &self.extra_classes, &self.extra_attrs);

        format!(
            "<form class=\"{}\" method=\"{}\"{}{}>{}</form>",
            classes.join(" "), html_escape(&self.method), action_attr, extra,
            self.children.join(""),
        )
    }
}

// ---------------------------------------------------------------------------
// Table (NEW)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Table {
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    caption: Option<String>,
    striped: bool,
    hoverable: bool,
    compact: bool,
    id: Option<String>,
    extra_classes: Vec<String>,
    extra_attrs: Vec<(String, String)>,
}

impl Table {
    pub fn new(headers: &[&str]) -> Self {
        Self {
            headers: headers.iter().map(|s| s.to_string()).collect(),
            rows: Vec::new(), caption: None,
            striped: false, hoverable: false, compact: false,
            id: None, extra_classes: Vec::new(), extra_attrs: Vec::new(),
        }
    }

    /// Add a row. Each cell is raw HTML (escape user content).
    pub fn row(mut self, cells: &[&str]) -> Self {
        self.rows.push(cells.iter().map(|s| s.to_string()).collect());
        self
    }

    /// Add multiple rows.
    pub fn rows(mut self, rows: Vec<Vec<String>>) -> Self {
        self.rows.extend(rows);
        self
    }

    pub fn caption(mut self, text: &str) -> Self { self.caption = Some(text.to_string()); self }
    pub fn striped(mut self) -> Self { self.striped = true; self }
    pub fn hoverable(mut self) -> Self { self.hoverable = true; self }
    pub fn compact(mut self) -> Self { self.compact = true; self }
    pub fn id(mut self, id: &str) -> Self { self.id = Some(id.to_string()); self }
    pub fn class(mut self, cls: &str) -> Self { self.extra_classes.push(cls.to_string()); self }
    pub fn attr(mut self, key: &str, value: &str) -> Self {
        self.extra_attrs.push((key.to_string(), value.to_string())); self
    }

    pub fn render(&self) -> String {
        let mut classes = vec!["au-table".to_string()];
        if self.striped { classes.push("au-table--striped".into()); }
        if self.hoverable { classes.push("au-table--hoverable".into()); }
        if self.compact { classes.push("au-table--compact".into()); }
        classes.extend(self.extra_classes.iter().cloned());

        let caption_html = match &self.caption {
            Some(c) => format!("<caption>{}</caption>", html_escape(c)),
            None => String::new(),
        };

        let thead: String = self.headers.iter()
            .map(|h| format!("<th scope=\"col\">{}</th>", html_escape(h)))
            .collect();

        let tbody: String = self.rows.iter()
            .map(|row| {
                let cells: String = row.iter().map(|c| format!("<td>{}</td>", c)).collect();
                format!("<tr>{}</tr>", cells)
            })
            .collect();

        let extra = render_extra(&self.id, &self.extra_classes, &self.extra_attrs);

        format!(
            "<div class=\"au-table-container\"><table class=\"{}\"{}>{}\
            <thead><tr>{}</tr></thead>\
            <tbody>{}</tbody>\
            </table></div>",
            classes.join(" "), extra, caption_html, thead, tbody,
        )
    }
}

// ---------------------------------------------------------------------------
// Modal (NEW)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Modal {
    modal_id: String,
    title: Option<String>,
    body: String,
    footer: Option<String>,
    id: Option<String>,
    extra_classes: Vec<String>,
    extra_attrs: Vec<(String, String)>,
}

impl Modal {
    pub fn new(modal_id: &str, body: &str) -> Self {
        Self {
            modal_id: modal_id.to_string(), title: None, body: body.to_string(),
            footer: None, id: None, extra_classes: Vec::new(), extra_attrs: Vec::new(),
        }
    }

    pub fn title(mut self, text: &str) -> Self { self.title = Some(text.to_string()); self }
    pub fn footer(mut self, html: &str) -> Self { self.footer = Some(html.to_string()); self }
    pub fn id(mut self, id: &str) -> Self { self.id = Some(id.to_string()); self }
    pub fn class(mut self, cls: &str) -> Self { self.extra_classes.push(cls.to_string()); self }
    pub fn attr(mut self, key: &str, value: &str) -> Self {
        self.extra_attrs.push((key.to_string(), value.to_string())); self
    }

    pub fn render(&self) -> String {
        let mut classes = vec!["au-modal".to_string()];
        classes.extend(self.extra_classes.iter().cloned());

        let title_id = format!("{}-title", self.modal_id);
        let title_html = match &self.title {
            Some(t) => format!("<h2 class=\"au-modal__title\" id=\"{}\">{}</h2>",
                html_escape(&title_id), html_escape(t)),
            None => String::new(),
        };

        let footer_html = match &self.footer {
            Some(f) => format!("<div class=\"au-modal__footer\">{}</div>", f),
            None => String::new(),
        };

        let aria_labelledby = if self.title.is_some() {
            format!(" aria-labelledby=\"{}\"", html_escape(&title_id))
        } else {
            String::new()
        };

        let extra = render_extra(&self.id, &self.extra_classes, &self.extra_attrs);

        format!(
            "<div class=\"au-modal__backdrop\" data-modal=\"{}\" style=\"display:none\">\
                <div class=\"{}\" role=\"dialog\" aria-modal=\"true\"{}{}>\
                    <button class=\"au-modal__close\" aria-label=\"Close\" data-modal-close=\"{}\">\
                        <svg width=\"14\" height=\"14\" viewBox=\"0 0 14 14\" fill=\"none\" aria-hidden=\"true\">\
                            <path d=\"M1 1l12 12M13 1L1 13\" stroke=\"currentColor\" stroke-width=\"1.5\" stroke-linecap=\"round\"/>\
                        </svg>\
                    </button>\
                    {}\
                    <div class=\"au-modal__body\">{}</div>\
                    {}\
                </div>\
            </div>",
            html_escape(&self.modal_id),
            classes.join(" "), aria_labelledby, extra,
            html_escape(&self.modal_id),
            title_html, self.body, footer_html,
        )
    }
}

// ---------------------------------------------------------------------------
// Breadcrumb
// ---------------------------------------------------------------------------

pub struct Breadcrumb;

impl Breadcrumb {
    pub fn render(segments: &[(&str, Option<&str>)]) -> String {
        if segments.is_empty() { return String::new(); }

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
                    </span>", html_escape(label)));
            } else if let Some(url) = href {
                items.push(format!(
                    "<span class=\"au-breadcrumb__item\">\
                        <a class=\"au-breadcrumb__link\" href=\"{}\">{}</a>\
                    </span>", html_escape(url), html_escape(label)));
            } else {
                items.push(format!(
                    "<span class=\"au-breadcrumb__item\">\
                        <span class=\"au-breadcrumb__link\">{}</span>\
                    </span>", html_escape(label)));
            }
        }

        format!("<nav class=\"au-breadcrumb\" aria-label=\"Breadcrumb\">{}</nav>", items.join(""))
    }
}

// ---------------------------------------------------------------------------
// Pagination (NEW)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Pagination {
    current: u64,
    total: u64,
    base_url: String,
    param_name: String,
}

impl Pagination {
    pub fn new(current: u64, total: u64) -> Self {
        Self {
            current: current.max(1).min(total.max(1)),
            total: total.max(1),
            base_url: String::new(),
            param_name: "page".to_string(),
        }
    }

    pub fn base_url(mut self, url: &str) -> Self { self.base_url = url.to_string(); self }
    pub fn param_name(mut self, name: &str) -> Self { self.param_name = name.to_string(); self }

    fn page_url(&self, page: u64) -> String {
        let sep = if self.base_url.contains('?') { "&" } else { "?" };
        format!("{}{}{}={}", self.base_url, sep, html_escape(&self.param_name), page)
    }

    pub fn render(&self) -> String {
        let mut items = Vec::new();

        // Prev
        if self.current > 1 {
            items.push(format!(
                "<a class=\"au-pagination__item\" href=\"{}\" aria-label=\"Previous page\">&laquo;</a>",
                self.page_url(self.current - 1)));
        } else {
            items.push("<span class=\"au-pagination__item au-pagination__item--disabled\" aria-disabled=\"true\">&laquo;</span>".to_string());
        }

        // Page numbers (show max 7: first ... middle ... last)
        let pages = self.visible_pages();
        for p in pages {
            if p == 0 {
                items.push("<span class=\"au-pagination__ellipsis\">&hellip;</span>".to_string());
            } else if p == self.current {
                items.push(format!(
                    "<span class=\"au-pagination__item au-pagination__item--active\" aria-current=\"page\">{}</span>", p));
            } else {
                items.push(format!(
                    "<a class=\"au-pagination__item\" href=\"{}\">{}</a>",
                    self.page_url(p), p));
            }
        }

        // Next
        if self.current < self.total {
            items.push(format!(
                "<a class=\"au-pagination__item\" href=\"{}\" aria-label=\"Next page\">&raquo;</a>",
                self.page_url(self.current + 1)));
        } else {
            items.push("<span class=\"au-pagination__item au-pagination__item--disabled\" aria-disabled=\"true\">&raquo;</span>".to_string());
        }

        format!("<nav class=\"au-pagination\" aria-label=\"Pagination\">{}</nav>", items.join(""))
    }

    fn visible_pages(&self) -> Vec<u64> {
        if self.total <= 7 {
            return (1..=self.total).collect();
        }
        let mut pages = Vec::new();
        pages.push(1);
        if self.current > 3 { pages.push(0); } // ellipsis
        let start = (self.current.saturating_sub(1)).max(2);
        let end = (self.current + 1).min(self.total - 1);
        for p in start..=end { pages.push(p); }
        if self.current < self.total - 2 { pages.push(0); } // ellipsis
        pages.push(self.total);
        pages
    }
}

// ---------------------------------------------------------------------------
// Toast (NEW)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastLevel { Info, Success, Warning, Error }

#[derive(Debug, Clone)]
pub struct Toast {
    level: ToastLevel,
    message: String,
    duration_ms: u32,
    id: Option<String>,
}

impl Toast {
    pub fn info(message: &str) -> Self { Self::new(message, ToastLevel::Info) }
    pub fn success(message: &str) -> Self { Self::new(message, ToastLevel::Success) }
    pub fn warning(message: &str) -> Self { Self::new(message, ToastLevel::Warning) }
    pub fn error(message: &str) -> Self { Self::new(message, ToastLevel::Error) }

    fn new(message: &str, level: ToastLevel) -> Self {
        Self { level, message: message.to_string(), duration_ms: 5000, id: None }
    }

    pub fn duration_ms(mut self, ms: u32) -> Self { self.duration_ms = ms; self }
    pub fn id(mut self, id: &str) -> Self { self.id = Some(id.to_string()); self }

    pub fn render(&self) -> String {
        let variant = match self.level {
            ToastLevel::Info => "au-toast--info",
            ToastLevel::Success => "au-toast--success",
            ToastLevel::Warning => "au-toast--warning",
            ToastLevel::Error => "au-toast--error",
        };
        let id_attr = match &self.id {
            Some(id) => format!(" id=\"{}\"", html_escape(id)),
            None => String::new(),
        };

        format!(
            "<div class=\"au-toast {}\" role=\"status\" aria-live=\"polite\"{} data-toast-duration=\"{}\">\
                <span class=\"au-toast__message\">{}</span>\
                <button class=\"au-toast__dismiss\" aria-label=\"Dismiss\" \
                    onclick=\"this.closest('.au-toast').remove()\">\
                    <svg width=\"12\" height=\"12\" viewBox=\"0 0 14 14\" fill=\"none\" aria-hidden=\"true\">\
                        <path d=\"M1 1l12 12M13 1L1 13\" stroke=\"currentColor\" stroke-width=\"1.5\" stroke-linecap=\"round\"/>\
                    </svg>\
                </button>\
            </div>",
            variant, id_attr, self.duration_ms, html_escape(&self.message),
        )
    }
}

// ---------------------------------------------------------------------------
// Skeleton (NEW)
// ---------------------------------------------------------------------------

pub struct Skeleton;

impl Skeleton {
    /// Render skeleton text lines.
    pub fn text(lines: u32) -> String {
        let items: String = (0..lines).map(|i| {
            let width = if i == lines - 1 { "60%" } else { "100%" };
            format!("<div class=\"au-skeleton au-skeleton--text\" style=\"width:{}\"></div>", width)
        }).collect();
        format!("<div class=\"au-skeleton-group\">{}</div>", items)
    }

    /// Render a skeleton card placeholder.
    pub fn card() -> String {
        "<div class=\"au-skeleton au-skeleton--card\"></div>".to_string()
    }

    /// Render a skeleton circle (avatar placeholder).
    pub fn circle() -> String {
        "<div class=\"au-skeleton au-skeleton--circle\"></div>".to_string()
    }

    /// Render a skeleton rectangle.
    pub fn rect(width: &str, height: &str) -> String {
        format!(
            "<div class=\"au-skeleton au-skeleton--rect\" style=\"width:{};height:{}\"></div>",
            html_escape(width), html_escape(height))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- Button --
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
    fn button_action_and_data_id() {
        let html = Button::primary("Delete").action("delete_user").data_id("42").render();
        assert!(html.contains("data-action=\"delete_user\""));
        assert!(html.contains("data-id=\"42\""));
    }

    #[test]
    fn button_as_link() {
        let html = Button::primary("Dashboard").href("/dashboard").render();
        assert!(html.contains("<a href=\"/dashboard\""));
        assert!(html.contains("au-btn--primary"));
        assert!(!html.contains("<button"));
    }

    #[test]
    fn button_custom_attr_and_class() {
        let html = Button::primary("Go").class("my-btn").attr("data-custom", "val").id("btn1").render();
        assert!(html.contains("my-btn"));
        assert!(html.contains("data-custom=\"val\""));
        assert!(html.contains("id=\"btn1\""));
    }

    // -- Input --
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
    fn input_with_id() {
        let html = Input::text("name").id("input-name").render();
        assert!(html.contains("id=\"input-name\""));
    }

    // -- Textarea --
    #[test]
    fn textarea_renders() {
        let html = Textarea::new("bio").placeholder("Tell us about yourself").rows(6).render();
        assert!(html.contains("<textarea"));
        assert!(html.contains("au-textarea"));
        assert!(html.contains("name=\"bio\""));
        assert!(html.contains("rows=\"6\""));
        assert!(html.contains("placeholder=\"Tell us about yourself\""));
    }

    #[test]
    fn textarea_with_value() {
        let html = Textarea::new("note").value("Hello <world>").render();
        assert!(html.contains("Hello &lt;world&gt;"));
    }

    // -- Select --
    #[test]
    fn select_renders() {
        let html = Select::new("status")
            .placeholder("Choose status")
            .option("active", "Active")
            .option("inactive", "Inactive")
            .selected("active")
            .render();
        assert!(html.contains("<select"));
        assert!(html.contains("au-select"));
        assert!(html.contains("name=\"status\""));
        assert!(html.contains("disabled selected>Choose status"));
        assert!(html.contains("value=\"active\" selected>Active"));
        assert!(html.contains("value=\"inactive\">Inactive"));
    }

    // -- Toggle --
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

    // -- Card --
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
    fn card_with_id_and_attr() {
        let html = Card::flat("x").id("my-card").attr("data-section", "main").render();
        assert!(html.contains("id=\"my-card\""));
        assert!(html.contains("data-section=\"main\""));
    }

    // -- Alert --
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
    fn alert_dismissible_has_onclick() {
        let html = Alert::warning("Watch out").dismissible().render();
        assert!(html.contains("au-alert__dismiss"));
        assert!(html.contains("aria-label=\"Dismiss\""));
        assert!(html.contains("onclick="));
    }

    // -- Badge --
    #[test]
    fn badge_renders() {
        let html = Badge::new("Active", BadgeVariant::Success).render();
        assert!(html.contains("au-badge"));
        assert!(html.contains("au-badge--success"));
        assert!(html.contains("Active"));
    }

    // -- Avatar --
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
    fn avatar_size_enum() {
        let html = Avatar::initials("A", "Test").small().render();
        assert!(html.contains("au-avatar--sm"));
        let html2 = Avatar::initials("B", "Test").large().render();
        assert!(html2.contains("au-avatar--lg"));
    }

    // -- Progress --
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
    fn progress_indeterminate() {
        let html = Progress::indeterminate().render();
        assert!(html.contains("au-progress--indeterminate"));
        assert!(!html.contains("aria-valuenow"));
    }

    // -- Spinner --
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

    // -- FormGroup --
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
        let html = FormGroup::new("Bio", &input).help("Keep it brief").render();
        assert!(html.contains("au-form-group__help"));
        assert!(html.contains("Keep it brief"));
    }

    #[test]
    fn form_group_label_association() {
        let input = Input::text("name").id("field-name").render();
        let html = FormGroup::new("Name", &input)
            .input_id("field-name")
            .help("Your full name")
            .error("Required")
            .render();
        assert!(html.contains("for=\"field-name\""));
        assert!(html.contains("id=\"field-name-help\""));
        assert!(html.contains("id=\"field-name-error\""));
    }

    // -- Form --
    #[test]
    fn form_renders() {
        let html = Form::new()
            .action("/api/users")
            .method("POST")
            .child("<input name=\"name\" />")
            .render();
        assert!(html.contains("<form"));
        assert!(html.contains("action=\"/api/users\""));
        assert!(html.contains("method=\"POST\""));
        assert!(html.contains("<input name=\"name\" />"));
    }

    // -- Table --
    #[test]
    fn table_renders() {
        let html = Table::new(&["Name", "Email"])
            .row(&["Alice", "alice@ex.com"])
            .row(&["Bob", "bob@ex.com"])
            .striped()
            .render();
        assert!(html.contains("au-table"));
        assert!(html.contains("au-table--striped"));
        assert!(html.contains("scope=\"col\""));
        assert!(html.contains("Alice"));
        assert!(html.contains("bob@ex.com"));
    }

    #[test]
    fn table_with_caption() {
        let html = Table::new(&["Col"]).caption("My Table").render();
        assert!(html.contains("<caption>My Table</caption>"));
    }

    // -- Modal --
    #[test]
    fn modal_renders() {
        let html = Modal::new("confirm", "Are you sure?")
            .title("Confirm")
            .footer(&Button::destructive("Delete").render())
            .render();
        assert!(html.contains("role=\"dialog\""));
        assert!(html.contains("aria-modal=\"true\""));
        assert!(html.contains("data-modal=\"confirm\""));
        assert!(html.contains("aria-labelledby=\"confirm-title\""));
        assert!(html.contains("Are you sure?"));
        assert!(html.contains("au-modal__close"));
    }

    // -- Breadcrumb --
    #[test]
    fn breadcrumb_renders() {
        let html = Breadcrumb::render(&[
            ("Home", Some("/")), ("Products", Some("/products")), ("Widget", None),
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
        assert!(Breadcrumb::render(&[]).is_empty());
    }

    // -- Pagination --
    #[test]
    fn pagination_renders() {
        let html = Pagination::new(3, 10).base_url("/users").render();
        assert!(html.contains("au-pagination"));
        assert!(html.contains("aria-current=\"page\""));
        assert!(html.contains(">3<"));
        assert!(html.contains("page=2")); // prev link
        assert!(html.contains("page=4")); // next link
    }

    #[test]
    fn pagination_first_page() {
        let html = Pagination::new(1, 5).render();
        assert!(html.contains("au-pagination__item--disabled"));
        assert!(html.contains("aria-current=\"page\">1<"));
    }

    // -- Toast --
    #[test]
    fn toast_renders() {
        let html = Toast::success("Saved!").render();
        assert!(html.contains("au-toast--success"));
        assert!(html.contains("Saved!"));
        assert!(html.contains("onclick="));
        assert!(html.contains("data-toast-duration=\"5000\""));
    }

    // -- Skeleton --
    #[test]
    fn skeleton_text() {
        let html = Skeleton::text(3);
        assert!(html.contains("au-skeleton--text"));
        assert!(html.contains("width:60%")); // last line shorter
    }

    #[test]
    fn skeleton_card() {
        assert!(Skeleton::card().contains("au-skeleton--card"));
    }

    #[test]
    fn skeleton_circle() {
        assert!(Skeleton::circle().contains("au-skeleton--circle"));
    }

    // -- html_escape --
    #[test]
    fn html_escape_works() {
        assert_eq!(html_escape("<script>"), "&lt;script&gt;");
        assert_eq!(html_escape("a&b"), "a&amp;b");
        assert_eq!(html_escape("x\"y"), "x&quot;y");
    }
}
