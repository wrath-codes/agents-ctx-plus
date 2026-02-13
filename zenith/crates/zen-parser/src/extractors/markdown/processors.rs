use ast_grep_core::Node;
use std::collections::HashSet;

use crate::extractors::helpers::extract_source;
use crate::types::{CommonMetadataExt, ParsedItem, SymbolKind, SymbolMetadata, Visibility};

use super::markdown_helpers;

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
    metadata.push_attribute("md:kind:document");
    build_item(
        root,
        SymbolKind::Module,
        "$".to_string(),
        "document".to_string(),
        metadata,
    )
}

pub(super) fn heading_item<D: ast_grep_core::Doc>(node: &Node<D>) -> ParsedItem {
    let raw = node.text().to_string();
    let title = markdown_helpers::heading_text(&raw);
    let level = markdown_helpers::heading_level(&raw);

    let mut metadata = SymbolMetadata::default();
    metadata.push_attribute("md:kind:heading");
    if let Some(level) = level {
        metadata.push_attribute(format!("md:level:{level}"));
    }

    let line = node.start_pos().line() + 1;
    let name = if title.is_empty() {
        format!("heading-{line}")
    } else {
        title.clone()
    };

    build_item(node, SymbolKind::Module, name, title, metadata)
}

pub(super) fn code_fence_item<D: ast_grep_core::Doc>(node: &Node<D>) -> ParsedItem {
    let raw = node.text().to_string();
    let mut metadata = SymbolMetadata::default();
    metadata.push_attribute("md:kind:code_fence");
    if let Some(lang) = markdown_helpers::code_fence_language(&raw) {
        metadata.push_attribute(format!("md:code_lang:{lang}"));
    }

    let line = node.start_pos().line() + 1;
    build_item(
        node,
        SymbolKind::Property,
        format!("code-fence-{line}"),
        markdown_helpers::first_line(&raw),
        metadata,
    )
}

pub(super) fn list_item<D: ast_grep_core::Doc>(node: &Node<D>) -> ParsedItem {
    let raw = node.text().to_string();
    let count = markdown_helpers::list_item_count(&raw);

    let mut metadata = SymbolMetadata::default();
    metadata.push_attribute("md:kind:list");
    metadata.push_attribute(format!("md:list_items:{count}"));

    let line = node.start_pos().line() + 1;
    build_item(
        node,
        SymbolKind::Property,
        format!("list-{line}"),
        "list".to_string(),
        metadata,
    )
}

pub(super) fn table_item<D: ast_grep_core::Doc>(node: &Node<D>) -> ParsedItem {
    let rows = node.text().lines().count();

    let mut metadata = SymbolMetadata::default();
    metadata.push_attribute("md:kind:table");
    metadata.push_attribute(format!("md:table_rows:{rows}"));

    let line = node.start_pos().line() + 1;
    build_item(
        node,
        SymbolKind::Property,
        format!("table-{line}"),
        "table".to_string(),
        metadata,
    )
}

pub(super) fn link_reference_item<D: ast_grep_core::Doc>(node: &Node<D>) -> ParsedItem {
    let raw = node.text().to_string();
    let label = markdown_helpers::link_reference_label(&raw);
    let line = node.start_pos().line() + 1;

    let mut metadata = SymbolMetadata::default();
    metadata.push_attribute("md:kind:link_ref");

    let name = if label.is_empty() {
        format!("link-ref-{line}")
    } else {
        label.clone()
    };

    build_item(node, SymbolKind::Property, name, label, metadata)
}

pub(super) fn thematic_break_item<D: ast_grep_core::Doc>(node: &Node<D>) -> ParsedItem {
    let mut metadata = SymbolMetadata::default();
    metadata.push_attribute("md:kind:thematic_break");

    let line = node.start_pos().line() + 1;
    build_item(
        node,
        SymbolKind::Property,
        format!("hr-{line}"),
        "---".to_string(),
        metadata,
    )
}

pub(super) fn frontmatter_item<D: ast_grep_core::Doc>(node: &Node<D>, flavor: &str) -> ParsedItem {
    let mut metadata = SymbolMetadata::default();
    metadata.push_attribute(format!("md:kind:frontmatter:{flavor}"));

    let line = node.start_pos().line() + 1;
    build_item(
        node,
        SymbolKind::Property,
        format!("frontmatter-{flavor}-{line}"),
        format!("frontmatter:{flavor}"),
        metadata,
    )
}

pub(super) fn inline_items_from_node<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<ParsedItem> {
    let text = node.text().to_string();
    let base_line = node.start_pos().line() as u32 + 1;
    let mut out = Vec::new();

    for (idx, line) in text.lines().enumerate() {
        let line_no = base_line + idx as u32;
        let mut seen_urls = HashSet::<String>::new();
        let mut seen_refs = HashSet::<String>::new();

        for (image_idx, (alt, src)) in markdown_helpers::extract_inline_images(line)
            .into_iter()
            .enumerate()
        {
            let mut metadata = SymbolMetadata::default();
            metadata.push_attribute("md:kind:inline_image");
            metadata.push_attribute(format!("md:src:{src}"));

            let name = if alt.is_empty() {
                format!("inline-image-{line_no}-{}", image_idx + 1)
            } else {
                alt.clone()
            };
            out.push(ParsedItem {
                kind: SymbolKind::Property,
                name,
                signature: format!("![{alt}]({src})"),
                source: Some(line.trim().to_string()),
                doc_comment: String::new(),
                start_line: line_no,
                end_line: line_no,
                visibility: Visibility::Public,
                metadata,
            });
        }

        for (link_idx, (label, url)) in markdown_helpers::extract_inline_links(line)
            .into_iter()
            .enumerate()
        {
            let mut metadata = SymbolMetadata::default();
            metadata.push_attribute("md:kind:inline_link");
            metadata.push_attribute(format!("md:url:{url}"));

            let name = if label.is_empty() {
                format!("inline-link-{line_no}-{}", link_idx + 1)
            } else {
                label.clone()
            };
            out.push(ParsedItem {
                kind: SymbolKind::Property,
                name,
                signature: format!("[{label}]({url})"),
                source: Some(line.trim().to_string()),
                doc_comment: String::new(),
                start_line: line_no,
                end_line: line_no,
                visibility: Visibility::Public,
                metadata,
            });
        }

        for (ref_idx, (label, reference)) in markdown_helpers::extract_reference_links(line)
            .into_iter()
            .enumerate()
        {
            if !seen_refs.insert(reference.clone()) {
                continue;
            }

            let mut metadata = SymbolMetadata::default();
            metadata.push_attribute("md:kind:inline_ref_link");
            metadata.push_attribute(format!("md:ref:{reference}"));

            let name = if label.is_empty() {
                format!("inline-ref-{line_no}-{}", ref_idx + 1)
            } else {
                label.clone()
            };

            out.push(ParsedItem {
                kind: SymbolKind::Property,
                name,
                signature: format!("[{label}][{reference}]"),
                source: Some(line.trim().to_string()),
                doc_comment: String::new(),
                start_line: line_no,
                end_line: line_no,
                visibility: Visibility::Public,
                metadata,
            });
        }

        for (auto_idx, target) in markdown_helpers::extract_autolinks(line)
            .into_iter()
            .enumerate()
        {
            let mut metadata = SymbolMetadata::default();
            metadata.push_attribute("md:kind:autolink");
            metadata.push_attribute(format!("md:url:{target}"));

            out.push(ParsedItem {
                kind: SymbolKind::Property,
                name: format!("autolink-{line_no}-{}", auto_idx + 1),
                signature: format!("<{target}>"),
                source: Some(line.trim().to_string()),
                doc_comment: String::new(),
                start_line: line_no,
                end_line: line_no,
                visibility: Visibility::Public,
                metadata,
            });
            seen_urls.insert(target);
        }

        for (bare_idx, target) in markdown_helpers::extract_bare_urls(line)
            .into_iter()
            .enumerate()
        {
            if !seen_urls.insert(target.clone()) {
                continue;
            }

            let mut metadata = SymbolMetadata::default();
            metadata.push_attribute("md:kind:bare_url");
            metadata.push_attribute(format!("md:url:{target}"));

            out.push(ParsedItem {
                kind: SymbolKind::Property,
                name: format!("bare-url-{line_no}-{}", bare_idx + 1),
                signature: target,
                source: Some(line.trim().to_string()),
                doc_comment: String::new(),
                start_line: line_no,
                end_line: line_no,
                visibility: Visibility::Public,
                metadata,
            });
        }

        for (code_idx, code) in markdown_helpers::extract_inline_code(line)
            .into_iter()
            .enumerate()
        {
            let mut metadata = SymbolMetadata::default();
            metadata.push_attribute("md:kind:inline_code");

            out.push(ParsedItem {
                kind: SymbolKind::Property,
                name: format!("inline-code-{line_no}-{}", code_idx + 1),
                signature: code,
                source: Some(line.trim().to_string()),
                doc_comment: String::new(),
                start_line: line_no,
                end_line: line_no,
                visibility: Visibility::Public,
                metadata,
            });
        }
    }

    out
}
