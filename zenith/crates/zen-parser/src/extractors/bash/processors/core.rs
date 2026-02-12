use ast_grep_core::Node;
use std::fmt::Write as _;

use crate::types::{ParsedItem, SymbolKind, SymbolMetadata, Visibility};

use super::super::helpers::extract_source_limited;

pub(in super::super) fn process_shebang<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
) {
    let text = node.text().to_string();
    let interpreter = text.trim_start_matches("#!").trim().to_string();

    items.push(ParsedItem {
        kind: SymbolKind::Macro,
        name: "shebang".to_string(),
        signature: text.clone(),
        source: Some(text),
        doc_comment: String::new(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata: SymbolMetadata {
            attributes: vec![interpreter],
            ..Default::default()
        },
    });
}

pub(in super::super) fn process_function<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
) {
    let children: Vec<_> = node.children().collect();

    let has_function_keyword = children.iter().any(|c| c.kind().as_ref() == "function");

    // The function name is always in a `word` child
    let name = children
        .iter()
        .find(|c| c.kind().as_ref() == "word")
        .map_or_else(|| "anonymous".to_string(), |n| n.text().to_string());

    // Build signature
    let has_parens = children.iter().any(|c| c.kind().as_ref() == "(");

    let mut signature = String::new();
    if has_function_keyword {
        let _ = write!(signature, "function {name}");
    } else {
        let _ = write!(signature, "{name}");
    }
    if has_parens {
        signature.push_str("()");
    }

    items.push(ParsedItem {
        kind: SymbolKind::Function,
        name,
        signature,
        source: extract_source_limited(node, 20),
        doc_comment: doc_comment.to_string(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata: SymbolMetadata {
            attributes: if has_function_keyword {
                vec!["function_keyword".to_string()]
            } else {
                Vec::new()
            },
            ..Default::default()
        },
    });
}
