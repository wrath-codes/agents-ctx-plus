use ast_grep_core::Node;

use crate::types::{BashMetadataExt, ParsedItem, SymbolKind, SymbolMetadata, Visibility};

use super::super::helpers::{extract_source_limited, truncate_text};
use super::control_flow::{
    process_case_statement, process_for_statement, process_if_statement, process_while_statement,
};

pub(in super::super) fn process_pipeline<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
) {
    let children: Vec<_> = node.children().collect();

    let commands: Vec<String> = children
        .iter()
        .filter(|c| c.kind().as_ref() == "command")
        .map(|c| {
            // Extract just the command name for summary
            let cmd_children: Vec<_> = c.children().collect();
            cmd_children
                .iter()
                .find(|cc| cc.kind().as_ref() == "command_name")
                .map_or_else(|| c.text().to_string(), |cn| cn.text().to_string())
        })
        .collect();

    let pipeline_text = commands.join(" | ");
    let name = truncate_text(&pipeline_text, 60);
    let signature = node
        .text()
        .to_string()
        .replace('\n', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let sig_short = truncate_text(&signature, 80);

    items.push(ParsedItem {
        kind: SymbolKind::Macro,
        name,
        signature: sig_short,
        source: Some(node.text().to_string()),
        doc_comment: String::new(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata: SymbolMetadata {
            attributes: vec!["pipeline".to_string()],
            parameters: commands,
            ..Default::default()
        },
    });
}

pub(in super::super) fn process_subshell<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
) {
    let text = node.text().to_string();
    let short = truncate_text(&text.replace('\n', " "), 60);

    items.push(ParsedItem {
        kind: SymbolKind::Macro,
        name: format!("subshell {short}"),
        signature: short,
        source: Some(text),
        doc_comment: String::new(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata: SymbolMetadata {
            attributes: vec!["subshell".to_string()],
            ..Default::default()
        },
    });
}

pub(in super::super) fn process_command_group<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
) {
    let text = node.text().to_string();
    let short = truncate_text(&text.replace('\n', " "), 60);

    items.push(ParsedItem {
        kind: SymbolKind::Macro,
        name: format!("command_group {short}"),
        signature: short,
        source: Some(text),
        doc_comment: String::new(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata: SymbolMetadata {
            attributes: vec!["command_group".to_string()],
            ..Default::default()
        },
    });
}

pub(in super::super) fn process_redirected_statement<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
) {
    let children: Vec<_> = node.children().collect();

    // Process any nested statements (e.g., while/for loops with redirection)
    for child in &children {
        let k = child.kind();
        match k.as_ref() {
            "while_statement" => process_while_statement(child, items, ""),
            "for_statement" => process_for_statement(child, items, ""),
            "if_statement" => process_if_statement(child, items, ""),
            "case_statement" => process_case_statement(child, items, ""),
            _ => {}
        }
    }

    // Check for heredoc_redirect child
    let Some(heredoc) = children
        .iter()
        .find(|c| c.kind().as_ref() == "heredoc_redirect")
    else {
        return;
    };

    let heredoc_children: Vec<_> = heredoc.children().collect();

    let delimiter = heredoc_children
        .iter()
        .find(|c| c.kind().as_ref() == "heredoc_start")
        .map_or_else(|| "EOF".to_string(), |n| n.text().to_string());

    let is_indented = heredoc_children.iter().any(|c| c.kind().as_ref() == "<<-");

    // Get the command that uses the heredoc
    let cmd = children
        .iter()
        .find(|c| c.kind().as_ref() == "command")
        .map(|c| {
            let cc: Vec<_> = c.children().collect();
            cc.iter()
                .find(|n| n.kind().as_ref() == "command_name")
                .map_or_else(String::new, |n| n.text().to_string())
        })
        .unwrap_or_default();

    let operator = if is_indented { "<<-" } else { "<<" };
    let name = format!("heredoc {delimiter}");
    let signature = format!("{cmd} {operator}{delimiter}");

    items.push(ParsedItem {
        kind: SymbolKind::Const,
        name,
        signature,
        source: extract_source_limited(node, 15),
        doc_comment: String::new(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata: SymbolMetadata {
            attributes: vec!["heredoc".to_string(), delimiter],
            ..Default::default()
        },
    });
}

pub(in super::super) fn process_negated_command<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
) {
    let text = node.text().to_string();
    let inner = text.strip_prefix('!').unwrap_or(&text).trim();
    let short = truncate_text(inner, 60);

    items.push(ParsedItem {
        kind: SymbolKind::Macro,
        name: format!("! {short}"),
        signature: truncate_text(&text, 80),
        source: Some(text),
        doc_comment: doc_comment.to_string(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata: SymbolMetadata {
            attributes: vec!["negated".to_string()],
            ..Default::default()
        },
    });
}

pub(in super::super) fn process_test_command<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
) {
    let text = node.text().to_string();
    let short = truncate_text(&text, 60);

    items.push(ParsedItem {
        kind: SymbolKind::Macro,
        name: format!("test {short}"),
        signature: text.clone(),
        source: Some(text),
        doc_comment: doc_comment.to_string(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata: SymbolMetadata {
            attributes: vec!["test".to_string()],
            ..Default::default()
        },
    });
}

pub(in super::super) fn process_unset_command<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
) {
    let children: Vec<_> = node.children().collect();

    // Detect -f flag (unset function vs variable)
    let has_f_flag = children
        .iter()
        .any(|c| c.kind().as_ref() == "word" && c.text().as_ref() == "-f");

    // Find the target name (last word that isn't a flag)
    let target = children
        .iter()
        .filter(|c| c.kind().as_ref() == "word" || c.kind().as_ref() == "variable_name")
        .filter(|c| !c.text().as_ref().starts_with('-'))
        .last()
        .map_or_else(|| "unknown".to_string(), |n| n.text().to_string());

    let kind = if has_f_flag {
        SymbolKind::Function
    } else {
        SymbolKind::Static
    };

    let mut metadata = SymbolMetadata::default();
    metadata.push_attribute("unset");
    if has_f_flag {
        metadata.push_attribute("-f");
    }

    items.push(ParsedItem {
        kind,
        name: format!("unset {target}"),
        signature: node.text().to_string(),
        source: Some(node.text().to_string()),
        doc_comment: doc_comment.to_string(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata,
    });
}

pub(in super::super) fn process_list<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
) {
    let text = node.text().to_string();
    let normalized = text
        .replace('\n', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let short = truncate_text(&normalized, 80);

    items.push(ParsedItem {
        kind: SymbolKind::Macro,
        name: truncate_text(&normalized, 60),
        signature: short,
        source: Some(text),
        doc_comment: doc_comment.to_string(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata: SymbolMetadata {
            attributes: vec!["list".to_string()],
            ..Default::default()
        },
    });
}
