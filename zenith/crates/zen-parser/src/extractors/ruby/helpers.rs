use ast_grep_core::Node;

use crate::types::Visibility;

pub(super) fn extract_ruby_signature<D: ast_grep_core::Doc>(node: &Node<D>) -> String {
    node.text()
        .lines()
        .next()
        .map_or_else(String::new, |line| line.trim().to_string())
}

pub(super) fn extract_ruby_doc<D: ast_grep_core::Doc>(node: &Node<D>) -> String {
    let mut comments = Vec::new();
    let mut current = node.prev();
    while let Some(sibling) = current {
        if sibling.kind().as_ref() != "comment" {
            break;
        }

        let text = sibling.text().to_string();
        let trimmed = text.trim_start();
        if let Some(stripped) = trimmed.strip_prefix('#') {
            comments.push(stripped.trim().to_string());
        }

        current = sibling.prev();
    }

    comments.reverse();
    if !comments.is_empty() {
        return comments.join("\n");
    }

    extract_ruby_doc_by_line(node)
}

fn extract_ruby_doc_by_line<D: ast_grep_core::Doc>(node: &Node<D>) -> String {
    let mut top = node.clone();
    while let Some(parent) = top.parent() {
        top = parent;
    }
    let source = top.text().to_string();
    let lines: Vec<&str> = source.lines().collect();
    let start_line = node.start_pos().line();
    if start_line == 0 || lines.is_empty() {
        return String::new();
    }

    let mut idx = start_line.saturating_sub(1);
    while idx > 0 && lines.get(idx).is_some_and(|line| line.trim().is_empty()) {
        idx -= 1;
    }

    let mut docs = Vec::new();
    loop {
        let Some(line) = lines.get(idx) else {
            break;
        };
        let trimmed = line.trim_start();
        if let Some(stripped) = trimmed.strip_prefix('#') {
            docs.push(stripped.trim().to_string());
        } else {
            break;
        }

        if idx == 0 {
            break;
        }
        idx -= 1;
    }

    docs.reverse();
    docs.join("\n")
}

pub(super) fn normalize_const_path(path: &str) -> String {
    path.trim().trim_start_matches("::").to_string()
}

pub(super) fn extract_const_path<D: ast_grep_core::Doc>(node: &Node<D>) -> String {
    match node.kind().as_ref() {
        "scope_resolution" => {
            let scope = node.field("scope").map(|n| extract_const_path(&n));
            let name = node
                .field("name")
                .map(|n| normalize_const_path(&n.text()))
                .unwrap_or_default();
            match scope {
                Some(scope_path) if !scope_path.is_empty() => format!("{scope_path}::{name}"),
                _ => name,
            }
        }
        _ => normalize_const_path(&node.text()),
    }
}

pub(super) fn extract_method_name<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<String> {
    node.field("name").map(|name| name.text().to_string())
}

pub(super) fn extract_method_parameters<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    let Some(parameters) = node.field("parameters") else {
        return Vec::new();
    };

    parameters
        .children()
        .map(|child| child.text().trim().to_string())
        .filter(|parameter| !parameter.is_empty())
        .collect()
}

pub(super) fn call_method_name<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<String> {
    node.field("method").map(|method| method.text().to_string())
}

pub(super) fn call_receiver_text<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<String> {
    node.field("receiver")
        .map(|receiver| receiver.text().to_string())
}

pub(super) fn call_argument_texts<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    let mut args = Vec::new();

    if let Some(argument_list) = node.field("arguments") {
        for child in argument_list.children() {
            let text = child.text().trim().to_string();
            if !text.is_empty() {
                args.push(text);
            }
        }
        return args;
    }

    for child in node.children() {
        let kind = child.kind();
        let kr = kind.as_ref();
        if kr == "argument_list"
            || kr == "identifier"
            || kr == "constant"
            || kr == "simple_symbol"
            || kr == "bare_symbol"
            || kr == "symbol"
            || kr == "string"
            || kr == "hash"
            || kr == "array"
        {
            let text = child.text().trim().to_string();
            if !text.is_empty() {
                args.push(text);
            }
        }
    }

    args
}

pub(super) fn extract_symbol_like_name(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Some(symbol) = trimmed.strip_prefix(':') {
        return Some(symbol.trim_matches('"').trim_matches('\'').to_string());
    }

    let normalized = trimmed
        .trim_matches('"')
        .trim_matches('\'')
        .trim_matches(',')
        .to_string();

    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

pub(super) fn map_visibility(name: &str) -> Option<Visibility> {
    match name {
        "public" => Some(Visibility::Public),
        "private" => Some(Visibility::Private),
        "protected" => Some(Visibility::Protected),
        _ => None,
    }
}
