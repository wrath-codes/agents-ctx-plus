use ast_grep_core::Node;

use crate::types::{CssMetadataExt, ParsedItem, SymbolKind, SymbolMetadata, Visibility};

use super::css_helpers::{
    build_rule_name, build_rule_signature, classify_selector, extract_custom_properties,
    extract_media_query_text, extract_properties, extract_properties_from_block,
    extract_source_limited, extract_url_from_node,
};

pub(super) fn collect_nodes<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    parent_context: Option<&str>,
) {
    let kind = node.kind();
    let recurse = match kind.as_ref() {
        "rule_set" => {
            process_rule_set(node, items, parent_context);
            false // leaf — no further recursion needed
        }
        "media_statement" => {
            process_media_statement(node, items);
            false // handles its own recursion into block
        }
        "keyframes_statement" => {
            process_keyframes(node, items);
            false
        }
        "import_statement" => {
            process_import(node, items);
            false
        }
        "charset_statement" => {
            process_charset(node, items);
            false
        }
        "namespace_statement" => {
            process_namespace(node, items);
            false
        }
        "supports_statement" => {
            process_supports(node, items);
            false // handles its own recursion
        }
        "at_rule" => {
            process_at_rule(node, items);
            false // handles its own recursion
        }
        "scope_statement" => {
            process_scope(node, items);
            false // handles its own recursion
        }
        _ => true, // recurse for stylesheet, etc.
    };

    if recurse {
        let children: Vec<_> = node.children().collect();
        for child in &children {
            collect_nodes(child, items, parent_context);
        }
    }
}

// ── Rule set processing ────────────────────────────────────────────

fn process_rule_set<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    parent_context: Option<&str>,
) {
    let children: Vec<_> = node.children().collect();

    let selector_text = children
        .iter()
        .find(|c| c.kind().as_ref() == "selectors")
        .map(|s| s.text().to_string())
        .unwrap_or_default();

    let properties = extract_properties(node);
    let custom_props = extract_custom_properties(node);

    // Emit custom properties as individual items
    for (prop_name, prop_value) in &custom_props {
        let mut metadata = SymbolMetadata::default();
        metadata.set_selector(selector_text.clone());
        metadata.mark_custom_property();

        items.push(ParsedItem {
            kind: SymbolKind::Const,
            name: prop_name.clone(),
            signature: format!("{prop_name}: {prop_value}"),
            source: None,
            doc_comment: String::new(),
            start_line: node.start_pos().line() as u32 + 1,
            end_line: node.end_pos().line() as u32 + 1,
            visibility: Visibility::Public,
            metadata,
        });
    }

    // Determine the kind and name for the rule set
    let symbol_kind = classify_selector(&selector_text);
    let name = build_rule_name(&selector_text, parent_context);

    let signature = build_rule_signature(&selector_text, &properties);

    let mut metadata = SymbolMetadata::default();
    metadata.set_selector(selector_text);
    metadata.set_css_properties(properties);

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

// ── @media processing ──────────────────────────────────────────────

fn process_media_statement<D: ast_grep_core::Doc>(node: &Node<D>, items: &mut Vec<ParsedItem>) {
    let children: Vec<_> = node.children().collect();

    let query = extract_media_query_text(&children);
    let name = format!("@media {query}");
    let signature = name.clone();

    let mut metadata = SymbolMetadata::default();
    metadata.set_at_rule_name("media");
    metadata.set_media_query(query.clone());

    items.push(ParsedItem {
        kind: SymbolKind::Module,
        name,
        signature,
        source: extract_source_limited(node, 20),
        doc_comment: String::new(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata,
    });

    // Recurse into the block to collect nested rules
    for child in &children {
        if child.kind().as_ref() == "block" {
            let block_children: Vec<_> = child.children().collect();
            for bc in &block_children {
                collect_nodes(bc, items, Some(&query));
            }
        }
    }
}

// ── @keyframes processing ──────────────────────────────────────────

fn process_keyframes<D: ast_grep_core::Doc>(node: &Node<D>, items: &mut Vec<ParsedItem>) {
    let children: Vec<_> = node.children().collect();

    let anim_name = children
        .iter()
        .find(|c| c.kind().as_ref() == "keyframes_name")
        .map_or_else(|| "unnamed".to_string(), |n| n.text().to_string());

    let name = format!("@keyframes {anim_name}");
    let signature = name.clone();

    let mut metadata = SymbolMetadata::default();
    metadata.set_at_rule_name("keyframes");

    items.push(ParsedItem {
        kind: SymbolKind::Function,
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

// ── @import processing ─────────────────────────────────────────────

fn process_import<D: ast_grep_core::Doc>(node: &Node<D>, items: &mut Vec<ParsedItem>) {
    let url = extract_url_from_node(node).unwrap_or_else(|| "unknown".to_string());

    let name = url.clone();
    let signature = format!("@import url('{url}')");

    let mut metadata = SymbolMetadata::default();
    metadata.set_at_rule_name("import");

    items.push(ParsedItem {
        kind: SymbolKind::Module,
        name,
        signature,
        source: Some(node.text().to_string()),
        doc_comment: String::new(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata,
    });
}

// ── @charset processing ────────────────────────────────────────────

fn process_charset<D: ast_grep_core::Doc>(node: &Node<D>, items: &mut Vec<ParsedItem>) {
    let children: Vec<_> = node.children().collect();
    let charset = children
        .iter()
        .find(|c| c.kind().as_ref() == "string_value")
        .and_then(|sv| {
            sv.children()
                .find(|c| c.kind().as_ref() == "string_content")
                .map(|sc| sc.text().to_string())
        })
        .unwrap_or_else(|| "unknown".to_string());

    let name = format!("@charset {charset}");
    let signature = format!("@charset \"{charset}\"");

    let mut metadata = SymbolMetadata::default();
    metadata.set_at_rule_name("charset");

    items.push(ParsedItem {
        kind: SymbolKind::Const,
        name,
        signature,
        source: Some(node.text().to_string()),
        doc_comment: String::new(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata,
    });
}

// ── @namespace processing ──────────────────────────────────────────

fn process_namespace<D: ast_grep_core::Doc>(node: &Node<D>, items: &mut Vec<ParsedItem>) {
    let children: Vec<_> = node.children().collect();
    let ns_name = children
        .iter()
        .find(|c| c.kind().as_ref() == "namespace_name")
        .map_or_else(|| "default".to_string(), |n| n.text().to_string());

    let url = extract_url_from_node(node).unwrap_or_else(|| "unknown".to_string());

    let name = format!("@namespace {ns_name}");
    let signature = format!("@namespace {ns_name} url({url})");

    let mut metadata = SymbolMetadata::default();
    metadata.set_at_rule_name("namespace");

    items.push(ParsedItem {
        kind: SymbolKind::Module,
        name,
        signature,
        source: Some(node.text().to_string()),
        doc_comment: String::new(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata,
    });
}

// ── @supports processing ──────────────────────────────────────────

fn process_supports<D: ast_grep_core::Doc>(node: &Node<D>, items: &mut Vec<ParsedItem>) {
    let children: Vec<_> = node.children().collect();

    let query = children
        .iter()
        .find(|c| c.kind().as_ref() == "feature_query")
        .map(|fq| fq.text().to_string())
        .unwrap_or_default();

    let name = format!("@supports {query}");
    let signature = name.clone();

    let mut metadata = SymbolMetadata::default();
    metadata.set_at_rule_name("supports");
    metadata.set_media_query(query.clone());

    items.push(ParsedItem {
        kind: SymbolKind::Module,
        name,
        signature,
        source: extract_source_limited(node, 20),
        doc_comment: String::new(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata,
    });

    // Recurse into block for nested rules
    for child in &children {
        if child.kind().as_ref() == "block" {
            let block_children: Vec<_> = child.children().collect();
            for bc in &block_children {
                collect_nodes(bc, items, Some(&query));
            }
        }
    }
}

// ── @scope processing ──────────────────────────────────────────────

fn process_scope<D: ast_grep_core::Doc>(node: &Node<D>, items: &mut Vec<ParsedItem>) {
    let children: Vec<_> = node.children().collect();

    // Build scope description from the selectors between parentheses.
    // AST: @scope ( .from-selector ) to ( .to-selector ) block
    // We extract the class_selector children to build the name.
    let mut from_sel = None;
    let mut to_sel = None;
    let mut seen_to = false;
    let mut seen_first_close = false;

    for child in &children {
        let k = child.kind();
        match k.as_ref() {
            "to" => seen_to = true,
            ")" => seen_first_close = true,
            "class_selector"
            | "id_selector"
            | "tag_name"
            | "pseudo_class_selector"
            | "universal_selector" => {
                let text = child.text().to_string();
                if seen_to {
                    to_sel = Some(text);
                } else if seen_first_close {
                    // after first close paren = unexpected, ignore
                } else {
                    from_sel = Some(text);
                }
            }
            _ => {}
        }
    }

    let name = match (&from_sel, &to_sel) {
        (Some(f), Some(t)) => format!("@scope ({f}) to ({t})"),
        (Some(f), None) => format!("@scope ({f})"),
        _ => "@scope".to_string(),
    };
    let signature = name.clone();

    let mut metadata = SymbolMetadata::default();
    metadata.set_at_rule_name("scope");

    items.push(ParsedItem {
        kind: SymbolKind::Module,
        name,
        signature,
        source: extract_source_limited(node, 20),
        doc_comment: String::new(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata,
    });

    // Recurse into block for nested rules
    for child in &children {
        if child.kind().as_ref() == "block" {
            let block_children: Vec<_> = child.children().collect();
            for bc in &block_children {
                collect_nodes(bc, items, from_sel.as_deref());
            }
        }
    }
}

// ── Generic @rule processing (font-face, layer, container, etc.) ──

fn process_at_rule<D: ast_grep_core::Doc>(node: &Node<D>, items: &mut Vec<ParsedItem>) {
    let children: Vec<_> = node.children().collect();

    let at_keyword = children
        .iter()
        .find(|c| c.kind().as_ref() == "at_keyword")
        .map(|k| k.text().to_string())
        .unwrap_or_default();

    // Strip the leading '@'
    let rule_name = at_keyword.trim_start_matches('@');

    match rule_name {
        "font-face" => process_font_face(node, items, &children),
        "layer" => process_layer(node, items, &children),
        "container" => process_container(node, items, &children),
        _ => process_generic_at_rule(node, items, rule_name, &children),
    }
}

fn process_font_face<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    children: &[Node<D>],
) {
    let properties = extract_properties_from_block(children);

    // Try to extract font-family name from declarations
    let font_family = properties
        .iter()
        .find(|p| p.starts_with("font-family:"))
        .map_or_else(
            || "unnamed".to_string(),
            |p| p.trim_start_matches("font-family:").trim().to_string(),
        );

    let name = format!("@font-face {font_family}");
    let signature = format!("@font-face {{ font-family: {font_family} }}");

    let mut metadata = SymbolMetadata::default();
    metadata.set_at_rule_name("font-face");
    metadata.set_css_properties(properties);

    items.push(ParsedItem {
        kind: SymbolKind::Struct,
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

fn process_layer<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    children: &[Node<D>],
) {
    let layer_name = children
        .iter()
        .find(|c| c.kind().as_ref() == "keyword_query")
        .map_or_else(|| "anonymous".to_string(), |n| n.text().to_string());

    let name = format!("@layer {layer_name}");
    let signature = name.clone();

    let mut metadata = SymbolMetadata::default();
    metadata.set_at_rule_name("layer");

    items.push(ParsedItem {
        kind: SymbolKind::Module,
        name,
        signature,
        source: extract_source_limited(node, 20),
        doc_comment: String::new(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata,
    });

    // Recurse into block for nested rules
    for child in children {
        if child.kind().as_ref() == "block" {
            let block_children: Vec<_> = child.children().collect();
            for bc in &block_children {
                collect_nodes(bc, items, Some(&layer_name));
            }
        }
    }
}

fn process_container<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    children: &[Node<D>],
) {
    let container_name = children
        .iter()
        .find(|c| c.kind().as_ref() == "keyword_query")
        .map(|n| n.text().to_string());

    let query = children
        .iter()
        .find(|c| c.kind().as_ref() == "feature_query")
        .map(|fq| fq.text().to_string())
        .unwrap_or_default();

    let name = container_name.as_ref().map_or_else(
        || format!("@container {query}"),
        |cn| format!("@container {cn} {query}"),
    );
    let signature = name.clone();

    let mut metadata = SymbolMetadata::default();
    metadata.set_at_rule_name("container");
    metadata.set_media_query(query);

    items.push(ParsedItem {
        kind: SymbolKind::Module,
        name,
        signature,
        source: extract_source_limited(node, 20),
        doc_comment: String::new(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata,
    });

    // Recurse into block for nested rules
    for child in children {
        if child.kind().as_ref() == "block" {
            let block_children: Vec<_> = child.children().collect();
            for bc in &block_children {
                collect_nodes(bc, items, None);
            }
        }
    }
}

fn process_generic_at_rule<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    rule_name: &str,
    children: &[Node<D>],
) {
    // Extract optional keyword_query for named at-rules
    // (e.g., `@property --my-color`, `@counter-style thumbs`)
    let keyword = children
        .iter()
        .find(|c| c.kind().as_ref() == "keyword_query")
        .map(|n| n.text().to_string());

    let name = keyword.as_ref().map_or_else(
        || format!("@{rule_name}"),
        |kw| format!("@{rule_name} {kw}"),
    );

    let text = node.text().to_string();
    // Take the first line as signature
    let signature = text.lines().next().unwrap_or(&text).to_string();

    let properties = extract_properties_from_block(children);

    let mut metadata = SymbolMetadata::default();
    metadata.set_at_rule_name(rule_name.to_string());
    metadata.set_css_properties(properties);

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

    // Recurse into block for nested rules (e.g., `@starting-style`)
    for child in children {
        if child.kind().as_ref() == "block" {
            let block_children: Vec<_> = child.children().collect();
            for bc in &block_children {
                if bc.kind().as_ref() == "rule_set" {
                    collect_nodes(bc, items, keyword.as_deref());
                }
            }
        }
    }
}
