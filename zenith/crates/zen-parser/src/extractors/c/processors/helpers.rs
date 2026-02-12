//! Name extraction helpers for C declarator nodes.

use ast_grep_core::Node;

/// Extract the function name from a `function_declarator` node.
pub(super) fn extract_declarator_name<D: ast_grep_core::Doc>(node: &Node<D>) -> String {
    node.children()
        .find(|c| c.kind().as_ref() == "identifier")
        .map_or_else(String::new, |n| n.text().to_string())
}

/// Extract the variable name from an `init_declarator` node.
///
/// Handles direct identifiers, pointer declarators, and array declarators.
pub(super) fn extract_init_declarator_name<D: ast_grep_core::Doc>(node: &Node<D>) -> String {
    // Direct identifier
    if let Some(id) = node.children().find(|c| c.kind().as_ref() == "identifier") {
        return id.text().to_string();
    }

    // Pointer declarator: *VERSION = "..."
    if let Some(ptr) = node
        .children()
        .find(|c| c.kind().as_ref() == "pointer_declarator")
    {
        return extract_pointer_declarator_name(&ptr);
    }

    // Array declarator: prime_numbers[10] = {2, 3, ...}
    if let Some(arr) = node
        .children()
        .find(|c| c.kind().as_ref() == "array_declarator")
    {
        return extract_array_declarator_name(&arr);
    }

    String::new()
}

/// Recursively extract identifier from an `array_declarator` (may be nested for multi-dim).
pub(super) fn extract_array_declarator_name<D: ast_grep_core::Doc>(node: &Node<D>) -> String {
    let children: Vec<_> = node.children().collect();
    for child in &children {
        match child.kind().as_ref() {
            "identifier" => return child.text().to_string(),
            "array_declarator" => return extract_array_declarator_name(child),
            _ => {}
        }
    }
    String::new()
}

/// Extract identifier from a `pointer_declarator` (may be nested).
pub(super) fn extract_pointer_declarator_name<D: ast_grep_core::Doc>(node: &Node<D>) -> String {
    let children: Vec<_> = node.children().collect();
    for child in &children {
        match child.kind().as_ref() {
            "identifier" => return child.text().to_string(),
            "array_declarator" => {
                // *argv[] — name is inside the array_declarator
                if let Some(id) = child.children().find(|c| c.kind().as_ref() == "identifier") {
                    return id.text().to_string();
                }
            }
            "pointer_declarator" => {
                return extract_pointer_declarator_name(child);
            }
            _ => {}
        }
    }
    String::new()
}

/// Extract the return type from declaration children.
pub(super) fn extract_return_type<D: ast_grep_core::Doc>(children: &[Node<D>]) -> Option<String> {
    let mut parts = Vec::new();
    for child in children {
        match child.kind().as_ref() {
            "primitive_type"
            | "type_identifier"
            | "sized_type_specifier"
            | "type_qualifier"
            | "struct_specifier" => {
                parts.push(child.text().to_string());
            }
            // Stop at declarators or semicolons
            "function_declarator"
            | "init_declarator"
            | "identifier"
            | "array_declarator"
            | "pointer_declarator"
            | ";" => break,
            _ => {}
        }
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" "))
    }
}

/// Extract parameter names from a `function_declarator`'s `parameter_list`.
pub(super) fn extract_parameters<D: ast_grep_core::Doc>(func_decl: &Node<D>) -> Vec<String> {
    let children: Vec<_> = func_decl.children().collect();
    let Some(param_list) = children
        .iter()
        .find(|c| c.kind().as_ref() == "parameter_list")
    else {
        return Vec::new();
    };

    let params: Vec<_> = param_list.children().collect();
    params
        .iter()
        .filter(|c| {
            let k = c.kind();
            k.as_ref() == "parameter_declaration" || k.as_ref() == "variadic_parameter"
        })
        .map(|c| {
            let text = c.text().to_string();
            text.split_whitespace().collect::<Vec<_>>().join(" ")
        })
        .collect()
}
