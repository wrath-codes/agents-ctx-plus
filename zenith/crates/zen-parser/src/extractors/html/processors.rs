use ast_grep_core::Node;

use crate::types::{HtmlMetadataExt, ParsedItem, SymbolKind, SymbolMetadata, Visibility};

use super::html_helpers::{
    attr_value, build_signature, classify_tag, extract_source_limited, extract_start_tag_attrs,
    extract_tag_info, is_significant_tag,
};

pub(super) fn collect_elements<D: ast_grep_core::Doc>(node: &Node<D>, items: &mut Vec<ParsedItem>) {
    let kind = node.kind();
    match kind.as_ref() {
        "element" => process_element(node, items),
        "script_element" => process_script_element(node, items),
        "style_element" => process_style_element(node, items),
        _ => {}
    }

    // Recurse into children without holding borrows across iterations
    let children: Vec<_> = node.children().collect();
    for child in &children {
        collect_elements(child, items);
    }
}

fn process_element<D: ast_grep_core::Doc>(node: &Node<D>, items: &mut Vec<ParsedItem>) {
    let Some((tag_name, attrs)) = extract_tag_info(node) else {
        return;
    };

    let element_id = attr_value(&attrs, "id");
    let class_names = attr_value(&attrs, "class")
        .map(|c| c.split_whitespace().map(String::from).collect::<Vec<_>>())
        .unwrap_or_default();
    let is_custom = tag_name.contains('-');
    let has_end_tag = node.children().any(|c| c.kind().as_ref() == "end_tag");
    let is_self_closing = !has_end_tag;

    let should_extract = is_custom || element_id.is_some() || is_significant_tag(&tag_name);

    if !should_extract {
        return;
    }

    let symbol_kind = if is_custom {
        SymbolKind::Component
    } else {
        classify_tag(&tag_name)
    };

    let name = if is_custom {
        tag_name.clone()
    } else if let Some(ref id) = element_id {
        id.clone()
    } else {
        tag_name.clone()
    };

    let signature = build_signature(&tag_name, &attrs);

    let mut metadata = SymbolMetadata::default();
    metadata.set_tag_name(tag_name);
    metadata.set_element_id(element_id);
    metadata.set_class_names(class_names);
    metadata.set_html_attributes(attrs);
    metadata.set_custom_element(is_custom);
    metadata.set_self_closing(is_self_closing);

    items.push(ParsedItem {
        kind: symbol_kind,
        name,
        signature,
        source: extract_source_limited(node, 20),
        doc_comment: String::new(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata,
    });
}

fn process_script_element<D: ast_grep_core::Doc>(node: &Node<D>, items: &mut Vec<ParsedItem>) {
    let attrs = extract_start_tag_attrs(node);
    let src = attr_value(&attrs, "src");
    let script_type = attr_value(&attrs, "type");

    let name = match src {
        Some(ref s) => s.clone(),
        None if script_type.as_deref() == Some("module") => "inline-module".to_string(),
        None => "inline-script".to_string(),
    };

    let signature = build_signature("script", &attrs);

    let mut metadata = SymbolMetadata::default();
    metadata.set_tag_name("script");
    metadata.set_html_attributes(attrs);

    items.push(ParsedItem {
        kind: SymbolKind::Module,
        name,
        signature,
        source: extract_source_limited(node, 10),
        doc_comment: String::new(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata,
    });
}

fn process_style_element<D: ast_grep_core::Doc>(node: &Node<D>, items: &mut Vec<ParsedItem>) {
    let attrs = extract_start_tag_attrs(node);
    let signature = build_signature("style", &attrs);

    let mut metadata = SymbolMetadata::default();
    metadata.set_tag_name("style");
    metadata.set_html_attributes(attrs);

    items.push(ParsedItem {
        kind: SymbolKind::Module,
        name: "inline-style".to_string(),
        signature,
        source: extract_source_limited(node, 10),
        doc_comment: String::new(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata,
    });
}
