use std::collections::{HashMap, HashSet};

use crate::error::SsrError;

const MAX_LAYOUT_DEPTH: usize = 10;

pub struct CompiledLayout {
    pub name: String,
    pub parent: Option<String>,
    pub slots: Vec<SlotDefinition>,
    pub template_html: String,
}

pub struct SlotDefinition {
    pub name: Option<String>,
    pub fallback_html: Option<String>,
}

pub struct LayoutManager {
    layouts: HashMap<String, CompiledLayout>,
}

impl LayoutManager {
    pub fn new() -> Self {
        Self {
            layouts: HashMap::new(),
        }
    }

    pub fn register(&mut self, name: &str, layout: CompiledLayout) {
        self.layouts.insert(name.to_string(), layout);
    }

    pub fn has_layout(&self, name: &str) -> bool {
        self.layouts.contains_key(name)
    }

    pub fn get_template(&self, name: &str) -> Option<&str> {
        self.layouts.get(name).map(|l| l.template_html.as_str())
    }

    pub fn resolve_chain<'a>(&'a self, layout_name: &str) -> Result<Vec<&'a str>, SsrError> {
        let mut chain: Vec<&'a str> = Vec::new();
        let mut visited: HashSet<String> = HashSet::new();
        let mut current_name: String = layout_name.to_string();

        loop {
            if !visited.insert(current_name.clone()) {
                return Err(SsrError::LayoutCycle(current_name));
            }
            if chain.len() >= MAX_LAYOUT_DEPTH {
                return Err(SsrError::LayoutDepthExceeded(MAX_LAYOUT_DEPTH));
            }

            let (key, layout) = self.layouts.get_key_value(current_name.as_str())
                .ok_or_else(|| SsrError::LayoutNotFound(current_name.clone()))?;
            chain.push(key.as_str());

            match &layout.parent {
                Some(parent) => current_name = parent.clone(),
                None => break,
            }
        }

        Ok(chain)
    }

    pub fn compose(
        &self,
        layout_name: &str,
        page_default: &str,
        fills: &HashMap<String, String>,
    ) -> Result<String, SsrError> {
        let layout = self.layouts.get(layout_name)
            .ok_or_else(|| SsrError::LayoutNotFound(layout_name.to_string()))?;

        Ok(self.apply_slots(&layout.template_html, &layout.slots, page_default, fills))
    }

    pub fn compose_chain(
        &self,
        layout_name: &str,
        page_default: &str,
        page_fills: &HashMap<String, String>,
        layout_fills: &HashMap<String, HashMap<String, String>>,
    ) -> Result<String, SsrError> {
        let chain = self.resolve_chain(layout_name)?;

        let mut current_default = page_default.to_string();
        let mut current_fills = page_fills.clone();

        for &name in &chain {
            let layout = self.layouts.get(name).unwrap();
            let composed = self.apply_slots(
                &layout.template_html,
                &layout.slots,
                &current_default,
                &current_fills,
            );

            current_default = composed;
            current_fills = layout_fills
                .get(name)
                .cloned()
                .unwrap_or_default();
        }

        Ok(current_default)
    }

    fn apply_slots(
        &self,
        template: &str,
        slots: &[SlotDefinition],
        default_content: &str,
        fills: &HashMap<String, String>,
    ) -> String {
        let mut result = template.to_string();

        for slot in slots {
            let marker = match &slot.name {
                Some(name) => format!("<!-- slot:{} -->", name),
                None => "<!-- slot:default -->".to_string(),
            };

            let replacement = match &slot.name {
                Some(name) => {
                    fills.get(name.as_str())
                        .cloned()
                        .or_else(|| slot.fallback_html.clone())
                        .unwrap_or_default()
                }
                None => default_content.to_string(),
            };

            result = result.replace(&marker, &replacement);
        }

        // Backwards compatibility
        result = result.replace("{slot}", default_content);

        result
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
    fn register_and_compose_simple() {
        let mut mgr = LayoutManager::new();
        mgr.register("main", CompiledLayout {
            name: "main".into(),
            parent: None,
            slots: vec![
                SlotDefinition { name: None, fallback_html: None },
            ],
            template_html: "<html><body><nav>Menu</nav><main><!-- slot:default --></main></body></html>".into(),
        });

        let fills = HashMap::new();
        let result = mgr.compose("main", "<h1>Hello</h1>", &fills).unwrap();
        assert!(result.contains("<nav>Menu</nav>"));
        assert!(result.contains("<h1>Hello</h1>"));
    }

    #[test]
    fn named_slot_with_fill() {
        let mut mgr = LayoutManager::new();
        mgr.register("main", CompiledLayout {
            name: "main".into(),
            parent: None,
            slots: vec![
                SlotDefinition { name: Some("sidebar".into()), fallback_html: Some("<p>Default</p>".into()) },
                SlotDefinition { name: None, fallback_html: None },
            ],
            template_html: "<div><aside><!-- slot:sidebar --></aside><main><!-- slot:default --></main></div>".into(),
        });

        let mut fills = HashMap::new();
        fills.insert("sidebar".to_string(), "<nav>Custom sidebar</nav>".to_string());
        let result = mgr.compose("main", "<h1>Content</h1>", &fills).unwrap();
        assert!(result.contains("<nav>Custom sidebar</nav>"));
        assert!(result.contains("<h1>Content</h1>"));
        assert!(!result.contains("Default"));
    }

    #[test]
    fn named_slot_fallback_when_no_fill() {
        let mut mgr = LayoutManager::new();
        mgr.register("main", CompiledLayout {
            name: "main".into(),
            parent: None,
            slots: vec![
                SlotDefinition { name: Some("sidebar".into()), fallback_html: Some("<p>Default sidebar</p>".into()) },
                SlotDefinition { name: None, fallback_html: None },
            ],
            template_html: "<div><aside><!-- slot:sidebar --></aside><main><!-- slot:default --></main></div>".into(),
        });

        let fills = HashMap::new();
        let result = mgr.compose("main", "<h1>Content</h1>", &fills).unwrap();
        assert!(result.contains("<p>Default sidebar</p>"));
    }

    #[test]
    fn multi_level_chain() {
        let mut mgr = LayoutManager::new();
        mgr.register("base", CompiledLayout {
            name: "base".into(),
            parent: None,
            slots: vec![
                SlotDefinition { name: Some("head".into()), fallback_html: Some("<title>App</title>".into()) },
                SlotDefinition { name: None, fallback_html: None },
            ],
            template_html: "<html><head><!-- slot:head --></head><body><!-- slot:default --></body></html>".into(),
        });
        mgr.register("dashboard", CompiledLayout {
            name: "dashboard".into(),
            parent: Some("base".into()),
            slots: vec![
                SlotDefinition { name: None, fallback_html: None },
            ],
            template_html: "<div class=\"dash\"><!-- slot:default --></div>".into(),
        });

        let page_fills = HashMap::new();
        let layout_fills = HashMap::from([
            ("dashboard".to_string(), HashMap::from([
                ("head".to_string(), "<title>Dashboard</title>".to_string()),
            ])),
        ]);

        let result = mgr.compose_chain("dashboard", "<h1>Stats</h1>", &page_fills, &layout_fills).unwrap();
        assert!(result.contains("<title>Dashboard</title>"));
        assert!(result.contains("<div class=\"dash\">"));
        assert!(result.contains("<h1>Stats</h1>"));
    }

    #[test]
    fn cycle_detection() {
        let mut mgr = LayoutManager::new();
        mgr.register("a", CompiledLayout {
            name: "a".into(),
            parent: Some("b".into()),
            slots: vec![],
            template_html: "<!-- slot:default -->".into(),
        });
        mgr.register("b", CompiledLayout {
            name: "b".into(),
            parent: Some("a".into()),
            slots: vec![],
            template_html: "<!-- slot:default -->".into(),
        });

        let result = mgr.resolve_chain("a");
        assert!(result.is_err());
    }

    #[test]
    fn missing_layout_error() {
        let mgr = LayoutManager::new();
        let result = mgr.compose("nonexistent", "<p>test</p>", &HashMap::new());
        assert!(result.is_err());
    }

    #[test]
    fn has_layout_check() {
        let mut mgr = LayoutManager::new();
        assert!(!mgr.has_layout("main"));
        mgr.register("main", CompiledLayout {
            name: "main".into(),
            parent: None,
            slots: vec![],
            template_html: "<!-- slot:default -->".into(),
        });
        assert!(mgr.has_layout("main"));
    }

    #[test]
    fn backwards_compat_slot_marker() {
        let mut mgr = LayoutManager::new();
        mgr.register("legacy", CompiledLayout {
            name: "legacy".into(),
            parent: None,
            slots: vec![],
            template_html: "<html><body>{slot}</body></html>".into(),
        });
        let fills = HashMap::new();
        let result = mgr.compose("legacy", "<p>content</p>", &fills).unwrap();
        assert!(result.contains("<p>content</p>"));
    }
}
