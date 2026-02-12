//! Template processing: template declarations, instantiations, concepts.

use ast_grep_core::Node;

use crate::types::{CppMetadataExt, ParsedItem, SymbolKind, SymbolMetadata, Visibility};

use super::classes::process_class;
use super::helpers::{
    extract_field_names, extract_method_names, extract_parameters_from_declarator,
    extract_return_type_from_children, find_identifier_recursive,
};
use super::{extract_signature, extract_source_limited};

#[allow(clippy::only_used_in_recursion, clippy::too_many_lines)]
pub(super) fn process_template_declaration<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    source: &str,
    doc_comment: &str,
) {
    let children: Vec<_> = node.children().collect();

    let template_params = children
        .iter()
        .find(|c| c.kind().as_ref() == "template_parameter_list")
        .map(|c| c.text().to_string());

    // Detect requires clause on the template declaration itself
    let requires_constraint = children
        .iter()
        .find(|c| c.kind().as_ref() == "requires_clause")
        .map(|c| c.text().to_string());

    let items_before = items.len();

    for child in &children {
        match child.kind().as_ref() {
            "class_specifier" => {
                process_class(child, items, doc_comment, template_params.as_deref());
            }
            "struct_specifier" => {
                // Template struct — emit with template attribute
                let name = child
                    .children()
                    .find(|c| c.kind().as_ref() == "type_identifier")
                    .map_or_else(String::new, |n| n.text().to_string());
                if !name.is_empty() {
                    let fields = extract_field_names(child);
                    let methods = extract_method_names(child);
                    let mut metadata = SymbolMetadata::default();
                    metadata.set_fields(fields);
                    metadata.set_methods(methods);
                    metadata.set_generics(template_params.clone());
                    metadata.push_attribute("template");

                    items.push(ParsedItem {
                        kind: SymbolKind::Struct,
                        name,
                        signature: extract_signature(node),
                        source: extract_source_limited(node, 30),
                        doc_comment: doc_comment.to_string(),
                        start_line: node.start_pos().line() as u32 + 1,
                        end_line: node.end_pos().line() as u32 + 1,
                        visibility: Visibility::Public,
                        metadata,
                    });
                }
            }
            "function_definition" => {
                // Template function — emit directly
                process_template_function(
                    node,
                    child,
                    items,
                    doc_comment,
                    template_params.as_deref(),
                );
            }
            "declaration" => {
                // Could be template variable or template function prototype
                process_template_function_decl(
                    node,
                    child,
                    items,
                    doc_comment,
                    template_params.as_deref(),
                );
            }
            "alias_declaration" => {
                // Template alias: template<typename T> using Ptr = T*;
                let alias_name = child
                    .children()
                    .find(|c| c.kind().as_ref() == "type_identifier")
                    .map_or_else(String::new, |n| n.text().to_string());
                if !alias_name.is_empty() {
                    let mut metadata = SymbolMetadata::default();
                    metadata.set_generics(template_params.clone());
                    metadata.push_attribute("template");
                    metadata.push_attribute("using");

                    items.push(ParsedItem {
                        kind: SymbolKind::TypeAlias,
                        name: alias_name,
                        signature: extract_signature(node),
                        source: Some(node.text().to_string()),
                        doc_comment: doc_comment.to_string(),
                        start_line: node.start_pos().line() as u32 + 1,
                        end_line: node.end_pos().line() as u32 + 1,
                        visibility: Visibility::Public,
                        metadata,
                    });
                }
            }
            "concept_definition" => {
                process_concept(child, items, doc_comment, template_params.as_deref());
            }
            "template_declaration" => {
                // Nested template — recurse
                let inner_doc = doc_comment.to_string();
                process_template_declaration(child, items, source, &inner_doc);
            }
            _ => {}
        }
    }

    // Annotate newly emitted items with the requires clause if present
    if let Some(ref constraint) = requires_constraint {
        for item in items.iter_mut().skip(items_before) {
            let attr = format!("requires:{constraint}");
            if !item.metadata.attributes.contains(&attr) {
                item.metadata.attributes.push(attr);
            }
        }
    }
}

pub(super) fn process_template_instantiation<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
) {
    let sig = extract_signature(node);

    // tree-sitter-cpp wraps `template class V<int>;` as:
    //   template_instantiation
    //     class_specifier (or struct_specifier)
    //       template_type: "V<int>"
    let name = node
        .children()
        .find(|c| {
            let k = c.kind();
            k.as_ref() == "class_specifier" || k.as_ref() == "struct_specifier"
        })
        .and_then(|spec| {
            spec.children()
                .find(|c| c.kind().as_ref() == "template_type")
                .map(|n| n.text().to_string())
                .or_else(|| {
                    spec.children()
                        .find(|c| c.kind().as_ref() == "type_identifier")
                        .map(|n| n.text().to_string())
                })
        })
        .or_else(|| {
            node.children()
                .find(|c| c.kind().as_ref() == "template_type")
                .map(|n| n.text().to_string())
        })
        .or_else(|| {
            node.children()
                .find(|c| c.kind().as_ref() == "type_identifier")
                .map(|n| n.text().to_string())
        })
        .unwrap_or_default();
    if name.is_empty() {
        return;
    }
    let mut metadata = SymbolMetadata::default();
    metadata.push_attribute("explicit_instantiation");

    items.push(ParsedItem {
        kind: SymbolKind::Class,
        name,
        signature: sig,
        source: Some(node.text().to_string()),
        doc_comment: doc_comment.to_string(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata,
    });
}

fn process_template_function<D: ast_grep_core::Doc>(
    template_node: &Node<D>,
    func_node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
    template_params: Option<&str>,
) {
    let children: Vec<_> = func_node.children().collect();
    let Some(func_decl) = children
        .iter()
        .find(|c| c.kind().as_ref() == "function_declarator")
    else {
        return;
    };

    let name = find_identifier_recursive(func_decl);
    if name.is_empty() {
        return;
    }

    let return_type = extract_return_type_from_children(&children);
    let parameters = extract_parameters_from_declarator(func_decl);

    let mut attrs = vec!["template".to_string()];
    for child in &children {
        match child.kind().as_ref() {
            "storage_class_specifier" => {
                attrs.push(child.text().to_string());
            }
            "requires_clause" => {
                attrs.push(format!("requires:{}", child.text()));
            }
            _ => {}
        }
    }

    let mut metadata = SymbolMetadata::default();
    metadata.set_return_type(return_type);
    metadata.set_parameters(parameters);
    metadata.set_generics(template_params.map(String::from));
    for attr in attrs {
        metadata.push_attribute(attr);
    }

    items.push(ParsedItem {
        kind: SymbolKind::Function,
        name,
        signature: extract_signature(template_node),
        source: extract_source_limited(template_node, 30),
        doc_comment: doc_comment.to_string(),
        start_line: template_node.start_pos().line() as u32 + 1,
        end_line: template_node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata,
    });
}

fn process_template_function_decl<D: ast_grep_core::Doc>(
    template_node: &Node<D>,
    decl_node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
    template_params: Option<&str>,
) {
    let children: Vec<_> = decl_node.children().collect();
    let Some(func_decl) = children
        .iter()
        .find(|c| c.kind().as_ref() == "function_declarator")
    else {
        // Template variable — look for init_declarator
        for child in &children {
            if child.kind().as_ref() == "init_declarator" {
                let name = find_identifier_recursive(child);
                if !name.is_empty() {
                    let mut metadata = SymbolMetadata::default();
                    metadata.set_generics(template_params.map(String::from));
                    metadata.push_attribute("template");

                    items.push(ParsedItem {
                        kind: SymbolKind::Static,
                        name,
                        signature: extract_signature(template_node),
                        source: Some(template_node.text().to_string()),
                        doc_comment: doc_comment.to_string(),
                        start_line: template_node.start_pos().line() as u32 + 1,
                        end_line: template_node.end_pos().line() as u32 + 1,
                        visibility: Visibility::Public,
                        metadata,
                    });
                }
            }
        }
        return;
    };

    let name = find_identifier_recursive(func_decl);
    if name.is_empty() {
        return;
    }
    let return_type = extract_return_type_from_children(&children);
    let parameters = extract_parameters_from_declarator(func_decl);

    let mut metadata = SymbolMetadata::default();
    metadata.set_return_type(return_type);
    metadata.set_parameters(parameters);
    metadata.set_generics(template_params.map(String::from));
    metadata.push_attribute("template");
    metadata.push_attribute("prototype");

    items.push(ParsedItem {
        kind: SymbolKind::Function,
        name,
        signature: extract_signature(template_node),
        source: Some(template_node.text().to_string()),
        doc_comment: doc_comment.to_string(),
        start_line: template_node.start_pos().line() as u32 + 1,
        end_line: template_node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata,
    });
}

fn process_concept<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
    template_params: Option<&str>,
) {
    let name = node
        .children()
        .find(|c| c.kind().as_ref() == "identifier")
        .map_or_else(String::new, |n| n.text().to_string());
    if name.is_empty() {
        return;
    }

    let mut metadata = SymbolMetadata::default();
    metadata.set_generics(template_params.map(String::from));
    metadata.push_attribute("concept");

    items.push(ParsedItem {
        kind: SymbolKind::Trait,
        name,
        signature: extract_signature(node),
        source: Some(node.text().to_string()),
        doc_comment: doc_comment.to_string(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata,
    });
}
