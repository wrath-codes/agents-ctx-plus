//! Function definitions, prototypes, variable declarations, and function pointer processing.

use ast_grep_core::Node;

use crate::types::{CMetadataExt, ParsedItem, SymbolKind, SymbolMetadata};

use super::core::{
    Qualifiers, attributes_from_qualifiers, classify_variable, detect_qualifiers,
    visibility_from_qualifiers,
};
use super::helpers::{
    extract_array_declarator_name, extract_declarator_name, extract_init_declarator_name,
    extract_parameters, extract_pointer_declarator_name, extract_return_type,
};
use super::{extract_signature, extract_source_limited};

// ── Function definition processing ────────────────────────────────

pub(super) fn process_function_definition<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
) {
    let children: Vec<_> = node.children().collect();
    let q = detect_qualifiers(node);

    // Find the function_declarator child
    let Some(func_decl) = children
        .iter()
        .find(|c| c.kind().as_ref() == "function_declarator")
    else {
        return;
    };

    // Extract function name from the declarator
    let name = extract_declarator_name(func_decl);
    if name.is_empty() {
        return;
    }

    // Extract return type
    let return_type = extract_return_type(&children);

    // Extract parameters
    let parameters = extract_parameters(func_decl);

    // Check for variadic
    let is_variadic = func_decl.text().as_ref().contains("...");

    let mut metadata = SymbolMetadata {
        return_type,
        parameters,
        attributes: attributes_from_qualifiers(&q),
        ..Default::default()
    };
    if is_variadic {
        metadata.push_attribute("variadic");
    }

    items.push(ParsedItem {
        kind: SymbolKind::Function,
        name,
        signature: extract_signature(node),
        source: extract_source_limited(node, 30),
        doc_comment: doc_comment.to_string(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: visibility_from_qualifiers(&q),
        metadata,
    });
}

// ── Declaration processing (variables, prototypes, function pointers) ──

#[allow(clippy::too_many_lines)]
pub(super) fn process_declaration<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
) {
    let children: Vec<_> = node.children().collect();
    let q = detect_qualifiers(node);

    // Check if this is a function declaration (prototype) vs function pointer variable.
    // A function pointer: `void (*callback)(int, int);` has function_declarator with
    // parenthesized_declarator child. A prototype: `int add(int a, int b);` has
    // function_declarator with an identifier child directly.
    if let Some(func_decl) = children
        .iter()
        .find(|c| c.kind().as_ref() == "function_declarator")
    {
        let is_fn_pointer = func_decl
            .children()
            .any(|c| c.kind().as_ref() == "parenthesized_declarator");
        if is_fn_pointer {
            process_function_pointer_var(node, items, doc_comment, &q);
        } else {
            process_function_prototype(node, items, doc_comment, &children, &q);
        }
        return;
    }

    // Check for init_declarator(s) (variable with initializer, or function pointer with init)
    let init_decls: Vec<_> = children
        .iter()
        .filter(|c| c.kind().as_ref() == "init_declarator")
        .collect();
    if !init_decls.is_empty() {
        // Check first one for function pointer
        if has_function_declarator_descendant(init_decls[0]) {
            process_function_pointer_var(node, items, doc_comment, &q);
        } else {
            for init_decl in &init_decls {
                let name = extract_init_declarator_name(init_decl);
                if name.is_empty() {
                    continue;
                }
                let return_type = extract_return_type(&children);
                let (kind, visibility) = classify_variable(&q);
                let mut metadata = SymbolMetadata {
                    return_type,
                    attributes: attributes_from_qualifiers(&q),
                    ..Default::default()
                };
                if init_decl
                    .children()
                    .any(|c| c.kind().as_ref() == "array_declarator")
                {
                    metadata.push_attribute("array");
                }
                items.push(ParsedItem {
                    kind,
                    name,
                    signature: extract_signature(node),
                    source: Some(node.text().to_string()),
                    doc_comment: doc_comment.to_string(),
                    start_line: node.start_pos().line() as u32 + 1,
                    end_line: node.end_pos().line() as u32 + 1,
                    visibility,
                    metadata,
                });
            }
        }
        return;
    }

    // Check for array_declarator
    let has_array_decl = children
        .iter()
        .any(|c| c.kind().as_ref() == "array_declarator");

    if has_array_decl {
        process_array_declaration(node, items, doc_comment, &children, &q);
        return;
    }

    // Check for pointer_declarator (could be a function pointer or pointer variable)
    let has_pointer_decl = children
        .iter()
        .any(|c| c.kind().as_ref() == "pointer_declarator");

    if has_pointer_decl {
        // Check if the pointer_declarator contains a function_declarator (function pointer var)
        if let Some(ptr) = children
            .iter()
            .find(|c| c.kind().as_ref() == "pointer_declarator")
            && has_function_declarator_descendant(ptr)
        {
            process_function_pointer_var(node, items, doc_comment, &q);
            return;
        }
        process_pointer_variable(node, items, doc_comment, &children, &q);
        return;
    }

    // Plain identifier declarations: `extern int shared;` or `int x, y, z;`
    let identifiers: Vec<_> = children
        .iter()
        .filter(|c| c.kind().as_ref() == "identifier")
        .collect();
    for id in &identifiers {
        let name = id.text().to_string();
        let return_type = extract_return_type(&children);
        let (kind, visibility) = classify_variable(&q);

        items.push(ParsedItem {
            kind,
            name,
            signature: extract_signature(node),
            source: Some(node.text().to_string()),
            doc_comment: doc_comment.to_string(),
            start_line: node.start_pos().line() as u32 + 1,
            end_line: node.end_pos().line() as u32 + 1,
            visibility,
            metadata: SymbolMetadata {
                return_type,
                attributes: attributes_from_qualifiers(&q),
                ..Default::default()
            },
        });
    }
}

/// Check if a node or any descendant contains a `function_declarator`.
pub(super) fn has_function_declarator_descendant<D: ast_grep_core::Doc>(node: &Node<D>) -> bool {
    let children: Vec<_> = node.children().collect();
    for child in &children {
        if child.kind().as_ref() == "function_declarator" {
            return true;
        }
        if has_function_declarator_descendant(child) {
            return true;
        }
    }
    false
}

fn process_function_prototype<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
    children: &[Node<D>],
    q: &Qualifiers,
) {
    let Some(func_decl) = children
        .iter()
        .find(|c| c.kind().as_ref() == "function_declarator")
    else {
        return;
    };

    let name = extract_declarator_name(func_decl);
    if name.is_empty() {
        return;
    }

    let return_type = extract_return_type(children);
    let parameters = extract_parameters(func_decl);
    let is_variadic = func_decl.text().as_ref().contains("...");

    let mut metadata = SymbolMetadata {
        return_type,
        parameters,
        attributes: attributes_from_qualifiers(q),
        ..Default::default()
    };
    metadata.push_attribute("prototype");
    if is_variadic {
        metadata.push_attribute("variadic");
    }

    items.push(ParsedItem {
        kind: SymbolKind::Function,
        name,
        signature: extract_signature(node),
        source: Some(node.text().to_string()),
        doc_comment: doc_comment.to_string(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: visibility_from_qualifiers(q),
        metadata,
    });
}

fn process_array_declaration<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
    children: &[Node<D>],
    q: &Qualifiers,
) {
    let Some(arr_decl) = children
        .iter()
        .find(|c| c.kind().as_ref() == "array_declarator")
    else {
        return;
    };

    // Name may be nested for multi-dimensional arrays
    let name = extract_array_declarator_name(arr_decl);

    if name.is_empty() {
        return;
    }

    let return_type = extract_return_type(children);
    let (kind, visibility) = classify_variable(q);

    let mut metadata = SymbolMetadata {
        return_type,
        attributes: attributes_from_qualifiers(q),
        ..Default::default()
    };
    metadata.push_attribute("array");

    items.push(ParsedItem {
        kind,
        name,
        signature: extract_signature(node),
        source: Some(node.text().to_string()),
        doc_comment: doc_comment.to_string(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility,
        metadata,
    });
}

fn process_pointer_variable<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
    children: &[Node<D>],
    q: &Qualifiers,
) {
    // Find the identifier deep inside the pointer_declarator
    let Some(ptr_decl) = children
        .iter()
        .find(|c| c.kind().as_ref() == "pointer_declarator")
    else {
        return;
    };

    let name = extract_pointer_declarator_name(ptr_decl);
    if name.is_empty() {
        return;
    }

    let return_type = extract_return_type(children);
    let (kind, visibility) = classify_variable(q);

    items.push(ParsedItem {
        kind,
        name,
        signature: extract_signature(node),
        source: Some(node.text().to_string()),
        doc_comment: doc_comment.to_string(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility,
        metadata: SymbolMetadata {
            return_type,
            attributes: attributes_from_qualifiers(q),
            ..Default::default()
        },
    });
}

fn process_function_pointer_var<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
    q: &Qualifiers,
) {
    // A declaration like: void (*callback)(int, int) = NULL;
    // The name is deep inside: function_declarator > parenthesized_declarator > pointer_declarator > identifier
    let name = extract_function_pointer_name(node);
    if name.is_empty() {
        return;
    }

    let mut metadata = SymbolMetadata {
        attributes: attributes_from_qualifiers(q),
        ..Default::default()
    };
    metadata.push_attribute("function_pointer");

    items.push(ParsedItem {
        kind: SymbolKind::Static,
        name,
        signature: extract_signature(node),
        source: Some(node.text().to_string()),
        doc_comment: doc_comment.to_string(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: visibility_from_qualifiers(q),
        metadata,
    });
}

/// Extract the name from a function pointer declaration.
///
/// Traverses: `function_declarator` > `parenthesized_declarator` >
/// `pointer_declarator` > `identifier`.
pub(super) fn extract_function_pointer_name<D: ast_grep_core::Doc>(node: &Node<D>) -> String {
    fn find_identifier_in<D2: ast_grep_core::Doc>(node: &Node<D2>) -> Option<String> {
        let children: Vec<_> = node.children().collect();
        for child in &children {
            match child.kind().as_ref() {
                "identifier" | "type_identifier" => return Some(child.text().to_string()),
                "function_declarator"
                | "parenthesized_declarator"
                | "pointer_declarator"
                | "init_declarator" => {
                    if let Some(name) = find_identifier_in(child) {
                        return Some(name);
                    }
                }
                _ => {}
            }
        }
        None
    }
    find_identifier_in(node).unwrap_or_default()
}
