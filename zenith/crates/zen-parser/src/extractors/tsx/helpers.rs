use ast_grep_core::Node;

// ── Naming conventions ─────────────────────────────────────────────

/// React hook: starts with `use` followed by uppercase letter.
pub(super) fn is_hook_name(name: &str) -> bool {
    name.starts_with("use")
        && name.len() > 3
        && name[3..4].chars().next().is_some_and(char::is_uppercase)
}

/// HOC: starts with `with` followed by uppercase letter.
pub(super) fn is_hoc_name(name: &str) -> bool {
    name.starts_with("with")
        && name.len() > 4
        && name[4..5].chars().next().is_some_and(char::is_uppercase)
}

/// Component: starts with uppercase letter.
pub(super) fn is_component_name(name: &str) -> bool {
    name.chars().next().is_some_and(char::is_uppercase)
}

/// Check if a return type string indicates a React component.
pub(super) fn is_component_return_type(rt: Option<&str>) -> bool {
    rt.is_some_and(|t| {
        t.contains("JSX.Element")
            || t.contains("ReactNode")
            || t.contains("ReactElement")
            || t.contains("React.FC")
            || t.contains("React.FunctionComponent")
    })
}

// ── Props type extraction ──────────────────────────────────────────

/// Extract props type from function parameters.
///
/// Matches `({ ... }: PropsType)` in `formal_parameters`.
pub(super) fn extract_props_type_from_params<D: ast_grep_core::Doc>(
    func: &Node<D>,
) -> Option<String> {
    let params = func.field("parameters")?;
    let children: Vec<_> = params.children().collect();
    for child in &children {
        if child.kind().as_ref() == "required_parameter" {
            let param_children: Vec<_> = child.children().collect();
            let has_object_pattern = param_children
                .iter()
                .any(|c| c.kind().as_ref() == "object_pattern");
            if has_object_pattern {
                for pc in &param_children {
                    if pc.kind().as_ref() == "type_annotation" {
                        return Some(
                            pc.text()
                                .to_string()
                                .trim_start_matches(':')
                                .trim()
                                .to_string(),
                        );
                    }
                }
            }
        }
    }
    None
}

/// Extract props type from a type annotation like `React.FC<UserCardProps>`.
pub(super) fn extract_props_from_type_annotation(annotation: Option<&str>) -> Option<String> {
    let ann = annotation?;
    let start = ann.find('<')?;
    let end = ann.rfind('>')?;
    if end <= start {
        return None;
    }
    Some(ann[start + 1..end].to_string())
}

/// Extract props type from arrow function parameters.
pub(super) fn extract_props_from_arrow_params<D: ast_grep_core::Doc>(
    arrow: &Node<D>,
) -> Option<String> {
    extract_props_type_from_params(arrow)
}

// ── JSX detection ──────────────────────────────────────────────────

pub(super) fn has_jsx_recursive<D: ast_grep_core::Doc>(node: &Node<D>) -> bool {
    let kind = node.kind();
    let k = kind.as_ref();
    if k == "jsx_element" || k == "jsx_self_closing_element" || k == "jsx_fragment" {
        return true;
    }
    let children: Vec<_> = node.children().collect();
    children.iter().any(|c| has_jsx_recursive(c))
}

pub(super) fn collect_jsx_tags_recursive<D: ast_grep_core::Doc>(
    node: &Node<D>,
    tags: &mut Vec<String>,
) {
    let kind = node.kind();
    let k = kind.as_ref();
    if k == "jsx_opening_element" || k == "jsx_self_closing_element" {
        let children: Vec<_> = node.children().collect();
        for child in &children {
            let ck = child.kind();
            if ck.as_ref() == "identifier" || ck.as_ref() == "member_expression" {
                let tag = child.text().to_string();
                if !tag.is_empty() {
                    tags.push(tag);
                }
                break;
            }
        }
    }
    let children: Vec<_> = node.children().collect();
    for child in &children {
        collect_jsx_tags_recursive(child, tags);
    }
}

// ── Hook detection ─────────────────────────────────────────────────

pub(super) fn collect_hooks_recursive<D: ast_grep_core::Doc>(
    node: &Node<D>,
    hooks: &mut Vec<String>,
) {
    let kind = node.kind();
    if kind.as_ref() == "call_expression" {
        let children: Vec<_> = node.children().collect();
        if let Some(callee) = children.first() {
            let callee_text = callee.text().to_string();
            if is_hook_name(&callee_text) {
                hooks.push(callee_text);
            }
        }
    }
    let children: Vec<_> = node.children().collect();
    for child in &children {
        collect_hooks_recursive(child, hooks);
    }
}

// ── Directive detection ────────────────────────────────────────────

/// Detect `"use client"` or `"use server"` directive at file top.
pub(super) fn detect_directive<D: ast_grep_core::Doc>(root: &Node<D>) -> Option<String> {
    let children: Vec<_> = root.children().collect();
    for child in &children {
        let kind = child.kind();
        let k = kind.as_ref();
        if k == "expression_statement" {
            let inner: Vec<_> = child.children().collect();
            for ic in &inner {
                if ic.kind().as_ref() == "string" {
                    let text = ic.text().to_string();
                    let stripped = text.trim_matches(|c| c == '"' || c == '\'');
                    if stripped == "use client" || stripped == "use server" {
                        return Some(stripped.to_string());
                    }
                }
            }
        }
        // Stop after first non-comment, non-directive statement.
        if k != "comment" && k != "expression_statement" {
            break;
        }
    }
    None
}
