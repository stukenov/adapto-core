use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct PromptTemplate {
    pub name: String,
    pub system: Option<String>,
    pub user_template: String,
    pub variables: Vec<String>,
}

impl PromptTemplate {
    pub fn new(name: &str, user_template: &str) -> Self {
        let variables = extract_variables(user_template);
        Self {
            name: name.into(),
            system: None,
            user_template: user_template.into(),
            variables,
        }
    }

    pub fn with_system(mut self, system: &str) -> Self {
        self.system = Some(system.into());
        self
    }

    pub fn render(&self, vars: &HashMap<String, String>) -> Result<RenderedPrompt, PromptError> {
        for v in &self.variables {
            if !vars.contains_key(v) {
                return Err(PromptError::MissingVariable(v.clone()));
            }
        }

        let user_content = substitute(&self.user_template, vars);
        let system_content = self.system.as_ref().map(|s| substitute(s, vars));

        Ok(RenderedPrompt {
            system: system_content,
            user: user_content,
        })
    }

    pub fn required_variables(&self) -> &[String] {
        &self.variables
    }
}

#[derive(Debug, Clone)]
pub struct RenderedPrompt {
    pub system: Option<String>,
    pub user: String,
}

#[derive(Debug, Clone)]
pub enum PromptError {
    MissingVariable(String),
    TemplateNotFound(String),
}

impl std::fmt::Display for PromptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PromptError::MissingVariable(v) => write!(f, "missing variable: {{{{{}}}}}", v),
            PromptError::TemplateNotFound(n) => write!(f, "template not found: {}", n),
        }
    }
}

fn extract_variables(template: &str) -> Vec<String> {
    let mut vars = Vec::new();
    let mut i = 0;
    let bytes = template.as_bytes();
    while i < bytes.len().saturating_sub(1) {
        if bytes[i] == b'{' && bytes[i + 1] == b'{' {
            let start = i + 2;
            if let Some(end) = template[start..].find("}}") {
                let var = template[start..start + end].trim().to_string();
                if !var.is_empty() && !vars.contains(&var) {
                    vars.push(var);
                }
                i = start + end + 2;
                continue;
            }
        }
        i += 1;
    }
    vars
}

fn substitute(template: &str, vars: &HashMap<String, String>) -> String {
    let mut result = template.to_string();
    for (key, value) in vars {
        let pattern = format!("{{{{{}}}}}", key);
        result = result.replace(&pattern, value);
    }
    result
}

pub struct PromptLibrary {
    templates: HashMap<String, PromptTemplate>,
}

impl PromptLibrary {
    pub fn new() -> Self {
        Self {
            templates: HashMap::new(),
        }
    }

    pub fn add(mut self, template: PromptTemplate) -> Self {
        self.templates.insert(template.name.clone(), template);
        self
    }

    pub fn get(&self, name: &str) -> Result<&PromptTemplate, PromptError> {
        self.templates
            .get(name)
            .ok_or_else(|| PromptError::TemplateNotFound(name.into()))
    }

    pub fn render(
        &self,
        name: &str,
        vars: &HashMap<String, String>,
    ) -> Result<RenderedPrompt, PromptError> {
        self.get(name)?.render(vars)
    }

    pub fn list(&self) -> Vec<&str> {
        self.templates.keys().map(|k| k.as_str()).collect()
    }
}

impl Default for PromptLibrary {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vars(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    #[test]
    fn extract_variables_from_template() {
        let vars = extract_variables("Hello {{name}}, you are {{age}} years old");
        assert_eq!(vars, vec!["name", "age"]);
    }

    #[test]
    fn extract_no_variables() {
        let vars = extract_variables("no variables here");
        assert!(vars.is_empty());
    }

    #[test]
    fn extract_deduplicates() {
        let vars = extract_variables("{{x}} and {{x}} again");
        assert_eq!(vars, vec!["x"]);
    }

    #[test]
    fn render_substitutes() {
        let t = PromptTemplate::new("test", "Hello {{name}}!");
        let rendered = t.render(&vars(&[("name", "Alice")])).unwrap();
        assert_eq!(rendered.user, "Hello Alice!");
    }

    #[test]
    fn render_with_system() {
        let t = PromptTemplate::new("test", "{{query}}")
            .with_system("You are a {{role}} assistant");
        let rendered = t.render(&vars(&[("query", "help"), ("role", "coding")])).unwrap();
        assert_eq!(rendered.user, "help");
        assert_eq!(rendered.system.unwrap(), "You are a coding assistant");
    }

    #[test]
    fn render_missing_variable_fails() {
        let t = PromptTemplate::new("test", "{{name}} {{age}}");
        let err = t.render(&vars(&[("name", "Alice")]));
        assert!(err.is_err());
    }

    #[test]
    fn prompt_library_usage() {
        let lib = PromptLibrary::new()
            .add(PromptTemplate::new("summarize", "Summarize: {{text}}"))
            .add(PromptTemplate::new("translate", "Translate to {{lang}}: {{text}}"));

        let rendered = lib.render("summarize", &vars(&[("text", "hello world")])).unwrap();
        assert_eq!(rendered.user, "Summarize: hello world");

        assert!(lib.get("nonexistent").is_err());
        assert_eq!(lib.list().len(), 2);
    }

    #[test]
    fn required_variables() {
        let t = PromptTemplate::new("test", "{{a}} {{b}} {{c}}");
        assert_eq!(t.required_variables(), &["a", "b", "c"]);
    }
}
