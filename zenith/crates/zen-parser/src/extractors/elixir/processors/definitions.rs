use ast_grep_core::Node;

use crate::extractors::helpers;
use crate::types::{ElixirMetadataExt, ParsedItem, SymbolKind, SymbolMetadata, Visibility};

use super::super::elixir_helpers::{
    build_elixir_signature, extract_callbacks, extract_def_name, extract_def_params,
    extract_elixir_doc, extract_guard, extract_module_methods, extract_module_name,
    extract_moduledoc, extract_spec, extract_struct_fields_from_module, has_impl_attr,
    module_has_keyword,
};

// ── defmodule ──────────────────────────────────────────────────────

pub fn process_defmodule<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<ParsedItem> {
    let name = extract_module_name(node)?;
    let doc = extract_moduledoc(node);
    let methods = extract_module_methods(node);
    let fields = extract_struct_fields_from_module(node);
    let callbacks = extract_callbacks(node);
    let has_defexception = module_has_keyword(node, "defexception");

    Some(ParsedItem {
        kind: SymbolKind::Module,
        name,
        signature: helpers::extract_signature(node),
        source: helpers::extract_source(node, 50),
        doc_comment: doc,
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata: SymbolMetadata {
            methods,
            fields,
            associated_types: callbacks,
            is_error_type: has_defexception,
            ..Default::default()
        },
    })
}

// ── def / defp ─────────────────────────────────────────────────────

/// Process a `def`/`defp` call.
pub fn process_def<D: ast_grep_core::Doc>(
    node: &Node<D>,
    visibility: Visibility,
) -> Option<ParsedItem> {
    let name = extract_def_name(node)?;
    let doc = extract_elixir_doc(node);
    let params = extract_def_params(node);
    let guard = extract_guard(node);
    let spec = extract_spec(node);
    let is_callback_impl = has_impl_attr(node);
    let keyword = if visibility == Visibility::Public {
        "def"
    } else {
        "defp"
    };

    let mut metadata = SymbolMetadata::default();
    for param in params {
        metadata.push_parameter(param);
    }
    metadata.set_spec(spec);
    metadata.set_guard(guard);
    if is_callback_impl {
        metadata.mark_callback_impl();
    }

    Some(ParsedItem {
        kind: SymbolKind::Function,
        name,
        signature: build_elixir_signature(node, keyword),
        source: helpers::extract_source(node, 50),
        doc_comment: doc,
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility,
        metadata,
    })
}

// ── defmacro / defmacrop ───────────────────────────────────────────

/// Process a `defmacro`/`defmacrop` call.
pub fn process_defmacro<D: ast_grep_core::Doc>(
    node: &Node<D>,
    visibility: Visibility,
) -> Option<ParsedItem> {
    let name = extract_def_name(node)?;
    let doc = extract_elixir_doc(node);
    let params = extract_def_params(node);
    let keyword = if visibility == Visibility::Public {
        "defmacro"
    } else {
        "defmacrop"
    };

    let mut metadata = SymbolMetadata::default();
    for param in params {
        metadata.push_parameter(param);
    }

    Some(ParsedItem {
        kind: SymbolKind::Macro,
        name,
        signature: build_elixir_signature(node, keyword),
        source: helpers::extract_source(node, 50),
        doc_comment: doc,
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility,
        metadata,
    })
}

// ── defguard / defguardp ───────────────────────────────────────────

/// Process a `defguard`/`defguardp` call — extracted as `Macro`.
pub fn process_defguard<D: ast_grep_core::Doc>(
    node: &Node<D>,
    visibility: Visibility,
) -> Option<ParsedItem> {
    let name = extract_def_name(node)?;
    let doc = extract_elixir_doc(node);
    let params = extract_def_params(node);
    let guard = extract_guard(node);
    let keyword = if visibility == Visibility::Public {
        "defguard"
    } else {
        "defguardp"
    };

    let mut metadata = SymbolMetadata::default();
    for param in params {
        metadata.push_parameter(param);
    }
    metadata.set_guard(guard);

    Some(ParsedItem {
        kind: SymbolKind::Macro,
        name,
        signature: build_elixir_signature(node, keyword),
        source: helpers::extract_source(node, 50),
        doc_comment: doc,
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility,
        metadata,
    })
}

// ── defdelegate ────────────────────────────────────────────────────

/// Process a `defdelegate` call — extracted as a public `Function`.
pub fn process_defdelegate<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<ParsedItem> {
    let name = extract_def_name(node)?;
    let params = extract_def_params(node);
    let delegate_target = extract_delegate_target(node);

    let mut metadata = SymbolMetadata::default();
    for param in params {
        metadata.push_parameter(param);
    }
    metadata.set_delegate_target(delegate_target);

    Some(ParsedItem {
        kind: SymbolKind::Function,
        name,
        signature: build_elixir_signature(node, "defdelegate"),
        source: helpers::extract_source(node, 50),
        doc_comment: String::new(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata,
    })
}

/// Extract the `to:` target from a `defdelegate` call.
fn extract_delegate_target<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<String> {
    let args = node.children().find(|c| c.kind().as_ref() == "arguments")?;

    args.children().find_map(|c| {
        if c.kind().as_ref() != "keywords" {
            return None;
        }
        for pair in c.children() {
            if pair.kind().as_ref() != "pair" {
                continue;
            }
            let has_to = pair.children().any(|pc| {
                pc.kind().as_ref() == "keyword" && pc.text().to_string().starts_with("to")
            });
            if has_to {
                // The target is an alias child
                return pair
                    .children()
                    .find(|pc| pc.kind().as_ref() == "alias")
                    .map(|n| n.text().to_string());
            }
        }
        None
    })
}
