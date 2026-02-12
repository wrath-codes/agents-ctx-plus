#![allow(clippy::field_reassign_with_default)]

//! Class processing: class definitions, members, nested types, access specifiers.

use ast_grep_core::Node;

use crate::types::{CppMetadataExt, ParsedItem, SymbolKind, SymbolMetadata, Visibility};

use super::c_nodes::{process_c_enum, process_c_struct};
use super::helpers::find_identifier_recursive;
use super::templates::process_template_declaration;
use super::{extract_signature, extract_source_limited, process_alias_declaration};

// ── Class processing ───────────────────────────────────────────────

#[allow(clippy::too_many_lines)]
pub(super) fn process_class<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
    template_params: Option<&str>,
) {
    let children: Vec<_> = node.children().collect();

    let name = children
        .iter()
        .find(|c| c.kind().as_ref() == "type_identifier" || c.kind().as_ref() == "template_type")
        .map_or_else(String::new, |n| n.text().to_string());
    if name.is_empty() {
        return;
    }

    let is_final = children
        .iter()
        .any(|c| c.kind().as_ref() == "virtual_specifier" && c.text().as_ref() == "final");

    let base_classes = extract_base_classes(&children);
    let (methods, fields, has_pure_virtual, access_sections) = extract_class_members(node);
    let is_abstract = has_pure_virtual;

    let is_error_type = name.ends_with("Error")
        || name.ends_with("Exception")
        || base_classes.iter().any(|b| b.contains("exception"));

    let mut attrs = vec!["class".to_string()];
    if is_final {
        attrs.push("final".to_string());
    }
    if is_abstract {
        attrs.push("abstract".to_string());
    }
    if template_params.is_some() {
        attrs.push("template".to_string());
    }
    attrs.extend(access_sections);

    let mut metadata = SymbolMetadata::default();
    metadata.set_base_classes(base_classes);
    metadata.set_methods(methods);
    metadata.set_fields(fields);
    if is_abstract {
        metadata.mark_unsafe();
    }
    if is_error_type {
        metadata.mark_error_type();
    }
    metadata.set_generics(template_params.map(String::from));
    for attr in attrs {
        metadata.push_attribute(attr);
    }

    items.push(ParsedItem {
        kind: SymbolKind::Class,
        name: name.clone(),
        signature: extract_signature(node),
        source: extract_source_limited(node, 40),
        doc_comment: doc_comment.to_string(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata,
    });

    emit_class_member_items(node, &name, items);

    // Emit nested types (nested classes, structs, enums, aliases) as
    // separate ParsedItems.
    extract_nested_types(node, items);
}

fn emit_class_member_items<D: ast_grep_core::Doc>(
    node: &Node<D>,
    class_name: &str,
    items: &mut Vec<ParsedItem>,
) {
    let Some(body) = node
        .children()
        .find(|c| c.kind().as_ref() == "field_declaration_list")
    else {
        return;
    };

    let mut current_access = Visibility::Private;

    for child in body.children() {
        match child.kind().as_ref() {
            "access_specifier" => {
                let text = child.text().to_string();
                let trimmed = text.trim().trim_end_matches(':').trim();
                current_access = match trimmed {
                    "public" => Visibility::Public,
                    "protected" => Visibility::Protected,
                    _ => Visibility::Private,
                };
            }
            "function_definition" => {
                if let Some(name) = extract_method_name(&child) {
                    push_cpp_member_item(items, class_name, &name, &current_access, &child, true);
                }
            }
            "field_declaration" => {
                let children: Vec<_> = child.children().collect();
                let has_func_decl = children
                    .iter()
                    .any(|c| c.kind().as_ref() == "function_declarator");

                if has_func_decl {
                    if let Some(name) = extract_field_decl_method_name(&children) {
                        push_cpp_member_item(
                            items,
                            class_name,
                            &name,
                            &current_access,
                            &child,
                            true,
                        );
                    }
                } else {
                    for fc in &children {
                        if fc.kind().as_ref() == "field_identifier" {
                            push_cpp_member_item(
                                items,
                                class_name,
                                fc.text().as_ref(),
                                &current_access,
                                &child,
                                false,
                            );
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

fn push_cpp_member_item<D: ast_grep_core::Doc>(
    items: &mut Vec<ParsedItem>,
    class_name: &str,
    member_name: &str,
    visibility: &Visibility,
    node: &Node<D>,
    is_callable: bool,
) {
    let simple_name = member_name
        .split("::")
        .last()
        .unwrap_or(member_name)
        .to_string();
    let kind = if is_callable {
        if simple_name == class_name {
            SymbolKind::Constructor
        } else {
            SymbolKind::Method
        }
    } else {
        SymbolKind::Field
    };

    let mut metadata = SymbolMetadata::default();
    metadata.owner_name = Some(class_name.to_string());
    metadata.owner_kind = Some(SymbolKind::Class);
    metadata.is_static_member = node.children().any(|c| c.text().as_ref() == "static");

    items.push(ParsedItem {
        kind,
        name: format!("{class_name}::{simple_name}"),
        signature: extract_signature(node),
        source: extract_source_limited(node, 12),
        doc_comment: String::new(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: visibility.clone(),
        metadata,
    });
}

/// Walk a class/struct body and emit separate `ParsedItem`s for nested
/// type definitions (classes, structs, enums, aliases, templates).
fn extract_nested_types<D: ast_grep_core::Doc>(node: &Node<D>, items: &mut Vec<ParsedItem>) {
    let Some(body) = node
        .children()
        .find(|c| c.kind().as_ref() == "field_declaration_list")
    else {
        return;
    };
    for child in body.children() {
        dispatch_nested_type(&child, items);
    }
}

/// Dispatch a single child from a `field_declaration_list` to the
/// appropriate nested-type handler.  Also handles `field_declaration`
/// nodes that wrap type specifiers (tree-sitter-cpp wraps nested
/// `enum class E { … };` inside a `field_declaration`).
fn dispatch_nested_type<D: ast_grep_core::Doc>(node: &Node<D>, items: &mut Vec<ParsedItem>) {
    match node.kind().as_ref() {
        "class_specifier" => {
            process_class(node, items, "", None);
        }
        "struct_specifier" => {
            process_c_struct(node, items, "");
        }
        "enum_specifier" => {
            process_c_enum(node, items, "");
        }
        "alias_declaration" => {
            process_alias_declaration(node, items, "");
        }
        "template_declaration" => {
            process_template_declaration(node, items, "", "");
        }
        "field_declaration" => {
            // field_declaration may wrap a nested type specifier, e.g.:
            //   field_declaration: "enum class E { A, B };"
            //     enum_specifier: "enum class E { A, B }"
            for inner in node.children() {
                let k = inner.kind();
                if k.as_ref() == "enum_specifier"
                    || k.as_ref() == "class_specifier"
                    || k.as_ref() == "struct_specifier"
                    || k.as_ref() == "union_specifier"
                {
                    dispatch_nested_type(&inner, items);
                }
            }
        }
        _ => {}
    }
}

fn extract_base_classes<D: ast_grep_core::Doc>(children: &[Node<D>]) -> Vec<String> {
    let Some(clause) = children
        .iter()
        .find(|c| c.kind().as_ref() == "base_class_clause")
    else {
        return Vec::new();
    };
    let clause_children: Vec<_> = clause.children().collect();
    clause_children
        .iter()
        .filter(|c| {
            let k = c.kind();
            k.as_ref() == "type_identifier"
                || k.as_ref() == "qualified_identifier"
                || k.as_ref() == "template_type"
        })
        .map(|c| c.text().to_string())
        .collect()
}

/// Extract methods, fields, and detect pure virtuals from a class body.
///
/// Returns `(methods, fields, has_pure_virtual, access_sections)`.
/// `access_sections` contains markers like `"has_public_members"`,
/// `"has_protected_members"`, `"has_private_members"` for each access
/// level that contains at least one member, plus friend declarations
/// (as `"friend:Name"`) and method qualifier markers
/// (`"has_override"`, `"has_final_methods"`, `"has_deleted_members"`,
/// `"has_defaulted_members"`).
#[allow(clippy::too_many_lines)]
fn extract_class_members<D: ast_grep_core::Doc>(
    node: &Node<D>,
) -> (Vec<String>, Vec<String>, bool, Vec<String>) {
    let mut methods = Vec::new();
    let mut fields = Vec::new();
    let mut has_pure_virtual = false;
    let mut has_public = false;
    let mut has_protected = false;
    let mut has_private = false;
    let mut friends: Vec<String> = Vec::new();
    let mut has_override = false;
    let mut has_final_methods = false;
    let mut has_deleted_members = false;
    let mut has_defaulted_members = false;
    // Default access for `class` is private
    let mut current_access = "private";

    let Some(body) = node
        .children()
        .find(|c| c.kind().as_ref() == "field_declaration_list")
    else {
        return (methods, fields, has_pure_virtual, Vec::new());
    };

    let body_children: Vec<_> = body.children().collect();
    for child in &body_children {
        match child.kind().as_ref() {
            "access_specifier" => {
                let text = child.text().to_string();
                let trimmed = text.trim().trim_end_matches(':').trim();
                match trimmed {
                    "public" => current_access = "public",
                    "protected" => current_access = "protected",
                    "private" => current_access = "private",
                    _ => {}
                }
            }
            "function_definition" => {
                // Detect method qualifiers on function_definition
                detect_method_qualifiers(
                    child,
                    &mut has_override,
                    &mut has_final_methods,
                    &mut has_deleted_members,
                    &mut has_defaulted_members,
                );
                if let Some(name) = extract_method_name(child) {
                    match current_access {
                        "public" => has_public = true,
                        "protected" => has_protected = true,
                        _ => has_private = true,
                    }
                    methods.push(name);
                }
            }
            "field_declaration" => {
                // Could be: pure virtual method, regular field, or friend
                let fc_children: Vec<_> = child.children().collect();

                // Detect method qualifiers on field declarations with function_declarators
                detect_method_qualifiers(
                    child,
                    &mut has_override,
                    &mut has_final_methods,
                    &mut has_deleted_members,
                    &mut has_defaulted_members,
                );

                // Pure virtual: has function_declarator + "= 0"
                let has_func_decl = fc_children
                    .iter()
                    .any(|c| c.kind().as_ref() == "function_declarator");
                let has_zero = fc_children
                    .iter()
                    .any(|c| c.kind().as_ref() == "number_literal" && c.text().as_ref() == "0");

                if has_func_decl && has_zero {
                    has_pure_virtual = true;
                    if let Some(name) = extract_field_decl_method_name(&fc_children) {
                        match current_access {
                            "public" => has_public = true,
                            "protected" => has_protected = true,
                            _ => has_private = true,
                        }
                        methods.push(name);
                    }
                } else if has_func_decl {
                    // Non-pure virtual method declaration
                    if let Some(name) = extract_field_decl_method_name(&fc_children) {
                        match current_access {
                            "public" => has_public = true,
                            "protected" => has_protected = true,
                            _ => has_private = true,
                        }
                        methods.push(name);
                    }
                } else {
                    // Regular field — look for field_identifier
                    for fc in &fc_children {
                        if fc.kind().as_ref() == "field_identifier" {
                            match current_access {
                                "public" => has_public = true,
                                "protected" => has_protected = true,
                                _ => has_private = true,
                            }
                            fields.push(fc.text().to_string());
                        }
                    }
                    // Also check for class_specifier inside (nested class as field_declaration)
                }
            }
            "friend_declaration" => {
                // Extract the friend name from friend declarations.
                let fc: Vec<_> = child.children().collect();
                let friend_name = fc
                    .iter()
                    .filter(|c| c.kind().as_ref() != "friend" && c.kind().as_ref() != ";")
                    .find_map(|c| {
                        let k = c.kind();
                        if k.as_ref() == "type_identifier" || k.as_ref() == "identifier" {
                            Some(c.text().to_string())
                        } else if k.as_ref() == "function_declarator" || k.as_ref() == "declaration"
                        {
                            let name = find_identifier_recursive(c);
                            if name.is_empty() { None } else { Some(name) }
                        } else {
                            None
                        }
                    });
                if let Some(name) = friend_name
                    && !name.is_empty()
                {
                    friends.push(name);
                }
            }
            _ => {}
        }
    }

    let mut access_sections = Vec::new();
    if has_public {
        access_sections.push("has_public_members".to_string());
    }
    if has_protected {
        access_sections.push("has_protected_members".to_string());
    }
    if has_private {
        access_sections.push("has_private_members".to_string());
    }
    for friend in &friends {
        access_sections.push(format!("friend:{friend}"));
    }
    if has_override {
        access_sections.push("has_override".to_string());
    }
    if has_final_methods {
        access_sections.push("has_final_methods".to_string());
    }
    if has_deleted_members {
        access_sections.push("has_deleted_members".to_string());
    }
    if has_defaulted_members {
        access_sections.push("has_defaulted_members".to_string());
    }

    (methods, fields, has_pure_virtual, access_sections)
}

/// Detect method qualifiers (override, final, = delete, = default) on a node.
fn detect_method_qualifiers<D: ast_grep_core::Doc>(
    node: &Node<D>,
    has_override: &mut bool,
    has_final_methods: &mut bool,
    has_deleted_members: &mut bool,
    has_defaulted_members: &mut bool,
) {
    for child in node.children() {
        match child.kind().as_ref() {
            "virtual_specifier" => {
                let text = child.text();
                match text.as_ref() {
                    "override" => *has_override = true,
                    "final" => *has_final_methods = true,
                    _ => {}
                }
            }
            "default_method_clause" => *has_defaulted_members = true,
            "delete_method_clause" => *has_deleted_members = true,
            "function_declarator" => {
                // Recurse into function_declarator to find qualifiers
                for fc in child.children() {
                    if fc.kind().as_ref() == "virtual_specifier" {
                        let text = fc.text();
                        match text.as_ref() {
                            "override" => *has_override = true,
                            "final" => *has_final_methods = true,
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

pub(super) fn extract_method_name<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<String> {
    let children: Vec<_> = node.children().collect();

    // Check for destructor
    for child in &children {
        if child.kind().as_ref() == "function_declarator" {
            let dc: Vec<_> = child.children().collect();
            for d in &dc {
                if d.kind().as_ref() == "destructor_name" {
                    return Some(d.text().to_string());
                }
                if d.kind().as_ref() == "operator_name" {
                    return Some(d.text().to_string());
                }
                if d.kind().as_ref() == "field_identifier" {
                    return Some(d.text().to_string());
                }
                if d.kind().as_ref() == "identifier" {
                    return Some(d.text().to_string());
                }
            }
        }
        // operator_cast: `operator float() const`
        if child.kind().as_ref() == "operator_cast" {
            return Some(child.text().split('(').next()?.trim().to_string());
        }
        // reference_declarator wrapping function_declarator (e.g., `Vec2& operator++()`)
        if child.kind().as_ref() == "reference_declarator" {
            let rc: Vec<_> = child.children().collect();
            for r in &rc {
                if r.kind().as_ref() == "function_declarator" {
                    let fc: Vec<_> = r.children().collect();
                    for f in &fc {
                        if f.kind().as_ref() == "operator_name" {
                            return Some(f.text().to_string());
                        }
                        if f.kind().as_ref() == "field_identifier" {
                            return Some(f.text().to_string());
                        }
                    }
                }
            }
        }
    }
    None
}

fn extract_field_decl_method_name<D: ast_grep_core::Doc>(children: &[Node<D>]) -> Option<String> {
    for child in children {
        if child.kind().as_ref() == "function_declarator" {
            let dc: Vec<_> = child.children().collect();
            for d in &dc {
                if d.kind().as_ref() == "field_identifier"
                    || d.kind().as_ref() == "identifier"
                    || d.kind().as_ref() == "operator_name"
                    || d.kind().as_ref() == "destructor_name"
                {
                    return Some(d.text().to_string());
                }
            }
        }
    }
    None
}
