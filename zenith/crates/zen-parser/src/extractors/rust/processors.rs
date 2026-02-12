use ast_grep_core::Node;

use crate::extractors::helpers;
use crate::types::{ParsedItem, RustMetadataExt, SymbolKind, SymbolMetadata, Visibility};

/// Process one matched top-level-ish Rust item node.
pub(super) fn process_match_node<D: ast_grep_core::Doc>(
    node: &Node<D>,
    source: &str,
) -> Vec<ParsedItem> {
    let kind = node.kind();
    match kind.as_ref() {
        "impl_item" => process_impl_item(node, source),
        // Skip function_items that live inside impl/trait bodies —
        // they are already handled by process_impl_item / build_trait_metadata.
        "function_item" => {
            if !is_nested_in_body(node)
                && let Some(item) = process_rust_node(node, source)
            {
                vec![item]
            } else {
                Vec::new()
            }
        }
        "foreign_mod_item" => process_foreign_mod(node, source),
        "use_declaration" => process_use_declaration(node, source).into_iter().collect(),
        "extern_crate_declaration" => process_extern_crate(node, source).into_iter().collect(),
        "macro_invocation" => process_macro_invocation(node, source).into_iter().collect(),
        _ => process_rust_node(node, source).into_iter().collect(),
    }
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
