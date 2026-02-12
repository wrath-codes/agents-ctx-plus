//! C rich extractor.
//!
//! Extracts structurally significant elements from C source files:
//! functions (definitions and prototypes with static/inline/extern qualifiers),
//! structs (with field extraction, typedef structs), unions, enums (with
//! variant extraction), typedefs (simple, struct, function pointer),
//! global variables, constants, preprocessor directives (`#include`,
//! `#define` object-like and function-like macros, `#ifdef`/`#ifndef`,
//! `#pragma`), function pointers, bit fields, array declarations,
//! forward declarations, and doc comments (`/* */`, `/** */`, `//`).

use ast_grep_core::Node;
use ast_grep_language::SupportLang;
use std::fmt::Write as _;

use crate::types::{CMetadataExt, ParsedItem, SymbolKind, SymbolMetadata, Visibility};

/// Extract all significant elements from a C source file.
///
/// Walks the top-level `translation_unit` node collecting functions,
/// structs, unions, enums, typedefs, variables, constants, preprocessor
/// directives, and forward declarations.
///
/// # Errors
/// Returns `ParserError` if parsing fails.
#[allow(clippy::unnecessary_wraps)]
pub fn extract<D: ast_grep_core::Doc<Lang = SupportLang>>(
    root: &ast_grep_core::AstGrep<D>,
    source: &str,
) -> Result<Vec<ParsedItem>, crate::error::ParserError> {
    let mut items = Vec::new();
    let root_node = root.root();
    collect_top_level(&root_node, &mut items, source);
    Ok(items)
}

// ── Top-level node dispatcher ──────────────────────────────────────

fn collect_top_level<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    source: &str,
) {
    let children: Vec<_> = node.children().collect();

    for (idx, child) in children.iter().enumerate() {
        let kind = child.kind();
        match kind.as_ref() {
            "function_definition" => {
                let doc = collect_doc_comment(&children, idx, source);
                process_function_definition(child, items, &doc);
            }
            "declaration" => {
                let doc = collect_doc_comment(&children, idx, source);
                process_declaration(child, items, &doc);
            }
            "type_definition" => {
                let doc = collect_doc_comment(&children, idx, source);
                process_type_definition(child, items, &doc);
            }
            "struct_specifier" => {
                // Top-level `struct Foo { ... };` or `struct Foo;`
                let doc = collect_doc_comment(&children, idx, source);
                process_top_level_struct(child, items, &doc);
            }
            "union_specifier" => {
                let doc = collect_doc_comment(&children, idx, source);
                process_top_level_union(child, items, &doc);
            }
            "enum_specifier" => {
                let doc = collect_doc_comment(&children, idx, source);
                process_top_level_enum(child, items, &doc);
            }
            "preproc_include" => {
                process_preproc_include(child, items);
            }
            "preproc_def" => {
                let doc = collect_doc_comment(&children, idx, source);
                process_preproc_def(child, items, &doc);
            }
            "preproc_function_def" => {
                let doc = collect_doc_comment(&children, idx, source);
                process_preproc_function_def(child, items, &doc);
            }
            "preproc_ifdef" => {
                process_preproc_ifdef(child, items, source);
            }
            "preproc_if" => {
                process_preproc_if(child, items, source);
            }
            "preproc_call" => {
                process_preproc_call(child, items);
            }
            "expression_statement" => {
                // _Static_assert is parsed as expression_statement > call_expression
                let doc = collect_doc_comment(&children, idx, source);
                process_expression_statement(child, items, &doc);
            }
            _ => {} // Skip comments, punctuation, etc.
        }
    }
}

// ── Doc comment collection ─────────────────────────────────────────

/// Collect leading doc comments above a node.
///
/// Walks backward through siblings from `idx`, collecting contiguous
/// `comment` nodes. Supports `//`, `/* */`, and `/** */` styles.
/// Stops at any non-comment node or a blank-line gap.
fn collect_doc_comment<D: ast_grep_core::Doc>(
    siblings: &[Node<D>],
    idx: usize,
    _source: &str,
) -> String {
    let mut comments = Vec::new();
    let target_line = siblings[idx].start_pos().line();

    let mut i = idx;
    while i > 0 {
        i -= 1;
        let sibling = &siblings[i];
        if sibling.kind().as_ref() != "comment" {
            break;
        }

        // Check for line gap — comments must be contiguous
        let comment_end = sibling.end_pos().line();
        let next_start = if i + 1 < idx {
            siblings[i + 1].start_pos().line()
        } else {
            target_line
        };
        if next_start > comment_end + 1 {
            break;
        }

        let text = sibling.text().to_string();
        let stripped = strip_comment(&text);
        if !stripped.is_empty() {
            comments.push(stripped);
        }
    }

    comments.reverse();
    comments.join("\n")
}

/// Strip C comment markers from a comment string.
fn strip_comment(text: &str) -> String {
    let text = text.trim();

    // Single-line: // ...
    if let Some(rest) = text.strip_prefix("//") {
        return rest.trim().to_string();
    }

    // Multi-line: /* ... */ or /** ... */
    let inner = text
        .strip_prefix("/**")
        .or_else(|| text.strip_prefix("/*"))
        .unwrap_or(text);
    let inner = inner.strip_suffix("*/").unwrap_or(inner);

    // Process each line, stripping leading ` * ` decoration
    inner
        .lines()
        .map(|line| {
            let trimmed = line.trim();
            let stripped = trimmed
                .strip_prefix("* ")
                .unwrap_or_else(|| trimmed.strip_prefix('*').unwrap_or(trimmed));
            stripped.trim()
        })
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

// ── Source extraction helper ───────────────────────────────────────

/// Extract full source up to `max_lines` lines.
#[allow(clippy::unnecessary_wraps)]
fn extract_source_limited<D: ast_grep_core::Doc>(
    node: &Node<D>,
    max_lines: usize,
) -> Option<String> {
    let text = node.text().to_string();
    let lines: Vec<&str> = text.lines().collect();
    if lines.len() <= max_lines {
        Some(text)
    } else {
        let truncated: String = lines[..max_lines].join("\n");
        Some(format!(
            "{truncated}\n    // ... ({} more lines)",
            lines.len() - max_lines
        ))
    }
}

// ── Signature extraction ───────────────────────────────────────────

/// Extract signature from a node: everything before first `{`, whitespace-normalized.
fn extract_signature<D: ast_grep_core::Doc>(node: &Node<D>) -> String {
    let text = node.text().to_string();
    let brace = text.find('{');
    let semi = text.find(';');
    let end = match (brace, semi) {
        (Some(b), Some(s)) => b.min(s),
        (Some(b), None) => b,
        (None, Some(s)) => s,
        (None, None) => text.len(),
    };
    let sig = text[..end].trim();
    sig.split_whitespace().collect::<Vec<_>>().join(" ")
}

// ── Storage class / qualifier detection ────────────────────────────

/// Storage class specifiers and type qualifiers on a declaration.
#[allow(clippy::struct_excessive_bools)]
struct Qualifiers {
    is_static: bool,
    is_inline: bool,
    is_extern: bool,
    is_const: bool,
    is_volatile: bool,
    is_register: bool,
    /// GCC `__attribute__((…))` texts.
    gcc_attributes: Vec<String>,
    /// C11 qualifiers like `_Noreturn`, `_Atomic`, `restrict`, `_Alignas(…)`.
    c11_attrs: Vec<String>,
}

fn detect_qualifiers<D: ast_grep_core::Doc>(node: &Node<D>) -> Qualifiers {
    let mut q = Qualifiers {
        is_static: false,
        is_inline: false,
        is_extern: false,
        is_const: false,
        is_volatile: false,
        is_register: false,
        gcc_attributes: Vec::new(),
        c11_attrs: Vec::new(),
    };
    let children: Vec<_> = node.children().collect();
    for child in &children {
        match child.kind().as_ref() {
            "storage_class_specifier" => {
                let text = child.text();
                match text.as_ref() {
                    "static" => q.is_static = true,
                    "inline" => q.is_inline = true,
                    "extern" => q.is_extern = true,
                    "register" => q.is_register = true,
                    _ => {}
                }
            }
            "type_qualifier" => {
                let text = child.text();
                match text.as_ref() {
                    "const" => q.is_const = true,
                    "volatile" => q.is_volatile = true,
                    "_Noreturn" | "_Atomic" | "restrict" => {
                        q.c11_attrs.push(text.to_string());
                    }
                    other if other.starts_with("_Alignas") => {
                        q.c11_attrs.push(text.to_string());
                    }
                    _ => {}
                }
            }
            "attribute_specifier" => {
                q.gcc_attributes.push(child.text().to_string());
            }
            _ => {}
        }
    }
    q
}

/// Determine visibility from qualifiers.
const fn visibility_from_qualifiers(q: &Qualifiers) -> Visibility {
    if q.is_static {
        Visibility::Private
    } else {
        Visibility::Public
    }
}

/// Build attribute list from qualifiers.
fn attributes_from_qualifiers(q: &Qualifiers) -> Vec<String> {
    let mut metadata = SymbolMetadata::default();
    if q.is_static {
        metadata.push_attribute("static");
    }
    if q.is_inline {
        metadata.push_attribute("inline");
    }
    if q.is_extern {
        metadata.push_attribute("extern");
    }
    if q.is_const {
        metadata.push_attribute("const");
    }
    if q.is_volatile {
        metadata.push_attribute("volatile");
    }
    if q.is_register {
        metadata.push_attribute("register");
    }
    for attr in &q.gcc_attributes {
        metadata.push_attribute(attr.clone());
    }
    for eq in &q.c11_attrs {
        metadata.push_attribute(eq.clone());
    }
    metadata.attributes
}

// ── Function definition processing ────────────────────────────────

fn process_function_definition<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
) {
    let children: Vec<_> = node.children().collect();
    let q = detect_qualifiers(node);

    // Find the function_declarator child
    let Some(func_decl) = children
        .iter()
        .find(|c| c.kind().as_ref() == "function_declarator")
    else {
        return;
    };

    // Extract function name from the declarator
    let name = extract_declarator_name(func_decl);
    if name.is_empty() {
        return;
    }

    // Extract return type
    let return_type = extract_return_type(&children);

    // Extract parameters
    let parameters = extract_parameters(func_decl);

    // Check for variadic
    let is_variadic = func_decl.text().as_ref().contains("...");

    let mut metadata = SymbolMetadata {
        return_type,
        parameters,
        attributes: attributes_from_qualifiers(&q),
        ..Default::default()
    };
    if is_variadic {
        metadata.push_attribute("variadic");
    }

    items.push(ParsedItem {
        kind: SymbolKind::Function,
        name,
        signature: extract_signature(node),
        source: extract_source_limited(node, 30),
        doc_comment: doc_comment.to_string(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: visibility_from_qualifiers(&q),
        metadata,
    });
}

// ── Declaration processing (variables, prototypes, function pointers) ──

#[allow(clippy::too_many_lines)]
fn process_declaration<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
) {
    let children: Vec<_> = node.children().collect();
    let q = detect_qualifiers(node);

    // Check if this is a function declaration (prototype) vs function pointer variable.
    // A function pointer: `void (*callback)(int, int);` has function_declarator with
    // parenthesized_declarator child. A prototype: `int add(int a, int b);` has
    // function_declarator with an identifier child directly.
    if let Some(func_decl) = children
        .iter()
        .find(|c| c.kind().as_ref() == "function_declarator")
    {
        let is_fn_pointer = func_decl
            .children()
            .any(|c| c.kind().as_ref() == "parenthesized_declarator");
        if is_fn_pointer {
            process_function_pointer_var(node, items, doc_comment, &q);
        } else {
            process_function_prototype(node, items, doc_comment, &children, &q);
        }
        return;
    }

    // Check for init_declarator(s) (variable with initializer, or function pointer with init)
    let init_decls: Vec<_> = children
        .iter()
        .filter(|c| c.kind().as_ref() == "init_declarator")
        .collect();
    if !init_decls.is_empty() {
        // Check first one for function pointer
        if has_function_declarator_descendant(init_decls[0]) {
            process_function_pointer_var(node, items, doc_comment, &q);
        } else {
            for init_decl in &init_decls {
                let name = extract_init_declarator_name(init_decl);
                if name.is_empty() {
                    continue;
                }
                let return_type = extract_return_type(&children);
                let (kind, visibility) = classify_variable(&q);
                let mut metadata = SymbolMetadata {
                    return_type,
                    attributes: attributes_from_qualifiers(&q),
                    ..Default::default()
                };
                if init_decl
                    .children()
                    .any(|c| c.kind().as_ref() == "array_declarator")
                {
                    metadata.push_attribute("array");
                }
                items.push(ParsedItem {
                    kind,
                    name,
                    signature: extract_signature(node),
                    source: Some(node.text().to_string()),
                    doc_comment: doc_comment.to_string(),
                    start_line: node.start_pos().line() as u32 + 1,
                    end_line: node.end_pos().line() as u32 + 1,
                    visibility,
                    metadata,
                });
            }
        }
        return;
    }

    // Check for array_declarator
    let has_array_decl = children
        .iter()
        .any(|c| c.kind().as_ref() == "array_declarator");

    if has_array_decl {
        process_array_declaration(node, items, doc_comment, &children, &q);
        return;
    }

    // Check for pointer_declarator (could be a function pointer or pointer variable)
    let has_pointer_decl = children
        .iter()
        .any(|c| c.kind().as_ref() == "pointer_declarator");

    if has_pointer_decl {
        // Check if the pointer_declarator contains a function_declarator (function pointer var)
        if let Some(ptr) = children
            .iter()
            .find(|c| c.kind().as_ref() == "pointer_declarator")
            && has_function_declarator_descendant(ptr)
        {
            process_function_pointer_var(node, items, doc_comment, &q);
            return;
        }
        process_pointer_variable(node, items, doc_comment, &children, &q);
        return;
    }

    // Plain identifier declarations: `extern int shared;` or `int x, y, z;`
    let identifiers: Vec<_> = children
        .iter()
        .filter(|c| c.kind().as_ref() == "identifier")
        .collect();
    for id in &identifiers {
        let name = id.text().to_string();
        let return_type = extract_return_type(&children);
        let (kind, visibility) = classify_variable(&q);

        items.push(ParsedItem {
            kind,
            name,
            signature: extract_signature(node),
            source: Some(node.text().to_string()),
            doc_comment: doc_comment.to_string(),
            start_line: node.start_pos().line() as u32 + 1,
            end_line: node.end_pos().line() as u32 + 1,
            visibility,
            metadata: SymbolMetadata {
                return_type,
                attributes: attributes_from_qualifiers(&q),
                ..Default::default()
            },
        });
    }
}

/// Check if a node or any descendant contains a `function_declarator`.
fn has_function_declarator_descendant<D: ast_grep_core::Doc>(node: &Node<D>) -> bool {
    let children: Vec<_> = node.children().collect();
    for child in &children {
        if child.kind().as_ref() == "function_declarator" {
            return true;
        }
        if has_function_declarator_descendant(child) {
            return true;
        }
    }
    false
}

fn process_function_prototype<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
    children: &[Node<D>],
    q: &Qualifiers,
) {
    let Some(func_decl) = children
        .iter()
        .find(|c| c.kind().as_ref() == "function_declarator")
    else {
        return;
    };

    let name = extract_declarator_name(func_decl);
    if name.is_empty() {
        return;
    }

    let return_type = extract_return_type(children);
    let parameters = extract_parameters(func_decl);
    let is_variadic = func_decl.text().as_ref().contains("...");

    let mut metadata = SymbolMetadata {
        return_type,
        parameters,
        attributes: attributes_from_qualifiers(q),
        ..Default::default()
    };
    metadata.push_attribute("prototype");
    if is_variadic {
        metadata.push_attribute("variadic");
    }

    items.push(ParsedItem {
        kind: SymbolKind::Function,
        name,
        signature: extract_signature(node),
        source: Some(node.text().to_string()),
        doc_comment: doc_comment.to_string(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: visibility_from_qualifiers(q),
        metadata,
    });
}

fn process_array_declaration<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
    children: &[Node<D>],
    q: &Qualifiers,
) {
    let Some(arr_decl) = children
        .iter()
        .find(|c| c.kind().as_ref() == "array_declarator")
    else {
        return;
    };

    // Name may be nested for multi-dimensional arrays
    let name = extract_array_declarator_name(arr_decl);

    if name.is_empty() {
        return;
    }

    let return_type = extract_return_type(children);
    let (kind, visibility) = classify_variable(q);

    let mut metadata = SymbolMetadata {
        return_type,
        attributes: attributes_from_qualifiers(q),
        ..Default::default()
    };
    metadata.push_attribute("array");

    items.push(ParsedItem {
        kind,
        name,
        signature: extract_signature(node),
        source: Some(node.text().to_string()),
        doc_comment: doc_comment.to_string(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility,
        metadata,
    });
}

fn process_pointer_variable<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
    children: &[Node<D>],
    q: &Qualifiers,
) {
    // Find the identifier deep inside the pointer_declarator
    let Some(ptr_decl) = children
        .iter()
        .find(|c| c.kind().as_ref() == "pointer_declarator")
    else {
        return;
    };

    let name = extract_pointer_declarator_name(ptr_decl);
    if name.is_empty() {
        return;
    }

    let return_type = extract_return_type(children);
    let (kind, visibility) = classify_variable(q);

    items.push(ParsedItem {
        kind,
        name,
        signature: extract_signature(node),
        source: Some(node.text().to_string()),
        doc_comment: doc_comment.to_string(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility,
        metadata: SymbolMetadata {
            return_type,
            attributes: attributes_from_qualifiers(q),
            ..Default::default()
        },
    });
}

fn process_function_pointer_var<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
    q: &Qualifiers,
) {
    // A declaration like: void (*callback)(int, int) = NULL;
    // The name is deep inside: function_declarator > parenthesized_declarator > pointer_declarator > identifier
    let name = extract_function_pointer_name(node);
    if name.is_empty() {
        return;
    }

    let mut metadata = SymbolMetadata {
        attributes: attributes_from_qualifiers(q),
        ..Default::default()
    };
    metadata.push_attribute("function_pointer");

    items.push(ParsedItem {
        kind: SymbolKind::Static,
        name,
        signature: extract_signature(node),
        source: Some(node.text().to_string()),
        doc_comment: doc_comment.to_string(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: visibility_from_qualifiers(q),
        metadata,
    });
}

/// Extract the name from a function pointer declaration.
///
/// Traverses: `function_declarator` > `parenthesized_declarator` >
/// `pointer_declarator` > `identifier`.
fn extract_function_pointer_name<D: ast_grep_core::Doc>(node: &Node<D>) -> String {
    fn find_identifier_in<D2: ast_grep_core::Doc>(node: &Node<D2>) -> Option<String> {
        let children: Vec<_> = node.children().collect();
        for child in &children {
            match child.kind().as_ref() {
                "identifier" | "type_identifier" => return Some(child.text().to_string()),
                "function_declarator"
                | "parenthesized_declarator"
                | "pointer_declarator"
                | "init_declarator" => {
                    if let Some(name) = find_identifier_in(child) {
                        return Some(name);
                    }
                }
                _ => {}
            }
        }
        None
    }
    find_identifier_in(node).unwrap_or_default()
}

// ── Type definition processing ─────────────────────────────────────

fn process_type_definition<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
) {
    let children: Vec<_> = node.children().collect();

    // Find the typedef name (type_identifier at the end)
    let name = children
        .iter()
        .filter(|c| c.kind().as_ref() == "type_identifier")
        .last()
        .map_or_else(
            || {
                // Fallback: could be a primitive_type for things like `typedef unsigned long uint64_t`
                children
                    .iter()
                    .filter(|c| c.kind().as_ref() == "primitive_type")
                    .last()
                    .map_or_else(String::new, |n| n.text().to_string())
            },
            |n| n.text().to_string(),
        );

    if name.is_empty() {
        return;
    }

    // Check if this is a typedef struct or typedef enum
    let has_struct = children
        .iter()
        .any(|c| c.kind().as_ref() == "struct_specifier");
    let has_enum = children
        .iter()
        .any(|c| c.kind().as_ref() == "enum_specifier");
    let has_union = children
        .iter()
        .any(|c| c.kind().as_ref() == "union_specifier");
    let has_func_decl = children
        .iter()
        .any(|c| c.kind().as_ref() == "function_declarator")
        || children.iter().any(|c| {
            c.kind().as_ref() == "pointer_declarator" && has_function_declarator_descendant(c)
        });

    if has_func_decl {
        // For function pointer typedefs the name is nested deep inside
        // function_declarator > parenthesized_declarator > pointer_declarator > type_identifier.
        let fp_name = extract_function_pointer_name(node);
        let fp_name = if fp_name.is_empty() { &name } else { &fp_name };
        process_typedef_function_pointer(node, items, doc_comment, fp_name);
    } else if has_struct {
        // Without a body this is just an alias (e.g. `typedef struct Point Point2D;`)
        let has_body = specifier_has_body(&children, "struct_specifier", "field_declaration_list");
        if has_body {
            process_typedef_struct(node, items, doc_comment, &children, &name);
        } else {
            push_simple_typedef_alias(node, items, doc_comment, name);
        }
    } else if has_enum {
        let has_body = specifier_has_body(&children, "enum_specifier", "enumerator_list");
        if has_body {
            process_typedef_enum(node, items, doc_comment, &children, &name);
        } else {
            push_simple_typedef_alias(node, items, doc_comment, name);
        }
    } else if has_union {
        let has_body = specifier_has_body(&children, "union_specifier", "field_declaration_list");
        if has_body {
            process_typedef_union(node, items, doc_comment, &children, &name);
        } else {
            push_simple_typedef_alias(node, items, doc_comment, name);
        }
    } else {
        push_simple_typedef_alias(node, items, doc_comment, name);
    }
}

/// Check whether a specifier child (struct/union/enum) contains a body node.
fn specifier_has_body<D: ast_grep_core::Doc>(
    children: &[Node<D>],
    specifier_kind: &str,
    body_kind: &str,
) -> bool {
    children
        .iter()
        .find(|c| c.kind().as_ref() == specifier_kind)
        .is_some_and(|s| s.children().any(|c| c.kind().as_ref() == body_kind))
}

/// Emit a simple `TypeAlias` item for a typedef without a body.
fn push_simple_typedef_alias<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
    name: String,
) {
    items.push(ParsedItem {
        kind: SymbolKind::TypeAlias,
        name,
        signature: extract_signature(node),
        source: Some(node.text().to_string()),
        doc_comment: doc_comment.to_string(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata: SymbolMetadata {
            attributes: vec!["typedef".to_string()],
            ..Default::default()
        },
    });
}

fn process_typedef_struct<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
    children: &[Node<D>],
    name: &str,
) {
    let fields = children
        .iter()
        .find(|c| c.kind().as_ref() == "struct_specifier")
        .map_or_else(Vec::new, |s| extract_struct_fields(s));

    items.push(ParsedItem {
        kind: SymbolKind::Struct,
        name: name.to_string(),
        signature: extract_signature(node),
        source: extract_source_limited(node, 30),
        doc_comment: doc_comment.to_string(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata: SymbolMetadata {
            fields,
            attributes: vec!["typedef".to_string()],
            ..Default::default()
        },
    });
}

fn process_typedef_enum<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
    children: &[Node<D>],
    name: &str,
) {
    let variants = children
        .iter()
        .find(|c| c.kind().as_ref() == "enum_specifier")
        .map_or_else(Vec::new, |e| extract_enum_variants(e));

    items.push(ParsedItem {
        kind: SymbolKind::Enum,
        name: name.to_string(),
        signature: extract_signature(node),
        source: extract_source_limited(node, 30),
        doc_comment: doc_comment.to_string(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata: SymbolMetadata {
            variants,
            attributes: vec!["typedef".to_string()],
            ..Default::default()
        },
    });
}

fn process_typedef_union<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
    children: &[Node<D>],
    name: &str,
) {
    let fields = children
        .iter()
        .find(|c| c.kind().as_ref() == "union_specifier")
        .map_or_else(Vec::new, |u| extract_struct_fields(u));

    items.push(ParsedItem {
        kind: SymbolKind::Union,
        name: name.to_string(),
        signature: extract_signature(node),
        source: extract_source_limited(node, 30),
        doc_comment: doc_comment.to_string(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata: SymbolMetadata {
            fields,
            attributes: vec!["typedef".to_string()],
            ..Default::default()
        },
    });
}

fn process_typedef_function_pointer<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
    name: &str,
) {
    items.push(ParsedItem {
        kind: SymbolKind::TypeAlias,
        name: name.to_string(),
        signature: extract_signature(node),
        source: Some(node.text().to_string()),
        doc_comment: doc_comment.to_string(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata: SymbolMetadata {
            attributes: vec!["typedef".to_string(), "function_pointer".to_string()],
            ..Default::default()
        },
    });
}

// ── Top-level struct/union/enum processing ─────────────────────────

fn process_top_level_struct<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
) {
    let name = node
        .children()
        .find(|c| c.kind().as_ref() == "type_identifier")
        .map_or_else(String::new, |n| n.text().to_string());

    if name.is_empty() {
        return;
    }

    // Check if it has a body (field_declaration_list) or is a forward declaration
    let has_body = node
        .children()
        .any(|c| c.kind().as_ref() == "field_declaration_list");

    if has_body {
        let fields = extract_struct_fields(node);
        items.push(ParsedItem {
            kind: SymbolKind::Struct,
            name,
            signature: extract_signature(node),
            source: extract_source_limited(node, 30),
            doc_comment: doc_comment.to_string(),
            start_line: node.start_pos().line() as u32 + 1,
            end_line: node.end_pos().line() as u32 + 1,
            visibility: Visibility::Public,
            metadata: SymbolMetadata {
                fields,
                ..Default::default()
            },
        });
    } else {
        // Forward declaration: struct Foo;
        items.push(ParsedItem {
            kind: SymbolKind::Struct,
            name,
            signature: format!(
                "struct {}",
                node.text().as_ref().trim_end_matches(';').trim()
            ),
            source: Some(node.text().to_string()),
            doc_comment: doc_comment.to_string(),
            start_line: node.start_pos().line() as u32 + 1,
            end_line: node.end_pos().line() as u32 + 1,
            visibility: Visibility::Public,
            metadata: SymbolMetadata {
                attributes: vec!["forward_declaration".to_string()],
                ..Default::default()
            },
        });
    }
}

fn process_top_level_union<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
) {
    let name = node
        .children()
        .find(|c| c.kind().as_ref() == "type_identifier")
        .map_or_else(String::new, |n| n.text().to_string());

    if name.is_empty() {
        return;
    }

    let fields = extract_struct_fields(node);

    items.push(ParsedItem {
        kind: SymbolKind::Union,
        name,
        signature: extract_signature(node),
        source: extract_source_limited(node, 30),
        doc_comment: doc_comment.to_string(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata: SymbolMetadata {
            fields,
            ..Default::default()
        },
    });
}

fn process_top_level_enum<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
) {
    let name = node
        .children()
        .find(|c| c.kind().as_ref() == "type_identifier")
        .map_or_else(String::new, |n| n.text().to_string());

    if name.is_empty() {
        return;
    }

    let has_body = node
        .children()
        .any(|c| c.kind().as_ref() == "enumerator_list");

    if has_body {
        let variants = extract_enum_variants(node);
        items.push(ParsedItem {
            kind: SymbolKind::Enum,
            name,
            signature: extract_signature(node),
            source: extract_source_limited(node, 30),
            doc_comment: doc_comment.to_string(),
            start_line: node.start_pos().line() as u32 + 1,
            end_line: node.end_pos().line() as u32 + 1,
            visibility: Visibility::Public,
            metadata: SymbolMetadata {
                variants,
                ..Default::default()
            },
        });
    } else {
        // Forward declaration: enum Foo;
        items.push(ParsedItem {
            kind: SymbolKind::Enum,
            name,
            signature: format!("enum {}", node.text().as_ref().trim_end_matches(';').trim()),
            source: Some(node.text().to_string()),
            doc_comment: doc_comment.to_string(),
            start_line: node.start_pos().line() as u32 + 1,
            end_line: node.end_pos().line() as u32 + 1,
            visibility: Visibility::Public,
            metadata: SymbolMetadata {
                attributes: vec!["forward_declaration".to_string()],
                ..Default::default()
            },
        });
    }
}

// ── Field / variant extraction ─────────────────────────────────────

/// Extract field names from a struct or union.
fn extract_struct_fields<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    let mut fields = Vec::new();
    let children: Vec<_> = node.children().collect();
    for child in &children {
        if child.kind().as_ref() == "field_declaration_list" {
            let field_children: Vec<_> = child.children().collect();
            for fc in &field_children {
                if fc.kind().as_ref() == "field_declaration" {
                    // Field name may be nested inside pointer_declarator or
                    // array_declarator, so search recursively.
                    if let Some(field_name) = find_field_identifier(fc) {
                        // Check for bit field
                        let has_bitfield = fc
                            .children()
                            .any(|c| c.kind().as_ref() == "bitfield_clause");

                        if has_bitfield {
                            fields.push(format!("{field_name} (bitfield)"));
                        } else {
                            fields.push(field_name);
                        }
                    } else {
                        // Anonymous struct/union inside a field declaration
                        let fc_children: Vec<_> = fc.children().collect();
                        if fc_children
                            .iter()
                            .any(|c| c.kind().as_ref() == "struct_specifier")
                        {
                            fields.push("(anonymous struct)".to_string());
                        } else if fc_children
                            .iter()
                            .any(|c| c.kind().as_ref() == "union_specifier")
                        {
                            fields.push("(anonymous union)".to_string());
                        }
                    }
                }
            }
        }
    }
    fields
}

/// Recursively find a `field_identifier` inside a node.
///
/// Stops at nested `struct_specifier`, `union_specifier`, and
/// `enum_specifier` boundaries to avoid descending into anonymous
/// aggregate members.
fn find_field_identifier<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<String> {
    let children: Vec<_> = node.children().collect();
    for child in &children {
        if child.kind().as_ref() == "field_identifier" {
            return Some(child.text().to_string());
        }
        // Do not descend into nested aggregate types — they belong
        // to anonymous struct/union members handled separately.
        let k = child.kind();
        if matches!(
            k.as_ref(),
            "struct_specifier" | "union_specifier" | "enum_specifier"
        ) {
            continue;
        }
        if let Some(name) = find_field_identifier(child) {
            return Some(name);
        }
    }
    None
}

/// Extract variant names from an enum.
fn extract_enum_variants<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    let mut variants = Vec::new();
    let children: Vec<_> = node.children().collect();
    for child in &children {
        if child.kind().as_ref() == "enumerator_list" {
            let list_children: Vec<_> = child.children().collect();
            for lc in &list_children {
                if lc.kind().as_ref() == "enumerator"
                    && let Some(id) = lc.children().find(|c| c.kind().as_ref() == "identifier")
                {
                    variants.push(id.text().to_string());
                }
            }
        }
    }
    variants
}

// ── Preprocessor processing ────────────────────────────────────────

fn process_preproc_include<D: ast_grep_core::Doc>(node: &Node<D>, items: &mut Vec<ParsedItem>) {
    let children: Vec<_> = node.children().collect();

    // The path is either system_lib_string or string_literal
    let path = children
        .iter()
        .find(|c| {
            let k = c.kind();
            k.as_ref() == "system_lib_string" || k.as_ref() == "string_literal"
        })
        .map_or_else(String::new, |n| n.text().to_string());

    if path.is_empty() {
        return;
    }

    let is_system = children
        .iter()
        .any(|c| c.kind().as_ref() == "system_lib_string");

    let mut metadata = SymbolMetadata::default();
    metadata.push_attribute("include");
    if is_system {
        metadata.push_attribute("system");
    } else {
        metadata.push_attribute("local");
    }

    items.push(ParsedItem {
        kind: SymbolKind::Module,
        name: path,
        signature: node.text().to_string().trim().to_string(),
        source: Some(node.text().to_string()),
        doc_comment: String::new(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata,
    });
}

fn process_preproc_def<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
) {
    let children: Vec<_> = node.children().collect();

    let name = children
        .iter()
        .find(|c| c.kind().as_ref() == "identifier")
        .map_or_else(String::new, |n| n.text().to_string());

    if name.is_empty() {
        return;
    }

    let has_value = children.iter().any(|c| c.kind().as_ref() == "preproc_arg");

    let value = children
        .iter()
        .find(|c| c.kind().as_ref() == "preproc_arg")
        .map(|n| n.text().to_string().trim().to_string());

    // Object-like macros with a value are constants; header guard defines without value are macros
    let kind = if has_value {
        SymbolKind::Const
    } else {
        SymbolKind::Macro
    };

    let mut signature = String::new();
    let _ = write!(signature, "#define {name}");
    if let Some(ref v) = value {
        let _ = write!(signature, " {v}");
    }

    items.push(ParsedItem {
        kind,
        name,
        signature,
        source: Some(node.text().to_string()),
        doc_comment: doc_comment.to_string(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata: SymbolMetadata {
            attributes: vec!["define".to_string()],
            ..Default::default()
        },
    });
}

fn process_preproc_function_def<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
) {
    let children: Vec<_> = node.children().collect();

    let name = children
        .iter()
        .find(|c| c.kind().as_ref() == "identifier")
        .map_or_else(String::new, |n| n.text().to_string());

    if name.is_empty() {
        return;
    }

    let params = children
        .iter()
        .find(|c| c.kind().as_ref() == "preproc_params")
        .map_or_else(String::new, |n| n.text().to_string());

    let body = children
        .iter()
        .find(|c| c.kind().as_ref() == "preproc_arg")
        .map(|n| n.text().to_string().trim().to_string());

    let mut signature = String::new();
    let _ = write!(signature, "#define {name}{params}");
    if let Some(ref b) = body {
        let _ = write!(signature, " {b}");
    }

    // Extract parameter names
    let param_names: Vec<String> = children
        .iter()
        .find(|c| c.kind().as_ref() == "preproc_params")
        .map_or_else(Vec::new, |p| {
            p.children()
                .filter(|c| c.kind().as_ref() == "identifier" || c.kind().as_ref() == "...")
                .map(|c| c.text().to_string())
                .collect()
        });

    items.push(ParsedItem {
        kind: SymbolKind::Macro,
        name,
        signature,
        source: Some(node.text().to_string()),
        doc_comment: doc_comment.to_string(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata: SymbolMetadata {
            parameters: param_names,
            attributes: vec!["define".to_string(), "function_like".to_string()],
            ..Default::default()
        },
    });
}

fn process_preproc_ifdef<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    source: &str,
) {
    let children: Vec<_> = node.children().collect();

    // Determine if it's #ifdef or #ifndef
    let is_ifndef = children.iter().any(|c| c.kind().as_ref() == "#ifndef");

    let condition_name = children
        .iter()
        .find(|c| c.kind().as_ref() == "identifier")
        .map_or_else(String::new, |n| n.text().to_string());

    if condition_name.is_empty() {
        return;
    }

    let directive = if is_ifndef { "#ifndef" } else { "#ifdef" };

    items.push(ParsedItem {
        kind: SymbolKind::Macro,
        name: condition_name.clone(),
        signature: format!("{directive} {condition_name}"),
        source: extract_source_limited(node, 5),
        doc_comment: String::new(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata: SymbolMetadata {
            attributes: vec![directive.to_string()],
            ..Default::default()
        },
    });

    // Also process any children inside the ifdef block (same dispatch as top-level)
    process_ifdef_children(&children, items, source);
}

fn process_preproc_if<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    source: &str,
) {
    let children: Vec<_> = node.children().collect();

    // Extract condition: first child that isn't `#if`
    let condition = children
        .iter()
        .find(|c| {
            let k = c.kind();
            k.as_ref() != "#if"
                && k.as_ref() != "#endif"
                && k.as_ref() != "preproc_elif"
                && k.as_ref() != "preproc_else"
                && !matches!(
                    k.as_ref(),
                    "function_definition"
                        | "declaration"
                        | "type_definition"
                        | "struct_specifier"
                        | "union_specifier"
                        | "enum_specifier"
                        | "preproc_include"
                        | "preproc_def"
                        | "preproc_function_def"
                        | "preproc_ifdef"
                        | "preproc_if"
                        | "preproc_call"
                        | "expression_statement"
                        | "comment"
                )
        })
        .map_or_else(String::new, |n| n.text().to_string());

    if !condition.is_empty() {
        items.push(ParsedItem {
            kind: SymbolKind::Macro,
            name: condition.clone(),
            signature: format!("#if {condition}"),
            source: extract_source_limited(node, 5),
            doc_comment: String::new(),
            start_line: node.start_pos().line() as u32 + 1,
            end_line: node.end_pos().line() as u32 + 1,
            visibility: Visibility::Public,
            metadata: SymbolMetadata {
                attributes: vec!["#if".to_string()],
                ..Default::default()
            },
        });
    }

    // Process declarations inside the #if block
    process_ifdef_children(&children, items, source);

    // Handle nested preproc_elif and preproc_else
    for child in &children {
        match child.kind().as_ref() {
            "preproc_elif" => process_preproc_elif(child, items, source),
            "preproc_else" => process_preproc_else(child, items, source),
            _ => {}
        }
    }
}

fn process_preproc_elif<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    source: &str,
) {
    let children: Vec<_> = node.children().collect();

    // Extract condition (skip the `#elif` keyword itself)
    let condition = children
        .iter()
        .find(|c| {
            let k = c.kind();
            k.as_ref() != "#elif"
                && !matches!(
                    k.as_ref(),
                    "function_definition"
                        | "declaration"
                        | "type_definition"
                        | "struct_specifier"
                        | "union_specifier"
                        | "enum_specifier"
                        | "preproc_include"
                        | "preproc_def"
                        | "preproc_function_def"
                        | "preproc_ifdef"
                        | "preproc_if"
                        | "preproc_call"
                        | "expression_statement"
                        | "comment"
                        | "preproc_else"
                )
        })
        .map_or_else(String::new, |n| n.text().to_string());

    if !condition.is_empty() {
        items.push(ParsedItem {
            kind: SymbolKind::Macro,
            name: condition.clone(),
            signature: format!("#elif {condition}"),
            source: extract_source_limited(node, 5),
            doc_comment: String::new(),
            start_line: node.start_pos().line() as u32 + 1,
            end_line: node.end_pos().line() as u32 + 1,
            visibility: Visibility::Public,
            metadata: SymbolMetadata {
                attributes: vec!["#elif".to_string()],
                ..Default::default()
            },
        });
    }

    // Process declarations inside the #elif block
    process_ifdef_children(&children, items, source);

    // Handle nested preproc_else
    for child in &children {
        if child.kind().as_ref() == "preproc_else" {
            process_preproc_else(child, items, source);
        }
    }
}

fn process_preproc_else<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    source: &str,
) {
    let children: Vec<_> = node.children().collect();
    process_ifdef_children(&children, items, source);
}

/// Process children inside a `preproc_ifdef` block.
///
/// Uses the same dispatch logic as the top-level walker so that
/// structs, enums, unions, typedefs, etc. inside `#ifndef` guards
/// are correctly extracted.
fn process_ifdef_children<D: ast_grep_core::Doc>(
    children: &[Node<D>],
    items: &mut Vec<ParsedItem>,
    source: &str,
) {
    for (idx, child) in children.iter().enumerate() {
        let kind = child.kind();
        match kind.as_ref() {
            "function_definition" => {
                let doc = collect_doc_comment(children, idx, source);
                process_function_definition(child, items, &doc);
            }
            "declaration" => {
                let doc = collect_doc_comment(children, idx, source);
                process_declaration(child, items, &doc);
            }
            "type_definition" => {
                let doc = collect_doc_comment(children, idx, source);
                process_type_definition(child, items, &doc);
            }
            "struct_specifier" => {
                let doc = collect_doc_comment(children, idx, source);
                process_top_level_struct(child, items, &doc);
            }
            "union_specifier" => {
                let doc = collect_doc_comment(children, idx, source);
                process_top_level_union(child, items, &doc);
            }
            "enum_specifier" => {
                let doc = collect_doc_comment(children, idx, source);
                process_top_level_enum(child, items, &doc);
            }
            "preproc_include" => {
                process_preproc_include(child, items);
            }
            "preproc_def" => {
                let doc = collect_doc_comment(children, idx, source);
                process_preproc_def(child, items, &doc);
            }
            "preproc_function_def" => {
                let doc = collect_doc_comment(children, idx, source);
                process_preproc_function_def(child, items, &doc);
            }
            "preproc_ifdef" => {
                process_preproc_ifdef(child, items, source);
            }
            "preproc_if" => {
                process_preproc_if(child, items, source);
            }
            "preproc_call" => {
                process_preproc_call(child, items);
            }
            "expression_statement" => {
                let doc = collect_doc_comment(children, idx, source);
                process_expression_statement(child, items, &doc);
            }
            _ => {}
        }
    }
}

fn process_preproc_call<D: ast_grep_core::Doc>(node: &Node<D>, items: &mut Vec<ParsedItem>) {
    let children: Vec<_> = node.children().collect();

    let directive = children
        .iter()
        .find(|c| c.kind().as_ref() == "preproc_directive")
        .map_or_else(String::new, |n| n.text().to_string());

    if directive.is_empty() {
        return;
    }

    let args = children
        .iter()
        .find(|c| c.kind().as_ref() == "preproc_arg")
        .map(|n| n.text().to_string().trim().to_string());

    let name = args.as_deref().unwrap_or(&directive).to_string();

    let mut signature = directive.clone();
    if let Some(ref a) = args {
        let _ = write!(signature, " {a}");
    }

    items.push(ParsedItem {
        kind: SymbolKind::Macro,
        name,
        signature,
        source: Some(node.text().to_string()),
        doc_comment: String::new(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata: SymbolMetadata {
            attributes: vec![directive],
            ..Default::default()
        },
    });
}

// ── Expression statement processing (_Static_assert) ───────────────

fn process_expression_statement<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
) {
    // Look for call_expression with _Static_assert
    let children: Vec<_> = node.children().collect();
    let Some(call) = children
        .iter()
        .find(|c| c.kind().as_ref() == "call_expression")
    else {
        return;
    };

    let call_name = call
        .children()
        .find(|c| c.kind().as_ref() == "identifier")
        .map(|n| n.text().to_string());

    if call_name.as_deref() != Some("_Static_assert") {
        return;
    }

    items.push(ParsedItem {
        kind: SymbolKind::Macro,
        name: "_Static_assert".to_string(),
        signature: node
            .text()
            .to_string()
            .trim_end_matches(';')
            .trim()
            .to_string(),
        source: Some(node.text().to_string()),
        doc_comment: doc_comment.to_string(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata: SymbolMetadata {
            attributes: vec!["static_assert".to_string()],
            ..Default::default()
        },
    });
}

// ── Helper: name extraction from declarators ───────────────────────

/// Extract the function name from a `function_declarator` node.
fn extract_declarator_name<D: ast_grep_core::Doc>(node: &Node<D>) -> String {
    node.children()
        .find(|c| c.kind().as_ref() == "identifier")
        .map_or_else(String::new, |n| n.text().to_string())
}

/// Extract the variable name from an `init_declarator` node.
///
/// Handles direct identifiers, pointer declarators, and array declarators.
fn extract_init_declarator_name<D: ast_grep_core::Doc>(node: &Node<D>) -> String {
    // Direct identifier
    if let Some(id) = node.children().find(|c| c.kind().as_ref() == "identifier") {
        return id.text().to_string();
    }

    // Pointer declarator: *VERSION = "..."
    if let Some(ptr) = node
        .children()
        .find(|c| c.kind().as_ref() == "pointer_declarator")
    {
        return extract_pointer_declarator_name(&ptr);
    }

    // Array declarator: prime_numbers[10] = {2, 3, ...}
    if let Some(arr) = node
        .children()
        .find(|c| c.kind().as_ref() == "array_declarator")
    {
        return extract_array_declarator_name(&arr);
    }

    String::new()
}

/// Recursively extract identifier from an `array_declarator` (may be nested for multi-dim).
fn extract_array_declarator_name<D: ast_grep_core::Doc>(node: &Node<D>) -> String {
    let children: Vec<_> = node.children().collect();
    for child in &children {
        match child.kind().as_ref() {
            "identifier" => return child.text().to_string(),
            "array_declarator" => return extract_array_declarator_name(child),
            _ => {}
        }
    }
    String::new()
}

/// Extract identifier from a `pointer_declarator` (may be nested).
fn extract_pointer_declarator_name<D: ast_grep_core::Doc>(node: &Node<D>) -> String {
    let children: Vec<_> = node.children().collect();
    for child in &children {
        match child.kind().as_ref() {
            "identifier" => return child.text().to_string(),
            "array_declarator" => {
                // *argv[] — name is inside the array_declarator
                if let Some(id) = child.children().find(|c| c.kind().as_ref() == "identifier") {
                    return id.text().to_string();
                }
            }
            "pointer_declarator" => {
                return extract_pointer_declarator_name(child);
            }
            _ => {}
        }
    }
    String::new()
}

/// Extract the return type from declaration children.
fn extract_return_type<D: ast_grep_core::Doc>(children: &[Node<D>]) -> Option<String> {
    let mut parts = Vec::new();
    for child in children {
        match child.kind().as_ref() {
            "primitive_type"
            | "type_identifier"
            | "sized_type_specifier"
            | "type_qualifier"
            | "struct_specifier" => {
                parts.push(child.text().to_string());
            }
            // Stop at declarators or semicolons
            "function_declarator"
            | "init_declarator"
            | "identifier"
            | "array_declarator"
            | "pointer_declarator"
            | ";" => break,
            _ => {}
        }
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" "))
    }
}

/// Extract parameter names from a `function_declarator`'s `parameter_list`.
fn extract_parameters<D: ast_grep_core::Doc>(func_decl: &Node<D>) -> Vec<String> {
    let children: Vec<_> = func_decl.children().collect();
    let Some(param_list) = children
        .iter()
        .find(|c| c.kind().as_ref() == "parameter_list")
    else {
        return Vec::new();
    };

    let params: Vec<_> = param_list.children().collect();
    params
        .iter()
        .filter(|c| {
            let k = c.kind();
            k.as_ref() == "parameter_declaration" || k.as_ref() == "variadic_parameter"
        })
        .map(|c| {
            let text = c.text().to_string();
            text.split_whitespace().collect::<Vec<_>>().join(" ")
        })
        .collect()
}

/// Classify a variable by its qualifiers into (kind, visibility).
const fn classify_variable(q: &Qualifiers) -> (SymbolKind, Visibility) {
    if q.is_const {
        (SymbolKind::Const, visibility_from_qualifiers(q))
    } else {
        (SymbolKind::Static, visibility_from_qualifiers(q))
    }
}

// ── Tests ──────────────────────────────────────────────────────────
