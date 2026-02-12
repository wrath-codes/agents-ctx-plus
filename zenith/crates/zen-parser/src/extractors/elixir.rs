//! Elixir rich extractor — `call`-node-first strategy.
//!
//! In Elixir's tree-sitter grammar, all definitions are `call` nodes.
//! The first `identifier` child determines the construct:
//!
//! - `defmodule` → Module
//! - `def` → Function (public)
//! - `defp` → Function (private)
//! - `defmacro` → Macro (public)
//! - `defmacrop` → Macro (private)
//! - `defprotocol` → Interface (protocol)
//! - `defimpl` → Trait (protocol implementation)
//! - `defstruct` → Struct
//! - `defexception` → Struct (with `is_error_type`)
//! - `defguard` → Macro (public)
//! - `defguardp` → Macro (private)
//! - `defdelegate` → Function (public, delegated)
//!
//! Type definitions (`@type`, `@typep`, `@opaque`) are `unary_operator` nodes
//! extracted in a second pass as `TypeAlias` items.
//!
//! Doc comments are extracted from preceding `@doc`/`@moduledoc` siblings
//! (which are `unary_operator` nodes with `@` child).

use ast_grep_core::matcher::KindMatcher;
use ast_grep_core::Node;
use ast_grep_language::SupportLang;

use super::helpers;
use crate::types::{ParsedItem, SymbolKind, SymbolMetadata, Visibility};

/// Elixir definition keywords we extract at any nesting depth.
const ELIXIR_DEF_KEYWORDS: &[&str] = &[
    "def",
    "defp",
    "defmacro",
    "defmacrop",
    "defmodule",
    "defprotocol",
    "defimpl",
    "defstruct",
    "defexception",
    "defguard",
    "defguardp",
    "defdelegate",
];

/// Elixir type-definition attribute keywords (`@type`, `@typep`, `@opaque`).
const ELIXIR_TYPE_ATTRS: &[&str] = &["type", "typep", "opaque"];

/// Extract all API symbols from an Elixir source file.
///
/// # Errors
/// Returns `ParserError` if extraction fails.
pub fn extract<D: ast_grep_core::Doc<Lang = SupportLang>>(
    root: &ast_grep_core::AstGrep<D>,
) -> Result<Vec<ParsedItem>, crate::error::ParserError> {
    let mut items = Vec::new();
    let matcher = KindMatcher::new("call", SupportLang::Elixir);

    for node in root.root().find_all(&matcher) {
        let Some(keyword) = first_identifier_text(&node) else {
            continue;
        };

        if !ELIXIR_DEF_KEYWORDS.contains(&keyword.as_str()) {
            continue;
        }

        match keyword.as_str() {
            "defmodule" => {
                if let Some(item) = process_defmodule(&node) {
                    items.push(item);
                }
            }
            "def" => {
                if let Some(item) = process_def(&node, Visibility::Public) {
                    items.push(item);
                }
            }
            "defp" => {
                if let Some(item) = process_def(&node, Visibility::Private) {
                    items.push(item);
                }
            }
            "defmacro" => {
                if let Some(item) = process_defmacro(&node, Visibility::Public) {
                    items.push(item);
                }
            }
            "defmacrop" => {
                if let Some(item) = process_defmacro(&node, Visibility::Private) {
                    items.push(item);
                }
            }
            "defprotocol" => {
                if let Some(item) = process_defprotocol(&node) {
                    items.push(item);
                }
            }
            "defimpl" => {
                if let Some(item) = process_defimpl(&node) {
                    items.push(item);
                }
            }
            "defstruct" => {
                items.push(process_defstruct(&node));
            }
            "defexception" => {
                items.push(process_defexception(&node));
            }
            "defguard" => {
                if let Some(item) = process_defguard(&node, Visibility::Public) {
                    items.push(item);
                }
            }
            "defguardp" => {
                if let Some(item) = process_defguard(&node, Visibility::Private) {
                    items.push(item);
                }
            }
            "defdelegate" => {
                if let Some(item) = process_defdelegate(&node) {
                    items.push(item);
                }
            }
            _ => {}
        }
    }

    // Second pass: extract @type/@typep/@opaque from unary_operator nodes
    let unary_matcher = KindMatcher::new("unary_operator", SupportLang::Elixir);
    for node in root.root().find_all(&unary_matcher) {
        if let Some(item) = try_extract_type_attr(&node) {
            items.push(item);
        }
    }

    // Deduplicate multi-clause functions: keep only the first clause.
    dedup_multi_clause(&mut items);

    Ok(items)
}

// ── Helpers ──────────────────────────────────────────────────────

/// Get the first `identifier` child's text from a `call` node.
fn first_identifier_text<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<String> {
    node.children()
        .find(|c| c.kind().as_ref() == "identifier")
        .map(|n| n.text().to_string())
}

/// Extract module name from a `defmodule`/`defprotocol` call's `arguments` → `alias`.
fn extract_module_name<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<String> {
    let args = node
        .children()
        .find(|c| c.kind().as_ref() == "arguments")?;
    args.children()
        .find(|c| c.kind().as_ref() == "alias")
        .map(|n| n.text().to_string())
}

/// Extract function/macro name from a `def`/`defp`/`defmacro`/`defmacrop` call.
///
/// Two forms exist in the AST:
/// 1. `def process(items)` → `arguments` → first child is `call` → `identifier` = name
/// 2. `def process(items) when guard` → `arguments` → `binary_operator` → first `call` → `identifier`
/// 3. `def max_retries, do: ...` → `arguments` → first child is `identifier` = name (no-paren form)
fn extract_def_name<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<String> {
    let args = node
        .children()
        .find(|c| c.kind().as_ref() == "arguments")?;

    for child in args.children() {
        let k = child.kind();
        match k.as_ref() {
            // Direct call: `def process(items)`
            "call" => {
                return child
                    .children()
                    .find(|c| c.kind().as_ref() == "identifier")
                    .map(|n| n.text().to_string());
            }
            // Guard clause: `def process(items) when is_list(items)`
            "binary_operator" => {
                // The call is inside the binary_operator
                for inner in child.children() {
                    if inner.kind().as_ref() == "call" {
                        return inner
                            .children()
                            .find(|c| c.kind().as_ref() == "identifier")
                            .map(|n| n.text().to_string());
                    }
                }
            }
            // No-paren form: `def max_retries, do: @max_retries`
            "identifier" => {
                return Some(child.text().to_string());
            }
            _ => {}
        }
    }
    None
}

/// Extract function parameters from a `def` call.
///
/// Navigates: `call` → `arguments` → `call`/`binary_operator` → inner `call` → `arguments` children
fn extract_def_params<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    let Some(args) = node
        .children()
        .find(|c| c.kind().as_ref() == "arguments")
    else {
        return Vec::new();
    };

    // Find the inner call (possibly inside a binary_operator for guards)
    let inner_call = find_inner_def_call(&args);
    let Some(call) = inner_call else {
        return Vec::new();
    };

    // The call's `arguments` child has the actual params
    let Some(params_node) = call
        .children()
        .find(|c| c.kind().as_ref() == "arguments")
    else {
        return Vec::new();
    };

    params_node
        .children()
        .filter(|c| {
            let k = c.kind();
            let kr = k.as_ref();
            kr != "(" && kr != ")" && kr != ","
        })
        .map(|c| c.text().to_string())
        .collect()
}

/// Find the inner function `call` node inside a def's `arguments`.
fn find_inner_def_call<'a, D: ast_grep_core::Doc>(
    args: &Node<'a, D>,
) -> Option<Node<'a, D>> {
    for child in args.children() {
        let k = child.kind();
        match k.as_ref() {
            "call" => return Some(child),
            "binary_operator" => {
                for inner in child.children() {
                    if inner.kind().as_ref() == "call" {
                        return Some(inner);
                    }
                }
            }
            _ => {}
        }
    }
    None
}

/// Extract guard clause text from a def with `when`.
fn extract_guard<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<String> {
    let args = node
        .children()
        .find(|c| c.kind().as_ref() == "arguments")?;

    for child in args.children() {
        if child.kind().as_ref() == "binary_operator" {
            // Check if it contains a `when` keyword
            let has_when = child
                .children()
                .any(|c| c.kind().as_ref() == "when");
            if has_when {
                // The guard is the part after `when` — extract from binary_operator text
                let text = child.text().to_string();
                if let Some(when_pos) = text.find(" when ") {
                    return Some(text[when_pos + 6..].to_string());
                }
            }
        }
    }
    None
}

/// Extract `@doc` content from the preceding sibling of a def/defmacro call.
///
/// In Elixir's AST, `@doc "..."` is a `unary_operator` sibling with:
/// `@` child + `call` child (identifier="doc") + `arguments` child (string or heredoc).
///
/// `@doc false` means "no doc" — we return empty string.
fn extract_elixir_doc<D: ast_grep_core::Doc>(node: &Node<D>) -> String {
    let mut current = node.prev();
    while let Some(sibling) = current {
        let k = sibling.kind();
        match k.as_ref() {
            "unary_operator" => {
                if let Some(doc) = try_extract_doc_attr(&sibling, "doc") {
                    return doc;
                }
                // Skip @spec, @impl, and other @ attributes
                if is_at_attribute(&sibling) {
                    current = sibling.prev();
                } else {
                    break;
                }
            }
            // Skip comments between definitions
            "comment" => {
                current = sibling.prev();
            }
            _ => break,
        }
    }
    String::new()
}

/// Extract `@moduledoc` content from inside a defmodule's `do_block`.
fn extract_moduledoc<D: ast_grep_core::Doc>(node: &Node<D>) -> String {
    let Some(do_block) = node
        .children()
        .find(|c| c.kind().as_ref() == "do_block")
    else {
        return String::new();
    };

    for child in do_block.children() {
        if child.kind().as_ref() == "unary_operator"
            && let Some(doc) = try_extract_doc_attr(&child, "moduledoc")
        {
            return doc;
        }
    }
    String::new()
}

/// Try to extract a doc string from a `unary_operator` node representing `@doc` or `@moduledoc`.
///
/// Returns `None` if the node is not the expected attribute.
/// Returns `Some("")` for `@doc false`.
/// Returns `Some(content)` for `@doc "content"` or `@doc """content"""`.
fn try_extract_doc_attr<D: ast_grep_core::Doc>(
    node: &Node<D>,
    attr_name: &str,
) -> Option<String> {
    // Structure: unary_operator → @ + call(identifier=attr_name, arguments(string|boolean))
    let call_node = node
        .children()
        .find(|c| c.kind().as_ref() == "call")?;

    let id = call_node
        .children()
        .find(|c| c.kind().as_ref() == "identifier")?;

    if id.text().as_ref() != attr_name {
        return None;
    }

    let args = call_node
        .children()
        .find(|c| c.kind().as_ref() == "arguments")?;

    for child in args.children() {
        let k = child.kind();
        match k.as_ref() {
            "string" => {
                return Some(extract_string_content(&child));
            }
            "boolean" => {
                // @doc false means no documentation
                return Some(String::new());
            }
            _ => {}
        }
    }
    None
}

/// Check if a `unary_operator` is an `@` attribute (any kind).
fn is_at_attribute<D: ast_grep_core::Doc>(node: &Node<D>) -> bool {
    node.children().any(|c| c.kind().as_ref() == "@")
}

/// Extract string content from a `string` node (handles both `"..."` and `"""..."""`).
fn extract_string_content<D: ast_grep_core::Doc>(node: &Node<D>) -> String {
    // String node contains: `"` or `"""`, then `quoted_content`, then closing delimiter
    let content: Vec<String> = node
        .children()
        .filter(|c| c.kind().as_ref() == "quoted_content")
        .map(|c| c.text().to_string())
        .collect();

    let raw = content.join("");

    // For heredoc strings, trim leading/trailing whitespace per line
    if raw.starts_with('\n') {
        // Heredoc: trim common indentation
        let lines: Vec<&str> = raw.lines().collect();
        // Skip the first empty line and find minimum indentation
        let content_lines: Vec<&str> = lines
            .iter()
            .skip(1)
            .filter(|l| !l.trim().is_empty())
            .copied()
            .collect();

        let min_indent = content_lines
            .iter()
            .map(|l| l.len() - l.trim_start().len())
            .min()
            .unwrap_or(0);

        let trimmed: Vec<&str> = lines
            .iter()
            .skip(1)
            .map(|l| {
                if l.len() >= min_indent {
                    &l[min_indent..]
                } else {
                    l.trim()
                }
            })
            .collect();

        // Trim trailing empty line (from the closing """)
        let result = trimmed.join("\n");
        result.trim_end().to_string()
    } else {
        raw.trim().to_string()
    }
}

/// Build an Elixir signature from a def/defmacro call.
///
/// Format: `def name(params)` or `def name(params) when guard`
fn build_elixir_signature<D: ast_grep_core::Doc>(
    node: &Node<D>,
    keyword: &str,
) -> String {
    let name = extract_def_name(node).unwrap_or_default();
    let params = extract_def_params(node);
    let guard = extract_guard(node);

    let param_str = if params.is_empty() {
        String::new()
    } else {
        format!("({})", params.join(", "))
    };

    guard.map_or_else(
        || format!("{keyword} {name}{param_str}"),
        |g| format!("{keyword} {name}{param_str} when {g}"),
    )
}

/// Extract `@spec` from preceding siblings of a def call.
fn extract_spec<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<String> {
    let mut current = node.prev();
    while let Some(sibling) = current {
        let k = sibling.kind();
        match k.as_ref() {
            "unary_operator" => {
                if let Some(spec) = try_extract_at_attr_text(&sibling, "spec") {
                    return Some(spec);
                }
                if is_at_attribute(&sibling) {
                    current = sibling.prev();
                } else {
                    break;
                }
            }
            "comment" => {
                current = sibling.prev();
            }
            _ => break,
        }
    }
    None
}

/// Try to extract the full text of an `@attr ...` node.
fn try_extract_at_attr_text<D: ast_grep_core::Doc>(
    node: &Node<D>,
    attr_name: &str,
) -> Option<String> {
    let call_node = node
        .children()
        .find(|c| c.kind().as_ref() == "call")?;

    let id = call_node
        .children()
        .find(|c| c.kind().as_ref() == "identifier")?;

    if id.text().as_ref() != attr_name {
        return None;
    }

    let args = call_node
        .children()
        .find(|c| c.kind().as_ref() == "arguments")?;

    Some(args.text().to_string())
}

/// Check if a def has `@impl true` preceding it.
fn has_impl_attr<D: ast_grep_core::Doc>(node: &Node<D>) -> bool {
    let mut current = node.prev();
    while let Some(sibling) = current {
        let k = sibling.kind();
        match k.as_ref() {
            "unary_operator" => {
                if is_impl_true(&sibling) {
                    return true;
                }
                if is_at_attribute(&sibling) {
                    current = sibling.prev();
                } else {
                    break;
                }
            }
            "comment" => {
                current = sibling.prev();
            }
            _ => break,
        }
    }
    false
}

/// Check if a `unary_operator` is `@impl true`.
fn is_impl_true<D: ast_grep_core::Doc>(node: &Node<D>) -> bool {
    let call_node = node
        .children()
        .find(|c| c.kind().as_ref() == "call");
    let Some(call) = call_node else {
        return false;
    };
    let id = call
        .children()
        .find(|c| c.kind().as_ref() == "identifier");
    let Some(id) = id else {
        return false;
    };
    if id.text().as_ref() != "impl" {
        return false;
    }
    let args = call
        .children()
        .find(|c| c.kind().as_ref() == "arguments");
    args.is_some_and(|a| {
        a.children().any(|c| {
            c.kind().as_ref() == "boolean" && c.text().as_ref() == "true"
        })
    })
}

/// Check if a module's `do_block` contains a call with the given keyword.
fn module_has_keyword<D: ast_grep_core::Doc>(node: &Node<D>, keyword: &str) -> bool {
    let Some(do_block) = node
        .children()
        .find(|c| c.kind().as_ref() == "do_block")
    else {
        return false;
    };

    do_block.children().any(|child| {
        child.kind().as_ref() == "call"
            && first_identifier_text(&child).as_deref() == Some(keyword)
    })
}

/// Extract `@callback` definitions from a module's `do_block`.
fn extract_callbacks<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    let Some(do_block) = node
        .children()
        .find(|c| c.kind().as_ref() == "do_block")
    else {
        return Vec::new();
    };

    let mut callbacks = Vec::new();
    for child in do_block.children() {
        if child.kind().as_ref() == "unary_operator"
            && let Some(text) = try_extract_at_attr_text(&child, "callback")
            && let Some(name) = extract_callback_name(&text)
        {
            callbacks.push(name);
        }
    }
    callbacks
}

/// Extract the function name from a `@callback` spec text like
/// `handle_event(event :: term()) :: :ok | {:error, term()}`
fn extract_callback_name(spec_text: &str) -> Option<String> {
    // The spec is like: `handle_event(event :: term()) :: :ok | {:error, term()}`
    // We want just `handle_event`
    let text = spec_text.trim();
    let end = text.find('(').unwrap_or(text.len());
    let name = text[..end].trim();
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

/// Extract `defstruct` fields from inside a `defmodule` `do_block`.
fn extract_struct_fields_from_module<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    let Some(do_block) = node
        .children()
        .find(|c| c.kind().as_ref() == "do_block")
    else {
        return Vec::new();
    };

    for child in do_block.children() {
        if child.kind().as_ref() == "call" {
            let kw = first_identifier_text(&child);
            if matches!(kw.as_deref(), Some("defstruct" | "defexception")) {
                return extract_defstruct_fields(&child);
            }
        }
    }
    Vec::new()
}

/// Extract field names from a `defstruct` call.
fn extract_defstruct_fields<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    let Some(args) = node
        .children()
        .find(|c| c.kind().as_ref() == "arguments")
    else {
        return Vec::new();
    };

    let mut fields = Vec::new();

    // defstruct can have keywords (name: default) or a list of atoms
    for child in args.children() {
        let k = child.kind();
        match k.as_ref() {
            "keywords" => {
                for pair in child.children() {
                    if pair.kind().as_ref() == "pair" {
                        // keyword child text is like "name: " — extract just the key
                        for kw_child in pair.children() {
                            if kw_child.kind().as_ref() == "keyword" {
                                let text = kw_child.text().to_string();
                                let key = text.trim_end_matches(": ").trim_end_matches(':');
                                fields.push(key.to_string());
                            }
                        }
                    }
                }
            }
            "list" => {
                // List of atoms: [:field1, :field2]
                for item in child.children() {
                    if item.kind().as_ref() == "atom" {
                        let text = item.text().to_string();
                        fields.push(text.trim_start_matches(':').to_string());
                    }
                }
            }
            _ => {}
        }
    }
    fields
}

/// Extract methods defined inside a `defmodule`/`defprotocol`/`defimpl` `do_block`.
fn extract_module_methods<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    let Some(do_block) = node
        .children()
        .find(|c| c.kind().as_ref() == "do_block")
    else {
        return Vec::new();
    };

    let mut methods = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for child in do_block.children() {
        if child.kind().as_ref() == "call"
            && let Some(kw) = first_identifier_text(&child)
            && matches!(kw.as_str(), "def" | "defp" | "defdelegate")
            && let Some(name) = extract_def_name(&child)
            && seen.insert(name.clone())
        {
            methods.push(name);
        }
    }
    methods
}

// ── Node processors ──────────────────────────────────────────────

/// Process a `defmodule` call.
fn process_defmodule<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<ParsedItem> {
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
fn process_def<D: ast_grep_core::Doc>(
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

    Some(ParsedItem {
        kind: SymbolKind::Function,
        name,
        signature: build_elixir_signature(node, keyword),
        source: helpers::extract_source(node, 50),
        doc_comment: doc,
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility,
        metadata: SymbolMetadata {
            parameters: params,
            return_type: spec,
            // Store guard in where_clause (analogous to Rust where clause)
            where_clause: guard,
            // Mark callback implementations
            trait_name: if is_callback_impl {
                Some("@impl".to_string())
            } else {
                None
            },
            ..Default::default()
        },
    })
}

/// Process a `defmacro`/`defmacrop` call.
fn process_defmacro<D: ast_grep_core::Doc>(
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

    Some(ParsedItem {
        kind: SymbolKind::Macro,
        name,
        signature: build_elixir_signature(node, keyword),
        source: helpers::extract_source(node, 50),
        doc_comment: doc,
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility,
        metadata: SymbolMetadata {
            parameters: params,
            ..Default::default()
        },
    })
}

/// Process a `defprotocol` call.
fn process_defprotocol<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<ParsedItem> {
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
fn process_defimpl<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<ParsedItem> {
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
    let args = node
        .children()
        .find(|c| c.kind().as_ref() == "arguments")?;

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
                if pc.kind().as_ref() == "keyword"
                    && pc.text().to_string().starts_with("for")
                {
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
fn process_defstruct<D: ast_grep_core::Doc>(node: &Node<D>) -> ParsedItem {
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
fn process_defexception<D: ast_grep_core::Doc>(node: &Node<D>) -> ParsedItem {
    let fields = extract_defstruct_fields(node);

    ParsedItem {
        kind: SymbolKind::Struct,
        name: "defexception".to_string(),
        signature: helpers::extract_signature(node),
        source: helpers::extract_source(node, 50),
        doc_comment: String::new(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata: SymbolMetadata {
            fields,
            is_error_type: true,
            ..Default::default()
        },
    }
}

/// Process a `defguard`/`defguardp` call — extracted as `Macro`.
///
/// Guards have the same argument structure as `def` with `when` clauses:
/// `defguard is_pos(value) when is_integer(value) and value > 0`
fn process_defguard<D: ast_grep_core::Doc>(
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

    Some(ParsedItem {
        kind: SymbolKind::Macro,
        name,
        signature: build_elixir_signature(node, keyword),
        source: helpers::extract_source(node, 50),
        doc_comment: doc,
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility,
        metadata: SymbolMetadata {
            parameters: params,
            where_clause: guard,
            ..Default::default()
        },
    })
}

/// Process a `defdelegate` call — extracted as a public `Function`.
///
/// `defdelegate process(items), to: Sample.Processor`
fn process_defdelegate<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<ParsedItem> {
    let name = extract_def_name(node)?;
    let params = extract_def_params(node);
    let delegate_target = extract_delegate_target(node);

    Some(ParsedItem {
        kind: SymbolKind::Function,
        name,
        signature: build_elixir_signature(node, "defdelegate"),
        source: helpers::extract_source(node, 50),
        doc_comment: String::new(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata: SymbolMetadata {
            parameters: params,
            // Store delegation target in `for_type` (reuse field for "delegates to")
            for_type: delegate_target,
            ..Default::default()
        },
    })
}

/// Extract the `to:` target from a `defdelegate` call.
fn extract_delegate_target<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<String> {
    let args = node
        .children()
        .find(|c| c.kind().as_ref() == "arguments")?;

    args.children().find_map(|c| {
        if c.kind().as_ref() != "keywords" {
            return None;
        }
        for pair in c.children() {
            if pair.kind().as_ref() != "pair" {
                continue;
            }
            let has_to = pair
                .children()
                .any(|pc| pc.kind().as_ref() == "keyword" && pc.text().to_string().starts_with("to"));
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
fn try_extract_type_attr<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<ParsedItem> {
    // Must be a @ unary_operator
    if !node.children().any(|c| c.kind().as_ref() == "@") {
        return None;
    }

    let call_node = node
        .children()
        .find(|c| c.kind().as_ref() == "call")?;

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
        if let Some(id_node) = c.children().find(|inner| inner.kind().as_ref() == "identifier") {
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
fn dedup_multi_clause(items: &mut Vec<ParsedItem>) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use ast_grep_language::LanguageExt;
    use pretty_assertions::assert_eq;

    fn parse_and_extract(source: &str) -> Vec<ParsedItem> {
        let root = SupportLang::Elixir.ast_grep(source);
        extract(&root).expect("extraction should succeed")
    }

    fn find_by_name<'a>(items: &'a [ParsedItem], name: &str) -> &'a ParsedItem {
        items
            .iter()
            .find(|i| i.name == name)
            .unwrap_or_else(|| {
                let names: Vec<_> = items.iter().map(|i| format!("{}:{}", i.kind, i.name)).collect();
                panic!("no item named '{name}', available: {names:?}");
            })
    }

    fn find_all_by_name<'a>(items: &'a [ParsedItem], name: &str) -> Vec<&'a ParsedItem> {
        items.iter().filter(|i| i.name == name).collect()
    }

    // ── Module extraction ──────────────────────────────────────────

    #[test]
    fn defmodule_extracted() {
        let source = include_str!("../../tests/fixtures/sample.ex");
        let items = parse_and_extract(source);
        let m = find_by_name(&items, "Sample.Processor");
        assert_eq!(m.kind, SymbolKind::Module);
        assert_eq!(m.visibility, Visibility::Public);
    }

    #[test]
    fn moduledoc_heredoc_extracted() {
        let source = include_str!("../../tests/fixtures/sample.ex");
        let items = parse_and_extract(source);
        let m = find_by_name(&items, "Sample.Processor");
        assert!(
            m.doc_comment.contains("A sample processor module."),
            "doc: {:?}",
            m.doc_comment
        );
        assert!(
            m.doc_comment.contains("configurable strategies"),
            "doc should contain full text: {:?}",
            m.doc_comment
        );
    }

    #[test]
    fn moduledoc_inline_extracted() {
        let source = include_str!("../../tests/fixtures/sample.ex");
        let items = parse_and_extract(source);
        let m = find_by_name(&items, "Sample.Config");
        assert_eq!(m.doc_comment, "Configuration struct.");
    }

    #[test]
    fn moduledoc_false_is_empty() {
        let source = include_str!("../../tests/fixtures/sample.ex");
        let items = parse_and_extract(source);
        let m = find_by_name(&items, "Sample.Internal");
        assert_eq!(m.doc_comment, "");
    }

    #[test]
    fn module_methods_listed() {
        let source = include_str!("../../tests/fixtures/sample.ex");
        let items = parse_and_extract(source);
        let m = find_by_name(&items, "Sample.Processor");
        assert!(
            m.metadata.methods.contains(&"process".to_string()),
            "methods: {:?}",
            m.metadata.methods
        );
        assert!(
            m.metadata.methods.contains(&"process_one".to_string()),
            "methods: {:?}",
            m.metadata.methods
        );
    }

    #[test]
    fn all_modules_found() {
        let source = include_str!("../../tests/fixtures/sample.ex");
        let items = parse_and_extract(source);
        let module_names: Vec<_> = items
            .iter()
            .filter(|i| i.kind == SymbolKind::Module)
            .map(|i| i.name.as_str())
            .collect();
        assert!(module_names.contains(&"Sample.Processor"));
        assert!(module_names.contains(&"Sample.Config"));
        assert!(module_names.contains(&"Sample.Worker"));
        assert!(module_names.contains(&"Sample.Behaviour"));
        assert!(module_names.contains(&"Sample.Guards"));
        assert!(module_names.contains(&"Sample.Types"));
        assert!(module_names.contains(&"Sample.Constants"));
        assert!(module_names.contains(&"Sample.AppError"));
        assert!(module_names.contains(&"Sample.CustomGuards"));
        assert!(module_names.contains(&"Sample.Delegator"));
        assert!(module_names.contains(&"Sample.Internal"));
    }

    // ── Public function extraction ──────────────────────────────────

    #[test]
    fn public_function_extracted() {
        let source = include_str!("../../tests/fixtures/sample.ex");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "process");
        assert_eq!(f.kind, SymbolKind::Function);
        assert_eq!(f.visibility, Visibility::Public);
    }

    #[test]
    fn function_doc_heredoc() {
        let source = include_str!("../../tests/fixtures/sample.ex");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "process");
        assert!(
            f.doc_comment.contains("Process a list of items."),
            "doc: {:?}",
            f.doc_comment
        );
    }

    #[test]
    fn function_doc_inline() {
        let source = include_str!("../../tests/fixtures/sample.ex");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "process_one");
        assert_eq!(f.doc_comment, "Process a single item.");
    }

    #[test]
    fn function_params_extracted() {
        let source = include_str!("../../tests/fixtures/sample.ex");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "process");
        assert_eq!(f.metadata.parameters, vec!["items"]);
    }

    #[test]
    fn function_guard_extracted() {
        let source = include_str!("../../tests/fixtures/sample.ex");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "process");
        assert!(
            f.metadata.where_clause.is_some(),
            "should have guard clause"
        );
        assert!(
            f.metadata.where_clause.as_deref().unwrap().contains("is_list"),
            "guard: {:?}",
            f.metadata.where_clause
        );
    }

    #[test]
    fn function_spec_extracted() {
        let source = include_str!("../../tests/fixtures/sample.ex");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "process");
        assert!(
            f.metadata.return_type.is_some(),
            "should have @spec"
        );
        assert!(
            f.metadata.return_type.as_deref().unwrap().contains("list"),
            "spec: {:?}",
            f.metadata.return_type
        );
    }

    #[test]
    fn function_doc_false_is_empty() {
        let source = include_str!("../../tests/fixtures/sample.ex");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "internal_helper");
        assert_eq!(f.doc_comment, "");
    }

    #[test]
    fn oneline_function_extracted() {
        let source = include_str!("../../tests/fixtures/sample.ex");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "process_one");
        assert_eq!(f.kind, SymbolKind::Function);
        assert_eq!(f.visibility, Visibility::Public);
    }

    // ── Private function extraction ─────────────────────────────────

    #[test]
    fn private_function_extracted() {
        let source = include_str!("../../tests/fixtures/sample.ex");
        let items = parse_and_extract(source);
        // Find private "transform" — there may be a public one in Types module too
        let transforms: Vec<_> = items
            .iter()
            .filter(|i| i.name == "transform" && i.visibility == Visibility::Private)
            .collect();
        assert!(
            !transforms.is_empty(),
            "should find private transform function"
        );
    }

    #[test]
    fn private_function_with_default_params() {
        let source = include_str!("../../tests/fixtures/sample.ex");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "validate");
        assert_eq!(f.kind, SymbolKind::Function);
        assert_eq!(f.visibility, Visibility::Private);
        assert!(f.metadata.parameters.len() >= 2, "params: {:?}", f.metadata.parameters);
    }

    // ── Macro extraction ────────────────────────────────────────────

    #[test]
    fn public_macro_extracted() {
        let source = include_str!("../../tests/fixtures/sample.ex");
        let items = parse_and_extract(source);
        let m = find_by_name(&items, "define_handler");
        assert_eq!(m.kind, SymbolKind::Macro);
        assert_eq!(m.visibility, Visibility::Public);
    }

    #[test]
    fn macro_doc_extracted() {
        let source = include_str!("../../tests/fixtures/sample.ex");
        let items = parse_and_extract(source);
        let m = find_by_name(&items, "define_handler");
        assert!(
            m.doc_comment.contains("Define a handler"),
            "doc: {:?}",
            m.doc_comment
        );
    }

    #[test]
    fn macro_params_extracted() {
        let source = include_str!("../../tests/fixtures/sample.ex");
        let items = parse_and_extract(source);
        let m = find_by_name(&items, "define_handler");
        assert_eq!(m.metadata.parameters, vec!["name"]);
    }

    #[test]
    fn private_macro_extracted() {
        let source = include_str!("../../tests/fixtures/sample.ex");
        let items = parse_and_extract(source);
        let m = find_by_name(&items, "internal_macro");
        assert_eq!(m.kind, SymbolKind::Macro);
        assert_eq!(m.visibility, Visibility::Private);
    }

    // ── Struct extraction ───────────────────────────────────────────

    #[test]
    fn struct_module_has_fields() {
        let source = include_str!("../../tests/fixtures/sample.ex");
        let items = parse_and_extract(source);
        let m = find_by_name(&items, "Sample.Config");
        assert!(
            m.metadata.fields.contains(&"name".to_string()),
            "fields: {:?}",
            m.metadata.fields
        );
        assert!(
            m.metadata.fields.contains(&"retries".to_string()),
            "fields: {:?}",
            m.metadata.fields
        );
        assert!(
            m.metadata.fields.contains(&"timeout".to_string()),
            "fields: {:?}",
            m.metadata.fields
        );
    }

    #[test]
    fn defstruct_standalone_extracted() {
        let source = include_str!("../../tests/fixtures/sample.ex");
        let items = parse_and_extract(source);
        let structs: Vec<_> = items
            .iter()
            .filter(|i| i.kind == SymbolKind::Struct)
            .collect();
        assert!(!structs.is_empty(), "should extract defstruct");
        assert!(
            structs[0].metadata.fields.contains(&"name".to_string()),
            "fields: {:?}",
            structs[0].metadata.fields
        );
    }

    // ── Protocol extraction ─────────────────────────────────────────

    #[test]
    fn defprotocol_extracted() {
        let source = include_str!("../../tests/fixtures/sample.ex");
        let items = parse_and_extract(source);
        let p = find_by_name(&items, "Sample.Renderable");
        assert_eq!(p.kind, SymbolKind::Interface);
        assert_eq!(p.visibility, Visibility::Public);
    }

    #[test]
    fn protocol_doc_extracted() {
        let source = include_str!("../../tests/fixtures/sample.ex");
        let items = parse_and_extract(source);
        let p = find_by_name(&items, "Sample.Renderable");
        assert!(
            p.doc_comment.contains("rendering items"),
            "doc: {:?}",
            p.doc_comment
        );
    }

    #[test]
    fn protocol_methods_listed() {
        let source = include_str!("../../tests/fixtures/sample.ex");
        let items = parse_and_extract(source);
        let p = find_by_name(&items, "Sample.Renderable");
        assert!(
            p.metadata.methods.contains(&"render".to_string()),
            "methods: {:?}",
            p.metadata.methods
        );
    }

    // ── Protocol implementation ─────────────────────────────────────

    #[test]
    fn defimpl_extracted() {
        let source = include_str!("../../tests/fixtures/sample.ex");
        let items = parse_and_extract(source);
        let i = find_by_name(&items, "Sample.Renderable.BitString");
        assert_eq!(i.kind, SymbolKind::Trait);
    }

    #[test]
    fn defimpl_methods_listed() {
        let source = include_str!("../../tests/fixtures/sample.ex");
        let items = parse_and_extract(source);
        let i = find_by_name(&items, "Sample.Renderable.BitString");
        assert!(
            i.metadata.methods.contains(&"render".to_string()),
            "methods: {:?}",
            i.metadata.methods
        );
    }

    // ── GenServer / @impl callbacks ─────────────────────────────────

    #[test]
    fn genserver_module_extracted() {
        let source = include_str!("../../tests/fixtures/sample.ex");
        let items = parse_and_extract(source);
        let m = find_by_name(&items, "Sample.Worker");
        assert_eq!(m.kind, SymbolKind::Module);
    }

    #[test]
    fn impl_callback_detected() {
        let source = include_str!("../../tests/fixtures/sample.ex");
        let items = parse_and_extract(source);
        let init_items: Vec<_> = items
            .iter()
            .filter(|i| i.name == "init" && i.kind == SymbolKind::Function)
            .collect();
        assert!(!init_items.is_empty(), "should find init callback");
        let init = init_items[0];
        assert_eq!(init.metadata.trait_name.as_deref(), Some("@impl"));
    }

    #[test]
    fn handle_call_extracted() {
        let source = include_str!("../../tests/fixtures/sample.ex");
        let items = parse_and_extract(source);
        let hc: Vec<_> = items
            .iter()
            .filter(|i| i.name == "handle_call")
            .collect();
        assert!(!hc.is_empty(), "should find handle_call");
    }

    // ── Behaviour callbacks ─────────────────────────────────────────

    #[test]
    fn behaviour_module_extracted() {
        let source = include_str!("../../tests/fixtures/sample.ex");
        let items = parse_and_extract(source);
        let m = find_by_name(&items, "Sample.Behaviour");
        assert_eq!(m.kind, SymbolKind::Module);
    }

    #[test]
    fn behaviour_callbacks_listed() {
        let source = include_str!("../../tests/fixtures/sample.ex");
        let items = parse_and_extract(source);
        let m = find_by_name(&items, "Sample.Behaviour");
        assert!(
            m.metadata.associated_types.contains(&"handle_event".to_string()),
            "callbacks: {:?}",
            m.metadata.associated_types
        );
        assert!(
            m.metadata.associated_types.contains(&"format_output".to_string()),
            "callbacks: {:?}",
            m.metadata.associated_types
        );
    }

    // ── Multi-clause dedup ──────────────────────────────────────────

    #[test]
    fn multi_clause_deduped() {
        let source = include_str!("../../tests/fixtures/sample.ex");
        let items = parse_and_extract(source);
        let classify_items = find_all_by_name(&items, "classify");
        assert_eq!(
            classify_items.len(),
            1,
            "multi-clause classify should be deduped to 1, found: {}",
            classify_items.len()
        );
    }

    #[test]
    fn multi_clause_keeps_first_doc() {
        let source = include_str!("../../tests/fixtures/sample.ex");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "classify");
        assert!(
            f.doc_comment.contains("Classify a value"),
            "doc: {:?}",
            f.doc_comment
        );
    }

    // ── No-paren function ──────────────────────────────────────────

    #[test]
    fn no_paren_function_extracted() {
        let source = include_str!("../../tests/fixtures/sample.ex");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "get_state");
        assert_eq!(f.kind, SymbolKind::Function);
        assert_eq!(f.visibility, Visibility::Public);
    }

    // ── Constructor ────────────────────────────────────────────────

    #[test]
    fn constructor_function_extracted() {
        let source = include_str!("../../tests/fixtures/sample.ex");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "new");
        assert_eq!(f.kind, SymbolKind::Function);
        assert_eq!(f.visibility, Visibility::Public);
    }

    // ── Line numbers ───────────────────────────────────────────────

    #[test]
    fn line_numbers_are_one_based() {
        let source = include_str!("../../tests/fixtures/sample.ex");
        let items = parse_and_extract(source);
        let m = find_by_name(&items, "Sample.Processor");
        assert!(m.start_line >= 1, "start_line should be 1-based");
        assert!(
            m.end_line > m.start_line,
            "end_line {} should be > start_line {}",
            m.end_line,
            m.start_line
        );
    }

    // ── Signature format ───────────────────────────────────────────

    #[test]
    fn function_signature_format() {
        let source = include_str!("../../tests/fixtures/sample.ex");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "process");
        assert!(
            f.signature.starts_with("def process"),
            "sig: {:?}",
            f.signature
        );
    }

    #[test]
    fn private_function_signature() {
        let source = include_str!("../../tests/fixtures/sample.ex");
        let items = parse_and_extract(source);
        let transforms: Vec<_> = items
            .iter()
            .filter(|i| i.name == "transform" && i.visibility == Visibility::Private)
            .collect();
        assert!(!transforms.is_empty());
        assert!(
            transforms[0].signature.starts_with("defp transform"),
            "sig: {:?}",
            transforms[0].signature
        );
    }

    #[test]
    fn macro_signature_format() {
        let source = include_str!("../../tests/fixtures/sample.ex");
        let items = parse_and_extract(source);
        let m = find_by_name(&items, "define_handler");
        assert!(
            m.signature.starts_with("defmacro define_handler"),
            "sig: {:?}",
            m.signature
        );
    }

    // ── Constants module ───────────────────────────────────────────

    #[test]
    fn constants_module_extracted() {
        let source = include_str!("../../tests/fixtures/sample.ex");
        let items = parse_and_extract(source);
        let m = find_by_name(&items, "Sample.Constants");
        assert_eq!(m.kind, SymbolKind::Module);
        assert!(
            m.doc_comment.contains("constants"),
            "doc: {:?}",
            m.doc_comment
        );
    }

    #[test]
    fn constants_module_methods() {
        let source = include_str!("../../tests/fixtures/sample.ex");
        let items = parse_and_extract(source);
        let m = find_by_name(&items, "Sample.Constants");
        assert!(
            m.metadata.methods.contains(&"max_retries".to_string()),
            "methods: {:?}",
            m.metadata.methods
        );
        assert!(
            m.metadata.methods.contains(&"default_timeout".to_string()),
            "methods: {:?}",
            m.metadata.methods
        );
    }

    // ── defexception ───────────────────────────────────────────────

    #[test]
    fn defexception_module_extracted() {
        let source = include_str!("../../tests/fixtures/sample.ex");
        let items = parse_and_extract(source);
        let m = find_by_name(&items, "Sample.AppError");
        assert_eq!(m.kind, SymbolKind::Module);
        assert!(m.metadata.is_error_type, "should be marked as error type");
    }

    #[test]
    fn defexception_module_has_fields() {
        let source = include_str!("../../tests/fixtures/sample.ex");
        let items = parse_and_extract(source);
        let m = find_by_name(&items, "Sample.AppError");
        assert!(
            m.metadata.fields.contains(&"message".to_string()),
            "fields: {:?}",
            m.metadata.fields
        );
        assert!(
            m.metadata.fields.contains(&"code".to_string()),
            "fields: {:?}",
            m.metadata.fields
        );
    }

    #[test]
    fn defexception_standalone_extracted() {
        let source = include_str!("../../tests/fixtures/sample.ex");
        let items = parse_and_extract(source);
        let structs: Vec<_> = items
            .iter()
            .filter(|i| i.kind == SymbolKind::Struct && i.name == "defexception")
            .collect();
        assert!(!structs.is_empty(), "should extract defexception as struct");
        assert!(structs[0].metadata.is_error_type, "should be error type");
        assert!(
            structs[0].metadata.fields.contains(&"message".to_string()),
            "fields: {:?}",
            structs[0].metadata.fields
        );
    }

    #[test]
    fn defexception_module_doc() {
        let source = include_str!("../../tests/fixtures/sample.ex");
        let items = parse_and_extract(source);
        let m = find_by_name(&items, "Sample.AppError");
        assert_eq!(m.doc_comment, "Application error.");
    }

    #[test]
    fn defexception_module_methods() {
        let source = include_str!("../../tests/fixtures/sample.ex");
        let items = parse_and_extract(source);
        let m = find_by_name(&items, "Sample.AppError");
        assert!(
            m.metadata.methods.contains(&"from_code".to_string()),
            "methods: {:?}",
            m.metadata.methods
        );
    }

    // ── defguard / defguardp ───────────────────────────────────────

    #[test]
    fn defguard_extracted() {
        let source = include_str!("../../tests/fixtures/sample.ex");
        let items = parse_and_extract(source);
        let g = find_by_name(&items, "is_pos_integer");
        assert_eq!(g.kind, SymbolKind::Macro);
        assert_eq!(g.visibility, Visibility::Public);
    }

    #[test]
    fn defguard_has_doc() {
        let source = include_str!("../../tests/fixtures/sample.ex");
        let items = parse_and_extract(source);
        let g = find_by_name(&items, "is_pos_integer");
        assert!(
            g.doc_comment.contains("positive integer"),
            "doc: {:?}",
            g.doc_comment
        );
    }

    #[test]
    fn defguard_params_extracted() {
        let source = include_str!("../../tests/fixtures/sample.ex");
        let items = parse_and_extract(source);
        let g = find_by_name(&items, "is_pos_integer");
        assert_eq!(g.metadata.parameters, vec!["value"]);
    }

    #[test]
    fn defguard_guard_clause() {
        let source = include_str!("../../tests/fixtures/sample.ex");
        let items = parse_and_extract(source);
        let g = find_by_name(&items, "is_pos_integer");
        assert!(
            g.metadata.where_clause.is_some(),
            "should have guard clause"
        );
        assert!(
            g.metadata.where_clause.as_deref().unwrap().contains("is_integer"),
            "guard: {:?}",
            g.metadata.where_clause
        );
    }

    #[test]
    fn defguard_signature_format() {
        let source = include_str!("../../tests/fixtures/sample.ex");
        let items = parse_and_extract(source);
        let g = find_by_name(&items, "is_pos_integer");
        assert!(
            g.signature.starts_with("defguard is_pos_integer"),
            "sig: {:?}",
            g.signature
        );
    }

    #[test]
    fn defguardp_extracted() {
        let source = include_str!("../../tests/fixtures/sample.ex");
        let items = parse_and_extract(source);
        let g = find_by_name(&items, "is_internal");
        assert_eq!(g.kind, SymbolKind::Macro);
        assert_eq!(g.visibility, Visibility::Private);
    }

    #[test]
    fn defguardp_signature_format() {
        let source = include_str!("../../tests/fixtures/sample.ex");
        let items = parse_and_extract(source);
        let g = find_by_name(&items, "is_internal");
        assert!(
            g.signature.starts_with("defguardp is_internal"),
            "sig: {:?}",
            g.signature
        );
    }

    // ── defdelegate ────────────────────────────────────────────────

    #[test]
    fn defdelegate_extracted() {
        let source = include_str!("../../tests/fixtures/sample.ex");
        let items = parse_and_extract(source);
        // There are multiple "process" functions — find the delegate one
        let delegates: Vec<_> = items
            .iter()
            .filter(|i| i.metadata.for_type.as_deref() == Some("Sample.Processor"))
            .collect();
        assert!(!delegates.is_empty(), "should find delegated process");
        assert_eq!(delegates[0].kind, SymbolKind::Function);
        assert_eq!(delegates[0].visibility, Visibility::Public);
    }

    #[test]
    fn defdelegate_target_extracted() {
        let source = include_str!("../../tests/fixtures/sample.ex");
        let items = parse_and_extract(source);
        let delegates: Vec<_> = items
            .iter()
            .filter(|i| i.metadata.for_type.as_deref() == Some("Sample.Config"))
            .collect();
        assert!(!delegates.is_empty(), "should find delegated new");
        assert_eq!(
            delegates[0].metadata.for_type.as_deref(),
            Some("Sample.Config")
        );
    }

    #[test]
    fn defdelegate_signature_format() {
        let source = include_str!("../../tests/fixtures/sample.ex");
        let items = parse_and_extract(source);
        let delegates: Vec<_> = items
            .iter()
            .filter(|i| i.metadata.for_type.as_deref() == Some("Sample.Processor"))
            .collect();
        assert!(!delegates.is_empty());
        assert!(
            delegates[0].signature.starts_with("defdelegate"),
            "sig: {:?}",
            delegates[0].signature
        );
    }

    #[test]
    fn delegator_module_methods_include_delegates() {
        let source = include_str!("../../tests/fixtures/sample.ex");
        let items = parse_and_extract(source);
        let m = find_by_name(&items, "Sample.Delegator");
        assert!(
            m.metadata.methods.contains(&"process".to_string()),
            "methods should include delegated 'process': {:?}",
            m.metadata.methods
        );
    }

    // ── @type / @typep / @opaque ───────────────────────────────────

    #[test]
    fn type_attr_extracted() {
        let source = include_str!("../../tests/fixtures/sample.ex");
        let items = parse_and_extract(source);
        let t = find_by_name(&items, "direction");
        assert_eq!(t.kind, SymbolKind::TypeAlias);
        assert_eq!(t.visibility, Visibility::Public);
    }

    #[test]
    fn type_attr_signature() {
        let source = include_str!("../../tests/fixtures/sample.ex");
        let items = parse_and_extract(source);
        let t = find_by_name(&items, "direction");
        assert!(
            t.signature.starts_with("@type"),
            "sig: {:?}",
            t.signature
        );
    }

    #[test]
    fn parametric_type_extracted() {
        let source = include_str!("../../tests/fixtures/sample.ex");
        let items = parse_and_extract(source);
        let t = find_by_name(&items, "result");
        assert_eq!(t.kind, SymbolKind::TypeAlias);
        assert_eq!(t.visibility, Visibility::Public);
    }

    #[test]
    fn typep_extracted() {
        let source = include_str!("../../tests/fixtures/sample.ex");
        let items = parse_and_extract(source);
        let t = find_by_name(&items, "internal_state");
        assert_eq!(t.kind, SymbolKind::TypeAlias);
        assert_eq!(t.visibility, Visibility::Private);
    }

    #[test]
    fn opaque_type_extracted() {
        let source = include_str!("../../tests/fixtures/sample.ex");
        let items = parse_and_extract(source);
        let t = find_by_name(&items, "wrapped");
        assert_eq!(t.kind, SymbolKind::TypeAlias);
        assert_eq!(t.visibility, Visibility::Public);
    }

    #[test]
    fn opaque_type_signature() {
        let source = include_str!("../../tests/fixtures/sample.ex");
        let items = parse_and_extract(source);
        let t = find_by_name(&items, "wrapped");
        assert!(
            t.signature.starts_with("@opaque"),
            "sig: {:?}",
            t.signature
        );
    }

    #[test]
    fn struct_type_t_extracted() {
        let source = include_str!("../../tests/fixtures/sample.ex");
        let items = parse_and_extract(source);
        // The Sample.Config module has @type t :: %__MODULE__{}
        let t_types: Vec<_> = items
            .iter()
            .filter(|i| i.name == "t" && i.kind == SymbolKind::TypeAlias)
            .collect();
        assert!(!t_types.is_empty(), "should extract @type t");
    }
}
