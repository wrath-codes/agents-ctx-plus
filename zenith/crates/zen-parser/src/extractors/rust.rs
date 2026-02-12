//! Rust rich extractor — `KindMatcher`-first strategy (spike 0.8 validated).
//!
//! Extracts functions, structs, enums, traits, impl blocks, type aliases,
//! modules, consts, statics, macros, and unions with full metadata.

use ast_grep_core::Node;
use ast_grep_core::matcher::KindMatcher;
use ast_grep_core::ops::Any;
use ast_grep_language::SupportLang;

use super::helpers;
use crate::types::{ParsedItem, RustMetadataExt, SymbolKind, SymbolMetadata, Visibility};

const RUST_ITEM_KINDS: &[&str] = &[
    "function_item",
    "struct_item",
    "enum_item",
    "trait_item",
    "impl_item",
    "type_item",
    "mod_item",
    "const_item",
    "static_item",
    "macro_definition",
    "union_item",
    "foreign_mod_item",
    "use_declaration",
    "extern_crate_declaration",
    "macro_invocation",
];

/// Extract all API symbols from a Rust source file.
///
/// # Errors
/// Returns `ParserError` if parsing fails.
pub fn extract<D: ast_grep_core::Doc<Lang = SupportLang>>(
    root: &ast_grep_core::AstGrep<D>,
    source: &str,
) -> Result<Vec<ParsedItem>, crate::error::ParserError> {
    let mut items = Vec::new();
    let matchers: Vec<KindMatcher> = RUST_ITEM_KINDS
        .iter()
        .map(|k| KindMatcher::new(k, SupportLang::Rust))
        .collect();
    let matcher = Any::new(matchers);

    for node in root.root().find_all(&matcher) {
        let kind = node.kind();
        match kind.as_ref() {
            "impl_item" => items.extend(process_impl_item(&node, source)),
            // Skip function_items that live inside impl/trait bodies —
            // they are already handled by process_impl_item / build_trait_metadata.
            "function_item" => {
                if !is_nested_in_body(&node)
                    && let Some(item) = process_rust_node(&node, source)
                {
                    items.push(item);
                }
            }
            "foreign_mod_item" => items.extend(process_foreign_mod(&node, source)),
            "use_declaration" => {
                if let Some(item) = process_use_declaration(&node, source) {
                    items.push(item);
                }
            }
            "extern_crate_declaration" => {
                if let Some(item) = process_extern_crate(&node, source) {
                    items.push(item);
                }
            }
            "macro_invocation" => {
                if let Some(item) = process_macro_invocation(&node, source) {
                    items.push(item);
                }
            }
            _ => {
                if let Some(item) = process_rust_node(&node, source) {
                    items.push(item);
                }
            }
        }
    }
    Ok(items)
}

fn process_rust_node<D: ast_grep_core::Doc>(node: &Node<D>, source: &str) -> Option<ParsedItem> {
    let kind_str = node.kind();
    let k = kind_str.as_ref();

    let name = extract_name(node)?;
    let (symbol_kind, metadata) = build_metadata(node, k, source, &name);

    Some(ParsedItem {
        kind: symbol_kind,
        name,
        signature: helpers::extract_signature(node),
        source: helpers::extract_source(node, 50),
        doc_comment: helpers::extract_doc_comments_rust(node, source),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: helpers::extract_visibility_rust(node),
        metadata,
    })
}

fn extract_name<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<String> {
    node.field("name")
        .map(|n| n.text().to_string())
        .or_else(|| {
            node.children()
                .find(|c| {
                    let k = c.kind();
                    k.as_ref() == "identifier" || k.as_ref() == "type_identifier"
                })
                .map(|c| c.text().to_string())
        })
        .filter(|n| !n.is_empty())
}

/// Check if a node is nested inside a `declaration_list` (impl/trait body).
/// This prevents double-extraction of methods as free functions.
fn is_nested_in_body<D: ast_grep_core::Doc>(node: &Node<D>) -> bool {
    let mut current = node.parent();
    while let Some(parent) = current {
        let k = parent.kind();
        if k.as_ref() == "declaration_list" {
            return true;
        }
        // Stop at source_file level
        if k.as_ref() == "source_file" {
            return false;
        }
        current = parent.parent();
    }
    false
}

fn build_metadata<D: ast_grep_core::Doc>(
    node: &Node<D>,
    kind: &str,
    source: &str,
    name: &str,
) -> (SymbolKind, SymbolMetadata) {
    match kind {
        "function_item" => build_function_metadata(node, source, name),
        "struct_item" | "union_item" => build_struct_metadata(node, source, name),
        "enum_item" => build_enum_metadata(node, source, name),
        "trait_item" => build_trait_metadata(node, source),
        "type_item" => build_type_alias_metadata(node),
        "const_item" => build_const_metadata(node),
        "static_item" => build_static_metadata(node),
        "macro_definition" => build_macro_metadata(node, source),
        "mod_item" => (SymbolKind::Module, SymbolMetadata::default()),
        _ => (SymbolKind::Function, SymbolMetadata::default()),
    }
}

fn build_function_metadata<D: ast_grep_core::Doc>(
    node: &Node<D>,
    source: &str,
    name: &str,
) -> (SymbolKind, SymbolMetadata) {
    let (is_async, is_unsafe, is_const, abi) = helpers::detect_modifiers(node);
    let mut attrs = helpers::extract_attributes(node);
    let generics = helpers::extract_generics(node);
    let return_type = helpers::extract_return_type(node);
    let doc = helpers::extract_doc_comments_rust(node, source);
    let doc_sections = helpers::parse_rust_doc_sections(&doc);

    if is_const {
        attrs.push("const".to_string());
    }

    let mut metadata = SymbolMetadata {
        return_type: return_type.clone(),
        generics: generics.clone(),
        attributes: attrs.clone(),
        parameters: helpers::extract_parameters(node),
        lifetimes: helpers::extract_lifetimes(generics.as_deref()),
        where_clause: helpers::extract_where_clause(node),
        is_error_type: helpers::is_error_type_by_name(name),
        returns_result: helpers::returns_result(return_type.as_deref()),
        doc_sections,
        ..Default::default()
    };

    if is_async {
        metadata.mark_async();
    }
    if is_unsafe {
        metadata.mark_unsafe();
    }
    if let Some(abi) = abi {
        metadata.set_abi(abi);
    }
    if helpers::is_pyo3(&attrs) {
        metadata.mark_pyo3();
    }

    (SymbolKind::Function, metadata)
}

fn build_struct_metadata<D: ast_grep_core::Doc>(
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

fn build_enum_metadata<D: ast_grep_core::Doc>(
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

fn build_trait_metadata<D: ast_grep_core::Doc>(
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

fn build_type_alias_metadata<D: ast_grep_core::Doc>(
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

fn build_const_metadata<D: ast_grep_core::Doc>(node: &Node<D>) -> (SymbolKind, SymbolMetadata) {
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

fn build_static_metadata<D: ast_grep_core::Doc>(node: &Node<D>) -> (SymbolKind, SymbolMetadata) {
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

// ── impl block processing ──────────────────────────────────────────

fn process_impl_item<D: ast_grep_core::Doc>(node: &Node<D>, source: &str) -> Vec<ParsedItem> {
    let (trait_name, for_type) = extract_impl_targets(node);
    let is_unsafe_impl = node.children().any(|c| c.kind().as_ref() == "unsafe");
    let is_negative = node.children().any(|c| c.kind().as_ref() == "!");

    let mut items = Vec::new();
    let Some(body) = node.field("body") else {
        return items;
    };

    for child in body.children() {
        let k = child.kind();
        match k.as_ref() {
            "function_item" => {
                if let Some(mut method) =
                    process_impl_method(&child, source, trait_name.as_deref(), for_type.as_deref())
                {
                    if is_unsafe_impl {
                        method.metadata.mark_unsafe();
                    }
                    items.push(method);
                }
            }
            "const_item" => {
                if let Some(item) = process_impl_assoc_const(
                    &child,
                    source,
                    trait_name.as_deref(),
                    for_type.as_deref(),
                ) {
                    items.push(item);
                }
            }
            "type_item" => {
                if let Some(item) = process_impl_assoc_type(
                    &child,
                    source,
                    trait_name.as_deref(),
                    for_type.as_deref(),
                ) {
                    items.push(item);
                }
            }
            _ => {}
        }
    }

    // For negative impls with no body items, emit a marker
    if is_negative
        && items.is_empty()
        && let (Some(trait_n), Some(for_t)) = (&trait_name, &for_type)
    {
        items.push(ParsedItem {
            kind: SymbolKind::Trait,
            name: format!("!{trait_n}"),
            signature: helpers::extract_signature(node),
            source: helpers::extract_source(node, 10),
            doc_comment: String::new(),
            start_line: node.start_pos().line() as u32 + 1,
            end_line: node.end_pos().line() as u32 + 1,
            visibility: Visibility::Private,
            metadata: SymbolMetadata {
                trait_name: Some(format!("!{trait_n}")),
                for_type: Some(for_t.clone()),
                attributes: vec!["negative_impl".to_string()],
                ..Default::default()
            },
        });
    }
    items
}

fn extract_impl_targets<D: ast_grep_core::Doc>(node: &Node<D>) -> (Option<String>, Option<String>) {
    let mut trait_name = None;
    let mut for_type = None;

    // Walk children to find `type_identifier` and `for` keyword structure.
    // In `impl Trait for Type`, tree-sitter produces:
    //   impl, type_identifier(Trait), for, type_identifier(Type), declaration_list
    // In `impl Type`, tree-sitter produces:
    //   impl, type_identifier(Type), declaration_list
    let children: Vec<_> = node.children().collect();
    let mut found_for = false;
    for child in &children {
        let k = child.kind();
        if k.as_ref() == "for" {
            found_for = true;
        }
    }

    if found_for {
        // trait impl: first type-like child is trait, after `for` is the type
        let mut past_for = false;
        for child in &children {
            let k = child.kind();
            if k.as_ref() == "for" {
                past_for = true;
                continue;
            }
            if is_type_node(k.as_ref()) {
                if past_for {
                    for_type = Some(child.text().to_string());
                } else if trait_name.is_none() {
                    trait_name = Some(child.text().to_string());
                }
            }
        }
    } else {
        // inherent impl: first type-like child is the type
        for child in &children {
            if is_type_node(child.kind().as_ref()) {
                for_type = Some(child.text().to_string());
                break;
            }
        }
    }

    (trait_name, for_type)
}

fn is_type_node(kind: &str) -> bool {
    matches!(
        kind,
        "type_identifier"
            | "scoped_type_identifier"
            | "generic_type"
            | "scoped_identifier"
            | "reference_type"
            | "tuple_type"
            | "array_type"
            | "pointer_type"
            | "function_type"
            | "primitive_type"
            | "unit_type"
            | "abstract_type"
            | "dynamic_type"
            | "bounded_type"
            | "macro_invocation"
            | "never_type"
    )
}

fn process_impl_method<D: ast_grep_core::Doc>(
    child: &Node<D>,
    source: &str,
    trait_name: Option<&str>,
    for_type: Option<&str>,
) -> Option<ParsedItem> {
    let name = child
        .field("name")
        .map(|n| n.text().to_string())
        .filter(|n| !n.is_empty())?;

    let (is_async, is_unsafe, _is_const, _abi) = helpers::detect_modifiers(child);
    let attrs = helpers::extract_attributes(child);
    let generics = helpers::extract_generics(child);
    let return_type = helpers::extract_return_type(child);
    let doc = helpers::extract_doc_comments_rust(child, source);
    let doc_sections = helpers::parse_rust_doc_sections(&doc);

    Some(ParsedItem {
        kind: SymbolKind::Method,
        name,
        signature: helpers::extract_signature(child),
        source: helpers::extract_source(child, 50),
        doc_comment: doc,
        start_line: child.start_pos().line() as u32 + 1,
        end_line: child.end_pos().line() as u32 + 1,
        visibility: helpers::extract_visibility_rust(child),
        metadata: SymbolMetadata {
            is_async,
            is_unsafe,
            return_type: return_type.clone(),
            generics: generics.clone(),
            attributes: attrs.clone(),
            parameters: helpers::extract_parameters(child),
            lifetimes: helpers::extract_lifetimes(generics.as_deref()),
            where_clause: helpers::extract_where_clause(child),
            trait_name: trait_name.map(String::from),
            for_type: for_type.map(String::from),
            is_pyo3: helpers::is_pyo3(&attrs),
            returns_result: helpers::returns_result(return_type.as_deref()),
            doc_sections,
            ..Default::default()
        },
    })
}

// ── member extraction ──────────────────────────────────────────────

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

// ── Supertrait extraction ──────────────────────────────────────────

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

// ── Macro metadata ─────────────────────────────────────────────────

fn build_macro_metadata<D: ast_grep_core::Doc>(
    node: &Node<D>,
    source: &str,
) -> (SymbolKind, SymbolMetadata) {
    let attrs = helpers::extract_attributes(node);
    let doc = helpers::extract_doc_comments_rust(node, source);
    let doc_sections = helpers::parse_rust_doc_sections(&doc);
    let is_exported = attrs.iter().any(|a| a == "macro_export");
    let mut final_attrs = attrs;
    if is_exported {
        final_attrs.push("exported".to_string());
    }
    (
        SymbolKind::Macro,
        SymbolMetadata {
            attributes: final_attrs,
            is_exported,
            doc_sections,
            ..Default::default()
        },
    )
}

// ── Associated const/type in impl blocks ───────────────────────────

fn process_impl_assoc_const<D: ast_grep_core::Doc>(
    node: &Node<D>,
    source: &str,
    trait_name: Option<&str>,
    for_type: Option<&str>,
) -> Option<ParsedItem> {
    let name = node
        .field("name")
        .or_else(|| node.children().find(|c| c.kind().as_ref() == "identifier"))
        .map(|n| n.text().to_string())
        .filter(|n| !n.is_empty())?;

    let return_type =
        helpers::extract_return_type(node).or_else(|| helpers::extract_type_annotation(node));

    Some(ParsedItem {
        kind: SymbolKind::Const,
        name,
        signature: helpers::extract_signature(node),
        source: helpers::extract_source(node, 10),
        doc_comment: helpers::extract_doc_comments_rust(node, source),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: helpers::extract_visibility_rust(node),
        metadata: SymbolMetadata {
            return_type,
            trait_name: trait_name.map(String::from),
            for_type: for_type.map(String::from),
            ..Default::default()
        },
    })
}

fn process_impl_assoc_type<D: ast_grep_core::Doc>(
    node: &Node<D>,
    source: &str,
    trait_name: Option<&str>,
    for_type: Option<&str>,
) -> Option<ParsedItem> {
    let name = node
        .field("name")
        .or_else(|| {
            node.children()
                .find(|c| c.kind().as_ref() == "type_identifier")
        })
        .map(|n| n.text().to_string())
        .filter(|n| !n.is_empty())?;

    Some(ParsedItem {
        kind: SymbolKind::TypeAlias,
        name,
        signature: helpers::extract_signature(node),
        source: helpers::extract_source(node, 10),
        doc_comment: helpers::extract_doc_comments_rust(node, source),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: helpers::extract_visibility_rust(node),
        metadata: SymbolMetadata {
            trait_name: trait_name.map(String::from),
            for_type: for_type.map(String::from),
            ..Default::default()
        },
    })
}

// ── Foreign module (extern block) processing ───────────────────────

fn process_foreign_mod<D: ast_grep_core::Doc>(node: &Node<D>, source: &str) -> Vec<ParsedItem> {
    let mut items = Vec::new();

    // Extract ABI from extern_modifier
    let abi = node
        .children()
        .find(|c| c.kind().as_ref() == "extern_modifier")
        .and_then(|em| {
            em.children()
                .find(|c| c.kind().as_ref() == "string_literal")
                .map(|s| s.text().to_string().trim_matches('"').to_string())
        })
        .unwrap_or_else(|| "C".to_string());

    let Some(body) = node
        .children()
        .find(|c| c.kind().as_ref() == "declaration_list")
    else {
        return items;
    };

    for child in body.children() {
        let k = child.kind();
        match k.as_ref() {
            "function_signature_item" => {
                let name = child
                    .field("name")
                    .or_else(|| child.children().find(|c| c.kind().as_ref() == "identifier"))
                    .map(|n| n.text().to_string());
                if let Some(name) = name {
                    items.push(ParsedItem {
                        kind: SymbolKind::Function,
                        name,
                        signature: helpers::extract_signature(&child),
                        source: helpers::extract_source(&child, 10),
                        doc_comment: helpers::extract_doc_comments_rust(&child, source),
                        start_line: child.start_pos().line() as u32 + 1,
                        end_line: child.end_pos().line() as u32 + 1,
                        visibility: Visibility::Public,
                        metadata: SymbolMetadata {
                            abi: Some(abi.clone()),
                            parameters: helpers::extract_parameters(&child),
                            return_type: helpers::extract_return_type(&child),
                            attributes: vec!["extern".to_string()],
                            ..Default::default()
                        },
                    });
                }
            }
            "static_item" => {
                let name = child
                    .field("name")
                    .or_else(|| child.children().find(|c| c.kind().as_ref() == "identifier"))
                    .map(|n| n.text().to_string());
                if let Some(name) = name {
                    items.push(ParsedItem {
                        kind: SymbolKind::Static,
                        name,
                        signature: helpers::extract_signature(&child),
                        source: helpers::extract_source(&child, 10),
                        doc_comment: helpers::extract_doc_comments_rust(&child, source),
                        start_line: child.start_pos().line() as u32 + 1,
                        end_line: child.end_pos().line() as u32 + 1,
                        visibility: Visibility::Public,
                        metadata: SymbolMetadata {
                            abi: Some(abi.clone()),
                            return_type: helpers::extract_type_annotation(&child),
                            attributes: vec!["extern".to_string()],
                            ..Default::default()
                        },
                    });
                }
            }
            _ => {}
        }
    }
    items
}

// ── use declaration processing ─────────────────────────────────────

fn process_use_declaration<D: ast_grep_core::Doc>(
    node: &Node<D>,
    source: &str,
) -> Option<ParsedItem> {
    let vis = helpers::extract_visibility_rust(node);
    // Extract the use path — all children after `use` and before `;`
    let path = node
        .children()
        .filter(|c| {
            let k = c.kind();
            k.as_ref() != "use" && k.as_ref() != ";" && k.as_ref() != "visibility_modifier"
        })
        .map(|c| c.text().to_string())
        .collect::<String>();

    if path.is_empty() {
        return None;
    }

    // Derive name from last segment
    let name = path
        .rsplit("::")
        .next()
        .unwrap_or(&path)
        .trim_matches(|c: char| c == '{' || c == '}' || c == ' ')
        .to_string();

    let is_reexport = vis == Visibility::Public || vis == Visibility::PublicCrate;

    Some(ParsedItem {
        kind: SymbolKind::Module,
        name,
        signature: helpers::extract_signature(node),
        source: None,
        doc_comment: helpers::extract_doc_comments_rust(node, source),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: vis,
        metadata: SymbolMetadata {
            attributes: if is_reexport {
                vec!["use".to_string(), "reexport".to_string()]
            } else {
                vec!["use".to_string()]
            },
            ..Default::default()
        },
    })
}

// ── extern crate processing ────────────────────────────────────────

fn process_extern_crate<D: ast_grep_core::Doc>(node: &Node<D>, source: &str) -> Option<ParsedItem> {
    let name = node
        .children()
        .find(|c| c.kind().as_ref() == "identifier")
        .map(|c| c.text().to_string())
        .filter(|n| !n.is_empty())?;

    Some(ParsedItem {
        kind: SymbolKind::Module,
        name,
        signature: helpers::extract_signature(node),
        source: None,
        doc_comment: helpers::extract_doc_comments_rust(node, source),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: helpers::extract_visibility_rust(node),
        metadata: SymbolMetadata {
            attributes: vec!["extern_crate".to_string()],
            ..Default::default()
        },
    })
}

// ── Item-position macro invocation processing ──────────────────────

fn process_macro_invocation<D: ast_grep_core::Doc>(
    node: &Node<D>,
    source: &str,
) -> Option<ParsedItem> {
    let name = node
        .children()
        .find(|c| {
            let k = c.kind();
            k.as_ref() == "identifier" || k.as_ref() == "scoped_identifier"
        })
        .map(|c| c.text().to_string())
        .filter(|n| !n.is_empty())?;

    Some(ParsedItem {
        kind: SymbolKind::Macro,
        name,
        signature: helpers::extract_signature(node),
        source: helpers::extract_source(node, 10),
        doc_comment: helpers::extract_doc_comments_rust(node, source),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Private,
        metadata: SymbolMetadata {
            attributes: vec!["macro_invocation".to_string()],
            ..Default::default()
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Visibility;
    use ast_grep_language::LanguageExt;
    use pretty_assertions::assert_eq;

    fn parse_and_extract(source: &str) -> Vec<ParsedItem> {
        let root = SupportLang::Rust.ast_grep(source);
        extract(&root, source).expect("extraction should succeed")
    }

    fn find_by_name<'a>(items: &'a [ParsedItem], name: &str) -> &'a ParsedItem {
        items
            .iter()
            .find(|i| i.name == name)
            .unwrap_or_else(|| panic!("no item named '{name}' found"))
    }

    #[test]
    fn extract_from_fixture() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        let names: Vec<&str> = items.iter().map(|i| i.name.as_str()).collect();
        assert!(names.contains(&"process"), "missing 'process': {names:?}");
        assert!(
            names.contains(&"dangerous"),
            "missing 'dangerous': {names:?}"
        );
        assert!(names.contains(&"Config"), "missing 'Config': {names:?}");
        assert!(names.contains(&"Status"), "missing 'Status': {names:?}");
        assert!(names.contains(&"Handler"), "missing 'Handler': {names:?}");
        assert!(names.contains(&"MAX_SIZE"), "missing 'MAX_SIZE': {names:?}");
        assert!(names.contains(&"MyResult"), "missing 'MyResult': {names:?}");
    }

    #[test]
    fn async_function_detected() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        let process = find_by_name(&items, "process");
        assert_eq!(process.kind, SymbolKind::Function);
        assert!(process.metadata.is_async);
        assert!(!process.metadata.is_unsafe);
        assert_eq!(process.visibility, Visibility::Public);
    }

    #[test]
    fn unsafe_function_detected() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        let dangerous = find_by_name(&items, "dangerous");
        assert!(dangerous.metadata.is_unsafe);
        assert!(!dangerous.metadata.is_async);
        assert_eq!(dangerous.visibility, Visibility::Private);
    }

    #[test]
    fn doc_comments_extracted() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        let process = find_by_name(&items, "process");
        assert!(
            process.doc_comment.contains("documented async function"),
            "doc_comment: {:?}",
            process.doc_comment
        );
        assert!(
            process.doc_comment.contains("Second line"),
            "doc_comment: {:?}",
            process.doc_comment
        );
    }

    #[test]
    fn generics_extracted() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        let process = find_by_name(&items, "process");
        assert!(
            process.metadata.generics.is_some(),
            "generics should be Some"
        );
        let g = process.metadata.generics.as_deref().unwrap();
        assert!(g.contains("T"), "generics should contain T: {g}");
    }

    #[test]
    fn return_type_extracted() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        let process = find_by_name(&items, "process");
        assert!(process.metadata.returns_result);
        let rt = process.metadata.return_type.as_deref().unwrap();
        assert!(rt.contains("Result"), "return_type: {rt}");
    }

    #[test]
    fn struct_fields_extracted() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        let config = find_by_name(&items, "Config");
        assert_eq!(config.kind, SymbolKind::Struct);
        assert!(
            config.metadata.fields.len() >= 3,
            "fields: {:?}",
            config.metadata.fields
        );
    }

    #[test]
    fn struct_attributes_extracted() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        let config = find_by_name(&items, "Config");
        assert!(
            !config.metadata.attributes.is_empty(),
            "should have derive attributes"
        );
    }

    #[test]
    fn enum_variants_extracted() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        let status = find_by_name(&items, "Status");
        assert_eq!(status.kind, SymbolKind::Enum);
        assert!(
            status.metadata.variants.iter().any(|v| v == "Active"),
            "variants: {:?}",
            status.metadata.variants
        );
        assert!(
            status
                .metadata
                .variants
                .iter()
                .any(|v| v.starts_with("Inactive")),
            "variants: {:?}",
            status.metadata.variants
        );
    }

    #[test]
    fn trait_methods_extracted() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        let handler = find_by_name(&items, "Handler");
        assert_eq!(handler.kind, SymbolKind::Trait);
        assert!(
            handler.metadata.methods.contains(&"handle".to_string()),
            "methods: {:?}",
            handler.metadata.methods
        );
    }

    #[test]
    fn trait_associated_types_extracted() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        let handler = find_by_name(&items, "Handler");
        assert!(
            handler
                .metadata
                .associated_types
                .contains(&"Output".to_string()),
            "associated_types: {:?}",
            handler.metadata.associated_types
        );
    }

    #[test]
    fn impl_methods_as_separate_items() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        let methods: Vec<&ParsedItem> = items
            .iter()
            .filter(|i| i.kind == SymbolKind::Method)
            .collect();
        let method_names: Vec<&str> = methods.iter().map(|m| m.name.as_str()).collect();
        assert!(
            method_names.contains(&"new"),
            "should have 'new' method: {method_names:?}"
        );
        assert!(
            method_names.contains(&"handle"),
            "should have 'handle' method: {method_names:?}"
        );
    }

    #[test]
    fn trait_impl_methods_have_trait_name() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        let handle = items
            .iter()
            .find(|i| i.kind == SymbolKind::Method && i.name == "handle")
            .expect("should find handle method");
        assert!(
            handle.metadata.trait_name.is_some(),
            "trait impl method should have trait_name"
        );
        assert_eq!(
            handle.metadata.for_type.as_deref(),
            Some("Config"),
            "for_type should be Config"
        );
    }

    #[test]
    fn inherent_impl_methods_have_for_type_only() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        let new_method = items
            .iter()
            .find(|i| i.kind == SymbolKind::Method && i.name == "new")
            .expect("should find new method");
        assert!(
            new_method.metadata.trait_name.is_none(),
            "inherent impl should have no trait_name"
        );
        assert_eq!(
            new_method.metadata.for_type.as_deref(),
            Some("Config"),
            "for_type should be Config"
        );
    }

    #[test]
    fn const_item_extracted() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        let max = find_by_name(&items, "MAX_SIZE");
        assert_eq!(max.kind, SymbolKind::Const);
        assert_eq!(max.visibility, Visibility::Public);
    }

    #[test]
    fn type_alias_extracted() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        let my_result = find_by_name(&items, "MyResult");
        assert_eq!(my_result.kind, SymbolKind::TypeAlias);
    }

    #[test]
    fn signature_no_body_leak() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        for item in &items {
            if !item.signature.is_empty() {
                assert!(
                    !item.signature.contains('{'),
                    "signature for '{}' leaks body: {}",
                    item.name,
                    item.signature
                );
            }
        }
    }

    #[test]
    fn line_numbers_are_one_based() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        for item in &items {
            assert!(
                item.start_line >= 1,
                "'{}' start_line should be >= 1, got {}",
                item.name,
                item.start_line
            );
            assert!(
                item.end_line >= item.start_line,
                "'{}' end_line {} < start_line {}",
                item.name,
                item.end_line,
                item.start_line
            );
        }
    }

    // ── New fixture coverage tests ─────────────────────────────────

    #[test]
    fn lifetimes_extracted() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        let transform = find_by_name(&items, "transform");
        assert!(
            !transform.metadata.lifetimes.is_empty(),
            "lifetimes should be non-empty"
        );
        assert!(
            transform.metadata.lifetimes.contains(&"'a".to_string()),
            "should contain 'a: {:?}",
            transform.metadata.lifetimes
        );
        assert!(
            transform.metadata.lifetimes.contains(&"'b".to_string()),
            "should contain 'b: {:?}",
            transform.metadata.lifetimes
        );
    }

    #[test]
    fn where_clause_extracted() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        let transform = find_by_name(&items, "transform");
        assert!(
            transform.metadata.where_clause.is_some(),
            "should have where clause"
        );
        let wc = transform.metadata.where_clause.as_deref().unwrap();
        assert!(wc.contains("Clone"), "where clause: {wc}");
        assert!(wc.contains("Send"), "where clause: {wc}");
    }

    #[test]
    fn doc_sections_errors_extracted() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        let transform = find_by_name(&items, "transform");
        assert!(
            transform.metadata.doc_sections.errors.is_some(),
            "should have # Errors section"
        );
    }

    #[test]
    fn error_type_by_name_detected() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        let my_error = find_by_name(&items, "MyError");
        assert_eq!(my_error.kind, SymbolKind::Enum);
        assert!(
            my_error.metadata.is_error_type,
            "MyError should be detected as error type"
        );
    }

    #[test]
    fn error_enum_variants_extracted() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        let my_error = find_by_name(&items, "MyError");
        assert!(
            my_error
                .metadata
                .variants
                .iter()
                .any(|v| v.starts_with("Io")),
            "variants: {:?}",
            my_error.metadata.variants
        );
        assert!(
            my_error.metadata.variants.iter().any(|v| v == "NotFound"),
            "variants: {:?}",
            my_error.metadata.variants
        );
    }

    #[test]
    fn static_item_extracted() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        let global = find_by_name(&items, "GLOBAL_NAME");
        assert_eq!(global.kind, SymbolKind::Static);
    }

    #[test]
    fn module_extracted() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        let internal = find_by_name(&items, "internal");
        assert_eq!(internal.kind, SymbolKind::Module);
    }

    #[test]
    fn macro_definition_extracted() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        let getter = find_by_name(&items, "make_getter");
        assert_eq!(getter.kind, SymbolKind::Macro);
    }

    #[test]
    fn union_extracted() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        let raw = find_by_name(&items, "RawValue");
        assert_eq!(raw.kind, SymbolKind::Union);
    }

    #[test]
    fn from_impl_detected() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        let from_methods: Vec<&ParsedItem> = items
            .iter()
            .filter(|i| {
                i.kind == SymbolKind::Method
                    && i.metadata
                        .trait_name
                        .as_deref()
                        .is_some_and(|t| t.contains("From"))
            })
            .collect();
        assert!(
            from_methods.len() >= 2,
            "should find at least 2 From impls, found {}",
            from_methods.len()
        );
        let for_types: Vec<&str> = from_methods
            .iter()
            .filter_map(|m| m.metadata.for_type.as_deref())
            .collect();
        assert!(
            for_types.iter().all(|t| *t == "MyError"),
            "all From impls should be for MyError: {for_types:?}"
        );
    }

    #[test]
    fn from_impl_has_source_type_in_trait_name() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        let from_io: Option<&ParsedItem> = items.iter().find(|i| {
            i.kind == SymbolKind::Method
                && i.metadata
                    .trait_name
                    .as_deref()
                    .is_some_and(|t| t.contains("io::Error") || t.contains("io :: Error"))
        });
        assert!(from_io.is_some(), "should find From<std::io::Error> impl");
    }

    #[test]
    fn pyo3_function_detected() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        let py_add = find_by_name(&items, "py_add");
        assert!(py_add.metadata.is_pyo3, "py_add should be detected as PyO3");
    }

    // ── Extended fixture tests ─────────────────────────────────────

    #[test]
    fn const_fn_detected() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "const_add");
        assert_eq!(f.kind, SymbolKind::Function);
        assert!(
            f.metadata.attributes.contains(&"const".to_string()),
            "should have const attribute: {:?}",
            f.metadata.attributes
        );
    }

    #[test]
    fn extern_c_fn_detected() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "c_callback");
        assert_eq!(f.kind, SymbolKind::Function);
        assert_eq!(f.metadata.abi.as_deref(), Some("C"), "should have ABI 'C'");
    }

    #[test]
    fn unsafe_trait_detected() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        let t = find_by_name(&items, "ThreadSafe");
        assert_eq!(t.kind, SymbolKind::Trait);
        assert!(t.metadata.is_unsafe, "trait should be unsafe");
    }

    #[test]
    fn unsafe_impl_marks_methods() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        let verify_methods: Vec<&ParsedItem> = items
            .iter()
            .filter(|i| {
                i.kind == SymbolKind::Method
                    && i.name == "verify"
                    && i.metadata
                        .trait_name
                        .as_deref()
                        .is_some_and(|t| t == "ThreadSafe")
            })
            .collect();
        assert!(
            !verify_methods.is_empty(),
            "should find verify method from unsafe impl"
        );
        assert!(
            verify_methods[0].metadata.is_unsafe,
            "method in unsafe impl should be marked unsafe"
        );
    }

    #[test]
    fn supertraits_extracted() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        let validator = find_by_name(&items, "Validator");
        assert_eq!(validator.kind, SymbolKind::Trait);
        assert!(
            validator
                .metadata
                .base_classes
                .contains(&"Clone".to_string()),
            "supertraits: {:?}",
            validator.metadata.base_classes
        );
        assert!(
            validator
                .metadata
                .base_classes
                .contains(&"Send".to_string()),
            "supertraits: {:?}",
            validator.metadata.base_classes
        );
    }

    #[test]
    fn trait_constants_in_methods() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        let configurable = find_by_name(&items, "Configurable");
        assert_eq!(configurable.kind, SymbolKind::Trait);
        assert!(
            configurable
                .metadata
                .methods
                .iter()
                .any(|m| m.contains("MAX_ITEMS")),
            "methods should include const MAX_ITEMS: {:?}",
            configurable.metadata.methods
        );
    }

    #[test]
    fn trait_has_default_method() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        let configurable = find_by_name(&items, "Configurable");
        assert!(
            configurable.metadata.methods.contains(&"name".to_string()),
            "methods should include default method 'name': {:?}",
            configurable.metadata.methods
        );
    }

    #[test]
    fn gat_associated_type_has_params() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        let lending = find_by_name(&items, "Lending");
        assert_eq!(lending.kind, SymbolKind::Trait);
        assert!(
            lending
                .metadata
                .associated_types
                .iter()
                .any(|a| a.contains("Item") && a.contains("<")),
            "GAT should include type params: {:?}",
            lending.metadata.associated_types
        );
    }

    #[test]
    fn tuple_struct_fields() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        let point = find_by_name(&items, "Point");
        assert_eq!(point.kind, SymbolKind::Struct);
        assert!(
            !point.metadata.fields.is_empty(),
            "tuple struct should have fields"
        );
        assert!(
            point.metadata.fields.iter().any(|f| f.contains("f64")),
            "fields: {:?}",
            point.metadata.fields
        );
    }

    #[test]
    fn unit_struct_detected() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        let marker = find_by_name(&items, "Marker");
        assert_eq!(marker.kind, SymbolKind::Struct);
        assert!(
            marker.metadata.fields.is_empty(),
            "unit struct has no fields"
        );
    }

    #[test]
    fn enum_variant_payloads_captured() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        let msg = find_by_name(&items, "Message");
        assert_eq!(msg.kind, SymbolKind::Enum);
        assert!(
            msg.metadata.variants.iter().any(|v| v == "Quit"),
            "variants: {:?}",
            msg.metadata.variants
        );
        assert!(
            msg.metadata
                .variants
                .iter()
                .any(|v| v.starts_with("Move") && v.contains("x")),
            "Move variant should have struct payload: {:?}",
            msg.metadata.variants
        );
        assert!(
            msg.metadata
                .variants
                .iter()
                .any(|v| v.starts_with("Write") && v.contains("String")),
            "Write variant should have tuple payload: {:?}",
            msg.metadata.variants
        );
    }

    #[test]
    fn repr_c_in_attributes() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        let ffi = find_by_name(&items, "FfiPoint");
        assert_eq!(ffi.kind, SymbolKind::Struct);
        assert!(
            ffi.metadata.attributes.iter().any(|a| a.contains("repr")),
            "attributes: {:?}",
            ffi.metadata.attributes
        );
    }

    #[test]
    fn const_generics_struct() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        let buf = find_by_name(&items, "Buffer");
        assert_eq!(buf.kind, SymbolKind::Struct);
        assert!(
            buf.metadata
                .generics
                .as_deref()
                .is_some_and(|g| g.contains("const N")),
            "generics: {:?}",
            buf.metadata.generics
        );
    }

    #[test]
    fn extern_block_functions_extracted() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        let ext_func = find_by_name(&items, "external_func");
        assert_eq!(ext_func.kind, SymbolKind::Function);
        assert_eq!(ext_func.metadata.abi.as_deref(), Some("C"));
        assert!(
            ext_func.metadata.attributes.contains(&"extern".to_string()),
            "attrs: {:?}",
            ext_func.metadata.attributes
        );
    }

    #[test]
    fn extern_block_statics_extracted() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        let ext_var = find_by_name(&items, "EXTERNAL_VAR");
        assert_eq!(ext_var.kind, SymbolKind::Static);
        assert_eq!(ext_var.metadata.abi.as_deref(), Some("C"));
    }

    #[test]
    fn pub_use_reexport_extracted() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        let hashmap = find_by_name(&items, "HashMap");
        assert_eq!(hashmap.kind, SymbolKind::Module);
        assert_eq!(hashmap.visibility, Visibility::Public);
        assert!(
            hashmap
                .metadata
                .attributes
                .contains(&"reexport".to_string()),
            "attrs: {:?}",
            hashmap.metadata.attributes
        );
    }

    #[test]
    fn extern_crate_extracted() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        let alloc = find_by_name(&items, "alloc");
        assert_eq!(alloc.kind, SymbolKind::Module);
        assert!(
            alloc
                .metadata
                .attributes
                .contains(&"extern_crate".to_string()),
            "attrs: {:?}",
            alloc.metadata.attributes
        );
    }

    #[test]
    fn macro_invocation_extracted() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        let tl = find_by_name(&items, "thread_local");
        assert_eq!(tl.kind, SymbolKind::Macro);
        assert!(
            tl.metadata
                .attributes
                .contains(&"macro_invocation".to_string()),
            "attrs: {:?}",
            tl.metadata.attributes
        );
    }

    #[test]
    fn cfg_attribute_preserved() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "serde_only");
        assert!(
            f.metadata.attributes.iter().any(|a| a.starts_with("cfg(")),
            "should have cfg attr: {:?}",
            f.metadata.attributes
        );
    }

    #[test]
    fn deprecated_attribute_preserved() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "old_api");
        assert!(
            f.metadata
                .attributes
                .iter()
                .any(|a| a.starts_with("deprecated")),
            "should have deprecated attr: {:?}",
            f.metadata.attributes
        );
    }

    #[test]
    fn must_use_attribute_preserved() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "important_result");
        assert!(
            f.metadata.attributes.iter().any(|a| a == "must_use"),
            "should have must_use attr: {:?}",
            f.metadata.attributes
        );
    }

    #[test]
    fn doc_hidden_attribute_preserved() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "internal_only");
        assert!(
            f.metadata
                .attributes
                .iter()
                .any(|a| a.contains("doc(hidden)")),
            "should have doc(hidden) attr: {:?}",
            f.metadata.attributes
        );
    }

    #[test]
    fn block_doc_comment_extracted() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "block_documented");
        assert!(
            f.doc_comment.contains("Block documented"),
            "doc_comment: {:?}",
            f.doc_comment
        );
    }

    #[test]
    fn impl_trait_return_in_signature() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "make_iterator");
        assert!(
            f.metadata
                .return_type
                .as_deref()
                .is_some_and(|rt| rt.contains("impl") && rt.contains("Iterator")),
            "return_type: {:?}",
            f.metadata.return_type
        );
    }

    #[test]
    fn hrtb_in_where_clause() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "apply_fn");
        assert!(
            f.metadata
                .where_clause
                .as_deref()
                .is_some_and(|wc| wc.contains("for<'a>")),
            "where_clause: {:?}",
            f.metadata.where_clause
        );
    }

    #[test]
    fn pub_super_visibility() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "super_visible");
        assert_eq!(f.visibility, Visibility::Protected);
    }

    #[test]
    fn pub_in_path_visibility() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "path_visible");
        assert_eq!(f.visibility, Visibility::Protected);
    }

    #[test]
    fn impl_assoc_consts_extracted() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        let dc = items
            .iter()
            .find(|i| i.name == "DEFAULT_COUNT" && i.kind == SymbolKind::Const)
            .expect("should find DEFAULT_COUNT");
        assert_eq!(
            dc.metadata.for_type.as_deref(),
            Some("Config"),
            "for_type should be Config"
        );
        assert!(
            dc.metadata.return_type.is_some(),
            "should have a type: {:?}",
            dc.metadata.return_type
        );
    }

    #[test]
    fn impl_assoc_const_version() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        let v = items
            .iter()
            .find(|i| i.name == "VERSION" && i.kind == SymbolKind::Const)
            .expect("should find VERSION");
        assert_eq!(v.metadata.for_type.as_deref(), Some("Config"));
    }

    #[test]
    fn negative_impl_detected() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        let neg = items
            .iter()
            .find(|i| i.name.contains("!Send"))
            .expect("should find negative impl marker");
        assert!(
            neg.metadata
                .attributes
                .contains(&"negative_impl".to_string()),
            "attrs: {:?}",
            neg.metadata.attributes
        );
        assert_eq!(neg.metadata.for_type.as_deref(), Some("RawValue"));
    }

    #[test]
    fn receiver_forms_detected() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        let take = items
            .iter()
            .find(|i| {
                i.kind == SymbolKind::Method
                    && i.name == "take"
                    && i.metadata.for_type.as_deref() == Some("Receiver")
            })
            .expect("should find take method");
        assert!(
            take.metadata.parameters.iter().any(|p| p == "self"),
            "params: {:?}",
            take.metadata.parameters
        );

        let borrow = items
            .iter()
            .find(|i| {
                i.kind == SymbolKind::Method
                    && i.name == "borrow"
                    && i.metadata.for_type.as_deref() == Some("Receiver")
            })
            .expect("should find borrow method");
        assert!(
            borrow.metadata.parameters.iter().any(|p| p == "&self"),
            "params: {:?}",
            borrow.metadata.parameters
        );

        let mutate = items
            .iter()
            .find(|i| {
                i.kind == SymbolKind::Method
                    && i.name == "mutate"
                    && i.metadata.for_type.as_deref() == Some("Receiver")
            })
            .expect("should find mutate method");
        assert!(
            mutate.metadata.parameters.iter().any(|p| p == "&mut self"),
            "params: {:?}",
            mutate.metadata.parameters
        );
    }

    #[test]
    fn macro_export_detected() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        let m = find_by_name(&items, "exported_macro");
        assert_eq!(m.kind, SymbolKind::Macro);
        assert!(m.metadata.is_exported, "should be detected as exported");
        assert!(
            m.metadata.attributes.contains(&"macro_export".to_string()),
            "attrs: {:?}",
            m.metadata.attributes
        );
    }

    #[test]
    fn impl_assoc_type_extracted() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        let assoc = items
            .iter()
            .find(|i| {
                i.kind == SymbolKind::TypeAlias
                    && i.name == "Item"
                    && i.metadata.for_type.as_deref() == Some("Receiver")
            })
            .expect("should find associated type Item for Receiver");
        assert_eq!(assoc.metadata.trait_name.as_deref(), Some("Configurable"));
    }

    #[test]
    fn impl_for_reference_type() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        let fmt = items
            .iter()
            .find(|i| {
                i.kind == SymbolKind::Method
                    && i.name == "fmt"
                    && i.metadata
                        .for_type
                        .as_deref()
                        .is_some_and(|t| t.contains("&") && t.contains("RawValue"))
            })
            .expect("should find fmt for &RawValue");
        assert!(fmt.metadata.trait_name.is_some(), "should have trait_name");
    }

    #[test]
    fn const_item_has_type() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        let max = find_by_name(&items, "MAX_SIZE");
        assert!(
            max.metadata.return_type.is_some(),
            "const should have return_type: {:?}",
            max.metadata.return_type
        );
    }

    #[test]
    fn static_item_has_type() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        let global = find_by_name(&items, "GLOBAL_NAME");
        assert!(
            global.metadata.return_type.is_some(),
            "static should have return_type: {:?}",
            global.metadata.return_type
        );
    }

    #[test]
    fn no_duplicate_free_functions_from_impls() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        let free_fns: Vec<&ParsedItem> = items
            .iter()
            .filter(|i| i.kind == SymbolKind::Function && i.name == "handle")
            .collect();
        assert!(
            free_fns.is_empty(),
            "impl/trait methods should not appear as free functions: {:?}",
            free_fns.iter().map(|i| &i.name).collect::<Vec<_>>()
        );
    }

    #[test]
    fn no_duplicate_new_as_free_function() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        let free_new: Vec<&ParsedItem> = items
            .iter()
            .filter(|i| i.kind == SymbolKind::Function && i.name == "new")
            .collect();
        assert!(
            free_new.is_empty(),
            "impl method 'new' should not appear as free function"
        );
    }

    #[test]
    fn configurable_trait_assoc_type() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        let c = find_by_name(&items, "Configurable");
        assert!(
            c.metadata.associated_types.iter().any(|t| t == "Item"),
            "assoc types: {:?}",
            c.metadata.associated_types
        );
    }

    #[test]
    fn receiver_struct_detected() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        let r = find_by_name(&items, "Receiver");
        assert_eq!(r.kind, SymbolKind::Struct);
    }

    #[test]
    fn dyn_trait_param_in_signature() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "process_dyn");
        assert!(
            f.signature.contains("dyn"),
            "signature should contain dyn: {}",
            f.signature
        );
    }
}
