use ast_grep_core::Node;
use std::fmt::Write as _;

use crate::types::{ParsedItem, SymbolKind, SymbolMetadata, Visibility};

pub(in super::super) fn process_variable_assignment<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
    qualifier: Option<&str>,
) {
    let children: Vec<_> = node.children().collect();

    let var_name = children
        .iter()
        .find(|c| c.kind().as_ref() == "variable_name")
        .map_or_else(String::new, |n| n.text().to_string());

    if var_name.is_empty() {
        return;
    }

    let value = extract_assignment_value(&children);

    let (kind, visibility) = match qualifier {
        Some("readonly" | "declare -r") => (SymbolKind::Const, Visibility::Public),
        Some("export" | "declare -x") => (SymbolKind::Const, Visibility::Export),
        Some("local") => (SymbolKind::Static, Visibility::Private),
        _ => (SymbolKind::Static, Visibility::Public),
    };

    let mut signature = String::new();
    if let Some(q) = qualifier {
        let _ = write!(signature, "{q} ");
    }
    let _ = write!(signature, "{var_name}");
    if let Some(ref v) = value {
        let _ = write!(signature, "={v}");
    }

    items.push(ParsedItem {
        kind,
        name: var_name,
        signature,
        source: Some(node.text().to_string()),
        doc_comment: doc_comment.to_string(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility,
        metadata: SymbolMetadata {
            attributes: qualifier.map_or_else(Vec::new, |q| vec![q.to_string()]),
            ..Default::default()
        },
    });
}

/// Extract the value portion of a variable assignment.
fn extract_assignment_value<D: ast_grep_core::Doc>(children: &[Node<D>]) -> Option<String> {
    // Value is everything after the `=` sign
    let mut found_eq = false;
    for child in children {
        if child.kind().as_ref() == "=" {
            found_eq = true;
            continue;
        }
        if found_eq {
            let text = child.text().to_string();
            if !text.is_empty() {
                return Some(text);
            }
        }
    }
    None
}

pub(in super::super) fn process_declaration_command<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
) {
    let children: Vec<_> = node.children().collect();
    let Some(first) = children.first() else {
        return;
    };

    let qualifier_kind = first.kind();
    let qualifier = qualifier_kind.as_ref();

    match qualifier {
        "export" => process_export_declaration(node, items, doc_comment, &children),
        "readonly" => process_qualified_assignment(node, items, doc_comment, &children, "readonly"),
        "local" => process_qualified_assignment(node, items, doc_comment, &children, "local"),
        "declare" | "typeset" => {
            process_declare_command(node, items, doc_comment, &children);
        }
        _ => {}
    }
}

fn process_export_declaration<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
    children: &[Node<D>],
) {
    // Check if this is `export -f func_name` (function export)
    let has_flag = children
        .iter()
        .any(|c| c.kind().as_ref() == "word" && c.text().as_ref().starts_with('-'));

    if has_flag {
        // export -f func_name or export -n etc.
        let flag = children
            .iter()
            .find(|c| c.kind().as_ref() == "word" && c.text().as_ref().starts_with('-'))
            .map(|c| c.text().to_string())
            .unwrap_or_default();

        let target = children
            .iter()
            .filter(|c| c.kind().as_ref() == "word" && !c.text().as_ref().starts_with('-'))
            .last()
            .map_or_else(|| "unknown".to_string(), |c| c.text().to_string());

        let signature = format!("export {flag} {target}");

        items.push(ParsedItem {
            kind: SymbolKind::Const,
            name: target,
            signature,
            source: Some(node.text().to_string()),
            doc_comment: doc_comment.to_string(),
            start_line: node.start_pos().line() as u32 + 1,
            end_line: node.end_pos().line() as u32 + 1,
            visibility: Visibility::Export,
            metadata: SymbolMetadata {
                attributes: vec!["export".to_string(), flag],
                ..Default::default()
            },
        });
    } else {
        // export VAR=value
        for child in children {
            if child.kind().as_ref() == "variable_assignment" {
                process_variable_assignment(child, items, doc_comment, Some("export"));
                return;
            }
        }
        // Plain `export VAR` without assignment — still emit
        if let Some(word) = children.iter().find(|c| c.kind().as_ref() == "word") {
            items.push(ParsedItem {
                kind: SymbolKind::Const,
                name: word.text().to_string(),
                signature: node.text().to_string(),
                source: Some(node.text().to_string()),
                doc_comment: doc_comment.to_string(),
                start_line: node.start_pos().line() as u32 + 1,
                end_line: node.end_pos().line() as u32 + 1,
                visibility: Visibility::Export,
                metadata: SymbolMetadata {
                    attributes: vec!["export".to_string()],
                    ..Default::default()
                },
            });
        }
    }
}

fn process_qualified_assignment<D: ast_grep_core::Doc>(
    _parent: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
    children: &[Node<D>],
    qualifier: &str,
) {
    for child in children {
        if child.kind().as_ref() == "variable_assignment" {
            process_variable_assignment(child, items, doc_comment, Some(qualifier));
        }
    }
}

fn process_declare_command<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
    children: &[Node<D>],
) {
    // Collect flags like -a, -A, -x, -i, -r
    let flags: Vec<String> = children
        .iter()
        .filter(|c| c.kind().as_ref() == "word" && c.text().as_ref().starts_with('-'))
        .map(|c| c.text().to_string())
        .collect();

    let flag_str = flags.join(" ");
    let qualifier = format!("declare {flag_str}");

    // Check for array declarations (-a or -A)
    let is_array = flags.iter().any(|f| f == "-a");
    let is_assoc = flags.iter().any(|f| f == "-A");
    let is_exported = flags.iter().any(|f| f == "-x");
    let is_readonly = flags.iter().any(|f| f == "-r");

    // Find the variable assignment child
    if let Some(assignment) = children
        .iter()
        .find(|c| c.kind().as_ref() == "variable_assignment")
    {
        let assign_children: Vec<_> = assignment.children().collect();
        let var_name = assign_children
            .iter()
            .find(|c| c.kind().as_ref() == "variable_name")
            .map_or_else(String::new, |n| n.text().to_string());

        if var_name.is_empty() {
            return;
        }

        let (kind, visibility) = if is_readonly {
            (SymbolKind::Const, Visibility::Public)
        } else if is_exported {
            (SymbolKind::Const, Visibility::Export)
        } else {
            (SymbolKind::Static, Visibility::Public)
        };

        let mut signature = qualifier.clone();
        let _ = write!(signature, " {}", assignment.text());

        let mut attributes = vec![qualifier];
        if is_array {
            attributes.push("indexed_array".to_string());
        }
        if is_assoc {
            attributes.push("associative_array".to_string());
        }

        items.push(ParsedItem {
            kind,
            name: var_name,
            signature,
            source: Some(node.text().to_string()),
            doc_comment: doc_comment.to_string(),
            start_line: node.start_pos().line() as u32 + 1,
            end_line: node.end_pos().line() as u32 + 1,
            visibility,
            metadata: SymbolMetadata {
                attributes,
                ..Default::default()
            },
        });
    }
}
