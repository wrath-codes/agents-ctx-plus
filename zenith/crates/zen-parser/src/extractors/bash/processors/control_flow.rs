use ast_grep_core::Node;

use crate::types::{BashMetadataExt, ParsedItem, SymbolKind, SymbolMetadata, Visibility};

use super::super::helpers::{extract_source_limited, truncate_text};

pub(in super::super) fn process_if_statement<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
) {
    let children: Vec<_> = node.children().collect();

    // Extract the condition (first test_command or command after `if`)
    let condition = children
        .iter()
        .find(|c| {
            let k = c.kind();
            k.as_ref() == "test_command" || k.as_ref() == "command"
        })
        .map_or_else(|| "...".to_string(), |c| c.text().to_string());

    let condition_short = truncate_text(&condition, 60);
    let name = format!("if {condition_short}");
    let signature = format!("if {condition_short}; then ... fi");

    // Count elif clauses
    let elif_count = children
        .iter()
        .filter(|c| c.kind().as_ref() == "elif_clause")
        .count();
    let has_else = children
        .iter()
        .any(|c| c.kind().as_ref() == "else_clause" || c.kind().as_ref() == "else");

    let mut metadata = SymbolMetadata::default();
    if elif_count > 0 {
        metadata.push_attribute(format!("elif_count={elif_count}"));
    }
    if has_else {
        metadata.push_attribute("has_else");
    }

    items.push(ParsedItem {
        kind: SymbolKind::Enum,
        name,
        signature,
        source: extract_source_limited(node, 20),
        doc_comment: doc_comment.to_string(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata,
    });
}

pub(in super::super) fn process_case_statement<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
) {
    let children: Vec<_> = node.children().collect();

    // Extract the case expression
    let expr = children
        .iter()
        .find(|c| {
            let k = c.kind();
            k.as_ref() == "string"
                || k.as_ref() == "simple_expansion"
                || k.as_ref() == "word"
                || k.as_ref() == "expansion"
        })
        .map_or_else(|| "...".to_string(), |c| c.text().to_string());

    // Collect case patterns
    let patterns: Vec<String> = children
        .iter()
        .filter(|c| c.kind().as_ref() == "case_item")
        .map(|ci| {
            let ci_children: Vec<_> = ci.children().collect();
            ci_children
                .iter()
                .take_while(|c| c.kind().as_ref() != ")")
                .filter(|c| {
                    let k = c.kind();
                    k.as_ref() == "word"
                        || k.as_ref() == "concatenation"
                        || k.as_ref() == "extglob_pattern"
                })
                .map(|c| c.text().to_string())
                .collect::<Vec<_>>()
                .join("|")
        })
        .collect();

    let name = format!("case {expr}");
    let signature = format!("case {expr} in ... esac");

    items.push(ParsedItem {
        kind: SymbolKind::Enum,
        name,
        signature,
        source: extract_source_limited(node, 20),
        doc_comment: doc_comment.to_string(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata: SymbolMetadata {
            variants: patterns,
            ..Default::default()
        },
    });
}

pub(in super::super) fn process_for_statement<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
) {
    let children: Vec<_> = node.children().collect();

    // Detect if this is a `select` or `for` loop
    let is_select = children.iter().any(|c| c.kind().as_ref() == "select");

    let loop_var = children
        .iter()
        .find(|c| c.kind().as_ref() == "variable_name")
        .map_or_else(String::new, |n| n.text().to_string());

    let keyword = if is_select { "select" } else { "for" };

    // Collect iteration values
    let in_idx = children.iter().position(|c| c.kind().as_ref() == "in");

    let iter_values = in_idx.map_or_else(String::new, |idx| {
        let vals: Vec<String> = children[idx + 1..]
            .iter()
            .take_while(|c| {
                let k = c.kind();
                k.as_ref() != ";" && k.as_ref() != "do_group"
            })
            .map(|c| c.text().to_string())
            .collect();
        vals.join(" ")
    });

    let name = if iter_values.is_empty() {
        format!("{keyword} {loop_var}")
    } else {
        let short_vals = truncate_text(&iter_values, 40);
        format!("{keyword} {loop_var} in {short_vals}")
    };

    let signature = format!("{name}; do ... done");

    let kind = if is_select {
        SymbolKind::Enum
    } else {
        SymbolKind::Macro
    };

    let mut metadata = SymbolMetadata::default();
    metadata.push_attribute(keyword);
    if !loop_var.is_empty() {
        metadata.push_parameter(loop_var);
    }

    items.push(ParsedItem {
        kind,
        name,
        signature,
        source: extract_source_limited(node, 15),
        doc_comment: doc_comment.to_string(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata,
    });
}

pub(in super::super) fn process_while_statement<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
) {
    let children: Vec<_> = node.children().collect();

    // Detect if this is `until` or `while`
    let is_until = children.iter().any(|c| c.kind().as_ref() == "until");

    let keyword = if is_until { "until" } else { "while" };

    // Extract condition
    let condition = children
        .iter()
        .find(|c| {
            let k = c.kind();
            k.as_ref() == "test_command" || k.as_ref() == "command"
        })
        .map_or_else(|| "...".to_string(), |c| c.text().to_string());

    let condition_short = truncate_text(&condition, 60);
    let name = format!("{keyword} {condition_short}");
    let signature = format!("{keyword} {condition_short}; do ... done");

    items.push(ParsedItem {
        kind: SymbolKind::Macro,
        name,
        signature,
        source: extract_source_limited(node, 15),
        doc_comment: doc_comment.to_string(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata: SymbolMetadata {
            attributes: vec![keyword.to_string()],
            ..Default::default()
        },
    });
}

pub(in super::super) fn process_c_style_for<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
) {
    let text = node.text().to_string();
    // Extract the (( ... )) header from the full text
    let header = text.find("do").map_or_else(
        || truncate_text(&text, 60),
        |pos| {
            let h = text[..pos].trim().trim_end_matches(';').trim();
            h.to_string()
        },
    );

    let name = format!("for {header}");
    let signature = format!("{name}; do ... done");

    items.push(ParsedItem {
        kind: SymbolKind::Macro,
        name: truncate_text(&name, 80),
        signature,
        source: extract_source_limited(node, 15),
        doc_comment: doc_comment.to_string(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata: SymbolMetadata {
            attributes: vec!["for".to_string(), "c_style".to_string()],
            ..Default::default()
        },
    });
}
