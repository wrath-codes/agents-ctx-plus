use ast_grep_core::Node;

use crate::extractors::helpers;
use crate::types::{GoMetadataExt, ParsedItem, SymbolKind, SymbolMetadata};

use super::go_helpers::{
    extract_go_doc, extract_go_method_parameters, extract_go_parameters, extract_go_receiver,
    extract_go_return_type, extract_go_type_parameters, extract_go_type_params_from_spec,
    extract_interface_methods, extract_struct_fields, go_visibility,
};

pub(super) fn process_function<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<ParsedItem> {
    let name = node
        .children()
        .find(|c| c.kind().as_ref() == "identifier")
        .map(|n| n.text().to_string())?;

    let doc = extract_go_doc(node);
    let return_type = extract_go_return_type(node);
    let parameters = extract_go_parameters(node);
    let type_params = extract_go_type_parameters(node);

    let mut metadata = SymbolMetadata::default();
    metadata.set_return_type(return_type);
    for parameter in parameters {
        metadata.push_parameter(parameter);
    }
    metadata.set_type_parameters(type_params);

    Some(ParsedItem {
        kind: SymbolKind::Function,
        name: name.clone(),
        signature: helpers::extract_signature(node),
        source: helpers::extract_source(node, 50),
        doc_comment: doc,
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: go_visibility(&name),
        metadata,
    })
}

// ── method_declaration ────────────────────────────────────────────

pub(super) fn process_method<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<ParsedItem> {
    // Method name is a field_identifier child
    let name = node
        .children()
        .find(|c| c.kind().as_ref() == "field_identifier")
        .map(|n| n.text().to_string())?;

    let doc = extract_go_doc(node);
    let return_type = extract_go_return_type(node);
    let parameters = extract_go_method_parameters(node);
    let receiver = extract_go_receiver(node);

    let mut metadata = SymbolMetadata::default();
    metadata.set_return_type(return_type);
    for parameter in parameters {
        metadata.push_parameter(parameter);
    }
    metadata.set_receiver(receiver);

    Some(ParsedItem {
        kind: SymbolKind::Method,
        name: name.clone(),
        signature: helpers::extract_signature(node),
        source: helpers::extract_source(node, 50),
        doc_comment: doc,
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: go_visibility(&name),
        metadata,
    })
}

// ── type_declaration ──────────────────────────────────────────────

pub(super) fn process_type_declaration<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<ParsedItem> {
    let mut items = Vec::new();
    let doc = extract_go_doc(node);

    for child in node.children() {
        let k = child.kind();
        match k.as_ref() {
            "type_spec" => {
                if let Some(item) = process_type_spec(&child, &doc) {
                    items.push(item);
                }
            }
            "type_alias" => {
                if let Some(item) = process_type_alias(&child, &doc) {
                    items.push(item);
                }
            }
            _ => {}
        }
    }
    items
}

pub(super) fn process_type_spec<D: ast_grep_core::Doc>(
    node: &Node<D>,
    doc: &str,
) -> Option<ParsedItem> {
    let name = node
        .children()
        .find(|c| c.kind().as_ref() == "type_identifier")
        .map(|n| n.text().to_string())?;

    // Determine the symbol kind based on the type body
    let (symbol_kind, metadata) = classify_type_spec(node, &name);

    Some(ParsedItem {
        kind: symbol_kind,
        name: name.clone(),
        signature: helpers::extract_signature(node),
        source: helpers::extract_source(node, 50),
        doc_comment: doc.to_string(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: go_visibility(&name),
        metadata,
    })
}

pub(super) fn classify_type_spec<D: ast_grep_core::Doc>(
    node: &Node<D>,
    name: &str,
) -> (SymbolKind, SymbolMetadata) {
    for child in node.children() {
        let k = child.kind();
        match k.as_ref() {
            "struct_type" => {
                let fields = extract_struct_fields(&child);
                let is_error = helpers::is_error_type_by_name(name);
                let mut metadata = SymbolMetadata::default();
                metadata.set_fields(fields);
                metadata.set_type_parameters(extract_go_type_params_from_spec(node));
                if is_error {
                    metadata.mark_error_type();
                }
                return (SymbolKind::Struct, metadata);
            }
            "interface_type" => {
                let methods = extract_interface_methods(&child);
                let mut metadata = SymbolMetadata::default();
                metadata.set_methods(methods);
                return (SymbolKind::Interface, metadata);
            }
            "function_type" => {
                return (SymbolKind::TypeAlias, SymbolMetadata::default());
            }
            _ => {}
        }
    }
    // Bare type (e.g., `type Direction int`)
    (SymbolKind::TypeAlias, SymbolMetadata::default())
}

pub(super) fn process_type_alias<D: ast_grep_core::Doc>(
    node: &Node<D>,
    doc: &str,
) -> Option<ParsedItem> {
    let name = node
        .children()
        .find(|c| c.kind().as_ref() == "type_identifier")
        .map(|n| n.text().to_string())?;

    Some(ParsedItem {
        kind: SymbolKind::TypeAlias,
        name: name.clone(),
        signature: helpers::extract_signature(node),
        source: helpers::extract_source(node, 50),
        doc_comment: doc.to_string(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: go_visibility(&name),
        metadata: SymbolMetadata::default(),
    })
}

// ── const_declaration ─────────────────────────────────────────────

pub(super) fn process_const_declaration<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<ParsedItem> {
    let mut items = Vec::new();
    let doc = extract_go_doc(node);

    for child in node.children() {
        if child.kind().as_ref() == "const_spec"
            && let Some(item) = process_const_spec(&child, &doc)
        {
            items.push(item);
        }
    }
    items
}

pub(super) fn process_const_spec<D: ast_grep_core::Doc>(
    node: &Node<D>,
    parent_doc: &str,
) -> Option<ParsedItem> {
    let name = node
        .children()
        .find(|c| c.kind().as_ref() == "identifier")
        .map(|n| n.text().to_string())?;

    // Use the const_spec's own doc comment if available, else parent's
    let own_doc = extract_go_doc(node);
    let doc = if own_doc.is_empty() {
        parent_doc.to_string()
    } else {
        own_doc
    };

    Some(ParsedItem {
        kind: SymbolKind::Const,
        name: name.clone(),
        signature: node.text().to_string(),
        source: Some(node.text().to_string()),
        doc_comment: doc,
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: go_visibility(&name),
        metadata: SymbolMetadata::default(),
    })
}

// ── var_declaration ───────────────────────────────────────────────

pub(super) fn process_var_declaration<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<ParsedItem> {
    let mut items = Vec::new();
    let doc = extract_go_doc(node);

    for child in node.children() {
        let k = child.kind();
        if k.as_ref() == "var_spec" {
            if let Some(item) = process_var_spec(&child, &doc) {
                items.push(item);
            }
        } else if k.as_ref() == "var_spec_list" {
            // var ( ... ) block
            for spec in child.children() {
                if spec.kind().as_ref() == "var_spec"
                    && let Some(item) = process_var_spec(&spec, &doc)
                {
                    items.push(item);
                }
            }
        }
    }
    items
}

pub(super) fn process_var_spec<D: ast_grep_core::Doc>(
    node: &Node<D>,
    parent_doc: &str,
) -> Option<ParsedItem> {
    let name = node
        .children()
        .find(|c| c.kind().as_ref() == "identifier")
        .map(|n| n.text().to_string())?;

    let own_doc = extract_go_doc(node);
    let doc = if own_doc.is_empty() {
        parent_doc.to_string()
    } else {
        own_doc
    };

    Some(ParsedItem {
        kind: SymbolKind::Static,
        name: name.clone(),
        signature: node.text().to_string(),
        source: Some(node.text().to_string()),
        doc_comment: doc,
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: go_visibility(&name),
        metadata: SymbolMetadata::default(),
    })
}
