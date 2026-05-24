use std::collections::{HashMap, HashSet};

/// Tracks the relationship between state fields and dynamic template segments.
///
/// When a state field changes, the dependency graph tells us exactly which
/// template segments need re-rendering — enabling surgical DOM patches
/// instead of full re-renders.
#[derive(Debug, Clone, Default)]
pub struct DependencyGraph {
    /// state_field -> set of dynamic segment IDs that depend on it
    state_to_segments: HashMap<String, HashSet<String>>,
    /// dynamic segment ID -> set of state fields it depends on
    segment_to_deps: HashMap<String, HashSet<String>>,
}

impl DependencyGraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register that `segment_id` depends on `state_field`.
    pub fn add_dependency(&mut self, segment_id: &str, state_field: &str) {
        self.state_to_segments
            .entry(state_field.to_string())
            .or_default()
            .insert(segment_id.to_string());

        self.segment_to_deps
            .entry(segment_id.to_string())
            .or_default()
            .insert(state_field.to_string());
    }

    /// Given a set of dirty state fields, return all segment IDs that need updating.
    pub fn get_affected_segments(&self, dirty_fields: &[&str]) -> HashSet<String> {
        let mut affected = HashSet::new();
        for field in dirty_fields {
            if let Some(segments) = self.state_to_segments.get(*field) {
                affected.extend(segments.iter().cloned());
            }
        }
        affected
    }

    /// Return all state fields that a given segment depends on.
    pub fn get_deps_for_segment(&self, segment_id: &str) -> HashSet<String> {
        self.segment_to_deps
            .get(segment_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Return all known state fields in the graph.
    pub fn all_state_fields(&self) -> HashSet<String> {
        self.state_to_segments.keys().cloned().collect()
    }

    /// Return all known segment IDs in the graph.
    pub fn all_segments(&self) -> HashSet<String> {
        self.segment_to_deps.keys().cloned().collect()
    }

    /// Validate that all dependencies reference known state fields.
    /// Returns a list of unknown dependency names.
    pub fn validate(&self, known_state: &[&str]) -> Vec<String> {
        let known: HashSet<&str> = known_state.iter().copied().collect();
        let mut unknown = Vec::new();

        for deps in self.segment_to_deps.values() {
            for dep in deps {
                if !known.contains(dep.as_str()) && !unknown.contains(dep) {
                    unknown.push(dep.clone());
                }
            }
        }

        unknown.sort();
        unknown
    }
}
