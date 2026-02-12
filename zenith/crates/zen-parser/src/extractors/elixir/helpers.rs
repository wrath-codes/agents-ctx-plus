use ast_grep_core::Node;

pub(super) fn first_identifier_text<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<String> {
    node.children()
        .find(|c| c.kind().as_ref() == "identifier")
        .map(|n| n.text().to_string())
}

/// Extract module name from a `defmodule`/`defprotocol` call's `arguments` → `alias`.
pub(super) fn extract_module_name<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<String> {
    let args = node.children().find(|c| c.kind().as_ref() == "arguments")?;
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
pub(super) fn extract_def_name<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<String> {
    let args = node.children().find(|c| c.kind().as_ref() == "arguments")?;

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
pub(super) fn extract_def_params<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    let Some(args) = node.children().find(|c| c.kind().as_ref() == "arguments") else {
        return Vec::new();
    };

    // Find the inner call (possibly inside a binary_operator for guards)
    let inner_call = find_inner_def_call(&args);
    let Some(call) = inner_call else {
        return Vec::new();
    };

    // The call's `arguments` child has the actual params
    let Some(params_node) = call.children().find(|c| c.kind().as_ref() == "arguments") else {
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
pub(super) fn find_inner_def_call<'a, D: ast_grep_core::Doc>(
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
pub(super) fn extract_guard<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<String> {
    let args = node.children().find(|c| c.kind().as_ref() == "arguments")?;

    for child in args.children() {
        if child.kind().as_ref() == "binary_operator" {
            // Check if it contains a `when` keyword
            let has_when = child.children().any(|c| c.kind().as_ref() == "when");
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
pub(super) fn extract_elixir_doc<D: ast_grep_core::Doc>(node: &Node<D>) -> String {
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
pub(super) fn extract_moduledoc<D: ast_grep_core::Doc>(node: &Node<D>) -> String {
    let Some(do_block) = node.children().find(|c| c.kind().as_ref() == "do_block") else {
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
pub(super) fn try_extract_doc_attr<D: ast_grep_core::Doc>(
    node: &Node<D>,
    attr_name: &str,
) -> Option<String> {
    // Structure: unary_operator → @ + call(identifier=attr_name, arguments(string|boolean))
    let call_node = node.children().find(|c| c.kind().as_ref() == "call")?;

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
pub(super) fn is_at_attribute<D: ast_grep_core::Doc>(node: &Node<D>) -> bool {
    node.children().any(|c| c.kind().as_ref() == "@")
}

/// Extract string content from a `string` node (handles both `"..."` and `"""..."""`).
pub(super) fn extract_string_content<D: ast_grep_core::Doc>(node: &Node<D>) -> String {
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
pub(super) fn build_elixir_signature<D: ast_grep_core::Doc>(
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
pub(super) fn extract_spec<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<String> {
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
pub(super) fn try_extract_at_attr_text<D: ast_grep_core::Doc>(
    node: &Node<D>,
    attr_name: &str,
) -> Option<String> {
    let call_node = node.children().find(|c| c.kind().as_ref() == "call")?;

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
pub(super) fn has_impl_attr<D: ast_grep_core::Doc>(node: &Node<D>) -> bool {
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
pub(super) fn is_impl_true<D: ast_grep_core::Doc>(node: &Node<D>) -> bool {
    let call_node = node.children().find(|c| c.kind().as_ref() == "call");
    let Some(call) = call_node else {
        return false;
    };
    let id = call.children().find(|c| c.kind().as_ref() == "identifier");
    let Some(id) = id else {
        return false;
    };
    if id.text().as_ref() != "impl" {
        return false;
    }
    let args = call.children().find(|c| c.kind().as_ref() == "arguments");
    args.is_some_and(|a| {
        a.children()
            .any(|c| c.kind().as_ref() == "boolean" && c.text().as_ref() == "true")
    })
}

/// Check if a module's `do_block` contains a call with the given keyword.
pub(super) fn module_has_keyword<D: ast_grep_core::Doc>(node: &Node<D>, keyword: &str) -> bool {
    let Some(do_block) = node.children().find(|c| c.kind().as_ref() == "do_block") else {
        return false;
    };

    do_block.children().any(|child| {
        child.kind().as_ref() == "call" && first_identifier_text(&child).as_deref() == Some(keyword)
    })
}

/// Extract `@callback` definitions from a module's `do_block`.
pub(super) fn extract_callbacks<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    let Some(do_block) = node.children().find(|c| c.kind().as_ref() == "do_block") else {
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
pub(super) fn extract_callback_name(spec_text: &str) -> Option<String> {
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
pub(super) fn extract_struct_fields_from_module<D: ast_grep_core::Doc>(
    node: &Node<D>,
) -> Vec<String> {
    let Some(do_block) = node.children().find(|c| c.kind().as_ref() == "do_block") else {
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
pub(super) fn extract_defstruct_fields<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    let Some(args) = node.children().find(|c| c.kind().as_ref() == "arguments") else {
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
pub(super) fn extract_module_methods<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    let Some(do_block) = node.children().find(|c| c.kind().as_ref() == "do_block") else {
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
