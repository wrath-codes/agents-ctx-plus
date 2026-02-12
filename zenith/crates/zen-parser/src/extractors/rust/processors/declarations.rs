use ast_grep_core::Node;

use crate::extractors::helpers;
use crate::types::{ParsedItem, SymbolKind, SymbolMetadata, Visibility};

// ── Foreign module (extern block) processing ───────────────────────

pub(super) fn process_foreign_mod<D: ast_grep_core::Doc>(
    node: &Node<D>,
    source: &str,
) -> Vec<ParsedItem> {
    let mut items = Vec::new();

    // Extract ABI from extern_modifier
    let abi = node
        .children()
        .find(|c| c.kind().as_ref() == "extern_modifier")
        .and_then(|em| {
            em.children()
                .find(|c| c.kind().as_ref() == "string_literal")
                .map(|s| s.text().to_string().trim_matches('"').to_string())
        })
        .unwrap_or_else(|| "C".to_string());

    let Some(body) = node
        .children()
        .find(|c| c.kind().as_ref() == "declaration_list")
    else {
        return items;
    };

    for child in body.children() {
        let k = child.kind();
        match k.as_ref() {
            "function_signature_item" => {
                let name = child
                    .field("name")
                    .or_else(|| child.children().find(|c| c.kind().as_ref() == "identifier"))
                    .map(|n| n.text().to_string());
                if let Some(name) = name {
                    items.push(ParsedItem {
                        kind: SymbolKind::Function,
                        name,
                        signature: helpers::extract_signature(&child),
                        source: helpers::extract_source(&child, 10),
                        doc_comment: helpers::extract_doc_comments_rust(&child, source),
                        start_line: child.start_pos().line() as u32 + 1,
                        end_line: child.end_pos().line() as u32 + 1,
                        visibility: Visibility::Public,
                        metadata: SymbolMetadata {
                            abi: Some(abi.clone()),
                            parameters: helpers::extract_parameters(&child),
                            return_type: helpers::extract_return_type(&child),
                            attributes: vec!["extern".to_string()],
                            ..Default::default()
                        },
                    });
                }
            }
            "static_item" => {
                let name = child
                    .field("name")
                    .or_else(|| child.children().find(|c| c.kind().as_ref() == "identifier"))
                    .map(|n| n.text().to_string());
                if let Some(name) = name {
                    items.push(ParsedItem {
                        kind: SymbolKind::Static,
                        name,
                        signature: helpers::extract_signature(&child),
                        source: helpers::extract_source(&child, 10),
                        doc_comment: helpers::extract_doc_comments_rust(&child, source),
                        start_line: child.start_pos().line() as u32 + 1,
                        end_line: child.end_pos().line() as u32 + 1,
                        visibility: Visibility::Public,
                        metadata: SymbolMetadata {
                            abi: Some(abi.clone()),
                            return_type: helpers::extract_type_annotation(&child),
                            attributes: vec!["extern".to_string()],
                            ..Default::default()
                        },
                    });
                }
            }
            _ => {}
        }
    }
    items
}

// ── use declaration processing ─────────────────────────────────────

pub(super) fn process_use_declaration<D: ast_grep_core::Doc>(
    node: &Node<D>,
    source: &str,
) -> Option<ParsedItem> {
    let vis = helpers::extract_visibility_rust(node);
    // Extract the use path — all children after `use` and before `;`
    let path = node
        .children()
        .filter(|c| {
            let k = c.kind();
            k.as_ref() != "use" && k.as_ref() != ";" && k.as_ref() != "visibility_modifier"
        })
        .map(|c| c.text().to_string())
        .collect::<String>();

    if path.is_empty() {
        return None;
    }

    // Derive name from last segment
    let name = path
        .rsplit("::")
        .next()
        .unwrap_or(&path)
        .trim_matches(|c: char| c == '{' || c == '}' || c == ' ')
        .to_string();

    let is_reexport = vis == Visibility::Public || vis == Visibility::PublicCrate;

    Some(ParsedItem {
        kind: SymbolKind::Module,
        name,
        signature: helpers::extract_signature(node),
        source: None,
        doc_comment: helpers::extract_doc_comments_rust(node, source),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: vis,
        metadata: SymbolMetadata {
            attributes: if is_reexport {
                vec!["use".to_string(), "reexport".to_string()]
            } else {
                vec!["use".to_string()]
            },
            ..Default::default()
        },
    })
}

// ── extern crate processing ────────────────────────────────────────

pub(super) fn process_extern_crate<D: ast_grep_core::Doc>(
    node: &Node<D>,
    source: &str,
) -> Option<ParsedItem> {
    let name = node
        .children()
        .find(|c| c.kind().as_ref() == "identifier")
        .map(|c| c.text().to_string())
        .filter(|n| !n.is_empty())?;

    Some(ParsedItem {
        kind: SymbolKind::Module,
        name,
        signature: helpers::extract_signature(node),
        source: None,
        doc_comment: helpers::extract_doc_comments_rust(node, source),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: helpers::extract_visibility_rust(node),
        metadata: SymbolMetadata {
            attributes: vec!["extern_crate".to_string()],
            ..Default::default()
        },
    })
}

// ── Item-position macro invocation processing ──────────────────────

pub(super) fn process_macro_invocation<D: ast_grep_core::Doc>(
    node: &Node<D>,
    source: &str,
) -> Option<ParsedItem> {
    let name = node
        .children()
        .find(|c| {
            let k = c.kind();
            k.as_ref() == "identifier" || k.as_ref() == "scoped_identifier"
        })
        .map(|c| c.text().to_string())
        .filter(|n| !n.is_empty())?;

    Some(ParsedItem {
        kind: SymbolKind::Macro,
        name,
        signature: helpers::extract_signature(node),
        source: helpers::extract_source(node, 10),
        doc_comment: helpers::extract_doc_comments_rust(node, source),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Private,
        metadata: SymbolMetadata {
            attributes: vec!["macro_invocation".to_string()],
            ..Default::default()
        },
    })
}
