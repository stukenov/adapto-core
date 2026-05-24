use adapto_compiler::ir::*;
use adapto_compiler::dependency::DependencyGraph;
use adapto_runtime::state::StateStore;
use adapto_client_protocol::patch::*;

pub struct PatchGenerator;

impl PatchGenerator {
    /// Given dirty state fields, walk the dependency graph and generate the
    /// minimal set of patch operations needed to reconcile the client DOM.
    pub fn generate(
        state: &StateStore,
        dependency_graph: &DependencyGraph,
        dynamic_segments: &[DynamicSegment],
    ) -> Vec<PatchOp> {
        let dirty: Vec<&str> = state.get_dirty().iter().map(|s| s.as_str()).collect();
        if dirty.is_empty() {
            return vec![];
        }

        let affected = dependency_graph.get_affected_segments(&dirty);
        let mut ops = Vec::new();

        for segment in dynamic_segments {
            if affected.contains(&segment.id) {
                let value = Self::eval_expr(&segment.expr, state);
                match &segment.segment_type {
                    SegmentType::Text => {
                        ops.push(PatchOp::ReplaceText {
                            target: segment.id.clone(),
                            value,
                        });
                    }
                    SegmentType::Html => {
                        ops.push(PatchOp::ReplaceHtml {
                            target: segment.id.clone(),
                            html: value,
                        });
                    }
                    SegmentType::Attribute {
                        element_id,
                        attr_name,
                    } => {
                        ops.push(PatchOp::SetAttr {
                            target: element_id.clone(),
                            name: attr_name.clone(),
                            value,
                        });
                    }
                    _ => {
                        // Conditional, Loop, and Permission segments receive
                        // a full HTML replacement.
                        ops.push(PatchOp::ReplaceHtml {
                            target: segment.id.clone(),
                            html: value,
                        });
                    }
                }
            }
        }

        ops
    }

    /// Evaluate a simple dot-notation expression against the state store.
    ///
    /// `"customer.name"` resolves to `state["customer"]["name"]`.
    fn eval_expr(expr: &str, state: &StateStore) -> String {
        let bare = expr.strip_prefix("state.").unwrap_or(expr);
        let parts: Vec<&str> = bare.split('.').collect();
        let mut value = state.get(parts[0]);

        for part in &parts[1..] {
            value = value.and_then(|v| v.get(part));
        }

        match value {
            Some(v) => match v {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Null => String::new(),
                other => other.to_string(),
            },
            None => String::new(),
        }
    }
}
