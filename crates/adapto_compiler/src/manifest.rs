use serde::{Deserialize, Serialize};

/// The route manifest — a registry of all compiled routes in the application.
///
/// Used at startup to wire HTTP handlers and at build time to produce
/// a static route table for the client-side router.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteManifest {
    pub routes: Vec<RouteEntry>,
}

/// A single entry in the route manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteEntry {
    pub id: String,
    pub path: String,
    pub file: String,
    pub method: String,
    pub auth: String,
    pub tenant: String,
    pub permission: Option<String>,
    pub layout: Option<String>,
    pub cache: String,
}

/// The component manifest — a registry of all compiled components.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentManifest {
    pub components: Vec<ComponentEntry>,
}

/// A single entry in the component manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentEntry {
    pub id: String,
    pub name: String,
    pub file: String,
    pub is_island: bool,
    pub dependencies: Vec<String>,
}

impl RouteManifest {
    pub fn new() -> Self {
        Self { routes: Vec::new() }
    }

    pub fn add(&mut self, entry: RouteEntry) {
        self.routes.push(entry);
    }

    pub fn find_by_path(&self, path: &str) -> Option<&RouteEntry> {
        self.routes.iter().find(|r| r.path == path)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }
}

impl Default for RouteManifest {
    fn default() -> Self {
        Self::new()
    }
}

impl ComponentManifest {
    pub fn new() -> Self {
        Self {
            components: Vec::new(),
        }
    }

    pub fn add(&mut self, entry: ComponentEntry) {
        self.components.push(entry);
    }

    pub fn find_by_name(&self, name: &str) -> Option<&ComponentEntry> {
        self.components.iter().find(|c| c.name == name)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }
}

impl Default for ComponentManifest {
    fn default() -> Self {
        Self::new()
    }
}
