use ast_grep_core::Node;
use std::collections::{BTreeSet, HashMap, HashSet};

use crate::types::{CommonMetadataExt, ParsedItem, SymbolKind, SymbolMetadata, Visibility};

use super::yaml_helpers;

struct YamlContext {
    nonstandard_comments: bool,
    doc_count: usize,
    anchors: HashMap<String, String>,
}

pub(super) fn extract_stream<D: ast_grep_core::Doc>(root: &Node<D>) -> Vec<ParsedItem> {
    let mut items = Vec::new();
    let doc_count = root
        .children()
        .filter(|child| child.kind().as_ref() == "document")
        .count();
    let nonstandard_comments = contains_comment(root);

    let mut ctx = YamlContext {
        nonstandard_comments,
        doc_count,
        anchors: HashMap::new(),
    };

    let mut root_metadata = SymbolMetadata::default();
    root_metadata.set_return_type(Some(stream_type(root)));
    root_metadata.push_attribute(format!("yaml:documents:{doc_count}"));
    if nonstandard_comments {
        root_metadata.push_attribute("yaml:nonstandard:comments");
    }

    items.push(build_item(
        root,
        SymbolKind::Module,
        "$".to_string(),
        root_metadata,
        "$",
    ));

    let mut doc_index = 0usize;
    for doc in root.children() {
        if doc.kind().as_ref() != "document" {
            continue;
        }
        collect_document(&doc, doc_index, &mut ctx, &mut items);
        doc_index += 1;
    }

    items
}

fn collect_document<D: ast_grep_core::Doc>(
    document: &Node<D>,
    index: usize,
    ctx: &mut YamlContext,
    out: &mut Vec<ParsedItem>,
) {
    let doc_prefix = if ctx.doc_count <= 1 {
        String::new()
    } else {
        format!("doc[{index}]")
    };

    for child in document.children() {
        let kind = child.kind();
        let kr = kind.as_ref();

        if kr == "yaml_directive" || kr == "tag_directive" || kr == "reserved_directive" {
            let mut metadata = SymbolMetadata::default();
            metadata.push_attribute(format!("yaml:directive:{kr}"));
            if ctx.nonstandard_comments {
                metadata.push_attribute("yaml:nonstandard:comments");
            }
            let name = if doc_prefix.is_empty() {
                kr.to_string()
            } else {
                format!("{doc_prefix}.{kr}")
            };
            out.push(build_item(&child, SymbolKind::Module, name, metadata, kr));
            continue;
        }

        if kr == "block_node" || kr == "flow_node" {
            collect_value(&child, &doc_prefix, ctx, out);
        }
    }
}

fn collect_value<D: ast_grep_core::Doc>(
    node: &Node<D>,
    path: &str,
    ctx: &mut YamlContext,
    out: &mut Vec<ParsedItem>,
) {
    let wrapped = unwrap_yaml_value(node);
    if wrapped.alias_name.is_some() {
        return;
    }

    match wrapped.value.kind().as_ref() {
        "block_mapping" | "flow_mapping" => collect_mapping(&wrapped.value, path, ctx, out),
        "block_sequence" | "flow_sequence" => collect_sequence(&wrapped.value, path, ctx, out),
        _ => {}
    }
}

fn collect_mapping<D: ast_grep_core::Doc>(
    mapping: &Node<D>,
    path: &str,
    ctx: &mut YamlContext,
    out: &mut Vec<ParsedItem>,
) {
    let mut seen_keys = HashSet::new();
    let mut pair_count = 0usize;

    for child in mapping.children() {
        if !is_mapping_pair(&child) {
            continue;
        }
        pair_count += 1;

        let duplicate = child.field("key").and_then(|key| {
            let name = yaml_helpers::key_text(&key);
            if seen_keys.insert(name.clone()) {
                None
            } else {
                Some(name)
            }
        });

        collect_pair(&child, path, duplicate, ctx, out);
    }

    if !path.is_empty() && let Some(item) = out.iter_mut().find(|item| item.name == path) {
        item.metadata
            .attributes
            .push(format!("yaml:object_keys:{pair_count}"));
    }
}

fn collect_pair<D: ast_grep_core::Doc>(
    pair: &Node<D>,
    parent_path: &str,
    duplicate_key: Option<String>,
    ctx: &mut YamlContext,
    out: &mut Vec<ParsedItem>,
) {
    let Some(key_node) = pair.field("key") else {
        return;
    };
    let Some(value_node) = pair.field("value") else {
        return;
    };

    let key_name = yaml_helpers::key_text(&key_node);
    if key_name.is_empty() {
        return;
    }
    let full_path = yaml_helpers::path_join(parent_path, &key_name);
    let wrapped = unwrap_yaml_value(&value_node);

    let mut metadata = SymbolMetadata::default();
    metadata.set_owner_name(Some(if parent_path.is_empty() {
        "$".to_string()
    } else {
        parent_path.to_string()
    }));
    metadata.set_owner_kind(Some(SymbolKind::Module));
    metadata.set_return_type(Some(yaml_helpers::scalar_type_name(&wrapped.value)));
    metadata.push_attribute(format!("yaml:key:{key_name}"));

    if ctx.nonstandard_comments {
        metadata.push_attribute("yaml:nonstandard:comments");
    }
    if let Some(dup) = duplicate_key {
        metadata.push_attribute(format!("yaml:duplicate_key:{dup}"));
    }
    if key_name == "<<" {
        metadata.push_attribute("yaml:merge_key");
    }

    for anchor in &wrapped.anchors {
        metadata.push_attribute(format!("yaml:anchor:{anchor}"));
        ctx.anchors.insert(anchor.clone(), full_path.clone());
    }
    for tag in &wrapped.tags {
        metadata.push_attribute(format!("yaml:tag:{}", yaml_helpers::normalize_tag(tag)));
    }
    if let Some(alias) = wrapped.alias_name {
        metadata.push_attribute(format!("yaml:alias:{alias}"));
        if key_name == "<<" {
            metadata.push_attribute(format!("yaml:merge_alias:{alias}"));
        }
        if let Some(target) = ctx.anchors.get(&alias) {
            metadata.push_attribute(format!("yaml:alias_target:{target}"));
        }
    }

    enrich_shape(&wrapped.value, &mut metadata);
    enrich_block_scalar_style(&wrapped.value, &mut metadata);

    out.push(build_item(
        pair,
        SymbolKind::Property,
        full_path.clone(),
        metadata,
        &key_name,
    ));

    collect_value(&value_node, &full_path, ctx, out);
}

fn collect_sequence<D: ast_grep_core::Doc>(
    sequence: &Node<D>,
    path: &str,
    ctx: &mut YamlContext,
    out: &mut Vec<ParsedItem>,
) {
    let mut idx = 0usize;
    let mut kinds = BTreeSet::new();

    for child in sequence.children() {
        if child.kind().as_ref() != "block_sequence_item" && child.kind().as_ref() != "flow_node" {
            continue;
        }

        let value_node = if child.kind().as_ref() == "block_sequence_item" {
            child
                .children()
                .find(|node| node.kind().as_ref() == "block_node" || node.kind().as_ref() == "flow_node")
        } else {
            Some(child)
        };
        let Some(value_node) = value_node else {
            continue;
        };

        let wrapped = unwrap_yaml_value(&value_node);
        let kind_name = yaml_helpers::scalar_type_name(&wrapped.value);
        kinds.insert(kind_name.clone());

        let item_path = if path.is_empty() {
            format!("[{idx}]")
        } else {
            format!("{path}[{idx}]")
        };

        if kind_name != "object" && kind_name != "array" {
            let mut metadata = SymbolMetadata::default();
            metadata.set_owner_name(Some(if path.is_empty() {
                "$".to_string()
            } else {
                path.to_string()
            }));
            metadata.set_owner_kind(Some(SymbolKind::Module));
            metadata.set_return_type(Some(kind_name));
            metadata.push_attribute("yaml:array_element");
            if ctx.nonstandard_comments {
                metadata.push_attribute("yaml:nonstandard:comments");
            }
            if let Some(alias) = wrapped.alias_name {
                metadata.push_attribute(format!("yaml:alias:{alias}"));
                if let Some(target) = ctx.anchors.get(&alias) {
                    metadata.push_attribute(format!("yaml:alias_target:{target}"));
                }
            }
            for anchor in wrapped.anchors {
                metadata.push_attribute(format!("yaml:anchor:{anchor}"));
                ctx.anchors.insert(anchor, item_path.clone());
            }
            for tag in wrapped.tags {
                metadata.push_attribute(format!("yaml:tag:{}", yaml_helpers::normalize_tag(&tag)));
            }
            enrich_block_scalar_style(&wrapped.value, &mut metadata);

            out.push(build_item(
                &value_node,
                SymbolKind::Property,
                item_path.clone(),
                metadata,
                &item_path,
            ));
        }

        collect_value(&value_node, &item_path, ctx, out);
        idx += 1;
    }

    if !path.is_empty() && let Some(item) = out.iter_mut().find(|item| item.name == path) {
        item.metadata
            .attributes
            .push(format!("yaml:array_count:{idx}"));
        if kinds.is_empty() {
            item.metadata.attributes.push("yaml:array_elements:empty".to_string());
        } else {
            item.metadata.attributes.push(format!(
                "yaml:array_elements:{}",
                kinds.into_iter().collect::<Vec<_>>().join("|")
            ));
        }
    }
}

fn enrich_shape<D: ast_grep_core::Doc>(node: &Node<D>, metadata: &mut SymbolMetadata) {
    match node.kind().as_ref() {
        "block_mapping" | "flow_mapping" => {
            let count = node
                .children()
                .filter(|child| is_mapping_pair(child))
                .count();
            metadata.push_attribute(format!("yaml:object_keys:{count}"));
        }
        "block_sequence" | "flow_sequence" => {
            let count = node
                .children()
                .filter(|child| {
                    child.kind().as_ref() == "block_sequence_item"
                        || child.kind().as_ref() == "flow_node"
                })
                .count();
            metadata.push_attribute(format!("yaml:array_count:{count}"));
        }
        "alias" => metadata.push_attribute("yaml:alias"),
        _ => {}
    }
}

fn enrich_block_scalar_style<D: ast_grep_core::Doc>(node: &Node<D>, metadata: &mut SymbolMetadata) {
    if node.kind().as_ref() != "block_scalar" {
        return;
    }

    let raw = node.text();
    let text = raw.trim_start();
    if text.starts_with('|') {
        metadata.push_attribute("yaml:block_style:literal");
    } else if text.starts_with('>') {
        metadata.push_attribute("yaml:block_style:folded");
    }

    if let Some(header) = text.lines().next() {
        metadata.push_attribute(format!("yaml:block_header:{}", header.trim()));
    }
}

fn stream_type<D: ast_grep_core::Doc>(root: &Node<D>) -> String {
    for doc in root.children() {
        if doc.kind().as_ref() != "document" {
            continue;
        }
        for child in doc.children() {
            let kind = child.kind();
            let kr = kind.as_ref();
            if kr == "block_node" || kr == "flow_node" {
                return yaml_helpers::scalar_type_name(&child);
            }
        }
    }
    "unknown".to_string()
}

fn contains_comment<D: ast_grep_core::Doc>(node: &Node<D>) -> bool {
    if node.kind().as_ref() == "comment" {
        return true;
    }
    node.children().any(|child| contains_comment(&child))
}

fn is_mapping_pair<D: ast_grep_core::Doc>(node: &Node<D>) -> bool {
    node.kind().as_ref() == "block_mapping_pair" || node.kind().as_ref() == "flow_pair"
}

struct WrappedValue<'a, D: ast_grep_core::Doc> {
    value: Node<'a, D>,
    anchors: Vec<String>,
    tags: Vec<String>,
    alias_name: Option<String>,
}

fn unwrap_yaml_value<'a, D: ast_grep_core::Doc>(node: &Node<'a, D>) -> WrappedValue<'a, D> {
    let mut current = node.clone();
    let mut anchors = Vec::new();
    let mut tags = Vec::new();
    let mut alias_name = None;

    loop {
        let kind = current.kind();
        let kr = kind.as_ref();

        if kr != "block_node" && kr != "flow_node" {
            if kr == "alias" {
                alias_name = yaml_helpers::alias_name(&current);
            }
            break;
        }

        let mut next = None;
        for child in current.children() {
            match child.kind().as_ref() {
                "anchor" => {
                    if let Some(name) = yaml_helpers::anchor_name(&child) {
                        anchors.push(name);
                    }
                }
                "tag" => tags.push(child.text().to_string()),
                other => {
                    if other == "alias" {
                        alias_name = yaml_helpers::alias_name(&child);
                    }
                    if next.is_none() {
                        next = Some(child);
                    }
                }
            }
        }

        let Some(next_node) = next else {
            break;
        };
        current = next_node;

        if alias_name.is_some() {
            break;
        }
    }

    WrappedValue {
        value: current,
        anchors,
        tags,
        alias_name,
    }
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
