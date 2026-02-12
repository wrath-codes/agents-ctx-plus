use ast_grep_core::Node;

use crate::types::Visibility;

pub(super) fn go_visibility(name: &str) -> Visibility {
    if name.starts_with(char::is_uppercase) {
        Visibility::Public
    } else {
        Visibility::Private
    }
}

// ── Doc comment extraction ────────────────────────────────────────

/// Extract Go doc comments by walking backward through sibling `comment` nodes.
///
/// Go convention: doc comments are `//` comments immediately preceding
/// a declaration, with no blank lines in between.
pub(super) fn extract_go_doc<D: ast_grep_core::Doc>(node: &Node<D>) -> String {
    let mut comments = Vec::new();
    let mut current = node.prev();
    while let Some(sibling) = current {
        if sibling.kind().as_ref() == "comment" {
            let text = sibling.text().to_string();
            if let Some(stripped) = text.strip_prefix("//") {
                comments.push(stripped.trim().to_string());
            }
        } else {
            break;
        }
        current = sibling.prev();
    }
    comments.reverse();
    comments.join("\n")
}

// ── function_declaration ──────────────────────────────────────────

/// Extract return type from a Go function/method.
///
/// Go return type can be:
/// - A single `type_identifier` (e.g., `error`, `string`)
/// - A single `pointer_type` (e.g., `*Config`)
/// - A single `slice_type` (e.g., `[]U`)
/// - A `parameter_list` for multiple returns (e.g., `([]string, error)`)
pub(super) fn extract_go_return_type<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<String> {
    let children: Vec<_> = node.children().collect();

    // Find block (body) position — return type is between params and block
    let block_idx = children.iter().position(|c| c.kind().as_ref() == "block");

    let block_idx = block_idx?;

    // Look at the child immediately before the block
    if block_idx > 0 {
        let prev = &children[block_idx - 1];
        let k = prev.kind();
        let kr = k.as_ref();
        // The return type is any type node before the block
        // but NOT a parameter_list that is the actual params
        if kr == "type_identifier"
            || kr == "pointer_type"
            || kr == "slice_type"
            || kr == "qualified_type"
            || kr == "map_type"
            || kr == "channel_type"
            || kr == "array_type"
            || kr == "function_type"
        {
            return Some(prev.text().to_string());
        }
        // Multiple return values: the last parameter_list before block
        if kr == "parameter_list" {
            // For functions: param_list(params) param_list(returns) block
            // For methods: param_list(receiver) name param_list(params) param_list(returns) block
            // We need to check this is NOT the params list
            // Count parameter_lists before block
            let param_lists: Vec<_> = children[..block_idx]
                .iter()
                .filter(|c| c.kind().as_ref() == "parameter_list")
                .collect();

            let is_function = node.kind().as_ref() == "function_declaration";
            let min_param_lists = if is_function { 1 } else { 2 }; // methods have receiver + params

            if param_lists.len() > min_param_lists {
                return Some(prev.text().to_string());
            }
        }
    }
    None
}

/// Extract parameter declarations from a Go function (not method).
pub(super) fn extract_go_parameters<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    // For function_declaration: the first parameter_list is the params
    let Some(params) = node
        .children()
        .find(|c| c.kind().as_ref() == "parameter_list")
    else {
        return Vec::new();
    };
    extract_param_decls(&params)
}

/// Extract parameter declarations from a Go method (skip receiver).
pub(super) fn extract_go_method_parameters<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    // For method_declaration: first param_list is receiver, second is params
    let param_lists: Vec<_> = node
        .children()
        .filter(|c| c.kind().as_ref() == "parameter_list")
        .collect();

    if param_lists.len() >= 2 {
        extract_param_decls(&param_lists[1])
    } else {
        Vec::new()
    }
}

/// Extract the receiver type from a Go method.
pub(super) fn extract_go_receiver<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<String> {
    // First parameter_list is the receiver
    let receiver_list = node
        .children()
        .find(|c| c.kind().as_ref() == "parameter_list")?;

    // The receiver is a parameter_declaration inside
    for child in receiver_list.children() {
        if child.kind().as_ref() == "parameter_declaration" {
            // Extract the type part — could be `*Config` or `Config`
            for sub in child.children() {
                let k = sub.kind();
                let kr = k.as_ref();
                if kr == "pointer_type" || kr == "type_identifier" {
                    return Some(sub.text().to_string());
                }
            }
        }
    }
    None
}

/// Extract parameter declarations from a `parameter_list` node.
///
/// Includes both regular and variadic (`...`) parameter declarations.
pub(super) fn extract_param_decls<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    node.children()
        .filter(|c| {
            let k = c.kind();
            k.as_ref() == "parameter_declaration" || k.as_ref() == "variadic_parameter_declaration"
        })
        .map(|c| c.text().to_string())
        .collect()
}

/// Extract struct field names from a `struct_type` node.
///
/// Handles three cases:
/// - Named fields: `field_declaration` with `field_identifier` child (e.g., `Name string`)
/// - Embedded types: `field_declaration` with only `type_identifier` (e.g., `Config`)
/// - Embedded pointer types: `field_declaration` with `*` + `type_identifier` (e.g., `*Logger`)
pub(super) fn extract_struct_fields<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    let mut fields = Vec::new();
    for child in node.children() {
        if child.kind().as_ref() == "field_declaration_list" {
            for field in child.children() {
                if field.kind().as_ref() == "field_declaration" {
                    if let Some(name) = field
                        .children()
                        .find(|c| c.kind().as_ref() == "field_identifier")
                    {
                        // Named field: `Port int`
                        fields.push(name.text().to_string());
                    } else if let Some(type_id) = field
                        .children()
                        .find(|c| c.kind().as_ref() == "type_identifier")
                    {
                        // Embedded type: `Config` or `*Logger` (type_identifier is the name)
                        fields.push(type_id.text().to_string());
                    }
                }
            }
        }
    }
    fields
}

/// Extract interface method names from an `interface_type` node.
pub(super) fn extract_interface_methods<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    let mut methods = Vec::new();
    for child in node.children() {
        let k = child.kind();
        if (k.as_ref() == "method_spec" || k.as_ref() == "method_elem")
            && let Some(name) = child
                .children()
                .find(|c| c.kind().as_ref() == "field_identifier")
        {
            methods.push(name.text().to_string());
        }
    }
    methods
}

/// Extract type parameters from a `type_spec` node (Go generics).
pub(super) fn extract_go_type_params_from_spec<D: ast_grep_core::Doc>(
    node: &Node<D>,
) -> Option<String> {
    node.children()
        .find(|c| c.kind().as_ref() == "type_parameter_list")
        .map(|tp| tp.text().to_string())
}

/// Extract type parameters from a `function_declaration` node (Go generics).
pub(super) fn extract_go_type_parameters<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<String> {
    node.children()
        .find(|c| c.kind().as_ref() == "type_parameter_list")
        .map(|tp| tp.text().to_string())
}
