use ast_grep_core::Node;

use crate::extractors::helpers;
use crate::types::{ElixirMetadataExt, ParsedItem, SymbolKind, SymbolMetadata, Visibility};

use super::elixir_helpers::{
    build_elixir_signature, extract_callbacks, extract_def_name, extract_def_params,
    extract_defstruct_fields, extract_elixir_doc, extract_guard, extract_module_methods,
    extract_module_name, extract_moduledoc, extract_spec, extract_struct_fields_from_module,
    has_impl_attr, module_has_keyword,
};

const ELIXIR_TYPE_ATTRS: &[&str] = &["type", "typep", "opaque"];

pub(super) fn process_defmodule<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<ParsedItem> {
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

/// Process a `def`/`defp` call.
pub(super) fn process_def<D: ast_grep_core::Doc>(
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

/// Process a `defmacro`/`defmacrop` call.
pub(super) fn process_defmacro<D: ast_grep_core::Doc>(
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

/// Process a `defprotocol` call.
pub(super) fn process_defprotocol<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<ParsedItem> {
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

/// Process a `defimpl` call.
pub(super) fn process_defimpl<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<ParsedItem> {
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
pub(super) fn extract_defimpl_name<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<String> {
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

/// Process a `defstruct` call (standalone extraction for struct inside a module).
pub(super) fn process_defstruct<D: ast_grep_core::Doc>(node: &Node<D>) -> ParsedItem {
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

/// Process a `defexception` call — struct with `is_error_type: true`.
///
/// `defexception` has the same structure as `defstruct` (list of atoms or keywords).
pub(super) fn process_defexception<D: ast_grep_core::Doc>(node: &Node<D>) -> ParsedItem {
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

/// Process a `defguard`/`defguardp` call — extracted as `Macro`.
///
/// Guards have the same argument structure as `def` with `when` clauses:
/// `defguard is_pos(value) when is_integer(value) and value > 0`
pub(super) fn process_defguard<D: ast_grep_core::Doc>(
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

/// Process a `defdelegate` call — extracted as a public `Function`.
///
/// `defdelegate process(items), to: Sample.Processor`
pub(super) fn process_defdelegate<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<ParsedItem> {
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
pub(super) fn extract_delegate_target<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<String> {
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

/// Try to extract a `@type`/`@typep`/`@opaque` definition from a `unary_operator` node.
///
/// Structure: `unary_operator` → `@` + `call(identifier="type"/"typep"/"opaque")`
///            → `arguments` → `binary_operator(identifier :: type_expr)`
pub(super) fn try_extract_type_attr<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<ParsedItem> {
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

/// Deduplicate multi-clause functions.
///
/// Elixir allows multiple function clauses (e.g., `def classify(x) when is_integer(x)`
/// and `def classify(x) when is_float(x)`). We keep only the first clause per name+kind
/// **within the same scope** (determined by line proximity — clauses within 20 lines
/// of each other are considered the same function).
pub(super) fn dedup_multi_clause(items: &mut Vec<ParsedItem>) {
    // Map from (kind, name) → first occurrence line number
    let mut seen: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
    items.retain(|item| {
        // Only dedup functions and macros (modules, protocols, etc. are unique)
        if matches!(item.kind, SymbolKind::Function | SymbolKind::Macro) {
            let key = format!("{}:{}", item.kind, item.name);
            if let Some(&first_line) = seen.get(&key) {
                // Only dedup if within 20 lines of the first clause
                // (same module scope). Different modules will be far apart.
                item.start_line.abs_diff(first_line) > 20
            } else {
                seen.insert(key, item.start_line);
                true
            }
        } else {
            true
        }
    });
}
