//! Python extraction processors: classes, functions, module-level assignments.

mod classes;
mod functions;

use ast_grep_core::Node;

use crate::types::{ParsedItem, SymbolKind, SymbolMetadata};

use super::pyhelpers::{extract_decorators, python_visibility};

pub(super) use classes::{process_class, process_class_member_items};
pub(super) use functions::process_function;

pub(super) fn extract_dunder_all<D: ast_grep_core::Doc>(root: &Node<D>) -> Option<Vec<String>> {
    for child in root.children() {
        if child.kind().as_ref() != "expression_statement" {
            continue;
        }
        let text = child.text().to_string();
        let trimmed = text.trim();
        if !trimmed.starts_with("__all__") {
            continue;
        }
        // Extract names from __all__ = ["name1", "name2", ...]
        let mut names = Vec::new();
        if let Some(bracket_start) = text.find('[')
            && let Some(bracket_end) = text.rfind(']')
        {
            let inner = &text[bracket_start + 1..bracket_end];
            for part in inner.split(',') {
                let part = part.trim().trim_matches('"').trim_matches('\'');
                if !part.is_empty() {
                    names.push(part.to_string());
                }
            }
        }
        if !names.is_empty() {
            return Some(names);
        }
    }
    None
}

pub(super) fn process_decorated<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<ParsedItem> {
    let decorators = extract_decorators(node);
    let Some(inner) = node.children().find(|c| {
        let k = c.kind();
        k.as_ref() == "class_definition" || k.as_ref() == "function_definition"
    }) else {
        return Vec::new();
    };

    match inner.kind().as_ref() {
        "class_definition" => {
            let mut items = Vec::new();
            if let Some(class_item) = process_class(&inner, &decorators) {
                items.push(class_item);
            }
            items.extend(process_class_member_items(&inner));
            items
        }
        "function_definition" => process_function(&inner, &decorators).into_iter().collect(),
        _ => Vec::new(),
    }
}

pub(super) fn process_module_assignment<D: ast_grep_core::Doc>(
    expr_stmt: &Node<D>,
) -> Option<ParsedItem> {
    let assignment = expr_stmt
        .children()
        .find(|c| c.kind().as_ref() == "assignment")?;

    let name_node = assignment
        .children()
        .find(|c| c.kind().as_ref() == "identifier")?;
    let name = name_node.text().to_string();
    if name.is_empty() {
        return None;
    }

    // Skip __all__ (handled separately) and docstrings
    if name == "__all__" {
        return None;
    }

    let type_annotation = assignment
        .children()
        .find(|c| c.kind().as_ref() == "type")
        .map(|t| t.text().to_string());

    // Detect TypeAlias annotation
    let is_type_alias = type_annotation
        .as_ref()
        .is_some_and(|t| t.contains("TypeAlias"));

    let symbol_kind = if is_type_alias {
        SymbolKind::TypeAlias
    } else {
        SymbolKind::Const
    };

    let visibility = python_visibility(&name);

    Some(ParsedItem {
        kind: symbol_kind,
        name,
        signature: assignment.text().to_string(),
        source: Some(assignment.text().to_string()),
        doc_comment: String::new(),
        start_line: expr_stmt.start_pos().line() as u32 + 1,
        end_line: expr_stmt.end_pos().line() as u32 + 1,
        visibility,
        metadata: SymbolMetadata {
            return_type: type_annotation,
            ..Default::default()
        },
    })
}
