use ast_grep_core::Node;

use crate::extractors::helpers;
use crate::types::{RustMetadataExt, SymbolKind, SymbolMetadata};

pub(super) fn build_function_metadata<D: ast_grep_core::Doc>(
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

pub(super) fn build_macro_metadata<D: ast_grep_core::Doc>(
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
