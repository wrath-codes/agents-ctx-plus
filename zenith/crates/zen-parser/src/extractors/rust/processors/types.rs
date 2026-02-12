use ast_grep_core::Node;

use crate::extractors::helpers;
use crate::types::{SymbolKind, SymbolMetadata};

pub(super) fn build_struct_metadata<D: ast_grep_core::Doc>(
    node: &Node<D>,
    source: &str,
    name: &str,
) -> (SymbolKind, SymbolMetadata) {
    let attrs = helpers::extract_attributes(node);
    let generics = helpers::extract_generics(node);
    let fields = extract_struct_fields(node);
    let is_error =
        helpers::is_error_type_by_name(name) || attrs.iter().any(|a| a.contains("Error"));
    let doc = helpers::extract_doc_comments_rust(node, source);
    let doc_sections = helpers::parse_rust_doc_sections(&doc);

    let kind = if node.kind().as_ref() == "union_item" {
        SymbolKind::Union
    } else {
        SymbolKind::Struct
    };

    (
        kind,
        SymbolMetadata {
            generics: generics.clone(),
            attributes: attrs,
            lifetimes: helpers::extract_lifetimes(generics.as_deref()),
            where_clause: helpers::extract_where_clause(node),
            fields,
            is_error_type: is_error,
            doc_sections,
            ..Default::default()
        },
    )
}

pub(super) fn build_enum_metadata<D: ast_grep_core::Doc>(
    node: &Node<D>,
    source: &str,
    name: &str,
) -> (SymbolKind, SymbolMetadata) {
    let attrs = helpers::extract_attributes(node);
    let generics = helpers::extract_generics(node);
    let variants = extract_enum_variants(node);
    let is_error =
        helpers::is_error_type_by_name(name) || attrs.iter().any(|a| a.contains("Error"));
    let doc = helpers::extract_doc_comments_rust(node, source);
    let doc_sections = helpers::parse_rust_doc_sections(&doc);

    (
        SymbolKind::Enum,
        SymbolMetadata {
            generics: generics.clone(),
            attributes: attrs,
            lifetimes: helpers::extract_lifetimes(generics.as_deref()),
            variants,
            is_error_type: is_error,
            doc_sections,
            ..Default::default()
        },
    )
}

pub(super) fn build_trait_metadata<D: ast_grep_core::Doc>(
    node: &Node<D>,
    source: &str,
) -> (SymbolKind, SymbolMetadata) {
    let attrs = helpers::extract_attributes(node);
    let generics = helpers::extract_generics(node);
    let (methods, associated_types) = extract_trait_members(node);
    let doc = helpers::extract_doc_comments_rust(node, source);
    let doc_sections = helpers::parse_rust_doc_sections(&doc);

    // Detect `unsafe trait`
    let is_unsafe = node.children().any(|c| c.kind().as_ref() == "unsafe");

    // Extract supertraits from `trait_bounds` child
    let supertraits = extract_supertraits(node);

    (
        SymbolKind::Trait,
        SymbolMetadata {
            is_unsafe,
            generics: generics.clone(),
            attributes: attrs,
            lifetimes: helpers::extract_lifetimes(generics.as_deref()),
            where_clause: helpers::extract_where_clause(node),
            methods,
            associated_types,
            base_classes: supertraits,
            doc_sections,
            ..Default::default()
        },
    )
}

pub(super) fn build_type_alias_metadata<D: ast_grep_core::Doc>(
    node: &Node<D>,
) -> (SymbolKind, SymbolMetadata) {
    let generics = helpers::extract_generics(node);
    (
        SymbolKind::TypeAlias,
        SymbolMetadata {
            generics,
            ..Default::default()
        },
    )
}

pub(super) fn build_const_metadata<D: ast_grep_core::Doc>(
    node: &Node<D>,
) -> (SymbolKind, SymbolMetadata) {
    let return_type =
        helpers::extract_return_type(node).or_else(|| helpers::extract_type_annotation(node));
    (
        SymbolKind::Const,
        SymbolMetadata {
            return_type,
            ..Default::default()
        },
    )
}

pub(super) fn build_static_metadata<D: ast_grep_core::Doc>(
    node: &Node<D>,
) -> (SymbolKind, SymbolMetadata) {
    let return_type =
        helpers::extract_return_type(node).or_else(|| helpers::extract_type_annotation(node));
    let (_, is_unsafe, _, _) = helpers::detect_modifiers(node);
    (
        SymbolKind::Static,
        SymbolMetadata {
            is_unsafe,
            return_type,
            ..Default::default()
        },
    )
}

// ── Field / variant / member extraction ────────────────────────────

fn extract_struct_fields<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    if let Some(body) = node.field("body") {
        let body_kind = body.kind();
        if body_kind.as_ref() == "ordered_field_declaration_list" {
            // Tuple struct fields
            let mut fields = Vec::new();
            let mut idx = 0u32;
            for fc in body.children() {
                let k = fc.kind();
                let kr = k.as_ref();
                if kr == "(" || kr == ")" || kr == "," || kr == "visibility_modifier" {
                    continue;
                }
                fields.push(format!("{idx}: {}", fc.text()));
                idx += 1;
            }
            return fields;
        }
        // Named struct fields (field_declaration_list)
        return body
            .children()
            .filter(|c| c.kind().as_ref() == "field_declaration")
            .filter_map(|c| {
                c.field("name").map(|n| {
                    let name = n.text().to_string();
                    let ty = c
                        .field("type")
                        .map(|t| format!(": {}", t.text()))
                        .unwrap_or_default();
                    format!("{name}{ty}")
                })
            })
            .collect();
    }
    Vec::new()
}

fn extract_enum_variants<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    let Some(body) = node.field("body") else {
        return Vec::new();
    };
    body.children()
        .filter(|c| c.kind().as_ref() == "enum_variant")
        .filter_map(|c| {
            let name = c.field("name").map(|n| n.text().to_string())?;
            // Check for payload: tuple (ordered_field_declaration_list) or struct (field_declaration_list)
            let payload = c
                .children()
                .find(|ch| {
                    let k = ch.kind();
                    k.as_ref() == "ordered_field_declaration_list"
                        || k.as_ref() == "field_declaration_list"
                })
                .map(|ch| ch.text().to_string());
            match payload {
                Some(p) => Some(format!("{name}{p}")),
                None => Some(name),
            }
        })
        .collect()
}

fn extract_trait_members<D: ast_grep_core::Doc>(node: &Node<D>) -> (Vec<String>, Vec<String>) {
    let mut methods = Vec::new();
    let mut associated_types = Vec::new();

    let Some(body) = node.field("body") else {
        return (methods, associated_types);
    };

    for child in body.children() {
        let k = child.kind();
        match k.as_ref() {
            "function_signature_item" | "function_item" => {
                if let Some(name) = child.field("name") {
                    methods.push(name.text().to_string());
                }
            }
            "associated_type" => {
                let name = child
                    .field("name")
                    .map(|n| n.text().to_string())
                    .unwrap_or_default();
                // Include GAT type parameters
                let tp = child
                    .children()
                    .find(|c| c.kind().as_ref() == "type_parameters")
                    .map(|c| c.text().to_string())
                    .unwrap_or_default();
                associated_types.push(format!("{name}{tp}"));
            }
            "const_item" => {
                if let Some(name) = child
                    .field("name")
                    .or_else(|| child.children().find(|c| c.kind().as_ref() == "identifier"))
                {
                    let ty = helpers::extract_type_annotation(&child)
                        .map(|t| format!(": {t}"))
                        .unwrap_or_default();
                    methods.push(format!("const {}{ty}", name.text()));
                }
            }
            _ => {}
        }
    }
    (methods, associated_types)
}

fn extract_supertraits<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    let mut supers = Vec::new();
    for child in node.children() {
        if child.kind().as_ref() == "trait_bounds" {
            for bound in child.children() {
                let k = bound.kind();
                let kr = k.as_ref();
                if kr == "type_identifier" || kr == "scoped_type_identifier" || kr == "generic_type"
                {
                    supers.push(bound.text().to_string());
                }
            }
        }
    }
    supers
}
