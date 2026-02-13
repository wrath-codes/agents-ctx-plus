use ast_grep_core::Node;

use crate::extractors::helpers::extract_source;
use crate::types::{CommonMetadataExt, ParsedItem, SymbolKind, SymbolMetadata, Visibility};

use super::rst_helpers;

fn build_item<D: ast_grep_core::Doc>(
    node: &Node<D>,
    kind: SymbolKind,
    name: String,
    signature: String,
    metadata: SymbolMetadata,
) -> ParsedItem {
    ParsedItem {
        kind,
        name,
        signature,
        source: extract_source(node, 40),
        doc_comment: String::new(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata,
    }
}

pub(super) fn root_item<D: ast_grep_core::Doc>(root: &Node<D>) -> ParsedItem {
    let mut metadata = SymbolMetadata::default();
    metadata.push_attribute("rst:kind:document");
    build_item(
        root,
        SymbolKind::Module,
        "$".to_string(),
        "document".to_string(),
        metadata,
    )
}

pub(super) fn section_item<D: ast_grep_core::Doc>(node: &Node<D>, level: u8) -> ParsedItem {
    let title = rst_helpers::section_title(node);
    let mut metadata = SymbolMetadata::default();
    metadata.push_attribute("rst:kind:section");
    metadata.push_attribute(format!("rst:section_level:{level}"));
    build_item(node, SymbolKind::Module, title.clone(), title, metadata)
}

pub(super) fn directive_item<D: ast_grep_core::Doc>(node: &Node<D>) -> ParsedItem {
    let name = rst_helpers::directive_name(node);
    let (args_text, arg_count, option_count, body_lines) = rst_helpers::directive_parts(node);
    let options = rst_helpers::directive_option_pairs(node);

    let mut metadata = SymbolMetadata::default();
    metadata.push_attribute("rst:kind:directive");
    metadata.push_attribute(format!("rst:directive:{name}"));
    metadata.push_attribute(format!("rst:directive_args:{arg_count}"));
    metadata.push_attribute(format!("rst:directive_options:{option_count}"));
    metadata.push_attribute(format!("rst:directive_body_lines:{body_lines}"));

    if let Some(args) = &args_text {
        metadata.push_attribute(format!("rst:directive_args_text:{args}"));
    }
    if option_count == 0 && node.text().contains(':') && node.text().contains(".. ") {
        metadata.push_attribute("rst:directive_options_malformed");
    }

    if matches!(name.as_str(), "include" | "literalinclude") {
        metadata.push_attribute("rst:include_directive");
        if let Some(args) = args_text {
            metadata.push_attribute(format!("rst:include:path:{args}"));
        }
    }

    if matches!(name.as_str(), "code" | "code-block" | "sourcecode") {
        metadata.push_attribute("rst:code_directive");
        if let Some(args) = options
            .get("language")
            .cloned()
            .or_else(|| options.get("lang").cloned())
            .or_else(|| {
                rst_helpers::directive_parts(node)
                    .0
                    .and_then(|a| a.split_whitespace().next().map(|s| s.to_string()))
            })
        {
            metadata.push_attribute(format!("rst:code_lang:{args}"));
        }
    }

    if name.contains(':') {
        metadata.push_attribute("rst:sphinx_directive");
    }

    for (k, v) in options {
        if v.is_empty() {
            metadata.push_attribute(format!("rst:directive_option:{k}"));
        } else {
            metadata.push_attribute(format!("rst:directive_option:{k}={v}"));
        }
    }

    build_item(
        node,
        SymbolKind::Property,
        format!("directive:{name}"),
        name,
        metadata,
    )
}

pub(super) fn list_item<D: ast_grep_core::Doc>(node: &Node<D>, kind: &str) -> ParsedItem {
    let count = node
        .children()
        .filter(|c| c.kind().as_ref() == "list_item")
        .count();
    let mut metadata = SymbolMetadata::default();
    metadata.push_attribute(format!("rst:kind:{kind}"));
    metadata.push_attribute(format!("rst:list_items:{count}"));
    let line = node.start_pos().line() + 1;
    build_item(
        node,
        SymbolKind::Property,
        format!("{kind}-{line}"),
        kind.to_string(),
        metadata,
    )
}

pub(super) fn block_item<D: ast_grep_core::Doc>(node: &Node<D>, kind: &str) -> ParsedItem {
    let mut metadata = SymbolMetadata::default();
    metadata.push_attribute(format!("rst:kind:{kind}"));
    let line = node.start_pos().line() + 1;
    build_item(
        node,
        SymbolKind::Property,
        format!("{kind}-{line}"),
        kind.to_string(),
        metadata,
    )
}

pub(super) fn citation_item<D: ast_grep_core::Doc>(node: &Node<D>) -> ParsedItem {
    let label = node
        .field("name")
        .map(|n| rst_helpers::normalize_label(&rst_helpers::inline_text(&n)))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "citation".to_string());
    let mut metadata = SymbolMetadata::default();
    metadata.push_attribute("rst:kind:citation");
    metadata.push_attribute(format!("rst:label:citation:{label}"));
    build_item(
        node,
        SymbolKind::Property,
        format!("citation:{label}"),
        label,
        metadata,
    )
}

pub(super) fn substitution_definition_item<D: ast_grep_core::Doc>(node: &Node<D>) -> ParsedItem {
    let label = node
        .field("name")
        .map(|n| rst_helpers::normalize_label(&rst_helpers::inline_text(&n)))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "substitution".to_string());
    let mut metadata = SymbolMetadata::default();
    metadata.push_attribute("rst:kind:substitution_definition");
    metadata.push_attribute(format!("rst:label:substitution:{label}"));
    build_item(
        node,
        SymbolKind::Property,
        format!("substitution:{label}"),
        label,
        metadata,
    )
}

pub(super) fn comment_item<D: ast_grep_core::Doc>(node: &Node<D>) -> ParsedItem {
    let mut metadata = SymbolMetadata::default();
    metadata.push_attribute("rst:kind:comment");
    let line = node.start_pos().line() + 1;
    build_item(
        node,
        SymbolKind::Property,
        format!("comment-{line}"),
        "comment".to_string(),
        metadata,
    )
}

pub(super) fn footnote_item<D: ast_grep_core::Doc>(node: &Node<D>) -> ParsedItem {
    let label = node
        .field("name")
        .map(|n| rst_helpers::normalize_label(&rst_helpers::inline_text(&n)))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "footnote".to_string());
    let mut metadata = SymbolMetadata::default();
    metadata.push_attribute("rst:kind:footnote");
    metadata.push_attribute(format!("rst:label:footnote:{label}"));
    build_item(
        node,
        SymbolKind::Property,
        format!("footnote:{label}"),
        label,
        metadata,
    )
}

pub(super) fn target_item<D: ast_grep_core::Doc>(node: &Node<D>) -> ParsedItem {
    let name = rst_helpers::target_name(node);
    let mut metadata = SymbolMetadata::default();
    metadata.push_attribute("rst:kind:target");
    metadata.push_attribute(format!("rst:label:target:{name}"));
    build_item(
        node,
        SymbolKind::Property,
        format!("target:{name}"),
        name,
        metadata,
    )
}

pub(super) fn generic_named_item<D: ast_grep_core::Doc>(node: &Node<D>, kind: &str) -> ParsedItem {
    let mut metadata = SymbolMetadata::default();
    metadata.push_attribute(format!("rst:kind:{kind}"));
    let line = node.start_pos().line() + 1;
    build_item(
        node,
        SymbolKind::Property,
        format!("{kind}-{line}"),
        rst_helpers::first_line(&node.text()),
        metadata,
    )
}

pub(super) fn inline_item<D: ast_grep_core::Doc>(
    node: &Node<D>,
    owner: &str,
    inline_kind: &str,
) -> ParsedItem {
    let text = rst_helpers::inline_text(node);
    let line = node.start_pos().line() + 1;
    let mut metadata = SymbolMetadata::default();
    metadata.push_attribute(format!("rst:kind:{inline_kind}"));
    metadata.set_owner_name(Some(owner.to_string()));
    metadata.set_owner_kind(Some(SymbolKind::Module));

    match inline_kind {
        "reference" => metadata.push_attribute(format!(
            "rst:ref_label:{}",
            rst_helpers::parse_reference_label(&text)
        )),
        "footnote_reference" => metadata.push_attribute(format!(
            "rst:ref_label:footnote:{}",
            rst_helpers::parse_footnote_ref_label(&text)
        )),
        "citation_reference" => metadata.push_attribute(format!(
            "rst:ref_label:citation:{}",
            rst_helpers::parse_footnote_ref_label(&text)
        )),
        "substitution_reference" => metadata.push_attribute(format!(
            "rst:ref_label:substitution:{}",
            rst_helpers::normalize_label(&text)
        )),
        "interpreted_text" => {
            if let Some(role) = rst_helpers::extract_role_name(node) {
                metadata.push_attribute(format!("rst:role:{role}"));
                if role.contains(':') {
                    metadata.push_attribute("rst:sphinx_role");
                }
            }
        }
        _ => {}
    }

    build_item(
        node,
        SymbolKind::Property,
        format!("{inline_kind}-{line}"),
        text,
        metadata,
    )
}

pub(super) fn virtual_table_item(
    start_line: u32,
    end_line: u32,
    table_kind: &str,
    row_count: usize,
    col_count: usize,
    owner: &str,
) -> ParsedItem {
    let mut metadata = SymbolMetadata::default();
    metadata.push_attribute(format!("rst:kind:{table_kind}"));
    metadata.push_attribute(format!("rst:table_rows:{row_count}"));
    metadata.push_attribute(format!("rst:table_cols:{col_count}"));
    metadata.set_owner_name(Some(owner.to_string()));
    metadata.set_owner_kind(Some(SymbolKind::Module));

    ParsedItem {
        kind: SymbolKind::Property,
        name: format!("{table_kind}-{start_line}"),
        signature: table_kind.to_string(),
        source: None,
        doc_comment: String::new(),
        start_line,
        end_line,
        visibility: Visibility::Public,
        metadata,
    }
}
