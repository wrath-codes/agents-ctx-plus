//! TypeScript extraction processors: functions, classes, interfaces,
//! type aliases, enums, namespaces, and declarations.

mod classes;
mod declarations;
mod functions;
mod types;

use ast_grep_core::Node;

use crate::types::ParsedItem;

pub(super) use classes::{process_class, process_class_members};
pub(super) use declarations::{
    process_ambient_declaration, process_lexical_declaration, process_variable_declaration,
};
pub(super) use functions::{process_function, process_function_signature};
pub(super) use types::{
    process_enum, process_interface, process_interface_members, process_namespace,
    process_type_alias,
};

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
            "class_declaration" | "abstract_class_declaration" => {
                if let Some(item) = process_class(&child, export_node, true, is_default) {
                    items.push(item);
                }
                items.extend(process_class_members(&child, true));
            }
            "interface_declaration" => {
                if let Some(item) = process_interface(&child, export_node, true) {
                    items.push(item);
                }
                items.extend(process_interface_members(&child, true));
            }
            "type_alias_declaration" => {
                if let Some(item) = process_type_alias(&child, export_node, true) {
                    items.push(item);
                }
            }
            "enum_declaration" => {
                if let Some(item) = process_enum(&child, export_node, true) {
                    items.push(item);
                }
            }
            "lexical_declaration" => {
                items.extend(process_lexical_declaration(&child, export_node, true));
            }
            "internal_module" => {
                if let Some(item) = process_namespace(&child, export_node, true) {
                    items.push(item);
                }
            }
            _ => {}
        }
    }
    items
}
