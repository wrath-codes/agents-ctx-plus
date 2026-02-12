//! JavaScript extraction processors: functions, classes, generators,
//! and variable/lexical declarations.

mod classes;
mod declarations;
mod functions;

use ast_grep_core::Node;

use crate::types::ParsedItem;

pub(super) use classes::{process_class, process_class_members};
pub(super) use declarations::{process_lexical_declaration, process_variable_declaration};
pub(super) use functions::{process_function, process_generator_function};

// ── export_statement unwrapping ────────────────────────────────────

pub(super) fn process_export_statement<D: ast_grep_core::Doc>(
    export_node: &Node<D>,
) -> Vec<ParsedItem> {
    let is_default = export_node
        .children()
        .any(|c| c.kind().as_ref() == "default");

    let mut items = Vec::new();
    for child in export_node.children() {
        let k = child.kind();
        match k.as_ref() {
            "function_declaration" => {
                if let Some(item) = process_function(&child, export_node, true, is_default) {
                    items.push(item);
                }
            }
            "generator_function_declaration" => {
                if let Some(item) = process_generator_function(&child, export_node, true) {
                    items.push(item);
                }
            }
            "class_declaration" => {
                if let Some(item) = process_class(&child, export_node, true, is_default) {
                    items.push(item);
                }
                items.extend(process_class_members(&child, true));
            }
            "lexical_declaration" => {
                items.extend(process_lexical_declaration(&child, export_node, true));
            }
            _ => {}
        }
    }
    items
}
