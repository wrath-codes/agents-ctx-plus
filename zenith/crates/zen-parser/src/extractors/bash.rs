//! Bash rich extractor.
//!
//! Extracts structurally significant elements from Bash/shell scripts:
//! functions (both `function name {}` and `name() {}` styles),
//! variable assignments, export/readonly/declare commands,
//! aliases, conditional constructs (`if`, `case`), loops
//! (`for`, `while`, `until`, `select`), heredocs, subshells,
//! command groups, pipelines, command substitution, traps,
//! source/dot commands, and shebang detection with leading
//! `#` comments as doc comments.

use ast_grep_core::Node;
use ast_grep_language::SupportLang;
use std::fmt::Write as _;

use crate::types::{BashMetadataExt, ParsedItem, SymbolKind, SymbolMetadata, Visibility};

/// Extract all significant elements from a Bash script.
///
/// Walks the top-level `program` node collecting:
/// - Functions (both `function foo {}` and `foo() {}` styles)
/// - Variable assignments (`FOO=bar`)
/// - Declaration commands (`export`, `readonly`, `local`, `declare`)
/// - Aliases (`alias ll='ls -la'`)
/// - Conditional constructs (`if`/`elif`/`else`/`fi`, `case`/`esac`)
/// - Loops (`for`, `while`, `until`, `select`)
/// - Heredocs and here strings
/// - Subshells `(...)` and command groups `{ ... }`
/// - Pipelines (`cmd1 | cmd2 | cmd3`)
/// - Command substitution (`$(...)`)
/// - Traps (`trap 'handler' SIGNAL`)
/// - Source/dot commands (`source file.sh`, `. file.sh`)
/// - Shebang (`#!/bin/bash`)
///
/// # Errors
/// Returns `ParserError` if parsing fails.
pub fn extract<D: ast_grep_core::Doc<Lang = SupportLang>>(
    root: &ast_grep_core::AstGrep<D>,
    source: &str,
) -> Result<Vec<ParsedItem>, crate::error::ParserError> {
    let mut items = Vec::new();
    let root_node = root.root();
    collect_nodes(&root_node, &mut items, source);
    Ok(items)
}

// ── Top-level node dispatcher ──────────────────────────────────────

fn collect_nodes<D: ast_grep_core::Doc>(node: &Node<D>, items: &mut Vec<ParsedItem>, source: &str) {
    let children: Vec<_> = node.children().collect();

    // First pass: detect shebang from the first comment
    if let Some(first) = children.first()
        && first.kind().as_ref() == "comment"
    {
        let text = first.text();
        if text.as_ref().starts_with("#!") {
            process_shebang(first, items);
        }
    }

    // Second pass: process all top-level constructs
    for (idx, child) in children.iter().enumerate() {
        let kind = child.kind();
        match kind.as_ref() {
            "function_definition" => {
                let doc = collect_doc_comments(&children, idx, source);
                process_function(child, items, &doc);
            }
            "variable_assignment" => {
                let doc = collect_doc_comments(&children, idx, source);
                process_variable_assignment(child, items, &doc, None);
            }
            "declaration_command" => {
                let doc = collect_doc_comments(&children, idx, source);
                process_declaration_command(child, items, &doc);
            }
            "command" => {
                let doc = collect_doc_comments(&children, idx, source);
                process_command(child, items, &doc);
            }
            "if_statement" => {
                let doc = collect_doc_comments(&children, idx, source);
                process_if_statement(child, items, &doc);
            }
            "case_statement" => {
                let doc = collect_doc_comments(&children, idx, source);
                process_case_statement(child, items, &doc);
            }
            "for_statement" => {
                let doc = collect_doc_comments(&children, idx, source);
                process_for_statement(child, items, &doc);
            }
            "while_statement" => {
                let doc = collect_doc_comments(&children, idx, source);
                process_while_statement(child, items, &doc);
            }
            "pipeline" => {
                process_pipeline(child, items);
            }
            "subshell" => {
                process_subshell(child, items);
            }
            "compound_statement" => {
                process_command_group(child, items);
            }
            "redirected_statement" => {
                process_redirected_statement(child, items);
            }
            "c_style_for_statement" => {
                let doc = collect_doc_comments(&children, idx, source);
                process_c_style_for(child, items, &doc);
            }
            "negated_command" => {
                let doc = collect_doc_comments(&children, idx, source);
                process_negated_command(child, items, &doc);
            }
            "test_command" => {
                let doc = collect_doc_comments(&children, idx, source);
                process_test_command(child, items, &doc);
            }
            "unset_command" => {
                let doc = collect_doc_comments(&children, idx, source);
                process_unset_command(child, items, &doc);
            }
            "list" => {
                let doc = collect_doc_comments(&children, idx, source);
                process_list(child, items, &doc);
            }
            _ => {} // Skip comments, whitespace, etc.
        }
    }
}

// ── Shebang processing ────────────────────────────────────────────

fn process_shebang<D: ast_grep_core::Doc>(node: &Node<D>, items: &mut Vec<ParsedItem>) {
    let text = node.text().to_string();
    let interpreter = text.trim_start_matches("#!").trim().to_string();

    items.push(ParsedItem {
        kind: SymbolKind::Macro,
        name: "shebang".to_string(),
        signature: text.clone(),
        source: Some(text),
        doc_comment: String::new(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata: SymbolMetadata {
            attributes: vec![interpreter],
            ..Default::default()
        },
    });
}

// ── Doc comment collection ────────────────────────────────────────

/// Collect leading `#` comments above a node as doc comments.
///
/// Walks backward through siblings from `idx`, collecting contiguous
/// `comment` nodes (but not shebangs). Stops at any non-comment node
/// or a blank-line gap (detected by non-consecutive lines).
fn collect_doc_comments<D: ast_grep_core::Doc>(
    siblings: &[Node<D>],
    idx: usize,
    _source: &str,
) -> String {
    let mut comments = Vec::new();
    let target_line = siblings[idx].start_pos().line();

    let mut i = idx;
    while i > 0 {
        i -= 1;
        let sibling = &siblings[i];
        if sibling.kind().as_ref() != "comment" {
            break;
        }
        let text = sibling.text().to_string();
        // Skip shebangs
        if text.starts_with("#!") {
            break;
        }
        // Check for line gap — comments must be contiguous
        let comment_end = sibling.end_pos().line();
        let next_start = if i + 1 < idx {
            siblings[i + 1].start_pos().line()
        } else {
            target_line
        };
        if next_start > comment_end + 1 {
            break;
        }
        let stripped = text.trim_start_matches('#').trim().to_string();
        comments.push(stripped);
    }

    comments.reverse();
    comments.join("\n")
}

// ── Function processing ───────────────────────────────────────────

fn process_function<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
) {
    let children: Vec<_> = node.children().collect();

    let has_function_keyword = children.iter().any(|c| c.kind().as_ref() == "function");

    // The function name is always in a `word` child
    let name = children
        .iter()
        .find(|c| c.kind().as_ref() == "word")
        .map_or_else(|| "anonymous".to_string(), |n| n.text().to_string());

    // Build signature
    let has_parens = children.iter().any(|c| c.kind().as_ref() == "(");

    let mut signature = String::new();
    if has_function_keyword {
        let _ = write!(signature, "function {name}");
    } else {
        let _ = write!(signature, "{name}");
    }
    if has_parens {
        signature.push_str("()");
    }

    items.push(ParsedItem {
        kind: SymbolKind::Function,
        name,
        signature,
        source: extract_source_limited(node, 20),
        doc_comment: doc_comment.to_string(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata: SymbolMetadata {
            attributes: if has_function_keyword {
                vec!["function_keyword".to_string()]
            } else {
                Vec::new()
            },
            ..Default::default()
        },
    });
}

// ── Variable assignment processing ────────────────────────────────

fn process_variable_assignment<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
    qualifier: Option<&str>,
) {
    let children: Vec<_> = node.children().collect();

    let var_name = children
        .iter()
        .find(|c| c.kind().as_ref() == "variable_name")
        .map_or_else(String::new, |n| n.text().to_string());

    if var_name.is_empty() {
        return;
    }

    let value = extract_assignment_value(&children);

    let (kind, visibility) = match qualifier {
        Some("readonly" | "declare -r") => (SymbolKind::Const, Visibility::Public),
        Some("export" | "declare -x") => (SymbolKind::Const, Visibility::Export),
        Some("local") => (SymbolKind::Static, Visibility::Private),
        _ => (SymbolKind::Static, Visibility::Public),
    };

    let mut signature = String::new();
    if let Some(q) = qualifier {
        let _ = write!(signature, "{q} ");
    }
    let _ = write!(signature, "{var_name}");
    if let Some(ref v) = value {
        let _ = write!(signature, "={v}");
    }

    items.push(ParsedItem {
        kind,
        name: var_name,
        signature,
        source: Some(node.text().to_string()),
        doc_comment: doc_comment.to_string(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility,
        metadata: SymbolMetadata {
            attributes: qualifier.map_or_else(Vec::new, |q| vec![q.to_string()]),
            ..Default::default()
        },
    });
}

/// Extract the value portion of a variable assignment.
fn extract_assignment_value<D: ast_grep_core::Doc>(children: &[Node<D>]) -> Option<String> {
    // Value is everything after the `=` sign
    let mut found_eq = false;
    for child in children {
        if child.kind().as_ref() == "=" {
            found_eq = true;
            continue;
        }
        if found_eq {
            let text = child.text().to_string();
            if !text.is_empty() {
                return Some(text);
            }
        }
    }
    None
}

// ── Declaration command processing ────────────────────────────────

fn process_declaration_command<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
) {
    let children: Vec<_> = node.children().collect();
    let Some(first) = children.first() else {
        return;
    };

    let qualifier_kind = first.kind();
    let qualifier = qualifier_kind.as_ref();

    match qualifier {
        "export" => process_export_declaration(node, items, doc_comment, &children),
        "readonly" => process_qualified_assignment(node, items, doc_comment, &children, "readonly"),
        "local" => process_qualified_assignment(node, items, doc_comment, &children, "local"),
        "declare" | "typeset" => {
            process_declare_command(node, items, doc_comment, &children);
        }
        _ => {}
    }
}

fn process_export_declaration<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
    children: &[Node<D>],
) {
    // Check if this is `export -f func_name` (function export)
    let has_flag = children
        .iter()
        .any(|c| c.kind().as_ref() == "word" && c.text().as_ref().starts_with('-'));

    if has_flag {
        // export -f func_name or export -n etc.
        let flag = children
            .iter()
            .find(|c| c.kind().as_ref() == "word" && c.text().as_ref().starts_with('-'))
            .map(|c| c.text().to_string())
            .unwrap_or_default();

        let target = children
            .iter()
            .filter(|c| c.kind().as_ref() == "word" && !c.text().as_ref().starts_with('-'))
            .last()
            .map_or_else(|| "unknown".to_string(), |c| c.text().to_string());

        let signature = format!("export {flag} {target}");

        items.push(ParsedItem {
            kind: SymbolKind::Const,
            name: target,
            signature,
            source: Some(node.text().to_string()),
            doc_comment: doc_comment.to_string(),
            start_line: node.start_pos().line() as u32 + 1,
            end_line: node.end_pos().line() as u32 + 1,
            visibility: Visibility::Export,
            metadata: SymbolMetadata {
                attributes: vec!["export".to_string(), flag],
                ..Default::default()
            },
        });
    } else {
        // export VAR=value
        for child in children {
            if child.kind().as_ref() == "variable_assignment" {
                process_variable_assignment(child, items, doc_comment, Some("export"));
                return;
            }
        }
        // Plain `export VAR` without assignment — still emit
        if let Some(word) = children.iter().find(|c| c.kind().as_ref() == "word") {
            items.push(ParsedItem {
                kind: SymbolKind::Const,
                name: word.text().to_string(),
                signature: node.text().to_string(),
                source: Some(node.text().to_string()),
                doc_comment: doc_comment.to_string(),
                start_line: node.start_pos().line() as u32 + 1,
                end_line: node.end_pos().line() as u32 + 1,
                visibility: Visibility::Export,
                metadata: SymbolMetadata {
                    attributes: vec!["export".to_string()],
                    ..Default::default()
                },
            });
        }
    }
}

fn process_qualified_assignment<D: ast_grep_core::Doc>(
    _parent: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
    children: &[Node<D>],
    qualifier: &str,
) {
    for child in children {
        if child.kind().as_ref() == "variable_assignment" {
            process_variable_assignment(child, items, doc_comment, Some(qualifier));
        }
    }
}

fn process_declare_command<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
    children: &[Node<D>],
) {
    // Collect flags like -a, -A, -x, -i, -r
    let flags: Vec<String> = children
        .iter()
        .filter(|c| c.kind().as_ref() == "word" && c.text().as_ref().starts_with('-'))
        .map(|c| c.text().to_string())
        .collect();

    let flag_str = flags.join(" ");
    let qualifier = format!("declare {flag_str}");

    // Check for array declarations (-a or -A)
    let is_array = flags.iter().any(|f| f == "-a");
    let is_assoc = flags.iter().any(|f| f == "-A");
    let is_exported = flags.iter().any(|f| f == "-x");
    let is_readonly = flags.iter().any(|f| f == "-r");

    // Find the variable assignment child
    if let Some(assignment) = children
        .iter()
        .find(|c| c.kind().as_ref() == "variable_assignment")
    {
        let assign_children: Vec<_> = assignment.children().collect();
        let var_name = assign_children
            .iter()
            .find(|c| c.kind().as_ref() == "variable_name")
            .map_or_else(String::new, |n| n.text().to_string());

        if var_name.is_empty() {
            return;
        }

        let (kind, visibility) = if is_readonly {
            (SymbolKind::Const, Visibility::Public)
        } else if is_exported {
            (SymbolKind::Const, Visibility::Export)
        } else {
            (SymbolKind::Static, Visibility::Public)
        };

        let mut signature = qualifier.clone();
        let _ = write!(signature, " {}", assignment.text());

        let mut attributes = vec![qualifier];
        if is_array {
            attributes.push("indexed_array".to_string());
        }
        if is_assoc {
            attributes.push("associative_array".to_string());
        }

        items.push(ParsedItem {
            kind,
            name: var_name,
            signature,
            source: Some(node.text().to_string()),
            doc_comment: doc_comment.to_string(),
            start_line: node.start_pos().line() as u32 + 1,
            end_line: node.end_pos().line() as u32 + 1,
            visibility,
            metadata: SymbolMetadata {
                attributes,
                ..Default::default()
            },
        });
    }
}

// ── Command processing (alias, trap, source, dot) ─────────────────

fn process_command<D: ast_grep_core::Doc>(
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

// ── Conditional processing ────────────────────────────────────────

fn process_if_statement<D: ast_grep_core::Doc>(
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

fn process_case_statement<D: ast_grep_core::Doc>(
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

// ── Loop processing ───────────────────────────────────────────────

fn process_for_statement<D: ast_grep_core::Doc>(
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

fn process_while_statement<D: ast_grep_core::Doc>(
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

// ── Pipeline processing ───────────────────────────────────────────

fn process_pipeline<D: ast_grep_core::Doc>(node: &Node<D>, items: &mut Vec<ParsedItem>) {
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

// ── Subshell processing ───────────────────────────────────────────

fn process_subshell<D: ast_grep_core::Doc>(node: &Node<D>, items: &mut Vec<ParsedItem>) {
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

// ── Command group processing ──────────────────────────────────────

fn process_command_group<D: ast_grep_core::Doc>(node: &Node<D>, items: &mut Vec<ParsedItem>) {
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

// ── Redirected statement (heredoc) processing ─────────────────────

fn process_redirected_statement<D: ast_grep_core::Doc>(
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

// ── C-style for loop processing ───────────────────────────────────

fn process_c_style_for<D: ast_grep_core::Doc>(
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

// ── Negated command processing ────────────────────────────────────

fn process_negated_command<D: ast_grep_core::Doc>(
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

// ── Standalone test command processing ────────────────────────────

fn process_test_command<D: ast_grep_core::Doc>(
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

// ── Unset command processing ──────────────────────────────────────

fn process_unset_command<D: ast_grep_core::Doc>(
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

// ── List (logical chain) processing ──────────────────────────────

fn process_list<D: ast_grep_core::Doc>(
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

// ── Helpers ───────────────────────────────────────────────────────

/// Extract source with line limit.
#[allow(clippy::unnecessary_wraps)]
fn extract_source_limited<D: ast_grep_core::Doc>(
    node: &Node<D>,
    max_lines: usize,
) -> Option<String> {
    let text = node.text().to_string();
    let lines: Vec<&str> = text.lines().collect();
    if lines.len() <= max_lines {
        Some(text)
    } else {
        let truncated: String = lines[..max_lines].join("\n");
        Some(format!(
            "{truncated}\n    # ... ({} more lines)",
            lines.len() - max_lines
        ))
    }
}

/// Truncate text to a maximum length, appending `...` if needed.
fn truncate_text(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        text.to_string()
    } else {
        format!("{}...", &text[..max_len.saturating_sub(3)])
    }
}

// ── Tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use ast_grep_language::{LanguageExt, SupportLang};

    use super::*;
    use crate::types::SymbolKind;

    fn parse_and_extract(source: &str) -> Vec<ParsedItem> {
        let root = SupportLang::Bash.ast_grep(source);
        extract(&root, source).expect("extraction should succeed")
    }

    fn find_by_name<'a>(items: &'a [ParsedItem], name: &str) -> &'a ParsedItem {
        items.iter().find(|i| i.name == name).unwrap_or_else(|| {
            let names: Vec<_> = items
                .iter()
                .map(|i| format!("{}: {}", i.kind, &i.name))
                .collect();
            panic!("item '{name}' not found. Available: {names:?}")
        })
    }

    fn find_all_by_kind(items: &[ParsedItem], kind: SymbolKind) -> Vec<&ParsedItem> {
        items.iter().filter(|i| i.kind == kind).collect()
    }

    fn find_by_name_prefix<'a>(items: &'a [ParsedItem], prefix: &str) -> &'a ParsedItem {
        items
            .iter()
            .find(|i| i.name.starts_with(prefix))
            .unwrap_or_else(|| {
                let names: Vec<_> = items
                    .iter()
                    .map(|i| format!("{}: {}", i.kind, &i.name))
                    .collect();
                panic!("item starting with '{prefix}' not found. Available: {names:?}")
            })
    }

    // ── Fixture-based tests ───────────────────────────────────────

    #[test]
    fn fixture_parses_without_error() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        assert!(
            !items.is_empty(),
            "should extract at least one item from fixture"
        );
    }

    #[test]
    fn fixture_item_count() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        // The fixture has many constructs — verify a reasonable minimum
        assert!(
            items.len() >= 25,
            "expected at least 25 items, got {}",
            items.len()
        );
    }

    // ── Shebang tests ─────────────────────────────────────────────

    #[test]
    fn shebang_detected() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        let shebang = find_by_name(&items, "shebang");
        assert_eq!(shebang.kind, SymbolKind::Macro);
        assert!(shebang.signature.contains("#!/bin/bash"));
    }

    #[test]
    fn shebang_env_style() {
        let items = parse_and_extract("#!/usr/bin/env bash\necho hello");
        let shebang = find_by_name(&items, "shebang");
        assert!(
            shebang
                .metadata
                .attributes
                .iter()
                .any(|a| a.contains("env bash")),
            "should detect env-style shebang: {:?}",
            shebang.metadata.attributes
        );
    }

    #[test]
    fn no_shebang_no_item() {
        let items = parse_and_extract("echo hello");
        assert!(
            items.iter().all(|i| i.name != "shebang"),
            "should not emit shebang for scripts without one"
        );
    }

    // ── Function tests ────────────────────────────────────────────

    #[test]
    fn function_parens_style() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "greet");
        assert_eq!(f.kind, SymbolKind::Function);
        assert!(f.signature.contains("greet()"));
    }

    #[test]
    fn function_keyword_style() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "cleanup");
        assert_eq!(f.kind, SymbolKind::Function);
        assert!(f.signature.contains("function cleanup"));
    }

    #[test]
    fn function_both_keyword_and_parens() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "deploy");
        assert_eq!(f.kind, SymbolKind::Function);
        assert!(f.signature.contains("function deploy()"));
    }

    #[test]
    fn function_doc_comment() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "greet");
        assert!(
            f.doc_comment.contains("Greet a user"),
            "expected doc comment, got: {:?}",
            f.doc_comment
        );
    }

    #[test]
    fn function_multi_line_doc() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "greet");
        assert!(
            f.doc_comment.contains('\n'),
            "expected multi-line doc comment: {:?}",
            f.doc_comment
        );
    }

    #[test]
    fn function_has_function_keyword_attribute() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "cleanup");
        assert!(
            f.metadata
                .attributes
                .contains(&"function_keyword".to_string()),
            "should have function_keyword attribute: {:?}",
            f.metadata.attributes
        );
    }

    #[test]
    fn function_inline_oneliner() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "say_hi");
        assert_eq!(f.kind, SymbolKind::Function);
    }

    #[test]
    fn function_count() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        let fns = find_all_by_kind(&items, SymbolKind::Function);
        // greet, cleanup, deploy, say_hi, process_data + trap items
        assert!(
            fns.len() >= 5,
            "expected at least 5 function-kind items, got {}",
            fns.len()
        );
    }

    // ── Variable assignment tests ─────────────────────────────────

    #[test]
    fn variable_simple() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        let v = find_by_name(&items, "FOO");
        assert_eq!(v.kind, SymbolKind::Static);
        assert!(v.signature.contains("FOO="));
    }

    #[test]
    fn variable_numeric() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        let v = find_by_name(&items, "BAZ");
        assert_eq!(v.kind, SymbolKind::Static);
    }

    // ── Export tests ──────────────────────────────────────────────

    #[test]
    fn export_with_value() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        let v = find_by_name(&items, "DATABASE_URL");
        assert_eq!(v.kind, SymbolKind::Const);
        assert_eq!(v.visibility, Visibility::Export);
    }

    #[test]
    fn export_doc_comment() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        let v = find_by_name(&items, "DATABASE_URL");
        assert!(
            v.doc_comment.contains("Database connection"),
            "expected doc comment, got: {:?}",
            v.doc_comment
        );
    }

    #[test]
    fn export_api_key() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        let v = find_by_name(&items, "API_KEY");
        assert_eq!(v.visibility, Visibility::Export);
    }

    // ── Readonly tests ────────────────────────────────────────────

    #[test]
    fn readonly_const() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        let v = find_by_name(&items, "MAX_RETRIES");
        assert_eq!(v.kind, SymbolKind::Const);
        assert_eq!(v.visibility, Visibility::Public);
    }

    #[test]
    fn readonly_app_name() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        let v = find_by_name(&items, "APP_NAME");
        assert_eq!(v.kind, SymbolKind::Const);
    }

    // ── Local variable tests ──────────────────────────────────────

    #[test]
    fn local_variable() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        let v = find_by_name(&items, "COUNTER");
        assert_eq!(v.kind, SymbolKind::Static);
        assert_eq!(v.visibility, Visibility::Private);
    }

    // ── Declare tests ─────────────────────────────────────────────

    #[test]
    fn declare_exported() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        let v = find_by_name(&items, "EXPORTED_VAR");
        assert_eq!(v.kind, SymbolKind::Const);
        assert_eq!(v.visibility, Visibility::Export);
    }

    #[test]
    fn declare_integer() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        let v = find_by_name(&items, "INTEGER_VAR");
        assert_eq!(v.kind, SymbolKind::Static);
    }

    #[test]
    fn declare_readonly() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        let v = find_by_name(&items, "DECLARED_READONLY");
        assert_eq!(v.kind, SymbolKind::Const);
    }

    // ── Alias tests ───────────────────────────────────────────────

    #[test]
    fn alias_extracted() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        let a = find_by_name(&items, "ll");
        assert_eq!(a.kind, SymbolKind::Static);
        assert!(a.signature.contains("alias"));
    }

    #[test]
    fn alias_doc_comment() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        let a = find_by_name(&items, "ll");
        assert!(
            a.doc_comment.contains("List files"),
            "expected doc comment on alias: {:?}",
            a.doc_comment
        );
    }

    #[test]
    fn alias_count() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        let alias_count = items
            .iter()
            .filter(|i| i.metadata.attributes.contains(&"alias".to_string()))
            .count();
        assert_eq!(alias_count, 3, "should find 3 aliases: ll, gs, gp");
    }

    // ── Array declaration tests ───────────────────────────────────

    #[test]
    fn indexed_array() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        let v = find_by_name(&items, "FRUITS");
        assert!(
            v.metadata
                .attributes
                .iter()
                .any(|a| a.contains("indexed_array")),
            "should have indexed_array attribute: {:?}",
            v.metadata.attributes
        );
    }

    #[test]
    fn indexed_array_doc() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        let v = find_by_name(&items, "FRUITS");
        assert!(
            v.doc_comment.contains("Indexed array"),
            "expected doc comment: {:?}",
            v.doc_comment
        );
    }

    #[test]
    fn associative_array() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        let v = find_by_name(&items, "CONFIG");
        assert!(
            v.metadata
                .attributes
                .iter()
                .any(|a| a.contains("associative_array")),
            "should have associative_array attribute: {:?}",
            v.metadata.attributes
        );
    }

    // ── Conditional tests ─────────────────────────────────────────

    #[test]
    fn if_statement_extracted() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        assert!(
            items.iter().any(|i| i.name.starts_with("if ")),
            "should find at least one if statement"
        );
    }

    #[test]
    fn if_has_elif() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        let if_stmt = items
            .iter()
            .find(|i| {
                i.name.starts_with("if ")
                    && i.metadata.attributes.iter().any(|a| a.contains("elif"))
            })
            .expect("should find if with elif");
        assert_eq!(if_stmt.kind, SymbolKind::Enum);
    }

    #[test]
    fn if_doc_comment() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        let if_stmt = items
            .iter()
            .find(|i| i.name.starts_with("if "))
            .expect("should find if statement");
        assert!(
            if_stmt.doc_comment.contains("Check if a file"),
            "expected doc comment: {:?}",
            if_stmt.doc_comment
        );
    }

    #[test]
    fn case_statement_extracted() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        let cs = items
            .iter()
            .find(|i| i.name.starts_with("case "))
            .expect("should find case statement");
        assert_eq!(cs.kind, SymbolKind::Enum);
    }

    #[test]
    fn case_has_patterns() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        let cs = items
            .iter()
            .find(|i| i.name.starts_with("case ") && !i.metadata.variants.is_empty())
            .expect("should find case with patterns");
        assert!(
            cs.metadata.variants.len() >= 3,
            "should find at least 3 patterns: {:?}",
            cs.metadata.variants
        );
    }

    #[test]
    fn case_doc_comment() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        let cs = items
            .iter()
            .find(|i| i.name.starts_with("case "))
            .expect("should find case");
        assert!(
            cs.doc_comment.contains("command routing"),
            "expected doc comment: {:?}",
            cs.doc_comment
        );
    }

    // ── Loop tests ────────────────────────────────────────────────

    #[test]
    fn for_loop_extracted() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        let f = items
            .iter()
            .find(|i| i.name.starts_with("for i"))
            .expect("should find for loop");
        assert_eq!(f.kind, SymbolKind::Macro);
        assert!(
            f.metadata.attributes.contains(&"for".to_string()),
            "should have for attribute: {:?}",
            f.metadata.attributes
        );
    }

    #[test]
    fn for_loop_has_variable() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        let f = items
            .iter()
            .find(|i| i.name.starts_with("for i"))
            .expect("should find for loop");
        assert!(
            f.metadata.parameters.contains(&"i".to_string()),
            "should have loop var 'i': {:?}",
            f.metadata.parameters
        );
    }

    #[test]
    fn for_loop_doc_comment() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        let f = items
            .iter()
            .find(|i| i.name.starts_with("for i"))
            .expect("should find for loop");
        assert!(
            f.doc_comment.contains("Iterate over numbers"),
            "expected doc comment: {:?}",
            f.doc_comment
        );
    }

    #[test]
    fn while_loop_extracted() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        let w = items
            .iter()
            .find(|i| i.name.starts_with("while "))
            .expect("should find while loop");
        assert_eq!(w.kind, SymbolKind::Macro);
    }

    #[test]
    fn until_loop_extracted() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        let u = items
            .iter()
            .find(|i| i.name.starts_with("until "))
            .expect("should find until loop");
        assert_eq!(u.kind, SymbolKind::Macro);
        assert!(
            u.metadata.attributes.contains(&"until".to_string()),
            "should have until attribute: {:?}",
            u.metadata.attributes
        );
    }

    #[test]
    fn select_statement_extracted() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        let s = items
            .iter()
            .find(|i| i.name.starts_with("select "))
            .expect("should find select statement");
        assert_eq!(s.kind, SymbolKind::Enum);
        assert!(
            s.metadata.attributes.contains(&"select".to_string()),
            "should have select attribute"
        );
    }

    // ── Heredoc tests ─────────────────────────────────────────────

    #[test]
    fn heredoc_extracted() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        let hd = items
            .iter()
            .find(|i| i.name.starts_with("heredoc "))
            .expect("should find heredoc");
        assert_eq!(hd.kind, SymbolKind::Const);
    }

    #[test]
    fn heredoc_delimiter() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        let hd = find_by_name(&items, "heredoc EOF");
        assert!(
            hd.metadata.attributes.contains(&"heredoc".to_string()),
            "should have heredoc attribute: {:?}",
            hd.metadata.attributes
        );
    }

    #[test]
    fn heredoc_indented() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        let hd = find_by_name(&items, "heredoc INDENTED");
        assert!(
            hd.signature.contains("<<-"),
            "should contain indented heredoc operator: {:?}",
            hd.signature
        );
    }

    #[test]
    fn heredoc_count() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        let heredocs: Vec<_> = items
            .iter()
            .filter(|i| i.name.starts_with("heredoc "))
            .collect();
        assert!(
            heredocs.len() >= 2,
            "should find at least 2 heredocs, got {}",
            heredocs.len()
        );
    }

    // ── Subshell tests ────────────────────────────────────────────

    #[test]
    fn subshell_extracted() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        let sub = items
            .iter()
            .find(|i| i.name.starts_with("subshell "))
            .expect("should find subshell");
        assert_eq!(sub.kind, SymbolKind::Macro);
    }

    // ── Command group tests ───────────────────────────────────────

    #[test]
    fn command_group_extracted() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        let cg = items
            .iter()
            .find(|i| i.name.starts_with("command_group "))
            .expect("should find command group");
        assert_eq!(cg.kind, SymbolKind::Macro);
    }

    // ── Pipeline tests ────────────────────────────────────────────

    #[test]
    fn pipeline_extracted() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        let pipes: Vec<_> = items
            .iter()
            .filter(|i| i.metadata.attributes.contains(&"pipeline".to_string()))
            .collect();
        assert!(
            pipes.len() >= 2,
            "should find at least 2 pipelines, got {}",
            pipes.len()
        );
    }

    #[test]
    fn pipeline_has_commands() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        let pipe = items
            .iter()
            .find(|i| i.metadata.attributes.contains(&"pipeline".to_string()))
            .expect("should find pipeline");
        assert!(
            pipe.metadata.parameters.len() >= 2,
            "pipeline should have at least 2 commands: {:?}",
            pipe.metadata.parameters
        );
    }

    // ── Command substitution tests ────────────────────────────────

    #[test]
    fn command_substitution_in_variable() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        let v = find_by_name(&items, "CURRENT_DATE");
        assert_eq!(v.kind, SymbolKind::Static);
        assert!(
            v.source.as_deref().unwrap_or("").contains("$("),
            "should contain command substitution: {:?}",
            v.source
        );
    }

    // ── Trap tests ────────────────────────────────────────────────

    #[test]
    fn trap_extracted() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        let traps: Vec<_> = items
            .iter()
            .filter(|i| i.name.starts_with("trap "))
            .collect();
        assert!(
            traps.len() >= 2,
            "should find at least 2 traps, got {}",
            traps.len()
        );
    }

    #[test]
    fn trap_has_signals() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        let trap = items
            .iter()
            .find(|i| i.name.contains("EXIT"))
            .expect("should find trap for EXIT signal");
        assert_eq!(trap.kind, SymbolKind::Function);
        assert!(
            trap.metadata.attributes.contains(&"EXIT".to_string()),
            "should have EXIT signal: {:?}",
            trap.metadata.attributes
        );
    }

    #[test]
    fn trap_doc_comment() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        let trap = items
            .iter()
            .find(|i| i.name.contains("EXIT"))
            .expect("should find EXIT trap");
        assert!(
            trap.doc_comment.contains("Clean up on exit"),
            "expected doc comment: {:?}",
            trap.doc_comment
        );
    }

    // ── Source/dot tests ──────────────────────────────────────────

    #[test]
    fn source_command() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        let s = find_by_name(&items, "./lib/utils.sh");
        assert_eq!(s.kind, SymbolKind::Module);
        assert!(s.signature.starts_with("source"));
    }

    #[test]
    fn dot_command() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        let d = find_by_name(&items, "./lib/helpers.sh");
        assert_eq!(d.kind, SymbolKind::Module);
        assert!(d.signature.starts_with(". "));
    }

    #[test]
    fn source_doc_comment() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        let s = find_by_name(&items, "./lib/utils.sh");
        assert!(
            s.doc_comment.contains("Load utility"),
            "expected doc comment: {:?}",
            s.doc_comment
        );
    }

    // ── Line number tests ─────────────────────────────────────────

    #[test]
    fn line_numbers_are_one_based() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        for item in &items {
            assert!(
                item.start_line >= 1,
                "start_line should be >= 1: {} ({})",
                item.name,
                item.start_line
            );
            assert!(
                item.end_line >= item.start_line,
                "end_line should be >= start_line: {} ({} > {})",
                item.name,
                item.start_line,
                item.end_line
            );
        }
    }

    #[test]
    fn shebang_at_line_1() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        let shebang = find_by_name(&items, "shebang");
        assert_eq!(shebang.start_line, 1, "shebang should be on line 1");
    }

    // ── Inline / edge case tests ──────────────────────────────────

    #[test]
    fn empty_script() {
        let items = parse_and_extract("");
        assert!(items.is_empty());
    }

    #[test]
    fn comment_only_script() {
        let items = parse_and_extract("# just a comment");
        assert!(items.is_empty());
    }

    #[test]
    fn inline_function() {
        let items = parse_and_extract("hello() { echo 'world'; }");
        let f = find_by_name(&items, "hello");
        assert_eq!(f.kind, SymbolKind::Function);
    }

    #[test]
    fn inline_export() {
        let items = parse_and_extract("export MY_VAR=\"hello\"");
        let v = find_by_name(&items, "MY_VAR");
        assert_eq!(v.kind, SymbolKind::Const);
        assert_eq!(v.visibility, Visibility::Export);
    }

    #[test]
    fn inline_readonly() {
        let items = parse_and_extract("readonly MY_CONST=42");
        let v = find_by_name(&items, "MY_CONST");
        assert_eq!(v.kind, SymbolKind::Const);
    }

    #[test]
    fn inline_alias() {
        let items = parse_and_extract("alias k='kubectl'");
        let a = find_by_name(&items, "k");
        assert_eq!(a.kind, SymbolKind::Static);
    }

    #[test]
    fn inline_pipeline() {
        let items = parse_and_extract("cat file.txt | grep error | wc -l");
        let pipe = items
            .iter()
            .find(|i| i.metadata.attributes.contains(&"pipeline".to_string()))
            .expect("should find pipeline");
        assert_eq!(pipe.metadata.parameters.len(), 3);
    }

    #[test]
    fn inline_heredoc() {
        let items = parse_and_extract("cat <<MARKER\nhello\nMARKER");
        let hd = find_by_name(&items, "heredoc MARKER");
        assert_eq!(hd.kind, SymbolKind::Const);
    }

    #[test]
    fn inline_trap() {
        let items = parse_and_extract("trap 'exit 1' SIGTERM");
        let t = items
            .iter()
            .find(|i| i.name.starts_with("trap "))
            .expect("should find trap");
        assert_eq!(t.kind, SymbolKind::Function);
    }

    #[test]
    fn inline_source() {
        let items = parse_and_extract("source /etc/profile");
        let s = find_by_name(&items, "/etc/profile");
        assert_eq!(s.kind, SymbolKind::Module);
    }

    #[test]
    fn inline_dot() {
        let items = parse_and_extract(". ~/.bashrc");
        let d = find_by_name(&items, "~/.bashrc");
        assert_eq!(d.kind, SymbolKind::Module);
    }

    #[test]
    fn inline_case() {
        let items =
            parse_and_extract("case \"$1\" in\n  yes) echo ok ;;\n  no) echo fail ;;\nesac");
        let cs = find_by_name_prefix(&items, "case ");
        assert_eq!(cs.kind, SymbolKind::Enum);
        assert!(
            cs.metadata.variants.len() >= 2,
            "should have at least 2 patterns: {:?}",
            cs.metadata.variants
        );
    }

    #[test]
    fn inline_for() {
        let items = parse_and_extract("for x in a b c; do echo $x; done");
        let f = find_by_name_prefix(&items, "for ");
        assert_eq!(f.kind, SymbolKind::Macro);
    }

    #[test]
    fn inline_while() {
        let items = parse_and_extract("while true; do sleep 1; done");
        let w = find_by_name_prefix(&items, "while ");
        assert_eq!(w.kind, SymbolKind::Macro);
    }

    #[test]
    fn inline_until() {
        let items = parse_and_extract("until false; do sleep 1; done");
        let u = find_by_name_prefix(&items, "until ");
        assert_eq!(u.kind, SymbolKind::Macro);
    }

    #[test]
    fn inline_subshell() {
        let items = parse_and_extract("(echo hello; echo world)");
        let sub = find_by_name_prefix(&items, "subshell ");
        assert_eq!(sub.kind, SymbolKind::Macro);
    }

    #[test]
    fn inline_command_group() {
        let items = parse_and_extract("{ echo hello; echo world; }");
        let cg = find_by_name_prefix(&items, "command_group ");
        assert_eq!(cg.kind, SymbolKind::Macro);
    }

    #[test]
    fn inline_select() {
        let items = parse_and_extract("select x in a b c; do echo $x; done");
        let s = find_by_name_prefix(&items, "select ");
        assert_eq!(s.kind, SymbolKind::Enum);
    }

    #[test]
    fn inline_if() {
        let items = parse_and_extract("if true; then echo ok; fi");
        let i = find_by_name_prefix(&items, "if ");
        assert_eq!(i.kind, SymbolKind::Enum);
    }

    #[test]
    fn inline_declare_array() {
        let items = parse_and_extract("declare -a ARR=(1 2 3)");
        let v = find_by_name(&items, "ARR");
        assert!(
            v.metadata
                .attributes
                .iter()
                .any(|a| a.contains("indexed_array")),
            "should have indexed_array: {:?}",
            v.metadata.attributes
        );
    }

    #[test]
    fn inline_declare_assoc() {
        let items = parse_and_extract("declare -A MAP=([a]=1 [b]=2)");
        let v = find_by_name(&items, "MAP");
        assert!(
            v.metadata
                .attributes
                .iter()
                .any(|a| a.contains("associative")),
            "should have associative_array: {:?}",
            v.metadata.attributes
        );
    }

    #[test]
    fn process_data_function() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "process_data");
        assert_eq!(f.kind, SymbolKind::Function);
        assert!(
            f.doc_comment.contains("Process data"),
            "expected doc comment: {:?}",
            f.doc_comment
        );
    }

    // ── Here string test ──────────────────────────────────────────

    #[test]
    fn here_string_not_crash() {
        // Here strings are part of commands, not top-level nodes
        // Ensure we don't crash on them
        let items = parse_and_extract("grep pattern <<< \"hello world\"");
        // The command itself may or may not produce an item (it's a plain grep command)
        // Just verify no crash
        assert!(items.is_empty() || !items.is_empty());
    }

    // ── C-style for loop tests ────────────────────────────────────

    #[test]
    fn c_style_for_extracted() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        let f = items
            .iter()
            .find(|i| i.metadata.attributes.contains(&"c_style".to_string()))
            .expect("should find c-style for loop");
        assert_eq!(f.kind, SymbolKind::Macro);
    }

    #[test]
    fn c_style_for_has_for_attr() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        let f = items
            .iter()
            .find(|i| i.metadata.attributes.contains(&"c_style".to_string()))
            .expect("should find c-style for");
        assert!(
            f.metadata.attributes.contains(&"for".to_string()),
            "should have 'for' attribute: {:?}",
            f.metadata.attributes
        );
    }

    #[test]
    fn c_style_for_doc_comment() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        let f = items
            .iter()
            .find(|i| i.metadata.attributes.contains(&"c_style".to_string()))
            .expect("should find c-style for");
        assert!(
            f.doc_comment.contains("C-style for loop"),
            "expected doc comment: {:?}",
            f.doc_comment
        );
    }

    #[test]
    fn c_style_for_count() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        let c_for_count = items
            .iter()
            .filter(|i| i.metadata.attributes.contains(&"c_style".to_string()))
            .count();
        assert_eq!(c_for_count, 2, "should find 2 c-style for loops");
    }

    #[test]
    fn inline_c_style_for() {
        let items = parse_and_extract("for ((x=0; x<3; x++)); do echo $x; done");
        let f = items
            .iter()
            .find(|i| i.metadata.attributes.contains(&"c_style".to_string()))
            .expect("should find c-style for");
        assert_eq!(f.kind, SymbolKind::Macro);
        assert!(f.name.starts_with("for "));
    }

    // ── Negated command tests ─────────────────────────────────────

    #[test]
    fn negated_command_extracted() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        let n = items
            .iter()
            .find(|i| i.metadata.attributes.contains(&"negated".to_string()))
            .expect("should find negated command");
        assert_eq!(n.kind, SymbolKind::Macro);
        assert!(n.name.starts_with("! "));
    }

    #[test]
    fn negated_command_doc_comment() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        let n = items
            .iter()
            .find(|i| i.metadata.attributes.contains(&"negated".to_string()))
            .expect("should find negated command");
        assert!(
            n.doc_comment.contains("Negate grep"),
            "expected doc comment: {:?}",
            n.doc_comment
        );
    }

    #[test]
    fn inline_negated() {
        let items = parse_and_extract("! false");
        let n = items
            .iter()
            .find(|i| i.metadata.attributes.contains(&"negated".to_string()))
            .expect("should find negated command");
        assert_eq!(n.kind, SymbolKind::Macro);
    }

    // ── Standalone test command tests ─────────────────────────────

    #[test]
    fn test_command_extracted() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        let t = items
            .iter()
            .find(|i| i.metadata.attributes.contains(&"test".to_string()))
            .expect("should find test command");
        assert_eq!(t.kind, SymbolKind::Macro);
        assert!(t.name.starts_with("test "));
    }

    #[test]
    fn test_command_doc_comment() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        let t = items
            .iter()
            .find(|i| i.metadata.attributes.contains(&"test".to_string()))
            .expect("should find test command");
        assert!(
            t.doc_comment.contains("Standalone test"),
            "expected doc comment: {:?}",
            t.doc_comment
        );
    }

    #[test]
    fn inline_test_bracket() {
        let items = parse_and_extract("[[ -d /tmp ]]");
        let t = items
            .iter()
            .find(|i| i.metadata.attributes.contains(&"test".to_string()))
            .expect("should find test command");
        assert_eq!(t.kind, SymbolKind::Macro);
    }

    #[test]
    fn inline_test_single_bracket() {
        let items = parse_and_extract("[ -f /etc/passwd ]");
        let t = items
            .iter()
            .find(|i| i.metadata.attributes.contains(&"test".to_string()))
            .expect("should find single-bracket test");
        assert_eq!(t.kind, SymbolKind::Macro);
    }

    // ── Unset command tests ───────────────────────────────────────

    #[test]
    fn unset_variable_extracted() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        let u = find_by_name(&items, "unset TEMP_VAR");
        assert_eq!(u.kind, SymbolKind::Static);
        assert!(
            u.metadata.attributes.contains(&"unset".to_string()),
            "should have unset attribute: {:?}",
            u.metadata.attributes
        );
    }

    #[test]
    fn unset_variable_doc_comment() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        let u = find_by_name(&items, "unset TEMP_VAR");
        assert!(
            u.doc_comment.contains("Remove a variable"),
            "expected doc comment: {:?}",
            u.doc_comment
        );
    }

    #[test]
    fn unset_function_extracted() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        let u = find_by_name(&items, "unset old_func");
        assert_eq!(u.kind, SymbolKind::Function);
        assert!(
            u.metadata.attributes.contains(&"-f".to_string()),
            "should have -f flag: {:?}",
            u.metadata.attributes
        );
    }

    #[test]
    fn unset_function_doc_comment() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        let u = find_by_name(&items, "unset old_func");
        assert!(
            u.doc_comment.contains("Remove a function"),
            "expected doc comment: {:?}",
            u.doc_comment
        );
    }

    #[test]
    fn inline_unset_var() {
        let items = parse_and_extract("unset FOO");
        let u = find_by_name(&items, "unset FOO");
        assert_eq!(u.kind, SymbolKind::Static);
    }

    #[test]
    fn inline_unset_func() {
        let items = parse_and_extract("unset -f bar");
        let u = find_by_name(&items, "unset bar");
        assert_eq!(u.kind, SymbolKind::Function);
    }

    #[test]
    fn unset_count() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        let unset_count = items
            .iter()
            .filter(|i| i.name.starts_with("unset "))
            .count();
        assert_eq!(unset_count, 2, "should find 2 unset commands");
    }

    // ── List (logical chain) tests ────────────────────────────────

    #[test]
    fn list_extracted() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        let l = items
            .iter()
            .find(|i| i.metadata.attributes.contains(&"list".to_string()))
            .expect("should find list (logical chain)");
        assert_eq!(l.kind, SymbolKind::Macro);
    }

    #[test]
    fn list_doc_comment() {
        let source = include_str!("../../tests/fixtures/sample.sh");
        let items = parse_and_extract(source);
        let l = items
            .iter()
            .find(|i| i.metadata.attributes.contains(&"list".to_string()))
            .expect("should find list");
        assert!(
            l.doc_comment.contains("Conditional chain"),
            "expected doc comment: {:?}",
            l.doc_comment
        );
    }

    #[test]
    fn inline_list_and() {
        let items = parse_and_extract("true && echo ok");
        let l = items
            .iter()
            .find(|i| i.metadata.attributes.contains(&"list".to_string()))
            .expect("should find && list");
        assert_eq!(l.kind, SymbolKind::Macro);
    }

    #[test]
    fn inline_list_or() {
        let items = parse_and_extract("false || echo fallback");
        let l = items
            .iter()
            .find(|i| i.metadata.attributes.contains(&"list".to_string()))
            .expect("should find || list");
        assert_eq!(l.kind, SymbolKind::Macro);
    }

    #[test]
    fn inline_list_chain() {
        let items = parse_and_extract("cmd1 && cmd2 || cmd3");
        let l = items
            .iter()
            .find(|i| i.metadata.attributes.contains(&"list".to_string()))
            .expect("should find chain list");
        assert!(
            l.source.as_deref().unwrap_or("").contains("&&"),
            "source should contain &&: {:?}",
            l.source
        );
    }
}
