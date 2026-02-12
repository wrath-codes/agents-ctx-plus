//! Shared extraction helpers for C++ processing.

use ast_grep_core::Node;

pub(super) fn find_identifier_recursive<D: ast_grep_core::Doc>(node: &Node<D>) -> String {
    let children: Vec<_> = node.children().collect();
    for child in &children {
        match child.kind().as_ref() {
            // qualified_identifier preserves `::` separators (e.g. MyClass::method)
            "identifier"
            | "field_identifier"
            | "operator_name"
            | "destructor_name"
            | "qualified_identifier" => {
                return child.text().to_string();
            }
            "structured_binding_declarator" => {
                // Extract all identifiers and join as "[x, y]"
                let ids: Vec<String> = child
                    .children()
                    .filter(|c| c.kind().as_ref() == "identifier")
                    .map(|c| c.text().to_string())
                    .collect();
                if !ids.is_empty() {
                    return format!("[{}]", ids.join(", "));
                }
            }
            "pointer_declarator"
            | "reference_declarator"
            | "init_declarator"
            | "function_declarator"
            | "parenthesized_declarator"
            | "attributed_declarator" => {
                let name = find_identifier_recursive(child);
                if !name.is_empty() {
                    return name;
                }
            }
            _ => {}
        }
    }
    String::new()
}

pub(super) fn extract_return_type_from_children<D: ast_grep_core::Doc>(
    children: &[Node<D>],
) -> Option<String> {
    let mut parts = Vec::new();
    for child in children {
        match child.kind().as_ref() {
            "primitive_type"
            | "type_identifier"
            | "sized_type_specifier"
            | "type_qualifier"
            | "struct_specifier"
            | "qualified_identifier"
            | "template_type"
            | "decltype" => {
                parts.push(child.text().to_string());
            }
            "placeholder_type_specifier" => {
                parts.push("auto".to_string());
            }
            "function_declarator"
            | "init_declarator"
            | "identifier"
            | "array_declarator"
            | "pointer_declarator"
            | "reference_declarator"
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

pub(super) fn extract_parameters_from_declarator<D: ast_grep_core::Doc>(
    func_decl: &Node<D>,
) -> Vec<String> {
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
            k.as_ref() == "parameter_declaration"
                || k.as_ref() == "variadic_parameter"
                || k.as_ref() == "optional_parameter_declaration"
        })
        .map(|c| {
            c.text()
                .to_string()
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect()
}

pub(super) fn extract_field_names<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    let mut fields = Vec::new();
    let Some(body) = node
        .children()
        .find(|c| c.kind().as_ref() == "field_declaration_list")
    else {
        return fields;
    };
    let body_children: Vec<_> = body.children().collect();
    for child in &body_children {
        if child.kind().as_ref() == "field_declaration" {
            let fc: Vec<_> = child.children().collect();
            // Skip if it has a function_declarator (it's a method, not a field)
            if fc
                .iter()
                .any(|c| c.kind().as_ref() == "function_declarator")
            {
                continue;
            }
            for f in &fc {
                if f.kind().as_ref() == "field_identifier" {
                    fields.push(f.text().to_string());
                }
            }
        }
    }
    fields
}

pub(super) fn extract_method_names<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    let mut methods = Vec::new();
    let Some(body) = node
        .children()
        .find(|c| c.kind().as_ref() == "field_declaration_list")
    else {
        return methods;
    };
    let body_children: Vec<_> = body.children().collect();
    for child in &body_children {
        if child.kind().as_ref() == "function_definition"
            && let Some(name) = super::classes::extract_method_name(child)
        {
            methods.push(name);
        }
    }
    methods
}

pub(super) fn extract_enum_variants<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    let mut variants = Vec::new();
    let children: Vec<_> = node.children().collect();
    for child in &children {
        if child.kind().as_ref() == "enumerator_list" {
            let list_children: Vec<_> = child.children().collect();
            for lc in &list_children {
                if lc.kind().as_ref() == "enumerator"
                    && let Some(id) = lc.children().find(|c| c.kind().as_ref() == "identifier")
                {
                    variants.push(id.text().to_string());
                }
            }
        }
    }
    variants
}
