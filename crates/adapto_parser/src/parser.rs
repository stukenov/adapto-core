use crate::ast::*;
use crate::error::{ParseError, ParseResult};

// ---------------------------------------------------------------------------
// Top-level block extraction
// ---------------------------------------------------------------------------

/// Represents a raw block extracted from the source before detailed parsing.
#[derive(Debug)]
struct RawBlock {
    kind: String,
    attrs: String,
    content: String,
    /// Line number where the opening tag starts (1-based).
    start_line: usize,
}

/// Extract all top-level blocks from the `.adapto` source text.
///
/// A top-level block is `<kind attrs>...content...</kind>` where `kind` is one
/// of: route, script, template, style, resource, layout.
fn extract_blocks(input: &str) -> ParseResult<Vec<RawBlock>> {
    let known_blocks = ["route", "script", "template", "style", "resource", "layout"];
    let mut blocks = Vec::new();
    let mut remaining = input;
    let mut global_offset: usize = 0;

    while !remaining.is_empty() {
        // Find the next `<` that starts a known block
        let Some(lt_pos) = remaining.find('<') else {
            break;
        };

        let after_lt = &remaining[lt_pos + 1..];

        // Skip HTML comments <!-- -->
        if after_lt.starts_with("!--") {
            if let Some(end) = remaining[lt_pos..].find("-->") {
                let skip = lt_pos + end + 3;
                global_offset += skip;
                remaining = &remaining[skip..];
                continue;
            }
        }

        // Check if this `<` starts a known top-level block
        let mut matched_block = None;
        for &kind in &known_blocks {
            if after_lt.starts_with(kind) {
                let after_kind = &after_lt[kind.len()..];
                if after_kind.starts_with('>') || after_kind.starts_with(' ') || after_kind.starts_with('\n') {
                    matched_block = Some(kind);
                    break;
                }
            }
        }

        let Some(kind) = matched_block else {
            // Not a known block start — skip past this `<`
            global_offset += lt_pos + 1;
            remaining = &remaining[lt_pos + 1..];
            continue;
        };

        // Calculate start line
        let start_line = input[..global_offset + lt_pos]
            .chars()
            .filter(|&c| c == '\n')
            .count()
            + 1;

        // Find end of opening tag `>`
        let open_tag_start = lt_pos;
        let after_open = &remaining[open_tag_start..];
        let Some(gt_pos) = after_open.find('>') else {
            return Err(ParseError::Syntax {
                line: start_line,
                col: 1,
                message: format!("Unclosed opening tag for <{kind}>"),
            });
        };

        // Extract attributes from the opening tag
        let tag_inner = &after_open[1 + kind.len()..gt_pos];
        let attrs = tag_inner.trim().to_string();

        let content_start = open_tag_start + gt_pos + 1;

        // Find closing tag `</kind>`
        let close_tag = format!("</{kind}>");
        let search_area = &remaining[content_start..];
        let Some(close_pos) = find_matching_close(search_area, kind) else {
            return Err(ParseError::UnclosedBlock(kind.to_string()));
        };

        let content = remaining[content_start..content_start + close_pos].to_string();

        let skip = content_start + close_pos + close_tag.len();
        global_offset += skip;
        remaining = &remaining[skip..];

        blocks.push(RawBlock {
            kind: kind.to_string(),
            attrs,
            content,
            start_line,
        });
    }

    Ok(blocks)
}

/// Find the position of the matching `</kind>` tag, handling nesting.
fn find_matching_close(s: &str, kind: &str) -> Option<usize> {
    let open_tag = format!("<{kind}");
    let close_tag = format!("</{kind}>");
    let mut depth: usize = 1;
    let mut pos: usize = 0;

    while pos < s.len() {
        if s[pos..].starts_with(&close_tag) {
            depth -= 1;
            if depth == 0 {
                return Some(pos);
            }
            pos += close_tag.len();
        } else if s[pos..].starts_with(&open_tag) {
            let after = &s[pos + open_tag.len()..];
            if after.starts_with('>') || after.starts_with(' ') || after.starts_with('\n') {
                depth += 1;
            }
            pos += open_tag.len();
        } else {
            pos += 1;
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Parse an `.adapto` source string into an `AdaptoFile` AST.
pub fn parse(input: &str) -> ParseResult<AdaptoFile> {
    let blocks = extract_blocks(input)?;

    let mut file = AdaptoFile {
        route: None,
        script: None,
        template: None,
        style: None,
        resource: None,
        layout: None,
    };

    for block in blocks {
        match block.kind.as_str() {
            "route" => {
                if file.route.is_some() {
                    return Err(ParseError::DuplicateBlock("route".into()));
                }
                file.route = Some(parse_route_block(&block.content, block.start_line)?);
            }
            "script" => {
                if file.script.is_some() {
                    return Err(ParseError::DuplicateBlock("script".into()));
                }
                file.script = Some(parse_script_block(&block.content, block.start_line)?);
            }
            "template" => {
                if file.template.is_some() {
                    return Err(ParseError::DuplicateBlock("template".into()));
                }
                file.template = Some(parse_template_block(&block.content, block.start_line)?);
            }
            "style" => {
                if file.style.is_some() {
                    return Err(ParseError::DuplicateBlock("style".into()));
                }
                let scoped = block.attrs.contains("scoped");
                let global = block.attrs.contains("global");
                file.style = Some(StyleBlock {
                    scoped: scoped || !global,
                    content: block.content.trim().to_string(),
                });
            }
            "resource" => {
                if file.resource.is_some() {
                    return Err(ParseError::DuplicateBlock("resource".into()));
                }
                file.resource =
                    Some(parse_resource_block(&block.attrs, &block.content, block.start_line)?);
            }
            "layout" => {
                if file.layout.is_some() {
                    return Err(ParseError::DuplicateBlock("layout".into()));
                }
                file.layout =
                    Some(parse_layout_block(&block.attrs, &block.content, block.start_line)?);
            }
            other => {
                return Err(ParseError::UnknownBlock(other.to_string()));
            }
        }
    }

    Ok(file)
}

// ---------------------------------------------------------------------------
// Route block parser
// ---------------------------------------------------------------------------

fn parse_route_block(content: &str, base_line: usize) -> ParseResult<RouteBlock> {
    let mut route = RouteBlock {
        path: None,
        method: None,
        layout: None,
        page_title: None,
        auth: None,
        role: None,
        permission: None,
        tenant: None,
        cache: None,
        error: None,
        not_found: None,
    };

    for (i, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("//") {
            continue;
        }

        let Some((key, value)) = parse_kv_line(line) else {
            return Err(ParseError::Syntax {
                line: base_line + i + 1,
                col: 1,
                message: format!("Expected `key: value`, got: {line}"),
            });
        };

        let value_unquoted = unquote(&value);

        match key.as_str() {
            "path" => route.path = Some(value_unquoted),
            "method" => route.method = Some(value_unquoted),
            "layout" => route.layout = Some(value_unquoted),
            "page_title" => route.page_title = Some(value_unquoted),
            "auth" => {
                route.auth = Some(parse_auth_level(&value_unquoted).map_err(|reason| {
                    ParseError::InvalidValue {
                        field: "auth".into(),
                        value: value_unquoted.clone(),
                        reason,
                    }
                })?);
            }
            "role" => route.role = Some(value_unquoted),
            "permission" => route.permission = Some(value_unquoted),
            "tenant" => {
                route.tenant =
                    Some(parse_tenant_level(&value_unquoted).map_err(|reason| {
                        ParseError::InvalidValue {
                            field: "tenant".into(),
                            value: value_unquoted.clone(),
                            reason,
                        }
                    })?);
            }
            "cache" => {
                route.cache =
                    Some(parse_cache_policy(&value_unquoted).map_err(|reason| {
                        ParseError::InvalidValue {
                            field: "cache".into(),
                            value: value_unquoted.clone(),
                            reason,
                        }
                    })?);
            }
            "error" => route.error = Some(value_unquoted),
            "not_found" => route.not_found = Some(value_unquoted),
            other => {
                return Err(ParseError::Syntax {
                    line: base_line + i + 1,
                    col: 1,
                    message: format!("Unknown route field: {other}"),
                });
            }
        }
    }

    Ok(route)
}

fn parse_kv_line(line: &str) -> Option<(String, String)> {
    let colon = line.find(':')?;
    let key = line[..colon].trim().to_string();
    let value = line[colon + 1..].trim().to_string();
    if key.is_empty() {
        return None;
    }
    Some((key, value))
}

fn unquote(s: &str) -> String {
    let s = s.trim();
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

fn parse_auth_level(s: &str) -> Result<AuthLevel, String> {
    match s {
        "public" => Ok(AuthLevel::Public),
        "optional" => Ok(AuthLevel::Optional),
        "required" => Ok(AuthLevel::Required),
        other => Err(format!("Expected public|optional|required, got: {other}")),
    }
}

fn parse_tenant_level(s: &str) -> Result<TenantLevel, String> {
    match s {
        "none" => Ok(TenantLevel::None),
        "optional" => Ok(TenantLevel::Optional),
        "required" => Ok(TenantLevel::Required),
        other => Err(format!("Expected none|optional|required, got: {other}")),
    }
}

fn parse_cache_policy(s: &str) -> Result<CachePolicy, String> {
    match s {
        "no-store" => Ok(CachePolicy::NoStore),
        "private" => Ok(CachePolicy::Private),
        "public" => Ok(CachePolicy::Public),
        "static" => Ok(CachePolicy::Static),
        other => Err(format!(
            "Expected no-store|private|public|static, got: {other}"
        )),
    }
}

// ---------------------------------------------------------------------------
// Script block parser
// ---------------------------------------------------------------------------

fn parse_script_block(content: &str, base_line: usize) -> ParseResult<ScriptBlock> {
    let mut script = ScriptBlock {
        uses: Vec::new(),
        props: Vec::new(),
        states: Vec::new(),
        memos: Vec::new(),
        loaders: Vec::new(),
        actions: Vec::new(),
        server_fns: Vec::new(),
        forms: Vec::new(),
        ai_actions: Vec::new(),
    };

    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;

    // Accumulated attributes for the next action
    let mut pending_permission: Option<String> = None;
    let mut pending_audit: Option<String> = None;

    while i < lines.len() {
        let line = lines[i].trim();

        if line.is_empty() || line.starts_with("//") {
            i += 1;
            continue;
        }

        // Attribute annotations: #[permission("...")] or #[audit("...")]
        if line.starts_with("#[") {
            if let Some(attr) = parse_attribute_annotation(line) {
                match attr.0.as_str() {
                    "permission" => pending_permission = Some(attr.1),
                    "audit" => pending_audit = Some(attr.1),
                    _ => {}
                }
            }
            i += 1;
            continue;
        }

        // `use` statement
        if line.starts_with("use ") {
            let path = line
                .trim_start_matches("use ")
                .trim_end_matches(';')
                .trim()
                .to_string();
            script.uses.push(UseStatement { path });
            i += 1;
            continue;
        }

        // `prop` declaration
        if line.starts_with("prop ") {
            script.props.push(parse_prop_decl(line, base_line + i)?);
            i += 1;
            continue;
        }

        // `state` declaration (with optional `secret`)
        if line.starts_with("state ") {
            script.states.push(parse_state_decl(line, base_line + i)?);
            i += 1;
            continue;
        }

        // `memo` declaration
        if line.starts_with("memo ") {
            script.memos.push(parse_memo_decl(line, base_line + i)?);
            i += 1;
            continue;
        }

        // `load` declaration (may span multiple lines with braces)
        if line.starts_with("load ") {
            let (loader, consumed) =
                parse_loader_decl(&lines, i, base_line)?;
            script.loaders.push(loader);
            i += consumed;
            continue;
        }

        // `action` declaration
        if line.starts_with("action ") {
            let (mut action, consumed) =
                parse_action_decl(&lines, i, base_line)?;
            action.permission = pending_permission.take();
            action.audit = pending_audit.take();
            script.actions.push(action);
            i += consumed;
            continue;
        }

        // `server` declaration
        if line.starts_with("server ") {
            let (server_fn, consumed) =
                parse_server_fn_decl(&lines, i, base_line)?;
            script.server_fns.push(server_fn);
            i += consumed;
            continue;
        }

        // `form` declaration
        if line.starts_with("form ") {
            let (form, consumed) =
                parse_form_decl(&lines, i, base_line)?;
            script.forms.push(form);
            i += consumed;
            continue;
        }

        // `ai action` declaration
        if line.starts_with("ai action ") || line.starts_with("ai action\t") {
            let (ai_action, consumed) =
                parse_ai_action_decl(&lines, i, base_line)?;
            script.ai_actions.push(ai_action);
            i += consumed;
            continue;
        }

        // Unknown line — skip with warning (lenient parsing)
        i += 1;
    }

    Ok(script)
}

fn parse_attribute_annotation(line: &str) -> Option<(String, String)> {
    // #[permission("customers.delete")]  or  #[audit("customer.deleted")]
    let inner = line.trim_start_matches("#[").trim_end_matches(']');
    let paren = inner.find('(')?;
    let name = inner[..paren].trim().to_string();
    let value_part = &inner[paren + 1..];
    let value = value_part.trim_end_matches(')');
    Some((name, unquote(value)))
}

fn parse_prop_decl(line: &str, _line_num: usize) -> ParseResult<PropDecl> {
    // prop name: Type = default
    // prop name: Type
    let rest = line.trim_start_matches("prop ").trim();
    let (name_ty, default) = split_default(rest);
    let (name, ty) = split_name_type(&name_ty)?;
    Ok(PropDecl {
        name,
        ty,
        default: default.map(|d| d.to_string()),
    })
}

fn parse_state_decl(line: &str, _line_num: usize) -> ParseResult<StateDecl> {
    // state name: Type = default
    // state secret name: Type
    let rest = line.trim_start_matches("state ").trim();
    let (secret, rest) = if rest.starts_with("secret ") {
        (true, rest.trim_start_matches("secret ").trim())
    } else {
        (false, rest)
    };
    let (name_ty, default) = split_default(rest);
    let (name, ty) = split_name_type(&name_ty)?;
    Ok(StateDecl {
        name,
        ty,
        default: default.map(|d| d.to_string()),
        secret,
    })
}

fn parse_memo_decl(line: &str, _line_num: usize) -> ParseResult<MemoDecl> {
    // memo name: Type = expr
    let rest = line.trim_start_matches("memo ").trim();
    let (name_ty, default) = split_default(rest);
    let (name, ty) = split_name_type(&name_ty)?;
    let expr = default.unwrap_or("").to_string();
    Ok(MemoDecl { name, ty, expr })
}

/// Split `name: Type = default` into `("name: Type", Some("default"))`.
fn split_default(s: &str) -> (&str, Option<&str>) {
    // Find `=` that is not inside angle brackets or parens
    let mut depth = 0i32;
    for (i, c) in s.char_indices() {
        match c {
            '<' | '(' | '[' => depth += 1,
            '>' | ')' | ']' => depth -= 1,
            '=' if depth == 0 => {
                let left = s[..i].trim();
                let right = s[i + 1..].trim();
                return (left, Some(right));
            }
            _ => {}
        }
    }
    (s, None)
}

/// Split `name: Type` into `(name, Type)`.
fn split_name_type(s: &str) -> ParseResult<(String, String)> {
    let colon = s.find(':').ok_or_else(|| ParseError::Syntax {
        line: 0,
        col: 0,
        message: format!("Expected `name: Type` in declaration, got: {s}"),
    })?;
    let name = s[..colon].trim().to_string();
    let ty = s[colon + 1..].trim().to_string();
    Ok((name, ty))
}

/// Parse a loader declaration spanning potentially multiple lines.
/// Returns (LoaderDecl, lines_consumed).
fn parse_loader_decl(
    lines: &[&str],
    start: usize,
    base_line: usize,
) -> ParseResult<(LoaderDecl, usize)> {
    let first = lines[start].trim();
    // load [async] fn name(params) { body }
    let rest = first.trim_start_matches("load ").trim();
    let (is_async, rest) = if rest.starts_with("async ") {
        (true, rest.trim_start_matches("async ").trim())
    } else {
        (false, rest)
    };
    let rest = rest.trim_start_matches("fn ").trim();

    // Extract function name
    let paren_pos = rest.find('(').unwrap_or(rest.len());
    let name = rest[..paren_pos].trim().to_string();

    // Extract params
    let params = if let Some(start_p) = rest.find('(') {
        if let Some(end_p) = rest.find(')') {
            parse_param_list(&rest[start_p + 1..end_p])?
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    // Extract body in braces (may span multiple lines)
    let (body, consumed) = extract_braced_body(lines, start, base_line)?;

    Ok((
        LoaderDecl {
            name,
            is_async,
            params,
            body,
        },
        consumed,
    ))
}

/// Parse an action declaration.
fn parse_action_decl(
    lines: &[&str],
    start: usize,
    base_line: usize,
) -> ParseResult<(ActionDecl, usize)> {
    let first = lines[start].trim();
    // action [async] [fn] name(params) { body }
    let rest = first.trim_start_matches("action ").trim();
    let (is_async, rest) = if rest.starts_with("async ") {
        (true, rest.trim_start_matches("async ").trim())
    } else {
        (false, rest)
    };
    let rest = rest.trim_start_matches("fn ").trim();

    let paren_pos = rest.find('(').unwrap_or(rest.len());
    let name = rest[..paren_pos].trim().to_string();

    let params = if let Some(start_p) = rest.find('(') {
        if let Some(end_p) = rest.find(')') {
            parse_param_list(&rest[start_p + 1..end_p])?
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    let (body, consumed) = extract_braced_body(lines, start, base_line)?;

    Ok((
        ActionDecl {
            name,
            is_async,
            params,
            permission: None,
            audit: None,
            body,
        },
        consumed,
    ))
}

/// Parse a server function declaration.
fn parse_server_fn_decl(
    lines: &[&str],
    start: usize,
    base_line: usize,
) -> ParseResult<(ServerFnDecl, usize)> {
    let first = lines[start].trim();
    let rest = first.trim_start_matches("server ").trim();
    let (is_async, rest) = if rest.starts_with("async ") {
        (true, rest.trim_start_matches("async ").trim())
    } else {
        (false, rest)
    };
    let rest = rest.trim_start_matches("fn ").trim();

    let paren_pos = rest.find('(').unwrap_or(rest.len());
    let name = rest[..paren_pos].trim().to_string();

    let params = if let Some(start_p) = rest.find('(') {
        if let Some(end_p) = rest.find(')') {
            parse_param_list(&rest[start_p + 1..end_p])?
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    let (body, consumed) = extract_braced_body(lines, start, base_line)?;

    Ok((
        ServerFnDecl {
            name,
            is_async,
            params,
            body,
        },
        consumed,
    ))
}

/// Parse a form schema declaration.
fn parse_form_decl(
    lines: &[&str],
    start: usize,
    _base_line: usize,
) -> ParseResult<(FormDecl, usize)> {
    let first = lines[start].trim();
    // form FormName {
    let rest = first.trim_start_matches("form ").trim();
    let brace = rest.find('{').unwrap_or(rest.len());
    let name = rest[..brace].trim().to_string();

    let mut fields = Vec::new();
    let mut i = start + 1;
    while i < lines.len() {
        let line = lines[i].trim();
        if line == "}" || line.starts_with('}') {
            i += 1;
            break;
        }
        if line.is_empty() || line.starts_with("//") {
            i += 1;
            continue;
        }
        fields.push(parse_form_field(line)?);
        i += 1;
    }

    Ok((FormDecl { name, fields }, i - start))
}

fn parse_form_field(line: &str) -> ParseResult<FormFieldDecl> {
    // name: Type constraint1 constraint2 ...
    let colon = line.find(':').ok_or_else(|| ParseError::Syntax {
        line: 0,
        col: 0,
        message: format!("Expected `name: Type constraints` in form field, got: {line}"),
    })?;
    let name = line[..colon].trim().to_string();
    let rest = line[colon + 1..].trim();

    // Split type from constraints — type is the first token (may include generics)
    let (ty, constraint_str) = split_type_and_constraints(rest);
    let constraints = parse_field_constraints(&constraint_str);

    Ok(FormFieldDecl {
        name,
        ty,
        constraints,
    })
}

/// Parse an `ai action` declaration.
fn parse_ai_action_decl(
    lines: &[&str],
    start: usize,
    _base_line: usize,
) -> ParseResult<(AiActionDecl, usize)> {
    let first = lines[start].trim();
    // ai action name(input: Type) -> ReturnType {
    let rest = first.trim_start_matches("ai action ").trim();

    // Parse name
    let paren_pos = rest.find('(').unwrap_or(rest.len());
    let name = rest[..paren_pos].trim().to_string();

    // Parse input param
    let (input_param, input_type) = if let Some(p_start) = rest.find('(') {
        if let Some(p_end) = rest.find(')') {
            let inner = &rest[p_start + 1..p_end];
            if let Some(colon) = inner.find(':') {
                (
                    inner[..colon].trim().to_string(),
                    inner[colon + 1..].trim().to_string(),
                )
            } else {
                (inner.trim().to_string(), String::new())
            }
        } else {
            (String::new(), String::new())
        }
    } else {
        (String::new(), String::new())
    };

    // Parse return type
    let return_type = if let Some(arrow) = rest.find("->") {
        let after_arrow = &rest[arrow + 2..];
        let brace = after_arrow.find('{').unwrap_or(after_arrow.len());
        after_arrow[..brace].trim().to_string()
    } else {
        String::new()
    };

    // Parse body key-value pairs
    let mut model = String::new();
    let mut fallback = None;
    let mut temperature = None;
    let mut audit = false;
    let mut pii = None;
    let mut permission = None;
    // Also support `input:` field in body for the form used in lesson-tracker example
    let mut body_input_param = input_param.clone();

    let mut i = start + 1;
    while i < lines.len() {
        let line = lines[i].trim();
        if line == "}" || line.starts_with('}') {
            i += 1;
            break;
        }
        if line.is_empty() || line.starts_with("//") {
            i += 1;
            continue;
        }

        if let Some((key, value)) = parse_kv_line(line) {
            let v = unquote(&value);
            match key.as_str() {
                "model" => model = v,
                "fallback" => fallback = Some(v),
                "temperature" => temperature = v.parse::<f64>().ok(),
                "audit" => audit = v == "true",
                "pii" => pii = Some(v),
                "permission" => permission = Some(v),
                "input" => body_input_param = v,
                _ => {}
            }
        }
        i += 1;
    }

    Ok((
        AiActionDecl {
            name,
            input_param: body_input_param,
            input_type,
            return_type,
            model,
            fallback,
            temperature,
            audit,
            pii,
            permission,
        },
        i - start,
    ))
}

fn parse_param_list(s: &str) -> ParseResult<Vec<ParamDecl>> {
    let s = s.trim();
    if s.is_empty() {
        return Ok(Vec::new());
    }
    let mut params = Vec::new();
    // Split on commas, respecting angle brackets
    for part in split_params(s) {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let (name, ty) = split_name_type(part)?;
        params.push(ParamDecl { name, ty });
    }
    Ok(params)
}

fn split_params(s: &str) -> Vec<&str> {
    let mut result = Vec::new();
    let mut depth = 0i32;
    let mut last = 0;
    for (i, c) in s.char_indices() {
        match c {
            '<' | '(' | '[' => depth += 1,
            '>' | ')' | ']' => depth -= 1,
            ',' if depth == 0 => {
                result.push(&s[last..i]);
                last = i + 1;
            }
            _ => {}
        }
    }
    result.push(&s[last..]);
    result
}

/// Extract a braced body `{ ... }` that may span multiple lines.
/// Returns (body_content, lines_consumed).
fn extract_braced_body(
    lines: &[&str],
    start: usize,
    base_line: usize,
) -> ParseResult<(String, usize)> {
    let mut depth = 0i32;
    let mut body_lines = Vec::new();
    let mut found_open = false;
    let mut i = start;

    while i < lines.len() {
        let line = lines[i];
        for c in line.chars() {
            match c {
                '{' => {
                    if !found_open {
                        found_open = true;
                    }
                    depth += 1;
                }
                '}' => {
                    depth -= 1;
                }
                _ => {}
            }
        }

        if found_open {
            body_lines.push(line);
        }

        i += 1;

        if found_open && depth == 0 {
            break;
        }
    }

    if !found_open {
        return Err(ParseError::Syntax {
            line: base_line + start,
            col: 1,
            message: "Expected `{` to start body".into(),
        });
    }

    if depth != 0 {
        return Err(ParseError::UnclosedBlock("function body".into()));
    }

    // Strip the outer braces from body content
    let joined = body_lines.join("\n");
    let body = strip_outer_braces(&joined);

    Ok((body.trim().to_string(), i - start))
}

fn strip_outer_braces(s: &str) -> &str {
    let s = s.trim();
    if let Some(open) = s.find('{') {
        if let Some(close) = s.rfind('}') {
            if close > open {
                return &s[open + 1..close];
            }
        }
    }
    s
}

fn split_type_and_constraints(s: &str) -> (String, String) {
    // The type might be something like `Option<String>`, `Enum[active, inactive, blocked]`, etc.
    // Constraints come after the type: required, optional, max=120, min=2, unique, searchable, readonly
    let constraint_keywords = [
        "required",
        "optional",
        "unique",
        "searchable",
        "readonly",
        "min=",
        "max=",
        "default=",
    ];

    // Find where constraints start by looking for the first constraint keyword
    // that appears at a word boundary after the type
    let tokens: Vec<&str> = tokenize_type_constraints(s);

    let mut type_end = 0;
    let mut found_constraint = false;
    for (idx, token) in tokens.iter().enumerate() {
        let is_constraint = constraint_keywords
            .iter()
            .any(|kw| token.starts_with(kw));
        if is_constraint {
            found_constraint = true;
            type_end = idx;
            break;
        }
    }

    if !found_constraint {
        return (s.trim().to_string(), String::new());
    }

    let ty = tokens[..type_end].join(" ");
    let constraints = tokens[type_end..].join(" ");
    (ty.trim().to_string(), constraints.trim().to_string())
}

/// Tokenize a string into type and constraint tokens, respecting brackets.
fn tokenize_type_constraints(s: &str) -> Vec<&str> {
    let mut tokens = Vec::new();
    let mut depth = 0i32;
    let mut start = 0;
    let bytes = s.as_bytes();

    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'<' | b'(' | b'[' => depth += 1,
            b'>' | b')' | b']' => depth -= 1,
            b' ' | b'\t' if depth == 0 => {
                let token = &s[start..i];
                if !token.trim().is_empty() {
                    tokens.push(token.trim());
                }
                start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    let last = &s[start..];
    if !last.trim().is_empty() {
        tokens.push(last.trim());
    }
    tokens
}

fn parse_field_constraints(s: &str) -> Vec<FieldConstraint> {
    let mut constraints = Vec::new();
    for token in s.split_whitespace() {
        match token {
            "required" => constraints.push(FieldConstraint::Required),
            "optional" => constraints.push(FieldConstraint::Optional),
            "unique" => constraints.push(FieldConstraint::Unique),
            "searchable" => constraints.push(FieldConstraint::Searchable),
            "readonly" => constraints.push(FieldConstraint::Readonly),
            _ if token.starts_with("min=") => {
                if let Ok(n) = token[4..].parse() {
                    constraints.push(FieldConstraint::Min(n));
                }
            }
            _ if token.starts_with("max=") => {
                if let Ok(n) = token[4..].parse() {
                    constraints.push(FieldConstraint::Max(n));
                }
            }
            _ => {}
        }
    }
    constraints
}

// ---------------------------------------------------------------------------
// Template block parser
// ---------------------------------------------------------------------------

struct TemplateParser<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> TemplateParser<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }

    fn remaining(&self) -> &'a str {
        &self.input[self.pos..]
    }

    fn is_eof(&self) -> bool {
        self.pos >= self.input.len()
    }

    fn peek_char(&self) -> Option<char> {
        self.remaining().chars().next()
    }

    fn advance(&mut self, n: usize) {
        self.pos = (self.pos + n).min(self.input.len());
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek_char() {
            if c.is_whitespace() {
                self.advance(c.len_utf8());
            } else {
                break;
            }
        }
    }

    fn starts_with(&self, s: &str) -> bool {
        self.remaining().starts_with(s)
    }

    fn parse_children(&mut self, stop_conditions: &[&str]) -> ParseResult<Vec<TemplateNode>> {
        let mut nodes = Vec::new();

        while !self.is_eof() {
            // Check stop conditions
            let trimmed = self.remaining();
            for &stop in stop_conditions {
                if trimmed.trim_start().starts_with(stop) {
                    // Consume leading whitespace before the stop marker
                    self.skip_whitespace();
                    return Ok(nodes);
                }
            }

            if let Some(node) = self.parse_node(stop_conditions)? {
                nodes.push(node);
            }
        }

        Ok(nodes)
    }

    fn parse_node(&mut self, stop_conditions: &[&str]) -> ParseResult<Option<TemplateNode>> {
        if self.is_eof() {
            return Ok(None);
        }

        // Check stop conditions before parsing
        let trimmed_remaining = self.remaining().trim_start();
        for &stop in stop_conditions {
            if trimmed_remaining.starts_with(stop) {
                return Ok(None);
            }
        }

        let remaining = self.remaining();

        // Expression: {#if ...}, {#each ...}, {#match ...}, {#can ...}, {:else...}, {/...},
        //             {@html ...}, {expr}
        if remaining.starts_with('{') {
            // Check for control flow
            let after_brace = &remaining[1..];
            if after_brace.starts_with("#if ") || after_brace.starts_with("#if\t") {
                return self.parse_if_block().map(Some);
            }
            if after_brace.starts_with("#each ") || after_brace.starts_with("#each\t") {
                return self.parse_each_block().map(Some);
            }
            if after_brace.starts_with("#match ") || after_brace.starts_with("#match\t") {
                return self.parse_match_block().map(Some);
            }
            if after_brace.starts_with("#can ") || after_brace.starts_with("#can\t") {
                return self.parse_can_block().map(Some);
            }
            if after_brace.starts_with("@html ") || after_brace.starts_with("@html\t") {
                return self.parse_unsafe_html().map(Some);
            }
            // Stop markers — these are handled by the parent
            if after_brace.starts_with(":else")
                || after_brace.starts_with("/if")
                || after_brace.starts_with("/each")
                || after_brace.starts_with("/match")
                || after_brace.starts_with("/can")
            {
                return Ok(None);
            }
            // Plain expression
            return self.parse_expression().map(Some);
        }

        // HTML comment
        if remaining.starts_with("<!--") {
            if let Some(end) = remaining.find("-->") {
                self.advance(end + 3);
                return Ok(None); // skip comments
            }
        }

        // Element or component
        if remaining.starts_with('<') {
            // Closing tag — stop parsing children
            if remaining.starts_with("</") {
                return Ok(None);
            }
            return self.parse_element_or_component().map(Some);
        }

        // Text node
        self.parse_text(stop_conditions).map(|t| {
            if t.is_empty() {
                None
            } else {
                Some(TemplateNode::Text(t))
            }
        })
    }

    fn parse_expression(&mut self) -> ParseResult<TemplateNode> {
        // {expr}
        self.advance(1); // skip `{`
        let expr = self.consume_until_balanced_brace()?;
        Ok(TemplateNode::Expression(ExprNode {
            expr: expr.trim().to_string(),
            id: None,
        }))
    }

    fn parse_unsafe_html(&mut self) -> ParseResult<TemplateNode> {
        // {@html expr}
        self.advance(1); // skip `{`
        let content = self.consume_until_balanced_brace()?;
        let expr = content
            .trim_start_matches("@html")
            .trim()
            .to_string();
        Ok(TemplateNode::UnsafeHtml(expr))
    }

    fn parse_if_block(&mut self) -> ParseResult<TemplateNode> {
        // {#if condition}
        self.advance(1); // skip `{`
        let tag_content = self.consume_until_balanced_brace()?;
        let condition = tag_content
            .trim_start_matches("#if")
            .trim()
            .to_string();

        // Parse then branch children — stop at {:else if}, {:else}, {/if}
        let then_branch = self.parse_children(&["{:else if", "{:else}", "{/if}"])?;

        // Parse else-if branches
        let mut else_if_branches = Vec::new();
        while self.remaining().trim_start().starts_with("{:else if") {
            self.skip_whitespace();
            self.advance(1); // `{`
            let tag = self.consume_until_balanced_brace()?;
            let cond = tag
                .trim_start_matches(":else if")
                .trim()
                .to_string();
            let branch = self.parse_children(&["{:else if", "{:else}", "{/if}"])?;
            else_if_branches.push((cond, branch));
        }

        // Parse else branch
        let else_branch = if self.remaining().trim_start().starts_with("{:else}") {
            self.skip_whitespace();
            self.advance(1); // `{`
            let _ = self.consume_until_balanced_brace()?;
            let branch = self.parse_children(&["{/if}"])?;
            Some(branch)
        } else {
            None
        };

        // Consume {/if}
        self.skip_whitespace();
        if self.remaining().starts_with("{/if}") {
            self.advance(5);
        }

        Ok(TemplateNode::If(IfNode {
            condition,
            then_branch,
            else_if_branches,
            else_branch,
        }))
    }

    fn parse_each_block(&mut self) -> ParseResult<TemplateNode> {
        // {#each items as item}  or  {#each items as item, index}
        self.advance(1); // skip `{`
        let tag_content = self.consume_until_balanced_brace()?;
        let inner = tag_content.trim_start_matches("#each").trim();

        let (iterable, item, index) = if let Some(as_pos) = inner.find(" as ") {
            let iterable = inner[..as_pos].trim().to_string();
            let binding = inner[as_pos + 4..].trim();
            if let Some(comma) = binding.find(',') {
                let item = binding[..comma].trim().to_string();
                let index = binding[comma + 1..].trim().to_string();
                (iterable, item, Some(index))
            } else {
                (iterable, binding.to_string(), None)
            }
        } else {
            (inner.to_string(), "item".to_string(), None)
        };

        let children = self.parse_children(&["{/each}"])?;

        self.skip_whitespace();
        if self.remaining().starts_with("{/each}") {
            self.advance(7);
        }

        Ok(TemplateNode::Each(EachNode {
            iterable,
            item,
            index,
            children,
        }))
    }

    fn parse_match_block(&mut self) -> ParseResult<TemplateNode> {
        // {#match expr}
        self.advance(1); // skip `{`
        let tag_content = self.consume_until_balanced_brace()?;
        let expr = tag_content
            .trim_start_matches("#match")
            .trim()
            .to_string();

        let mut arms = Vec::new();
        // Parse arms: {:when pattern} ... children ...
        loop {
            self.skip_whitespace();
            if self.remaining().starts_with("{/match}") {
                self.advance(8);
                break;
            }
            if self.remaining().starts_with("{:when") {
                self.advance(1); // `{`
                let tag = self.consume_until_balanced_brace()?;
                let pattern = tag
                    .trim_start_matches(":when")
                    .trim()
                    .to_string();
                let children = self.parse_children(&["{:when", "{/match}"])?;
                arms.push((pattern, children));
            } else if self.is_eof() {
                break;
            } else {
                // Skip unexpected content
                self.advance(1);
            }
        }

        Ok(TemplateNode::Match(MatchNode { expr, arms }))
    }

    fn parse_can_block(&mut self) -> ParseResult<TemplateNode> {
        // {#can "permission"}
        self.advance(1); // skip `{`
        let tag_content = self.consume_until_balanced_brace()?;
        let perm_str = tag_content.trim_start_matches("#can").trim();
        let permission = unquote(perm_str);

        let children = self.parse_children(&["{/can}"])?;

        self.skip_whitespace();
        if self.remaining().starts_with("{/can}") {
            self.advance(6);
        }

        Ok(TemplateNode::Can(CanNode {
            permission,
            children,
        }))
    }

    fn parse_element_or_component(&mut self) -> ParseResult<TemplateNode> {
        self.advance(1); // skip `<`

        // Parse tag name
        let tag_name = self.consume_ident_or_tag();

        // Determine if this is a component (starts with uppercase) or element
        let is_component = tag_name
            .chars()
            .next()
            .map(|c| c.is_uppercase())
            .unwrap_or(false);

        // Special handling for slot
        if tag_name == "slot" {
            return self.parse_slot_element();
        }

        // Special handling for error-boundary
        if tag_name == "error-boundary" {
            return self.parse_error_boundary();
        }

        // Parse attributes, events, bindings
        let mut attributes = Vec::new();
        let mut events = Vec::new();
        let mut bindings = Vec::new();
        let mut is_island = false;
        let mut is_self_closing = false;

        loop {
            self.skip_whitespace();
            if self.is_eof() {
                break;
            }

            // Self-closing: />
            if self.starts_with("/>") {
                self.advance(2);
                is_self_closing = true;
                break;
            }

            // End of opening tag: >
            if self.starts_with(">") {
                self.advance(1);
                break;
            }

            // `island` flag (for components)
            if self.starts_with("island") {
                let after = &self.remaining()[6..];
                if after.starts_with(' ')
                    || after.starts_with('>')
                    || after.starts_with('/')
                    || after.starts_with('\n')
                    || after.starts_with('\t')
                {
                    is_island = true;
                    self.advance(6);
                    continue;
                }
            }

            // Event binding: on:event.modifier="handler"
            if self.starts_with("on:") {
                events.push(self.parse_event_binding()?);
                continue;
            }

            // Two-way binding: bind:kind="target"
            if self.starts_with("bind:") {
                bindings.push(self.parse_binding_decl()?);
                continue;
            }

            // Regular attribute: name="value" or name={expr}
            if let Some(attr) = self.try_parse_attribute()? {
                attributes.push(attr);
                continue;
            }

            // Safety: advance past unexpected character
            self.advance(1);
        }

        if is_component {
            // Parse children if not self-closing
            let children = if is_self_closing {
                Vec::new()
            } else {
                let close_tag = format!("</{tag_name}>");
                let children = self.parse_children_until_close(&tag_name)?;
                // Consume closing tag
                self.skip_whitespace();
                if self.remaining().starts_with(&close_tag) {
                    self.advance(close_tag.len());
                }
                children
            };

            Ok(TemplateNode::Component(ComponentNode {
                name: tag_name,
                props: attributes,
                events,
                bindings,
                children,
                is_island,
            }))
        } else {
            // HTML element
            let children = if is_self_closing || is_void_element(&tag_name) {
                Vec::new()
            } else {
                let children = self.parse_children_until_close(&tag_name)?;
                // Consume closing tag
                self.skip_whitespace();
                let close_tag = format!("</{tag_name}>");
                if self.remaining().starts_with(&close_tag) {
                    self.advance(close_tag.len());
                }
                children
            };

            Ok(TemplateNode::Element(ElementNode {
                tag: tag_name,
                attributes,
                events,
                bindings,
                children,
                self_closing: is_self_closing,
            }))
        }
    }

    fn parse_slot_element(&mut self) -> ParseResult<TemplateNode> {
        // Already consumed `<slot` — parse attributes and decide self-closing vs children
        let mut name: Option<String> = None;
        let mut is_self_closing = false;

        loop {
            self.skip_whitespace();
            if self.is_eof() {
                break;
            }
            if self.starts_with("/>") {
                self.advance(2);
                is_self_closing = true;
                break;
            }
            if self.starts_with(">") {
                self.advance(1);
                break;
            }
            if let Some(attr) = self.try_parse_attribute()? {
                if attr.name == "name" {
                    name = Some(match attr.value {
                        AttributeValue::Static(s) => s,
                        AttributeValue::Dynamic(s) => s,
                        AttributeValue::None => String::new(),
                    });
                }
                continue;
            }
            self.advance(1);
        }

        let fallback = if is_self_closing {
            Vec::new()
        } else {
            let children = self.parse_children_until_close("slot")?;
            self.skip_whitespace();
            if self.remaining().starts_with("</slot>") {
                self.advance(7);
            }
            children
        };

        Ok(TemplateNode::Slot(SlotNode { name, fallback }))
    }

    fn parse_error_boundary(&mut self) -> ParseResult<TemplateNode> {
        // Already consumed `<error-boundary` — skip to `>`
        loop {
            self.skip_whitespace();
            if self.is_eof() {
                break;
            }
            if self.starts_with(">") {
                self.advance(1);
                break;
            }
            self.advance(1);
        }

        let children = self.parse_children_until_close("error-boundary")?;
        self.skip_whitespace();
        if self.remaining().starts_with("</error-boundary>") {
            self.advance(17);
        }

        Ok(TemplateNode::ErrorBoundary(ErrorBoundaryNode {
            error_template: None,
            children,
        }))
    }

    fn parse_children_until_close(&mut self, tag_name: &str) -> ParseResult<Vec<TemplateNode>> {
        let close_tag = format!("</{tag_name}>");
        let mut nodes = Vec::new();

        while !self.is_eof() {
            // Check for our closing tag (allowing leading whitespace)
            let trimmed = self.remaining().trim_start();
            if trimmed.starts_with(&close_tag) {
                // Consume the whitespace before the close tag
                self.skip_whitespace();
                return Ok(nodes);
            }

            // Check for any other closing tag — means our parent or sibling is closing
            if trimmed.starts_with("</") {
                self.skip_whitespace();
                return Ok(nodes);
            }

            let before = self.pos;

            if let Some(node) = self.parse_node(&[])? {
                nodes.push(node);
            }

            // If no progress was made, advance to prevent infinite loop
            if self.pos == before {
                self.advance(1);
            }
        }

        Ok(nodes)
    }

    fn parse_event_binding(&mut self) -> ParseResult<EventBinding> {
        // on:event.modifier1.modifier2="handler"
        self.advance(3); // skip `on:`

        // Read event name and modifiers (until `=` or whitespace or `>` or `/`)
        let mut name_mods = String::new();
        while let Some(c) = self.peek_char() {
            if c == '=' || c.is_whitespace() || c == '>' || c == '/' {
                break;
            }
            name_mods.push(c);
            self.advance(c.len_utf8());
        }

        // Read handler value
        let handler = if self.starts_with("=") {
            self.advance(1); // skip `=`
            self.skip_whitespace();
            if self.starts_with("\"") {
                self.advance(1);
                let val = self.consume_until_char('"');
                self.advance(1); // skip closing `"`
                val
            } else if self.starts_with("'") {
                self.advance(1);
                let val = self.consume_until_char('\'');
                self.advance(1);
                val
            } else {
                // Bare value
                let mut val = String::new();
                while let Some(c) = self.peek_char() {
                    if c.is_whitespace() || c == '>' || c == '/' {
                        break;
                    }
                    val.push(c);
                    self.advance(c.len_utf8());
                }
                val
            }
        } else {
            String::new()
        };

        let parts: Vec<&str> = name_mods.split('.').collect();
        let event = parts.first().unwrap_or(&"").to_string();
        let mut modifiers = Vec::new();

        let mut i = 1;
        while i < parts.len() {
            match parts[i] {
                "prevent" => modifiers.push(EventModifier::Prevent),
                "stop" => modifiers.push(EventModifier::Stop),
                "debounce" => {
                    if i + 1 < parts.len() {
                        if let Ok(ms) = parts[i + 1].parse::<u32>() {
                            modifiers.push(EventModifier::Debounce(ms));
                            i += 1;
                        }
                    }
                }
                "throttle" => {
                    if i + 1 < parts.len() {
                        if let Ok(ms) = parts[i + 1].parse::<u32>() {
                            modifiers.push(EventModifier::Throttle(ms));
                            i += 1;
                        }
                    }
                }
                _ => {}
            }
            i += 1;
        }

        Ok(EventBinding {
            event,
            handler,
            modifiers,
        })
    }

    fn parse_binding_decl(&mut self) -> ParseResult<BindingDecl> {
        // bind:value="target"
        self.advance(5); // skip `bind:`

        // Read binding kind (until `=` or whitespace or `>` or `/`)
        let mut kind = String::new();
        while let Some(c) = self.peek_char() {
            if c == '=' || c.is_whitespace() || c == '>' || c == '/' {
                break;
            }
            kind.push(c);
            self.advance(c.len_utf8());
        }

        // Read target value
        let target = if self.starts_with("=") {
            self.advance(1);
            self.skip_whitespace();
            if self.starts_with("\"") {
                self.advance(1);
                let val = self.consume_until_char('"');
                self.advance(1);
                val
            } else if self.starts_with("'") {
                self.advance(1);
                let val = self.consume_until_char('\'');
                self.advance(1);
                val
            } else {
                let mut val = String::new();
                while let Some(c) = self.peek_char() {
                    if c.is_whitespace() || c == '>' || c == '/' {
                        break;
                    }
                    val.push(c);
                    self.advance(c.len_utf8());
                }
                val
            }
        } else {
            String::new()
        };

        Ok(BindingDecl { kind, target })
    }

    fn try_parse_attribute(&mut self) -> ParseResult<Option<Attribute>> {
        let remaining = self.remaining();
        if remaining.is_empty()
            || remaining.starts_with('>')
            || remaining.starts_with("/>")
        {
            return Ok(None);
        }

        // Peek: must start with an ident char
        let first = remaining.chars().next().unwrap();
        if !first.is_alphanumeric() && first != '_' && first != '#' {
            return Ok(None);
        }

        let name = self.consume_ident_or_tag();
        if name.is_empty() {
            return Ok(None);
        }

        self.skip_whitespace();

        if self.starts_with("=") {
            self.advance(1); // skip `=`
            self.skip_whitespace();

            let value = if self.starts_with("{") {
                self.advance(1);
                let expr = self.consume_until_balanced_brace()?;
                AttributeValue::Dynamic(expr.trim().to_string())
            } else if self.starts_with("\"") {
                self.advance(1);
                let val = self.consume_until_char('"');
                self.advance(1); // skip closing `"`
                // Check if the static value contains dynamic expressions like "badge-{tone}"
                if val.contains('{') {
                    AttributeValue::Dynamic(val)
                } else {
                    AttributeValue::Static(val)
                }
            } else if self.starts_with("'") {
                self.advance(1);
                let val = self.consume_until_char('\'');
                self.advance(1); // skip closing `'`
                AttributeValue::Static(val)
            } else {
                // Bare value (until whitespace or `>`)
                let val = self.consume_until_attr_end();
                AttributeValue::Static(val)
            };

            Ok(Some(Attribute { name, value }))
        } else {
            // Boolean attribute (no value)
            Ok(Some(Attribute {
                name,
                value: AttributeValue::None,
            }))
        }
    }

    fn parse_text(&mut self, _stop_conditions: &[&str]) -> ParseResult<String> {
        let mut text = String::new();
        while !self.is_eof() {
            let c = self.remaining().chars().next().unwrap();
            if c == '<' || c == '{' {
                break;
            }
            text.push(c);
            self.advance(c.len_utf8());
        }

        // Normalize whitespace in text nodes
        let trimmed = text.trim();
        if trimmed.is_empty() {
            Ok(String::new())
        } else {
            Ok(trimmed.to_string())
        }
    }

    fn consume_ident_or_tag(&mut self) -> String {
        let mut ident = String::new();
        while let Some(c) = self.peek_char() {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                ident.push(c);
                self.advance(c.len_utf8());
            } else {
                break;
            }
        }
        ident
    }

    fn consume_until_balanced_brace(&mut self) -> ParseResult<String> {
        let mut content = String::new();
        let mut depth = 1i32;

        while !self.is_eof() {
            let c = self.remaining().chars().next().unwrap();
            match c {
                '{' => {
                    depth += 1;
                    content.push(c);
                    self.advance(1);
                }
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        self.advance(1); // skip closing `}`
                        return Ok(content);
                    }
                    content.push(c);
                    self.advance(1);
                }
                _ => {
                    content.push(c);
                    self.advance(c.len_utf8());
                }
            }
        }

        Err(ParseError::UnexpectedToken(
            "Unclosed `{` in template expression".into(),
        ))
    }

    fn consume_until_char(&mut self, target: char) -> String {
        let mut content = String::new();
        while !self.is_eof() {
            let c = self.remaining().chars().next().unwrap();
            if c == target {
                return content;
            }
            content.push(c);
            self.advance(c.len_utf8());
        }
        content
    }

    fn consume_until_attr_end(&mut self) -> String {
        let mut content = String::new();
        // Attribute value is quoted or unquoted
        // If it starts with a quote, consume until the matching quote
        if self.starts_with("\"") {
            self.advance(1);
            let val = self.consume_until_char('"');
            self.advance(1);
            return val;
        }
        if self.starts_with("'") {
            self.advance(1);
            let val = self.consume_until_char('\'');
            self.advance(1);
            return val;
        }
        // Unquoted: consume until whitespace, >, or /
        while let Some(c) = self.peek_char() {
            if c.is_whitespace() || c == '>' || c == '/' {
                break;
            }
            content.push(c);
            self.advance(c.len_utf8());
        }
        content
    }
}

fn parse_template_block(content: &str, _base_line: usize) -> ParseResult<TemplateBlock> {
    let mut parser = TemplateParser::new(content);
    let children = parser.parse_children(&[])?;
    Ok(TemplateBlock { children })
}

/// HTML void elements that cannot have children.
fn is_void_element(tag: &str) -> bool {
    matches!(
        tag,
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}

// ---------------------------------------------------------------------------
// Resource block parser
// ---------------------------------------------------------------------------

fn parse_resource_block(
    attrs: &str,
    content: &str,
    base_line: usize,
) -> ParseResult<ResourceBlock> {
    // Parse attrs: name="Customer" table="customers"
    let attr_pairs = parse_tag_attrs(attrs);
    let name = attr_pairs
        .iter()
        .find(|(k, _)| k == "name")
        .map(|(_, v)| v.clone())
        .ok_or_else(|| ParseError::MissingField {
            block: "resource".into(),
            field: "name".into(),
        })?;
    let table = attr_pairs
        .iter()
        .find(|(k, _)| k == "table")
        .map(|(_, v)| v.clone())
        .ok_or_else(|| ParseError::MissingField {
            block: "resource".into(),
            field: "table".into(),
        })?;

    let mut tenant = TenantLevel::None;
    let mut primary_key = "id".to_string();
    let mut fields = Vec::new();
    let mut permissions = Vec::new();

    for (i, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("//") {
            continue;
        }

        if line.starts_with("tenant:") {
            let val = line.trim_start_matches("tenant:").trim();
            tenant = parse_tenant_level(val).map_err(|reason| ParseError::InvalidValue {
                field: "tenant".into(),
                value: val.to_string(),
                reason,
            })?;
            continue;
        }

        if line.starts_with("primary_key:") {
            primary_key = line
                .trim_start_matches("primary_key:")
                .trim()
                .to_string();
            continue;
        }

        if line.starts_with("field ") {
            fields.push(parse_resource_field(line, base_line + i)?);
            continue;
        }

        if line.starts_with("permission ") {
            permissions.push(parse_resource_permission(line, base_line + i)?);
            continue;
        }
    }

    Ok(ResourceBlock {
        name,
        table,
        tenant,
        primary_key,
        fields,
        permissions,
    })
}

fn parse_resource_field(line: &str, _line_num: usize) -> ParseResult<ResourceField> {
    // field name: Type constraint1 constraint2 ... [default=value]
    let rest = line.trim_start_matches("field ").trim();
    let colon = rest.find(':').ok_or_else(|| ParseError::Syntax {
        line: _line_num,
        col: 1,
        message: format!("Expected `field name: Type constraints` got: {line}"),
    })?;
    let name = rest[..colon].trim().to_string();
    let type_and_rest = rest[colon + 1..].trim();

    let (ty, constraint_str) = split_type_and_constraints(type_and_rest);
    let constraints = parse_field_constraints(&constraint_str);

    let searchable = constraints.contains(&FieldConstraint::Searchable);
    let readonly = constraints.contains(&FieldConstraint::Readonly);

    // Extract default value from constraints
    let default = extract_default_from_rest(type_and_rest);

    Ok(ResourceField {
        name,
        ty,
        constraints,
        searchable,
        readonly,
        default,
    })
}

fn extract_default_from_rest(s: &str) -> Option<String> {
    for token in s.split_whitespace() {
        if let Some(val) = token.strip_prefix("default=") {
            return Some(val.to_string());
        }
    }
    None
}

fn parse_resource_permission(line: &str, _line_num: usize) -> ParseResult<ResourcePermission> {
    // permission action: "perm.string"
    let rest = line.trim_start_matches("permission ").trim();
    if let Some((action, perm)) = parse_kv_line(rest) {
        Ok(ResourcePermission {
            action,
            permission: unquote(&perm),
        })
    } else {
        Err(ParseError::Syntax {
            line: _line_num,
            col: 1,
            message: format!("Expected `permission action: \"perm\"`, got: {line}"),
        })
    }
}

// ---------------------------------------------------------------------------
// Layout block parser
// ---------------------------------------------------------------------------

fn parse_layout_block(
    attrs: &str,
    content: &str,
    _base_line: usize,
) -> ParseResult<LayoutBlock> {
    let attr_pairs = parse_tag_attrs(attrs);
    let name = attr_pairs
        .iter()
        .find(|(k, _)| k == "name")
        .map(|(_, v)| v.clone())
        .unwrap_or_default();

    let mut auth = None;
    let mut tenant = None;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("//") {
            continue;
        }
        if let Some((key, value)) = parse_kv_line(line) {
            let v = unquote(&value);
            match key.as_str() {
                "auth" => {
                    auth = Some(parse_auth_level(&v).map_err(|reason| {
                        ParseError::InvalidValue {
                            field: "auth".into(),
                            value: v.clone(),
                            reason,
                        }
                    })?);
                }
                "tenant" => {
                    tenant = Some(parse_tenant_level(&v).map_err(|reason| {
                        ParseError::InvalidValue {
                            field: "tenant".into(),
                            value: v.clone(),
                            reason,
                        }
                    })?);
                }
                // Silently ignore other fields in layout — they may be future extensions
                _ => {}
            }
        }
    }

    Ok(LayoutBlock {
        name,
        auth,
        tenant,
    })
}

// ---------------------------------------------------------------------------
// Utilities
// ---------------------------------------------------------------------------

/// Parse tag attributes like `name="Customer" table="customers"` into pairs.
fn parse_tag_attrs(s: &str) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    let mut remaining = s.trim();

    while !remaining.is_empty() {
        // Skip whitespace
        remaining = remaining.trim_start();
        if remaining.is_empty() {
            break;
        }

        // Read attribute name
        let name_end = remaining
            .find(|c: char| c == '=' || c.is_whitespace())
            .unwrap_or(remaining.len());
        let name = remaining[..name_end].to_string();
        remaining = remaining[name_end..].trim_start();

        if remaining.starts_with('=') {
            remaining = remaining[1..].trim_start();
            // Read value
            if remaining.starts_with('"') {
                remaining = &remaining[1..];
                let end = remaining.find('"').unwrap_or(remaining.len());
                let value = remaining[..end].to_string();
                remaining = if end < remaining.len() {
                    &remaining[end + 1..]
                } else {
                    ""
                };
                pairs.push((name, value));
            } else if remaining.starts_with('\'') {
                remaining = &remaining[1..];
                let end = remaining.find('\'').unwrap_or(remaining.len());
                let value = remaining[..end].to_string();
                remaining = if end < remaining.len() {
                    &remaining[end + 1..]
                } else {
                    ""
                };
                pairs.push((name, value));
            } else {
                // Unquoted value
                let end = remaining
                    .find(char::is_whitespace)
                    .unwrap_or(remaining.len());
                let value = remaining[..end].to_string();
                remaining = &remaining[end..];
                pairs.push((name, value));
            }
        } else {
            // Boolean attribute (no value)
            pairs.push((name, String::new()));
        }
    }

    pairs
}
