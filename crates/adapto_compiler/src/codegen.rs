use crate::ir::*;

/// Generates Rust source code from a ComponentIR.
///
/// Produces a state struct, a `Component` impl with `render` and
/// `handle_event` methods, and optional form validation structs.
pub struct CodeGenerator {
    indent: usize,
    output: String,
}

impl CodeGenerator {
    pub fn new() -> Self {
        Self {
            indent: 0,
            output: String::new(),
        }
    }

    /// Generate the complete Rust component code from IR.
    pub fn generate_component(&mut self, ir: &ComponentIR) -> String {
        self.output.clear();
        self.indent = 0;

        // State struct
        let state = self.gen_state_struct(ir);
        self.output.push_str(&state);
        self.output.push('\n');

        // Form structs
        for form in &ir.form_schemas {
            let form_code = self.gen_form_struct(form);
            self.output.push_str(&form_code);
            self.output.push('\n');
        }

        // Component impl
        self.write_line(&format!("impl Component for {} {{", ir.name));
        self.indent += 1;
        self.write_line(&format!("type State = {}State;", ir.name));
        self.output.push('\n');

        // render()
        let render = self.gen_render_fn(ir);
        self.output.push_str(&render);
        self.output.push('\n');

        // handle_event()
        let handler = self.gen_event_handler(ir);
        self.output.push_str(&handler);

        self.indent -= 1;
        self.write_line("}");

        self.output.clone()
    }

    /// Generate the state struct for the component.
    fn gen_state_struct(&mut self, ir: &ComponentIR) -> String {
        let mut out = String::new();

        out.push_str(&format!("pub struct {}State {{\n", ir.name));
        for field in &ir.state_fields {
            out.push_str(&format!("    pub {}: {},\n", field.name, field.ty));
        }
        out.push_str("}\n");

        out
    }

    /// Generate the render function body.
    fn gen_render_fn(&mut self, ir: &ComponentIR) -> String {
        let mut out = String::new();
        let indent = "    ".repeat(self.indent);

        out.push_str(&format!(
            "{indent}fn render(&self, state: &Self::State) -> Rendered {{\n"
        ));
        out.push_str(&format!("{indent}    Rendered::new()\n"));

        // Interleave static and dynamic segments in order.
        // Static segments are emitted as .static_part("..."),
        // dynamic segments as .dynamic_text("id", expr, deps![...]).
        let mut static_idx = 0;
        let mut dynamic_idx = 0;

        // Build a merged sequence: we place static parts first, then dynamic
        // segments between them. The convention is:
        // static[0], dynamic[0], static[1], dynamic[1], ..., static[N]
        let total_steps = ir.static_segments.len() + ir.dynamic_segments.len();
        let mut step = 0;
        let mut next_is_static = true;

        while step < total_steps {
            if next_is_static && static_idx < ir.static_segments.len() {
                let seg = &ir.static_segments[static_idx];
                out.push_str(&format!(
                    "{indent}        .static_part(\"{}\")\n",
                    escape_rust_string(seg)
                ));
                static_idx += 1;
                step += 1;
                next_is_static = false;
            } else if dynamic_idx < ir.dynamic_segments.len() {
                let dyn_seg = &ir.dynamic_segments[dynamic_idx];
                let deps_str = dyn_seg
                    .deps
                    .iter()
                    .map(|d| format!("\"{}\"", d))
                    .collect::<Vec<_>>()
                    .join(", ");

                match &dyn_seg.segment_type {
                    SegmentType::Text => {
                        out.push_str(&format!(
                            "{indent}        .dynamic_text(\"{}\", {}.to_string(), deps![{}])\n",
                            dyn_seg.id, dyn_seg.expr, deps_str
                        ));
                    }
                    SegmentType::Html => {
                        out.push_str(&format!(
                            "{indent}        .dynamic_html(\"{}\", {}, deps![{}])\n",
                            dyn_seg.id, dyn_seg.expr, deps_str
                        ));
                    }
                    SegmentType::Attribute {
                        element_id,
                        attr_name,
                    } => {
                        out.push_str(&format!(
                            "{indent}        .dynamic_attr(\"{}\", \"{}\", \"{}\", {}.to_string(), deps![{}])\n",
                            dyn_seg.id, element_id, attr_name, dyn_seg.expr, deps_str
                        ));
                    }
                    SegmentType::Conditional => {
                        out.push_str(&format!(
                            "{indent}        .dynamic_cond(\"{}\", {}, deps![{}])\n",
                            dyn_seg.id, dyn_seg.expr, deps_str
                        ));
                    }
                    SegmentType::Loop => {
                        out.push_str(&format!(
                            "{indent}        .dynamic_loop(\"{}\", {}, deps![{}])\n",
                            dyn_seg.id, dyn_seg.expr, deps_str
                        ));
                    }
                    SegmentType::Permission => {
                        out.push_str(&format!(
                            "{indent}        .dynamic_perm(\"{}\", \"{}\", deps![{}])\n",
                            dyn_seg.id, dyn_seg.expr, deps_str
                        ));
                    }
                }

                dynamic_idx += 1;
                step += 1;
                next_is_static = true;
            } else if static_idx < ir.static_segments.len() {
                // Remaining static segments
                let seg = &ir.static_segments[static_idx];
                out.push_str(&format!(
                    "{indent}        .static_part(\"{}\")\n",
                    escape_rust_string(seg)
                ));
                static_idx += 1;
                step += 1;
            } else {
                break;
            }
        }

        out.push_str(&format!("{indent}}}\n"));

        out
    }

    /// Generate the event handler match block.
    fn gen_event_handler(&mut self, ir: &ComponentIR) -> String {
        let mut out = String::new();
        let indent = "    ".repeat(self.indent);

        out.push_str(&format!(
            "{indent}fn handle_event(&mut self, event: Event, state: &mut Self::State) -> Result<()> {{\n"
        ));
        out.push_str(&format!(
            "{indent}    match event.handler.as_str() {{\n"
        ));

        for action in &ir.actions {
            out.push_str(&format!(
                "{indent}        \"{}\" => {{\n",
                action.name
            ));

            // Emit the action body lines
            for line in action.body.lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    out.push_str(&format!("{indent}            {}\n", trimmed));
                }
            }

            out.push_str(&format!("{indent}            Ok(())\n"));
            out.push_str(&format!("{indent}        }}\n"));
        }

        out.push_str(&format!(
            "{indent}        _ => Err(Error::UnknownHandler)\n"
        ));
        out.push_str(&format!("{indent}    }}\n"));
        out.push_str(&format!("{indent}}}\n"));

        out
    }

    /// Generate a form validation struct.
    fn gen_form_struct(&mut self, schema: &FormSchemaIR) -> String {
        let mut out = String::new();

        out.push_str(&format!("pub struct {} {{\n", schema.name));
        for field in &schema.fields {
            out.push_str(&format!("    pub {}: {},\n", field.name, field.ty));
        }
        out.push_str("}\n");

        // Generate a validate() method
        out.push_str(&format!("\nimpl {} {{\n", schema.name));
        out.push_str("    pub fn validate(&self) -> Result<(), Vec<String>> {\n");
        out.push_str("        let mut errors = Vec::new();\n");

        for field in &schema.fields {
            if field.required {
                // For String types, check emptiness
                if field.ty == "String" {
                    out.push_str(&format!(
                        "        if self.{name}.is_empty() {{\n            errors.push(\"{name} is required\".to_string());\n        }}\n",
                        name = field.name
                    ));
                }
            }
            if let Some(min) = field.min {
                if field.ty == "String" {
                    out.push_str(&format!(
                        "        if self.{name}.len() < {min} {{\n            errors.push(format!(\"{name} must be at least {min} characters\"));\n        }}\n",
                        name = field.name, min = min
                    ));
                }
            }
            if let Some(max) = field.max {
                if field.ty == "String" {
                    out.push_str(&format!(
                        "        if self.{name}.len() > {max} {{\n            errors.push(format!(\"{name} must be at most {max} characters\"));\n        }}\n",
                        name = field.name, max = max
                    ));
                }
            }
        }

        out.push_str("        if errors.is_empty() { Ok(()) } else { Err(errors) }\n");
        out.push_str("    }\n");
        out.push_str("}\n");

        out
    }

    fn write_line(&mut self, line: &str) {
        let indent = "    ".repeat(self.indent);
        self.output.push_str(&format!("{indent}{line}\n"));
    }
}

impl Default for CodeGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// Escape a string for embedding in a Rust string literal.
fn escape_rust_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}
