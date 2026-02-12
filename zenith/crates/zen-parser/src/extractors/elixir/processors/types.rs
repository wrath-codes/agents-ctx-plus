use ast_grep_core::Node;

use crate::extractors::helpers;
use crate::types::{ElixirMetadataExt, ParsedItem, SymbolKind, SymbolMetadata, Visibility};

use super::super::elixir_helpers::{
    extract_defstruct_fields, extract_module_methods, extract_module_name, extract_moduledoc,
};

const ELIXIR_TYPE_ATTRS: &[&str] = &["type", "typep", "opaque"];

// ── defprotocol ────────────────────────────────────────────────────

/// Process a `defprotocol` call.
pub fn process_defprotocol<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<ParsedItem> {
    let name = extract_module_name(node)?;
    let doc = extract_moduledoc(node);
    let methods = extract_module_methods(node);

    Some(ParsedItem {
        kind: SymbolKind::Interface,
        name,
        signature: helpers::extract_signature(node),
        source: helpers::extract_source(node, 50),
        doc_comment: doc,
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata: SymbolMetadata {
            methods,
            ..Default::default()
        },
    })
}

// ── defimpl ────────────────────────────────────────────────────────

/// Process a `defimpl` call.
pub fn process_defimpl<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<ParsedItem> {
    let name = extract_defimpl_name(node)?;
    let methods = extract_module_methods(node);

    Some(ParsedItem {
        kind: SymbolKind::Trait,
        name,
        signature: helpers::extract_signature(node),
        source: helpers::extract_source(node, 50),
        doc_comment: String::new(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata: SymbolMetadata {
            methods,
            ..Default::default()
        },
    })
}

/// Extract `defimpl Protocol, for: Type` name as `Protocol.Type`.
fn extract_defimpl_name<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<String> {
    let args = node.children().find(|c| c.kind().as_ref() == "arguments")?;

    let protocol_name = args
        .children()
        .find(|c| c.kind().as_ref() == "alias")
        .map(|n| n.text().to_string())?;

    // Look for `for:` keyword
    let for_type = args.children().find_map(|c| {
        if c.kind().as_ref() != "keywords" {
            return None;
        }
        for pair in c.children() {
            if pair.kind().as_ref() != "pair" {
                continue;
            }
            let mut has_for = false;
            let mut value = None;
            for pc in pair.children() {
                if pc.kind().as_ref() == "keyword" && pc.text().to_string().starts_with("for") {
                    has_for = true;
                }
                if has_for && pc.kind().as_ref() == "alias" {
                    value = Some(pc.text().to_string());
                }
            }
            if value.is_some() {
                return value;
            }
        }
        None
    });

    Some(for_type.map_or_else(
        || protocol_name.clone(),
        |ft| format!("{protocol_name}.{ft}"),
    ))
}

// ── defstruct ──────────────────────────────────────────────────────

/// Process a `defstruct` call (standalone extraction for struct inside a module).
pub fn process_defstruct<D: ast_grep_core::Doc>(node: &Node<D>) -> ParsedItem {
    let fields = extract_defstruct_fields(node);

    ParsedItem {
        kind: SymbolKind::Struct,
        name: "defstruct".to_string(),
        signature: helpers::extract_signature(node),
        source: helpers::extract_source(node, 50),
        doc_comment: String::new(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata: SymbolMetadata {
            fields,
            ..Default::default()
        },
    }
}

// ── defexception ───────────────────────────────────────────────────

/// Process a `defexception` call — struct with `is_error_type: true`.
pub fn process_defexception<D: ast_grep_core::Doc>(node: &Node<D>) -> ParsedItem {
    let fields = extract_defstruct_fields(node);
    let mut metadata = SymbolMetadata {
        fields,
        ..Default::default()
    };
    metadata.mark_error_type();

    ParsedItem {
        kind: SymbolKind::Struct,
        name: "defexception".to_string(),
        signature: helpers::extract_signature(node),
        source: helpers::extract_source(node, 50),
        doc_comment: String::new(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata,
    }
}

// ── @type / @typep / @opaque ───────────────────────────────────────

/// Try to extract a `@type`/`@typep`/`@opaque` definition from a `unary_operator` node.
pub fn try_extract_type_attr<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<ParsedItem> {
    // Must be a @ unary_operator
    if !node.children().any(|c| c.kind().as_ref() == "@") {
        return None;
    }

    let call_node = node.children().find(|c| c.kind().as_ref() == "call")?;

    let id = call_node
        .children()
        .find(|c| c.kind().as_ref() == "identifier")?;

    let attr_name = id.text().to_string();
    if !ELIXIR_TYPE_ATTRS.contains(&attr_name.as_str()) {
        return None;
    }

    let args = call_node
        .children()
        .find(|c| c.kind().as_ref() == "arguments")?;

    // The type name is in a binary_operator (name :: type_expr)
    // Simple type: `direction :: :north | :south` → binary_operator → identifier
    // Parametric:  `result(ok, err) :: ...` → binary_operator → call → identifier
    let name = args.children().find_map(|c| {
        if c.kind().as_ref() != "binary_operator" {
            return None;
        }
        // Try direct identifier first (simple type)
        if let Some(id_node) = c
            .children()
            .find(|inner| inner.kind().as_ref() == "identifier")
        {
            return Some(id_node.text().to_string());
        }
        // Then try call → identifier (parametric type)
        c.children()
            .find(|inner| inner.kind().as_ref() == "call")
            .and_then(|call| {
                call.children()
                    .find(|inner| inner.kind().as_ref() == "identifier")
                    .map(|n| n.text().to_string())
            })
    })?;

    let visibility = if attr_name == "typep" {
        Visibility::Private
    } else {
        Visibility::Public
    };

    // Get the full spec text (e.g., "direction :: :north | :south")
    let spec_text = args.text().to_string();

    Some(ParsedItem {
        kind: SymbolKind::TypeAlias,
        name,
        signature: format!("@{attr_name} {spec_text}"),
        source: Some(node.text().to_string()),
        doc_comment: String::new(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility,
        metadata: SymbolMetadata {
            return_type: Some(spec_text),
            ..Default::default()
        },
    })
}
