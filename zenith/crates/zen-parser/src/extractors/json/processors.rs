use ast_grep_core::Node;
use std::collections::{BTreeSet, HashSet};

use crate::types::{CommonMetadataExt, ParsedItem, SymbolKind, SymbolMetadata, Visibility};

use super::json_helpers;

pub(super) fn extract_document<D: ast_grep_core::Doc>(root: &Node<D>) -> Vec<ParsedItem> {
    let mut items = Vec::new();
    let Some(value) = root.children().next() else {
        return items;
    };

    let nonstandard_comments = contains_comment(root);
    let mut root_metadata = SymbolMetadata::default();
    root_metadata.set_return_type(Some(json_helpers::value_type_name(&value)));
    enrich_value_shape_metadata(&value, &mut root_metadata);
    if nonstandard_comments {
        root_metadata.push_attribute("json:nonstandard");
        root_metadata.push_attribute("json:nonstandard:comments");
    }

    items.push(build_item(
        root,
        SymbolKind::Module,
        "$".to_string(),
        root_metadata,
        "$",
    ));

    collect_value(&value, "", nonstandard_comments, &mut items);
    items
}

fn collect_value<D: ast_grep_core::Doc>(
    node: &Node<D>,
    path: &str,
    nonstandard_comments: bool,
    out: &mut Vec<ParsedItem>,
) {
    match node.kind().as_ref() {
        "object" => collect_object(node, path, nonstandard_comments, out),
        "array" => collect_array(node, path, nonstandard_comments, out),
        _ => {}
    }
}

fn collect_object<D: ast_grep_core::Doc>(
    node: &Node<D>,
    path: &str,
    nonstandard_comments: bool,
    out: &mut Vec<ParsedItem>,
) {
    let mut seen_keys = HashSet::new();
    for child in node.children() {
        if child.kind().as_ref() != "pair" {
            continue;
        }

        let duplicate_key = child.field("key").and_then(|key| {
            let key_name = json_helpers::unquote_json_string(&key.text());
            if seen_keys.insert(key_name.clone()) {
                None
            } else {
                Some(key_name)
            }
        });

        collect_pair(&child, path, nonstandard_comments, duplicate_key, out);
    }
}

fn collect_array<D: ast_grep_core::Doc>(
    node: &Node<D>,
    path: &str,
    nonstandard_comments: bool,
    out: &mut Vec<ParsedItem>,
) {
    let mut idx = 0usize;
    for child in node.children() {
        let kind = child.kind();
        let kr = kind.as_ref();
        if !matches!(
            kr,
            "object" | "array" | "string" | "number" | "true" | "false" | "null"
        ) {
            continue;
        }

        let next_path = if path.is_empty() {
            format!("[{idx}]")
        } else {
            format!("{path}[{idx}]")
        };

        if matches!(kr, "string" | "number" | "true" | "false" | "null") {
            out.push(build_array_primitive_item(
                &child,
                &next_path,
                path,
                nonstandard_comments,
            ));
        }

        collect_value(&child, &next_path, nonstandard_comments, out);
        idx += 1;
    }
}

fn collect_pair<D: ast_grep_core::Doc>(
    pair: &Node<D>,
    parent_path: &str,
    nonstandard_comments: bool,
    duplicate_key: Option<String>,
    out: &mut Vec<ParsedItem>,
) {
    let Some(key_node) = pair.field("key") else {
        return;
    };
    let Some(value_node) = pair.field("value") else {
        return;
    };

    let key_name = json_helpers::unquote_json_string(&key_node.text());
    let full_path = json_helpers::path_join(parent_path, &key_name);
    let owner_name = if parent_path.is_empty() {
        "$".to_string()
    } else {
        parent_path.to_string()
    };

    let mut metadata = SymbolMetadata::default();
    metadata.set_owner_name(Some(owner_name));
    metadata.set_owner_kind(Some(SymbolKind::Module));
    metadata.set_return_type(Some(json_helpers::value_type_name(&value_node)));
    metadata.push_attribute(format!("json:key:{key_name}"));
    if nonstandard_comments {
        metadata.push_attribute("json:nonstandard");
        metadata.push_attribute("json:nonstandard:comments");
    }
    if let Some(duplicate) = duplicate_key {
        metadata.push_attribute(format!("json:duplicate_key:{duplicate}"));
    }
    enrich_value_shape_metadata(&value_node, &mut metadata);

    out.push(build_item(
        pair,
        SymbolKind::Property,
        full_path.clone(),
        metadata,
        &key_name,
    ));

    collect_value(&value_node, &full_path, nonstandard_comments, out);
}

fn enrich_value_shape_metadata<D: ast_grep_core::Doc>(
    node: &Node<D>,
    metadata: &mut SymbolMetadata,
) {
    match node.kind().as_ref() {
        "array" => {
            let element_count = array_element_count(node);
            metadata.push_attribute(format!("json:array_count:{element_count}"));

            let element_types = array_element_types(node);
            if element_types.is_empty() {
                metadata.push_attribute("json:array_elements:empty");
            } else {
                metadata.push_attribute(format!("json:array_elements:{}", element_types.join("|")));
                if element_types.len() > 1 {
                    metadata.push_attribute("json:array_mixed");
                }
                if element_types.iter().any(|kind| kind == "null") {
                    metadata.push_attribute("json:array_nullable");
                }
            }
        }
        "object" => {
            let pair_count = node
                .children()
                .filter(|child| child.kind().as_ref() == "pair")
                .count();
            metadata.push_attribute(format!("json:object_keys:{pair_count}"));
        }
        _ => {}
    }
}

fn array_element_types<D: ast_grep_core::Doc>(array: &Node<D>) -> Vec<String> {
    let mut kinds = BTreeSet::new();
    for child in array.children() {
        let kind = child.kind();
        let kr = kind.as_ref();
        if matches!(
            kr,
            "object" | "array" | "string" | "number" | "true" | "false" | "null"
        ) {
            kinds.insert(json_helpers::value_type_name(&child));
        }
    }
    kinds.into_iter().collect()
}

fn array_element_count<D: ast_grep_core::Doc>(array: &Node<D>) -> usize {
    array
        .children()
        .filter(|child| {
            matches!(
                child.kind().as_ref(),
                "object" | "array" | "string" | "number" | "true" | "false" | "null"
            )
        })
        .count()
}

fn build_array_primitive_item<D: ast_grep_core::Doc>(
    node: &Node<D>,
    path: &str,
    owner_path: &str,
    nonstandard_comments: bool,
) -> ParsedItem {
    let mut metadata = SymbolMetadata::default();
    metadata.set_owner_name(Some(if owner_path.is_empty() {
        "$".to_string()
    } else {
        owner_path.to_string()
    }));
    metadata.set_owner_kind(Some(SymbolKind::Module));
    metadata.set_return_type(Some(json_helpers::value_type_name(node)));
    metadata.push_attribute("json:array_element");
    if nonstandard_comments {
        metadata.push_attribute("json:nonstandard");
        metadata.push_attribute("json:nonstandard:comments");
    }

    build_item(node, SymbolKind::Property, path.to_string(), metadata, path)
}

fn contains_comment<D: ast_grep_core::Doc>(node: &Node<D>) -> bool {
    if node.kind().as_ref() == "comment" {
        return true;
    }
    node.children().any(|child| contains_comment(&child))
}

fn build_item<D: ast_grep_core::Doc>(
    node: &Node<D>,
    kind: SymbolKind,
    name: String,
    metadata: SymbolMetadata,
    signature_name: &str,
) -> ParsedItem {
    ParsedItem {
        kind,
        name,
        signature: signature_name.to_string(),
        source: crate::extractors::helpers::extract_source(node, 40),
        doc_comment: String::new(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata,
    }
}
