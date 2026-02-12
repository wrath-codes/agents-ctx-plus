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

use crate::types::ParsedItem;

#[path = "../bash/doc.rs"]
mod doc;
#[path = "../bash/helpers.rs"]
mod helpers;
#[path = "../bash/processors/mod.rs"]
mod processors;

use doc::collect_doc_comments;
use processors::{
    process_c_style_for, process_case_statement, process_command, process_command_group,
    process_declaration_command, process_for_statement, process_function, process_if_statement,
    process_list, process_negated_command, process_pipeline, process_redirected_statement,
    process_shebang, process_subshell, process_test_command, process_unset_command,
    process_variable_assignment, process_while_statement,
};

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

// Top-level node dispatcher
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

#[cfg(test)]
#[path = "../bash/tests/mod.rs"]
mod tests;
