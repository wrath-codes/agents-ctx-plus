use ast_grep_core::Node;

use crate::extractors::helpers;
use crate::types::{ParsedItem, RustMetadataExt, SymbolKind, SymbolMetadata, Visibility};

pub(super) fn process_impl_item<D: ast_grep_core::Doc>(
    node: &Node<D>,
    source: &str,
) -> Vec<ParsedItem> {
    let (trait_name, for_type) = extract_impl_targets(node);
    let is_unsafe_impl = node.children().any(|c| c.kind().as_ref() == "unsafe");
    let is_negative = node.children().any(|c| c.kind().as_ref() == "!");

    let mut items = Vec::new();
    let Some(body) = node.field("body") else {
        return items;
    };

    for child in body.children() {
        let k = child.kind();
        match k.as_ref() {
            "function_item" => {
                if let Some(mut method) =
                    process_impl_method(&child, source, trait_name.as_deref(), for_type.as_deref())
                {
                    if is_unsafe_impl {
                        method.metadata.mark_unsafe();
                    }
                    items.push(method);
                }
            }
            "const_item" => {
                if let Some(item) = process_impl_assoc_const(
                    &child,
                    source,
                    trait_name.as_deref(),
                    for_type.as_deref(),
                ) {
                    items.push(item);
                }
            }
            "type_item" => {
                if let Some(item) = process_impl_assoc_type(
                    &child,
                    source,
                    trait_name.as_deref(),
                    for_type.as_deref(),
                ) {
                    items.push(item);
                }
            }
            _ => {}
        }
    }

    // For negative impls with no body items, emit a marker
    if is_negative
        && items.is_empty()
        && let (Some(trait_n), Some(for_t)) = (&trait_name, &for_type)
    {
        items.push(ParsedItem {
            kind: SymbolKind::Trait,
            name: format!("!{trait_n}"),
            signature: helpers::extract_signature(node),
            source: helpers::extract_source(node, 10),
            doc_comment: String::new(),
            start_line: node.start_pos().line() as u32 + 1,
            end_line: node.end_pos().line() as u32 + 1,
            visibility: Visibility::Private,
            metadata: SymbolMetadata {
                trait_name: Some(format!("!{trait_n}")),
                for_type: Some(for_t.clone()),
                attributes: vec!["negative_impl".to_string()],
                ..Default::default()
            },
        });
    }
    items
}

fn extract_impl_targets<D: ast_grep_core::Doc>(node: &Node<D>) -> (Option<String>, Option<String>) {
    let mut trait_name = None;
    let mut for_type = None;

    let children: Vec<_> = node.children().collect();
    let mut found_for = false;
    for child in &children {
        let k = child.kind();
        if k.as_ref() == "for" {
            found_for = true;
        }
    }

    if found_for {
        let mut past_for = false;
        for child in &children {
            let k = child.kind();
            if k.as_ref() == "for" {
                past_for = true;
                continue;
            }
            if is_type_node(k.as_ref()) {
                if past_for {
                    for_type = Some(child.text().to_string());
                } else if trait_name.is_none() {
                    trait_name = Some(child.text().to_string());
                }
            }
        }
    } else {
        for child in &children {
            if is_type_node(child.kind().as_ref()) {
                for_type = Some(child.text().to_string());
                break;
            }
        }
    }

    (trait_name, for_type)
}

fn is_type_node(kind: &str) -> bool {
    matches!(
        kind,
        "type_identifier"
            | "scoped_type_identifier"
            | "generic_type"
            | "scoped_identifier"
            | "reference_type"
            | "tuple_type"
            | "array_type"
            | "pointer_type"
            | "function_type"
            | "primitive_type"
            | "unit_type"
            | "abstract_type"
            | "dynamic_type"
            | "bounded_type"
            | "macro_invocation"
            | "never_type"
    )
}

fn process_impl_method<D: ast_grep_core::Doc>(
    child: &Node<D>,
    source: &str,
    trait_name: Option<&str>,
    for_type: Option<&str>,
) -> Option<ParsedItem> {
    let name = child
        .field("name")
        .map(|n| n.text().to_string())
        .filter(|n| !n.is_empty())?;

    let (is_async, is_unsafe, _is_const, _abi) = helpers::detect_modifiers(child);
    let attrs = helpers::extract_attributes(child);
    let generics = helpers::extract_generics(child);
    let return_type = helpers::extract_return_type(child);
    let doc = helpers::extract_doc_comments_rust(child, source);
    let doc_sections = helpers::parse_rust_doc_sections(&doc);

    Some(ParsedItem {
        kind: SymbolKind::Method,
        name,
        signature: helpers::extract_signature(child),
        source: helpers::extract_source(child, 50),
        doc_comment: doc,
        start_line: child.start_pos().line() as u32 + 1,
        end_line: child.end_pos().line() as u32 + 1,
        visibility: helpers::extract_visibility_rust(child),
        metadata: SymbolMetadata {
            is_async,
            is_unsafe,
            return_type: return_type.clone(),
            generics: generics.clone(),
            attributes: attrs.clone(),
            parameters: helpers::extract_parameters(child),
            lifetimes: helpers::extract_lifetimes(generics.as_deref()),
            where_clause: helpers::extract_where_clause(child),
            trait_name: trait_name.map(String::from),
            for_type: for_type.map(String::from),
            is_pyo3: helpers::is_pyo3(&attrs),
            returns_result: helpers::returns_result(return_type.as_deref()),
            doc_sections,
            ..Default::default()
        },
    })
}

fn process_impl_assoc_const<D: ast_grep_core::Doc>(
    node: &Node<D>,
    source: &str,
    trait_name: Option<&str>,
    for_type: Option<&str>,
) -> Option<ParsedItem> {
    let name = node
        .field("name")
        .or_else(|| node.children().find(|c| c.kind().as_ref() == "identifier"))
        .map(|n| n.text().to_string())
        .filter(|n| !n.is_empty())?;

    let return_type =
        helpers::extract_return_type(node).or_else(|| helpers::extract_type_annotation(node));

    Some(ParsedItem {
        kind: SymbolKind::Const,
        name,
        signature: helpers::extract_signature(node),
        source: helpers::extract_source(node, 10),
        doc_comment: helpers::extract_doc_comments_rust(node, source),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: helpers::extract_visibility_rust(node),
        metadata: SymbolMetadata {
            return_type,
            trait_name: trait_name.map(String::from),
            for_type: for_type.map(String::from),
            ..Default::default()
        },
    })
}

fn process_impl_assoc_type<D: ast_grep_core::Doc>(
    node: &Node<D>,
    source: &str,
    trait_name: Option<&str>,
    for_type: Option<&str>,
) -> Option<ParsedItem> {
    let name = node
        .field("name")
        .or_else(|| {
            node.children()
                .find(|c| c.kind().as_ref() == "type_identifier")
        })
        .map(|n| n.text().to_string())
        .filter(|n| !n.is_empty())?;

    Some(ParsedItem {
        kind: SymbolKind::TypeAlias,
        name,
        signature: helpers::extract_signature(node),
        source: helpers::extract_source(node, 10),
        doc_comment: helpers::extract_doc_comments_rust(node, source),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: helpers::extract_visibility_rust(node),
        metadata: SymbolMetadata {
            trait_name: trait_name.map(String::from),
            for_type: for_type.map(String::from),
            ..Default::default()
        },
    })
}
