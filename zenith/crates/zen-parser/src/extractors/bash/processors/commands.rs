use ast_grep_core::Node;

use crate::types::{BashMetadataExt, ParsedItem, SymbolKind, SymbolMetadata, Visibility};

pub(in super::super) fn process_command<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
) {
    let children: Vec<_> = node.children().collect();
    let Some(cmd_name_node) = children
        .iter()
        .find(|c| c.kind().as_ref() == "command_name")
    else {
        return;
    };

    let cmd_name_children: Vec<_> = cmd_name_node.children().collect();
    let cmd_text = cmd_name_children
        .first()
        .map_or_else(String::new, |n| n.text().to_string());

    match cmd_text.as_str() {
        "alias" => process_alias(node, items, doc_comment, &children),
        "trap" => process_trap(node, items, doc_comment, &children),
        "source" => process_source(node, items, doc_comment, &children, "source"),
        "." => process_source(node, items, doc_comment, &children, "."),
        _ => {}
    }
}

fn process_alias<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
    children: &[Node<D>],
) {
    // alias node has a concatenation child like `ll='ls -la'`
    // or individual word + raw_string children
    let alias_def = children
        .iter()
        .find(|c| {
            let k = c.kind();
            k.as_ref() == "concatenation" || k.as_ref() == "word" && c.text().as_ref().contains('=')
        })
        .map(|c| c.text().to_string())
        .unwrap_or_default();

    // Parse alias name from "name='value'" or "name=value"
    let alias_name = alias_def.split('=').next().unwrap_or("unknown").to_string();

    let alias_value = alias_def
        .split_once('=')
        .map_or_else(String::new, |(_, v)| v.to_string());

    items.push(ParsedItem {
        kind: SymbolKind::Static,
        name: alias_name,
        signature: format!("alias {alias_def}"),
        source: Some(node.text().to_string()),
        doc_comment: doc_comment.to_string(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata: SymbolMetadata {
            attributes: vec!["alias".to_string(), format!("value={alias_value}")],
            ..Default::default()
        },
    });
}

fn process_trap<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
    children: &[Node<D>],
) {
    // trap 'handler' SIGNAL1 SIGNAL2 ...
    // Children after command_name: raw_string/string (handler), word (signals)
    let mut handler = String::new();
    let mut signals = Vec::new();

    let mut past_name = false;
    for child in children {
        if child.kind().as_ref() == "command_name" {
            past_name = true;
            continue;
        }
        if !past_name {
            continue;
        }
        let k = child.kind();
        match k.as_ref() {
            "raw_string" | "string" => {
                handler = child.text().to_string();
            }
            "word" => {
                signals.push(child.text().to_string());
            }
            _ => {}
        }
    }

    let signal_str = signals.join(" ");
    let name = format!("trap {signal_str}");
    let signature = format!("trap {handler} {signal_str}");

    let mut metadata = SymbolMetadata::default();
    metadata.push_attribute("trap");
    for signal in &signals {
        metadata.push_attribute(signal.clone());
    }

    items.push(ParsedItem {
        kind: SymbolKind::Function,
        name,
        signature,
        source: Some(node.text().to_string()),
        doc_comment: doc_comment.to_string(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata,
    });
}

fn process_source<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
    children: &[Node<D>],
    command: &str,
) {
    // source ./file.sh or . ./file.sh
    let file_path = children
        .iter()
        .filter(|c| c.kind().as_ref() == "word")
        .nth(1) // Skip the command name word
        .or_else(|| {
            // For `.` command, the command name itself is `.`, so the file is the first `word` after `command_name`
            let mut past_name = false;
            children.iter().find(|c| {
                if c.kind().as_ref() == "command_name" {
                    past_name = true;
                    return false;
                }
                past_name && c.kind().as_ref() == "word"
            })
        })
        .map_or_else(|| "unknown".to_string(), |n| n.text().to_string());

    items.push(ParsedItem {
        kind: SymbolKind::Module,
        name: file_path.clone(),
        signature: format!("{command} {file_path}"),
        source: Some(node.text().to_string()),
        doc_comment: doc_comment.to_string(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata: SymbolMetadata {
            attributes: vec![command.to_string()],
            ..Default::default()
        },
    });
}
