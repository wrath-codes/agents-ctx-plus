use ast_grep_core::Node;

use crate::types::SymbolKind;

pub(super) fn extract_name<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<String> {
    if let Some(name) = node.field("name") {
        let text = name.text().trim().to_string();
        if !text.is_empty() {
            return Some(text);
        }
    }

    parse_name_from_text(node.text().as_ref())
}

pub(super) fn extract_module_name<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<String> {
    if let Some(module) = node.field("module") {
        let text = module.text().trim().to_string();
        if !text.is_empty() {
            return Some(text);
        }
    }
    extract_name(node)
}

pub(super) fn classify_data_type_hybrid<D: ast_grep_core::Doc>(node: &Node<D>) -> SymbolKind {
    let text = node.text().to_string();
    if text.contains('|') {
        SymbolKind::Enum
    } else if text.contains('{') && text.contains('}') {
        SymbolKind::Struct
    } else {
        SymbolKind::Enum
    }
}

pub(super) fn extract_data_constructors<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    let text = node.text().to_string();
    let mut constructors = Vec::new();

    if let Some((_, rhs)) = text.split_once('=') {
        for part in rhs.split('|') {
            let segment = part.trim();
            if segment.is_empty() {
                continue;
            }

            let candidate = segment.split(['{', '(', ' ']).next().unwrap_or("").trim();

            if is_constructor_name(candidate) {
                constructors.push(candidate.to_string());
            }
        }
    }

    constructors
}

pub(super) fn extract_record_fields<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    let text = node.text().to_string();
    let Some(start) = text.find('{') else {
        return Vec::new();
    };
    let Some(end) = text.rfind('}') else {
        return Vec::new();
    };
    if end <= start {
        return Vec::new();
    }

    let body = &text[start + 1..end];
    body.split(',')
        .filter_map(|field| {
            let candidate = field
                .split("::")
                .next()
                .unwrap_or("")
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim();
            if candidate.is_empty() {
                None
            } else {
                Some(candidate.to_string())
            }
        })
        .collect()
}

fn parse_name_from_text(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Some((lhs, _)) = trimmed.split_once("::") {
        if trimmed.starts_with("foreign import") || trimmed.starts_with("foreign export") {
            return lhs
                .split_whitespace()
                .last()
                .map(|tok| tok.trim().to_string())
                .filter(|s| !s.is_empty());
        }
        return extract_name_token(lhs);
    }
    if let Some((lhs, _)) = trimmed.split_once('=') {
        return extract_name_token(lhs);
    }
    extract_name_token(trimmed)
}

fn extract_name_token(text: &str) -> Option<String> {
    text.split_whitespace().find_map(|token| {
        let cleaned = token
            .trim_matches('(')
            .trim_matches(')')
            .trim_matches(',')
            .trim_matches(';');
        if cleaned.is_empty() || cleaned == "foreign" || cleaned == "import" || cleaned == "export"
        {
            return None;
        }
        if cleaned.chars().all(|c| c == ':' || c == '-' || c == '>') {
            return None;
        }
        Some(cleaned.to_string())
    })
}

fn is_constructor_name(name: &str) -> bool {
    name.chars().next().is_some_and(char::is_uppercase)
}
