//! Rust rich extractor — `KindMatcher`-first strategy (spike 0.8 validated).
//!
//! Extracts functions, structs, enums, traits, impl blocks, type aliases,
//! modules, consts, statics, macros, and unions with full metadata.

use ast_grep_core::matcher::KindMatcher;
use ast_grep_core::ops::Any;
use ast_grep_core::Node;
use ast_grep_language::SupportLang;

use super::helpers;
use crate::types::{ParsedItem, SymbolKind, SymbolMetadata};

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
            _ => {
                if let Some(item) = process_rust_node(&node, source) {
                    items.push(item);
                }
            }
        }
    }
    Ok(items)
}

fn process_rust_node<D: ast_grep_core::Doc>(
    node: &Node<D>,
    source: &str,
) -> Option<ParsedItem> {
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
        "macro_definition" => (SymbolKind::Macro, SymbolMetadata::default()),
        "mod_item" => (SymbolKind::Module, SymbolMetadata::default()),
        _ => (SymbolKind::Function, SymbolMetadata::default()),
    }
}

fn build_function_metadata<D: ast_grep_core::Doc>(
    node: &Node<D>,
    source: &str,
    name: &str,
) -> (SymbolKind, SymbolMetadata) {
    let (is_async, is_unsafe) = helpers::detect_modifiers(node);
    let attrs = helpers::extract_attributes(node);
    let generics = helpers::extract_generics(node);
    let return_type = helpers::extract_return_type(node);
    let doc = helpers::extract_doc_comments_rust(node, source);
    let doc_sections = helpers::parse_rust_doc_sections(&doc);

    (
        SymbolKind::Function,
        SymbolMetadata {
            is_async,
            is_unsafe,
            return_type: return_type.clone(),
            generics: generics.clone(),
            attributes: attrs.clone(),
            parameters: helpers::extract_parameters(node),
            lifetimes: helpers::extract_lifetimes(generics.as_deref()),
            where_clause: helpers::extract_where_clause(node),
            is_pyo3: helpers::is_pyo3(&attrs),
            is_error_type: helpers::is_error_type_by_name(name),
            returns_result: helpers::returns_result(return_type.as_deref()),
            doc_sections,
            ..Default::default()
        },
    )
}

fn build_struct_metadata<D: ast_grep_core::Doc>(
    node: &Node<D>,
    source: &str,
    name: &str,
) -> (SymbolKind, SymbolMetadata) {
    let attrs = helpers::extract_attributes(node);
    let generics = helpers::extract_generics(node);
    let fields = extract_struct_fields(node);
    let is_error = helpers::is_error_type_by_name(name)
        || attrs.iter().any(|a| a.contains("Error"));
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
    let is_error = helpers::is_error_type_by_name(name)
        || attrs.iter().any(|a| a.contains("Error"));
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

    (
        SymbolKind::Trait,
        SymbolMetadata {
            generics: generics.clone(),
            attributes: attrs,
            lifetimes: helpers::extract_lifetimes(generics.as_deref()),
            where_clause: helpers::extract_where_clause(node),
            methods,
            associated_types,
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

fn build_const_metadata<D: ast_grep_core::Doc>(
    node: &Node<D>,
) -> (SymbolKind, SymbolMetadata) {
    let return_type = helpers::extract_return_type(node);
    (
        SymbolKind::Const,
        SymbolMetadata {
            return_type,
            ..Default::default()
        },
    )
}

fn build_static_metadata<D: ast_grep_core::Doc>(
    node: &Node<D>,
) -> (SymbolKind, SymbolMetadata) {
    let return_type = helpers::extract_return_type(node);
    let (_, is_unsafe) = helpers::detect_modifiers(node);
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

fn process_impl_item<D: ast_grep_core::Doc>(
    node: &Node<D>,
    source: &str,
) -> Vec<ParsedItem> {
    let (trait_name, for_type) = extract_impl_targets(node);

    let mut methods = Vec::new();
    let Some(body) = node.field("body") else {
        return methods;
    };

    for child in body.children() {
        if child.kind().as_ref() == "function_item"
            && let Some(method) =
                process_impl_method(&child, source, trait_name.as_deref(), for_type.as_deref())
        {
            methods.push(method);
        }
    }
    methods
}

fn extract_impl_targets<D: ast_grep_core::Doc>(
    node: &Node<D>,
) -> (Option<String>, Option<String>) {
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

    let (is_async, is_unsafe) = helpers::detect_modifiers(child);
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
    let Some(body) = node.field("body") else {
        return Vec::new();
    };
    body.children()
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
        .collect()
}

fn extract_enum_variants<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    let Some(body) = node.field("body") else {
        return Vec::new();
    };
    body.children()
        .filter(|c| c.kind().as_ref() == "enum_variant")
        .filter_map(|c| {
            c.field("name").map(|n| n.text().to_string())
        })
        .collect()
}

fn extract_trait_members<D: ast_grep_core::Doc>(
    node: &Node<D>,
) -> (Vec<String>, Vec<String>) {
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
                if let Some(name) = child.field("name") {
                    associated_types.push(name.text().to_string());
                }
            }
            _ => {}
        }
    }
    (methods, associated_types)
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
        assert!(names.contains(&"dangerous"), "missing 'dangerous': {names:?}");
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
            status.metadata.variants.contains(&"Active".to_string()),
            "variants: {:?}",
            status.metadata.variants
        );
        assert!(
            status.metadata.variants.contains(&"Inactive".to_string()),
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
            handler.metadata.associated_types.contains(&"Output".to_string()),
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
            my_error.metadata.variants.contains(&"Io".to_string()),
            "variants: {:?}",
            my_error.metadata.variants
        );
        assert!(
            my_error.metadata.variants.contains(&"NotFound".to_string()),
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
        assert!(
            from_io.is_some(),
            "should find From<std::io::Error> impl"
        );
    }

    #[test]
    fn pyo3_function_detected() {
        let source = include_str!("../../tests/fixtures/sample.rs");
        let items = parse_and_extract(source);
        let py_add = find_by_name(&items, "py_add");
        assert!(
            py_add.metadata.is_pyo3,
            "py_add should be detected as PyO3"
        );
    }
}
