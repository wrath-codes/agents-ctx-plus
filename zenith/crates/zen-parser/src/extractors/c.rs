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

use crate::types::{ParsedItem, SymbolKind, SymbolMetadata, Visibility};

/// Extract all significant elements from a C source file.
///
/// Walks the top-level `translation_unit` node collecting functions,
/// structs, unions, enums, typedefs, variables, constants, preprocessor
/// directives, and forward declarations.
///
/// # Errors
/// Returns `ParserError` if parsing fails.
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
    let mut attrs = Vec::new();
    if q.is_static {
        attrs.push("static".to_string());
    }
    if q.is_inline {
        attrs.push("inline".to_string());
    }
    if q.is_extern {
        attrs.push("extern".to_string());
    }
    if q.is_const {
        attrs.push("const".to_string());
    }
    if q.is_volatile {
        attrs.push("volatile".to_string());
    }
    if q.is_register {
        attrs.push("register".to_string());
    }
    for attr in &q.gcc_attributes {
        attrs.push(attr.clone());
    }
    for eq in &q.c11_attrs {
        attrs.push(eq.clone());
    }
    attrs
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

    let mut attrs = attributes_from_qualifiers(&q);
    if is_variadic {
        attrs.push("variadic".to_string());
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
        metadata: SymbolMetadata {
            return_type,
            parameters,
            attributes: attrs,
            ..Default::default()
        },
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
                let mut attrs = attributes_from_qualifiers(&q);
                if init_decl
                    .children()
                    .any(|c| c.kind().as_ref() == "array_declarator")
                {
                    attrs.push("array".to_string());
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
                    metadata: SymbolMetadata {
                        return_type,
                        attributes: attrs,
                        ..Default::default()
                    },
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

    let mut attrs = attributes_from_qualifiers(q);
    attrs.push("prototype".to_string());
    if is_variadic {
        attrs.push("variadic".to_string());
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
        metadata: SymbolMetadata {
            return_type,
            parameters,
            attributes: attrs,
            ..Default::default()
        },
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

    let mut attrs = attributes_from_qualifiers(q);
    attrs.push("array".to_string());

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
            attributes: attrs,
            ..Default::default()
        },
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

    let mut attrs = attributes_from_qualifiers(q);
    attrs.push("function_pointer".to_string());

    items.push(ParsedItem {
        kind: SymbolKind::Static,
        name,
        signature: extract_signature(node),
        source: Some(node.text().to_string()),
        doc_comment: doc_comment.to_string(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: visibility_from_qualifiers(q),
        metadata: SymbolMetadata {
            attributes: attrs,
            ..Default::default()
        },
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

    let mut attrs = vec!["include".to_string()];
    if is_system {
        attrs.push("system".to_string());
    } else {
        attrs.push("local".to_string());
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
        metadata: SymbolMetadata {
            attributes: attrs,
            ..Default::default()
        },
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

#[cfg(test)]
mod tests {
    use super::*;
    use ast_grep_language::LanguageExt;

    fn parse_and_extract(source: &str) -> Vec<ParsedItem> {
        let root = SupportLang::C.ast_grep(source);
        extract(&root, source).expect("extraction should succeed")
    }

    fn find_by_name<'a>(items: &'a [ParsedItem], name: &str) -> &'a ParsedItem {
        items.iter().find(|i| i.name == name).unwrap_or_else(|| {
            let available: Vec<_> = items
                .iter()
                .map(|i| format!("{:?}: {}", i.kind, &i.name))
                .collect();
            panic!(
                "item {name:?} not found. Available items:\n{}",
                available.join("\n")
            );
        })
    }

    fn find_all_by_kind(items: &[ParsedItem], kind: SymbolKind) -> Vec<&ParsedItem> {
        items.iter().filter(|i| i.kind == kind).collect()
    }

    fn find_by_name_prefix<'a>(items: &'a [ParsedItem], prefix: &str) -> Vec<&'a ParsedItem> {
        items
            .iter()
            .filter(|i| i.name.starts_with(prefix))
            .collect()
    }

    // ── Fixture parsing ───────────────────────────────────────────

    #[test]
    fn fixture_parses_without_error() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        assert!(items.len() >= 40, "expected 40+ items, got {}", items.len());
    }

    #[test]
    fn fixture_has_functions() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let functions = find_all_by_kind(&items, SymbolKind::Function);
        assert!(
            functions.len() >= 10,
            "expected 10+ functions, got {}",
            functions.len()
        );
    }

    #[test]
    fn fixture_has_structs() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let structs = find_all_by_kind(&items, SymbolKind::Struct);
        assert!(
            structs.len() >= 4,
            "expected 4+ structs, got {}",
            structs.len()
        );
    }

    #[test]
    fn fixture_has_enums() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let enums = find_all_by_kind(&items, SymbolKind::Enum);
        assert!(enums.len() >= 3, "expected 3+ enums, got {}", enums.len());
    }

    #[test]
    fn fixture_has_unions() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let unions = find_all_by_kind(&items, SymbolKind::Union);
        assert!(
            unions.len() >= 2,
            "expected 2+ unions, got {}",
            unions.len()
        );
    }

    #[test]
    fn fixture_has_typedefs() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let typedefs = items
            .iter()
            .filter(|i| i.kind == SymbolKind::TypeAlias)
            .count();
        assert!(typedefs >= 3, "expected 3+ typedefs, got {typedefs}",);
    }

    #[test]
    fn fixture_has_modules() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let modules = find_all_by_kind(&items, SymbolKind::Module);
        assert!(
            modules.len() >= 5,
            "expected 5+ #include modules, got {}",
            modules.len()
        );
    }

    #[test]
    fn fixture_has_macros() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let macros = find_all_by_kind(&items, SymbolKind::Macro);
        assert!(
            macros.len() >= 5,
            "expected 5+ macros, got {}",
            macros.len()
        );
    }

    // ── Function tests ────────────────────────────────────────────

    #[test]
    fn function_add_extracted() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let funcs: Vec<_> = items
            .iter()
            .filter(|i| i.name == "add" && i.kind == SymbolKind::Function)
            .collect();
        // One prototype + one definition
        assert!(
            funcs.len() >= 2,
            "expected at least 2 'add' items (prototype + def), got {}",
            funcs.len()
        );
    }

    #[test]
    fn function_add_has_params() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let add_def = items
            .iter()
            .find(|i| i.name == "add" && !i.metadata.attributes.contains(&"prototype".to_string()))
            .expect("should find add definition");
        assert_eq!(
            add_def.metadata.parameters.len(),
            2,
            "add should have 2 params: {:?}",
            add_def.metadata.parameters
        );
    }

    #[test]
    fn function_add_return_type() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let add_def = items
            .iter()
            .find(|i| i.name == "add" && !i.metadata.attributes.contains(&"prototype".to_string()))
            .expect("should find add definition");
        assert_eq!(
            add_def.metadata.return_type.as_deref(),
            Some("int"),
            "add should return int"
        );
    }

    #[test]
    fn function_add_doc_comment() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let add_def = items
            .iter()
            .find(|i| i.name == "add" && !i.metadata.attributes.contains(&"prototype".to_string()))
            .expect("should find add definition");
        assert!(
            add_def.doc_comment.contains("Add two integers"),
            "expected doc comment about adding, got: {:?}",
            add_def.doc_comment
        );
    }

    #[test]
    fn function_clamp_is_static_inline() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let clamp = find_by_name(&items, "clamp_value");
        assert_eq!(clamp.kind, SymbolKind::Function);
        assert_eq!(clamp.visibility, Visibility::Private);
        assert!(
            clamp.metadata.attributes.contains(&"static".to_string()),
            "should have static attr: {:?}",
            clamp.metadata.attributes
        );
        assert!(
            clamp.metadata.attributes.contains(&"inline".to_string()),
            "should have inline attr: {:?}",
            clamp.metadata.attributes
        );
    }

    #[test]
    fn function_multiply_is_extern() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let mul = find_by_name(&items, "multiply");
        assert_eq!(mul.kind, SymbolKind::Function);
        assert!(
            mul.metadata.attributes.contains(&"extern".to_string()),
            "should have extern attr: {:?}",
            mul.metadata.attributes
        );
    }

    #[test]
    fn function_variadic_log() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let vlog = find_by_name(&items, "variadic_log");
        assert_eq!(vlog.kind, SymbolKind::Function);
        assert!(
            vlog.metadata.attributes.contains(&"variadic".to_string()),
            "should have variadic attr: {:?}",
            vlog.metadata.attributes
        );
    }

    #[test]
    fn function_make_point_returns_struct() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let mp = find_by_name(&items, "make_point");
        assert_eq!(mp.kind, SymbolKind::Function);
        assert!(
            mp.metadata
                .return_type
                .as_deref()
                .is_some_and(|rt| rt.contains("Point")),
            "make_point should return struct Point: {:?}",
            mp.metadata.return_type
        );
    }

    // ── Prototype tests ───────────────────────────────────────────

    #[test]
    fn prototype_add_extracted() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let proto = items
            .iter()
            .find(|i| i.name == "add" && i.metadata.attributes.contains(&"prototype".to_string()))
            .expect("should find add prototype");
        assert_eq!(proto.kind, SymbolKind::Function);
    }

    #[test]
    fn prototype_process_data() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let proto = items
            .iter()
            .find(|i| {
                i.name == "process_data" && i.metadata.attributes.contains(&"prototype".to_string())
            })
            .expect("should find process_data prototype");
        assert_eq!(proto.kind, SymbolKind::Function);
    }

    #[test]
    fn prototype_shutdown_subsystem() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let proto = items
            .iter()
            .find(|i| {
                i.name == "shutdown_subsystem"
                    && i.metadata.attributes.contains(&"prototype".to_string())
            })
            .expect("should find shutdown_subsystem prototype");
        assert_eq!(proto.kind, SymbolKind::Function);
    }

    // ── Struct tests ──────────────────────────────────────────────

    #[test]
    fn struct_point_extracted() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let point = find_by_name(&items, "Point");
        assert_eq!(point.kind, SymbolKind::Struct);
    }

    #[test]
    fn struct_point_has_fields() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let point = find_by_name(&items, "Point");
        assert!(
            point.metadata.fields.len() >= 2,
            "Point should have at least 2 fields: {:?}",
            point.metadata.fields
        );
        assert!(
            point.metadata.fields.contains(&"x".to_string()),
            "Point should have field x: {:?}",
            point.metadata.fields
        );
        assert!(
            point.metadata.fields.contains(&"y".to_string()),
            "Point should have field y: {:?}",
            point.metadata.fields
        );
    }

    #[test]
    fn struct_point_doc_comment() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let point = find_by_name(&items, "Point");
        assert!(
            point.doc_comment.contains("2D point"),
            "expected doc about 2D point, got: {:?}",
            point.doc_comment
        );
    }

    #[test]
    fn struct_rectangle_typedef() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let rect = find_by_name(&items, "Rectangle");
        assert_eq!(rect.kind, SymbolKind::Struct);
        assert!(
            rect.metadata.attributes.contains(&"typedef".to_string()),
            "Rectangle should be a typedef: {:?}",
            rect.metadata.attributes
        );
    }

    #[test]
    fn struct_rectangle_has_fields() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let rect = find_by_name(&items, "Rectangle");
        assert!(
            rect.metadata.fields.len() >= 3,
            "Rectangle should have at least 3 fields: {:?}",
            rect.metadata.fields
        );
    }

    #[test]
    fn struct_node_has_fields() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let nodes: Vec<_> = items
            .iter()
            .filter(|i| i.name == "Node" && !i.metadata.fields.is_empty())
            .collect();
        assert!(!nodes.is_empty(), "should find Node struct with fields");
        let node = nodes[0];
        assert!(
            node.metadata.fields.contains(&"value".to_string()),
            "Node should have 'value' field: {:?}",
            node.metadata.fields
        );
    }

    #[test]
    fn struct_hardware_register_bitfields() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let hw = find_by_name(&items, "HardwareRegister");
        assert_eq!(hw.kind, SymbolKind::Struct);
        let bitfields: Vec<_> = hw
            .metadata
            .fields
            .iter()
            .filter(|f| f.contains("bitfield"))
            .collect();
        assert!(
            bitfields.len() >= 3,
            "HardwareRegister should have 3+ bitfields: {:?}",
            hw.metadata.fields
        );
    }

    #[test]
    fn struct_config_has_many_fields() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let config = find_by_name(&items, "Config");
        assert_eq!(config.kind, SymbolKind::Struct);
        assert!(
            config.metadata.fields.len() >= 5,
            "Config should have 5+ fields: {:?}",
            config.metadata.fields
        );
    }

    #[test]
    fn struct_forward_declaration_node() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let fwd = items
            .iter()
            .find(|i| {
                i.name == "Node"
                    && i.metadata
                        .attributes
                        .contains(&"forward_declaration".to_string())
            })
            .expect("should find Node forward declaration");
        assert_eq!(fwd.kind, SymbolKind::Struct);
    }

    #[test]
    fn struct_forward_declaration_opaque() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let fwd = items
            .iter()
            .find(|i| {
                i.name == "OpaqueHandle"
                    && i.metadata
                        .attributes
                        .contains(&"forward_declaration".to_string())
            })
            .expect("should find OpaqueHandle forward declaration");
        assert_eq!(fwd.kind, SymbolKind::Struct);
    }

    // ── Enum tests ────────────────────────────────────────────────

    #[test]
    fn enum_color_extracted() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let color = find_by_name(&items, "Color");
        assert_eq!(color.kind, SymbolKind::Enum);
    }

    #[test]
    fn enum_color_has_variants() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let color = find_by_name(&items, "Color");
        assert!(
            color.metadata.variants.len() >= 5,
            "Color should have 5 variants: {:?}",
            color.metadata.variants
        );
        assert!(
            color.metadata.variants.contains(&"COLOR_RED".to_string()),
            "should have COLOR_RED"
        );
    }

    #[test]
    fn enum_color_doc_comment() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let color = find_by_name(&items, "Color");
        assert!(
            color.doc_comment.contains("Color constants"),
            "expected doc about color constants, got: {:?}",
            color.doc_comment
        );
    }

    #[test]
    fn enum_status_code_typedef() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let sc = find_by_name(&items, "StatusCode");
        assert_eq!(sc.kind, SymbolKind::Enum);
        assert!(
            sc.metadata.attributes.contains(&"typedef".to_string()),
            "StatusCode should be typedef: {:?}",
            sc.metadata.attributes
        );
    }

    #[test]
    fn enum_status_code_has_variants() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let sc = find_by_name(&items, "StatusCode");
        assert!(
            sc.metadata.variants.len() >= 4,
            "StatusCode should have 4+ variants: {:?}",
            sc.metadata.variants
        );
    }

    #[test]
    fn enum_log_level_extracted() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let ll = find_by_name(&items, "LogLevel");
        assert_eq!(ll.kind, SymbolKind::Enum);
        assert!(
            ll.metadata.variants.len() >= 6,
            "LogLevel should have 6 variants: {:?}",
            ll.metadata.variants
        );
    }

    #[test]
    fn enum_forward_declaration() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let fwd = items
            .iter()
            .find(|i| {
                i.name == "Status"
                    && i.metadata
                        .attributes
                        .contains(&"forward_declaration".to_string())
            })
            .expect("should find Status forward declaration");
        assert_eq!(fwd.kind, SymbolKind::Enum);
    }

    // ── Union tests ───────────────────────────────────────────────

    #[test]
    fn union_value_extracted() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let value = find_by_name(&items, "Value");
        assert_eq!(value.kind, SymbolKind::Union);
    }

    #[test]
    fn union_value_has_fields() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let value = find_by_name(&items, "Value");
        assert!(
            value.metadata.fields.len() >= 4,
            "Value union should have 4+ fields: {:?}",
            value.metadata.fields
        );
    }

    #[test]
    fn union_value_doc_comment() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let value = find_by_name(&items, "Value");
        assert!(
            value.doc_comment.contains("tagged value"),
            "expected doc about tagged value, got: {:?}",
            value.doc_comment
        );
    }

    #[test]
    fn union_network_address() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let na = find_by_name(&items, "NetworkAddress");
        assert_eq!(na.kind, SymbolKind::Union);
        assert!(
            na.metadata.fields.len() >= 3,
            "NetworkAddress should have 3 fields: {:?}",
            na.metadata.fields
        );
    }

    // ── Typedef tests ─────────────────────────────────────────────

    #[test]
    fn typedef_byte() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let byte = find_by_name(&items, "Byte");
        assert_eq!(byte.kind, SymbolKind::TypeAlias);
    }

    #[test]
    fn typedef_size() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let size = find_by_name(&items, "Size");
        assert_eq!(size.kind, SymbolKind::TypeAlias);
    }

    #[test]
    fn typedef_point2d() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let p2d = find_by_name(&items, "Point2D");
        assert_eq!(p2d.kind, SymbolKind::TypeAlias);
    }

    #[test]
    fn typedef_comparator_function_pointer() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let cmp = find_by_name(&items, "Comparator");
        assert_eq!(cmp.kind, SymbolKind::TypeAlias);
        assert!(
            cmp.metadata
                .attributes
                .contains(&"function_pointer".to_string()),
            "Comparator should be a function pointer typedef: {:?}",
            cmp.metadata.attributes
        );
    }

    #[test]
    fn typedef_event_callback() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let ec = find_by_name(&items, "EventCallback");
        assert_eq!(ec.kind, SymbolKind::TypeAlias);
        assert!(
            ec.metadata
                .attributes
                .contains(&"function_pointer".to_string()),
            "EventCallback should be a function pointer typedef"
        );
    }

    #[test]
    fn typedef_allocator() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let alloc = find_by_name(&items, "Allocator");
        assert_eq!(alloc.kind, SymbolKind::TypeAlias);
    }

    // ── Variable tests ────────────────────────────────────────────

    #[test]
    fn variable_global_counter() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let gc = find_by_name(&items, "global_counter");
        assert_eq!(gc.kind, SymbolKind::Static);
        assert_eq!(gc.visibility, Visibility::Public);
    }

    #[test]
    fn variable_internal_state_static() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let is_ = find_by_name(&items, "internal_state");
        assert_eq!(is_.kind, SymbolKind::Static);
        assert_eq!(is_.visibility, Visibility::Private);
        assert!(
            is_.metadata.attributes.contains(&"static".to_string()),
            "should have static attr"
        );
    }

    #[test]
    fn variable_shared_value_extern() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let sv = find_by_name(&items, "shared_value");
        assert_eq!(sv.visibility, Visibility::Public);
        assert!(
            sv.metadata.attributes.contains(&"extern".to_string()),
            "should have extern attr"
        );
    }

    #[test]
    fn constant_max_items() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let mi = find_by_name(&items, "MAX_ITEMS");
        assert_eq!(mi.kind, SymbolKind::Const);
        assert!(
            mi.metadata.attributes.contains(&"const".to_string()),
            "should have const attr"
        );
    }

    #[test]
    fn constant_default_timeout() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let dt = find_by_name(&items, "DEFAULT_TIMEOUT_MS");
        assert_eq!(dt.kind, SymbolKind::Const);
    }

    #[test]
    fn variable_build_tag_static_const() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let bt = find_by_name(&items, "BUILD_TAG");
        assert_eq!(bt.kind, SymbolKind::Const);
        assert_eq!(bt.visibility, Visibility::Private);
        assert!(
            bt.metadata.attributes.contains(&"static".to_string()),
            "should have static"
        );
        assert!(
            bt.metadata.attributes.contains(&"const".to_string()),
            "should have const"
        );
    }

    // ── Array declaration tests ───────────────────────────────────

    #[test]
    fn array_lookup_table() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let lt = find_by_name(&items, "lookup_table");
        assert_eq!(lt.kind, SymbolKind::Static);
        assert!(
            lt.metadata.attributes.contains(&"array".to_string()),
            "should have array attr: {:?}",
            lt.metadata.attributes
        );
    }

    #[test]
    fn array_prime_numbers_static() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let pn = find_by_name(&items, "prime_numbers");
        assert_eq!(pn.visibility, Visibility::Private);
        assert!(
            pn.metadata.attributes.contains(&"static".to_string()),
            "should have static attr"
        );
    }

    // ── Preprocessor tests ────────────────────────────────────────

    #[test]
    fn include_stdio() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let stdio = find_by_name(&items, "<stdio.h>");
        assert_eq!(stdio.kind, SymbolKind::Module);
        assert!(
            stdio.metadata.attributes.contains(&"system".to_string()),
            "should be a system include"
        );
    }

    #[test]
    fn include_mylib() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let mylib = items
            .iter()
            .find(|i| i.name.contains("mylib"))
            .expect("should find mylib include");
        assert_eq!(mylib.kind, SymbolKind::Module);
        assert!(
            mylib.metadata.attributes.contains(&"local".to_string()),
            "should be a local include"
        );
    }

    #[test]
    fn define_max_buffer() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let mb = find_by_name(&items, "MAX_BUFFER");
        assert_eq!(mb.kind, SymbolKind::Const);
        assert!(
            mb.signature.contains("1024"),
            "should have value 1024 in signature: {:?}",
            mb.signature
        );
    }

    #[test]
    fn define_square_function_like() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let sq = find_by_name(&items, "SQUARE");
        assert_eq!(sq.kind, SymbolKind::Macro);
        assert!(
            sq.metadata
                .attributes
                .contains(&"function_like".to_string()),
            "should be function-like macro: {:?}",
            sq.metadata.attributes
        );
        assert!(
            sq.metadata.parameters.contains(&"x".to_string()),
            "should have parameter 'x': {:?}",
            sq.metadata.parameters
        );
    }

    #[test]
    fn define_min_function_like() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let min = find_by_name(&items, "MIN");
        assert_eq!(min.kind, SymbolKind::Macro);
        assert_eq!(
            min.metadata.parameters.len(),
            2,
            "MIN should have 2 params: {:?}",
            min.metadata.parameters
        );
    }

    #[test]
    fn define_debug_log_variadic_macro() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let dl = find_by_name(&items, "DEBUG_LOG");
        assert_eq!(dl.kind, SymbolKind::Macro);
        assert!(
            dl.metadata.parameters.len() >= 2,
            "DEBUG_LOG should have 2+ params: {:?}",
            dl.metadata.parameters
        );
    }

    #[test]
    fn ifdef_debug_extracted() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let dbg = items
            .iter()
            .find(|i| i.name == "DEBUG" && i.metadata.attributes.contains(&"#ifdef".to_string()))
            .expect("should find #ifdef DEBUG");
        assert_eq!(dbg.kind, SymbolKind::Macro);
    }

    #[test]
    fn ifndef_header_guard() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let hg = items
            .iter()
            .find(|i| {
                i.name == "SAMPLE_H" && i.metadata.attributes.contains(&"#ifndef".to_string())
            })
            .expect("should find #ifndef SAMPLE_H header guard");
        assert_eq!(hg.kind, SymbolKind::Macro);
    }

    #[test]
    fn pragma_once_extracted() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let pragma = items
            .iter()
            .find(|i| i.name == "once")
            .expect("should find #pragma once");
        assert_eq!(pragma.kind, SymbolKind::Macro);
    }

    #[test]
    fn pragma_pack_extracted() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let pragmas: Vec<_> = items
            .iter()
            .filter(|i| i.metadata.attributes.contains(&"#pragma".to_string()))
            .collect();
        assert!(
            pragmas.len() >= 2,
            "should have at least 2 pragma directives"
        );
    }

    // ── Function pointer variable tests ───────────────────────────

    #[test]
    fn function_pointer_var_on_event() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let cb = find_by_name(&items, "on_event_callback");
        assert_eq!(cb.kind, SymbolKind::Static);
        assert!(
            cb.metadata
                .attributes
                .contains(&"function_pointer".to_string()),
            "should have function_pointer attr: {:?}",
            cb.metadata.attributes
        );
    }

    #[test]
    fn function_pointer_var_cleanup() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let ch = find_by_name(&items, "cleanup_handler");
        assert_eq!(ch.kind, SymbolKind::Static);
        assert_eq!(ch.visibility, Visibility::Private);
    }

    // ── Static assert tests ───────────────────────────────────────

    #[test]
    fn static_assert_extracted() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let asserts: Vec<_> = find_by_name_prefix(&items, "_Static_assert");
        assert!(
            asserts.len() >= 2,
            "should find at least 2 _Static_assert, got {}",
            asserts.len()
        );
    }

    #[test]
    fn static_assert_has_attribute() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let sa = items
            .iter()
            .find(|i| i.name == "_Static_assert")
            .expect("should find _Static_assert");
        assert!(
            sa.metadata
                .attributes
                .contains(&"static_assert".to_string()),
            "should have static_assert attr"
        );
    }

    // ── Line number tests ─────────────────────────────────────────

    #[test]
    fn line_numbers_are_positive() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        for item in &items {
            assert!(
                item.start_line > 0,
                "start_line should be > 0 for {}: got {}",
                item.name,
                item.start_line
            );
            assert!(
                item.end_line >= item.start_line,
                "end_line should be >= start_line for {}",
                item.name
            );
        }
    }

    #[test]
    fn function_definition_spans_multiple_lines() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let pd = items
            .iter()
            .find(|i| {
                i.name == "process_data"
                    && !i.metadata.attributes.contains(&"prototype".to_string())
            })
            .expect("should find process_data definition");
        assert!(
            pd.end_line > pd.start_line,
            "process_data should span multiple lines"
        );
    }

    // ── Doc comment style tests ───────────────────────────────────

    #[test]
    fn doc_comment_doxygen_style() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let add_def = items
            .iter()
            .find(|i| i.name == "add" && !i.metadata.attributes.contains(&"prototype".to_string()))
            .expect("should find add definition");
        assert!(
            add_def.doc_comment.contains("@param"),
            "should contain @param tags: {:?}",
            add_def.doc_comment
        );
    }

    #[test]
    fn doc_comment_single_line_style() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let ll = find_by_name(&items, "LogLevel");
        assert!(
            ll.doc_comment.contains("Log level"),
            "LogLevel should have single-line doc: {:?}",
            ll.doc_comment
        );
    }

    #[test]
    fn doc_comment_multiline_block() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let value = find_by_name(&items, "Value");
        assert!(
            !value.doc_comment.is_empty(),
            "Value should have a doc comment"
        );
    }

    // ── Inline / edge case tests ──────────────────────────────────

    #[test]
    fn inline_empty_source() {
        let items = parse_and_extract("");
        assert!(items.is_empty(), "empty source should yield no items");
    }

    #[test]
    fn inline_single_function() {
        let items = parse_and_extract("int main(void) { return 0; }");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].kind, SymbolKind::Function);
        assert_eq!(items[0].name, "main");
    }

    #[test]
    fn inline_single_struct() {
        let items = parse_and_extract("struct Foo { int bar; };");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].kind, SymbolKind::Struct);
        assert_eq!(items[0].name, "Foo");
        assert!(items[0].metadata.fields.contains(&"bar".to_string()));
    }

    #[test]
    fn inline_single_enum() {
        let items = parse_and_extract("enum Dir { NORTH, SOUTH, EAST, WEST };");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].kind, SymbolKind::Enum);
        assert_eq!(items[0].metadata.variants.len(), 4);
    }

    #[test]
    fn inline_single_typedef() {
        let items = parse_and_extract("typedef int MyInt;");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].kind, SymbolKind::TypeAlias);
    }

    #[test]
    fn inline_global_variable() {
        let items = parse_and_extract("int x = 42;");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].kind, SymbolKind::Static);
        assert_eq!(items[0].name, "x");
    }

    #[test]
    fn inline_const_variable() {
        let items = parse_and_extract("const int Y = 100;");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].kind, SymbolKind::Const);
    }

    #[test]
    fn inline_static_variable() {
        let items = parse_and_extract("static int hidden = 0;");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].visibility, Visibility::Private);
    }

    #[test]
    fn inline_extern_variable() {
        let items = parse_and_extract("extern int external;");
        assert_eq!(items.len(), 1);
        assert!(items[0].metadata.attributes.contains(&"extern".to_string()));
    }

    #[test]
    fn inline_include_system() {
        let items = parse_and_extract("#include <math.h>\n");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].kind, SymbolKind::Module);
        assert!(items[0].metadata.attributes.contains(&"system".to_string()));
    }

    #[test]
    fn inline_include_local() {
        let items = parse_and_extract("#include \"header.h\"\n");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].kind, SymbolKind::Module);
        assert!(items[0].metadata.attributes.contains(&"local".to_string()));
    }

    #[test]
    fn inline_define_object() {
        let items = parse_and_extract("#define FOO 42\n");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].kind, SymbolKind::Const);
        assert_eq!(items[0].name, "FOO");
    }

    #[test]
    fn inline_define_function() {
        let items = parse_and_extract("#define ADD(a,b) ((a)+(b))\n");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].kind, SymbolKind::Macro);
        assert_eq!(items[0].name, "ADD");
        assert!(items[0].metadata.parameters.contains(&"a".to_string()));
        assert!(items[0].metadata.parameters.contains(&"b".to_string()));
    }

    #[test]
    fn inline_union() {
        let items = parse_and_extract("union U { int a; float b; };");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].kind, SymbolKind::Union);
        assert_eq!(items[0].metadata.fields.len(), 2);
    }

    #[test]
    fn inline_forward_declaration() {
        let items = parse_and_extract("struct Forward;");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].kind, SymbolKind::Struct);
        assert!(
            items[0]
                .metadata
                .attributes
                .contains(&"forward_declaration".to_string())
        );
    }

    #[test]
    fn inline_function_pointer_typedef() {
        let items = parse_and_extract("typedef void (*Handler)(int);");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].kind, SymbolKind::TypeAlias);
        assert!(
            items[0]
                .metadata
                .attributes
                .contains(&"function_pointer".to_string())
        );
    }

    #[test]
    fn inline_prototype() {
        let items = parse_and_extract("int foo(int x);");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].kind, SymbolKind::Function);
        assert!(
            items[0]
                .metadata
                .attributes
                .contains(&"prototype".to_string())
        );
    }

    #[test]
    fn inline_static_assert() {
        let items = parse_and_extract("_Static_assert(1, \"always true\");");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "_Static_assert");
    }

    #[test]
    fn inline_array_declaration() {
        let items = parse_and_extract("int data[100];");
        assert_eq!(items.len(), 1);
        assert!(items[0].metadata.attributes.contains(&"array".to_string()));
    }

    #[test]
    fn inline_typedef_struct() {
        let items = parse_and_extract("typedef struct { int x; } Wrapper;");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].kind, SymbolKind::Struct);
        assert_eq!(items[0].name, "Wrapper");
        assert!(items[0].metadata.fields.contains(&"x".to_string()));
    }

    #[test]
    fn inline_typedef_enum() {
        let items = parse_and_extract("typedef enum { A, B, C } Letters;");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].kind, SymbolKind::Enum);
        assert_eq!(items[0].name, "Letters");
        assert_eq!(items[0].metadata.variants.len(), 3);
    }

    #[test]
    fn inline_doc_comment_above_function() {
        let items = parse_and_extract("/* My function */\nint f(void) { return 0; }");
        assert_eq!(items.len(), 1);
        assert!(
            items[0].doc_comment.contains("My function"),
            "doc comment: {:?}",
            items[0].doc_comment
        );
    }

    #[test]
    fn inline_multiple_items() {
        let source = "int x = 1;\nint y = 2;\nint z = 3;";
        let items = parse_and_extract(source);
        assert_eq!(items.len(), 3);
    }

    #[test]
    fn inline_variadic_prototype() {
        let items = parse_and_extract("int printf(const char *fmt, ...);");
        assert_eq!(items.len(), 1);
        assert!(
            items[0]
                .metadata
                .attributes
                .contains(&"variadic".to_string()),
            "should detect variadic: {:?}",
            items[0].metadata.attributes
        );
    }

    #[test]
    fn inline_function_pointer_variable() {
        let items = parse_and_extract("void (*handler)(int) = 0;");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "handler");
        assert!(
            items[0]
                .metadata
                .attributes
                .contains(&"function_pointer".to_string())
        );
    }

    #[test]
    fn visibility_static_is_private() {
        let items = parse_and_extract("static void internal(void) {}");
        assert_eq!(items[0].visibility, Visibility::Private);
    }

    #[test]
    fn visibility_extern_is_public() {
        let items = parse_and_extract("extern int api_func(void);");
        assert_eq!(items[0].visibility, Visibility::Public);
    }

    #[test]
    fn visibility_default_is_public() {
        let items = parse_and_extract("int regular(void) { return 0; }");
        assert_eq!(items[0].visibility, Visibility::Public);
    }

    // ── Gap 1: #if / #elif / #else ────────────────────────────────

    #[test]
    fn preproc_if_extracted() {
        let items = parse_and_extract("#if __STDC_VERSION__ >= 201112L\nint c11 = 1;\n#endif\n");
        let if_item = items
            .iter()
            .find(|i| i.metadata.attributes.contains(&"#if".to_string()))
            .expect("should find #if");
        assert_eq!(if_item.kind, SymbolKind::Macro);
    }

    #[test]
    fn preproc_if_contains_condition() {
        let items = parse_and_extract("#if __STDC_VERSION__ >= 201112L\nint c11 = 1;\n#endif\n");
        let if_item = items
            .iter()
            .find(|i| i.metadata.attributes.contains(&"#if".to_string()))
            .expect("should find #if");
        assert!(
            if_item.name.contains("__STDC_VERSION__"),
            "should have condition in name: {:?}",
            if_item.name
        );
    }

    #[test]
    fn preproc_if_children_extracted() {
        let items = parse_and_extract("#if __STDC_VERSION__ >= 201112L\nint c11 = 1;\n#endif\n");
        let c11 = find_by_name(&items, "c11");
        assert_eq!(c11.kind, SymbolKind::Static);
    }

    #[test]
    fn preproc_elif_extracted() {
        let items =
            parse_and_extract("#if X\nint a=1;\n#elif Y\nint b=2;\n#else\nint c=3;\n#endif\n");
        let elif = items
            .iter()
            .find(|i| i.metadata.attributes.contains(&"#elif".to_string()))
            .expect("should find #elif");
        assert_eq!(elif.kind, SymbolKind::Macro);
    }

    #[test]
    fn preproc_elif_children_extracted() {
        let items =
            parse_and_extract("#if X\nint a=1;\n#elif Y\nint b=2;\n#else\nint c=3;\n#endif\n");
        let b = find_by_name(&items, "b");
        assert_eq!(b.kind, SymbolKind::Static);
    }

    #[test]
    fn preproc_else_children_extracted() {
        let items =
            parse_and_extract("#if X\nint a=1;\n#elif Y\nint b=2;\n#else\nint c=3;\n#endif\n");
        let c = find_by_name(&items, "c");
        assert_eq!(c.kind, SymbolKind::Static);
    }

    #[test]
    fn preproc_if_all_branches_have_items() {
        let items =
            parse_and_extract("#if X\nint a=1;\n#elif Y\nint b=2;\n#else\nint c=3;\n#endif\n");
        // Should have: #if macro, a, #elif macro, b, c = 5 items
        assert!(
            items.len() >= 5,
            "expected at least 5 items, got {}",
            items.len()
        );
    }

    #[test]
    fn fixture_has_preproc_if() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let if_items: Vec<_> = items
            .iter()
            .filter(|i| i.metadata.attributes.contains(&"#if".to_string()))
            .collect();
        assert!(
            if_items.len() >= 2,
            "expected at least 2 #if items, got {}",
            if_items.len()
        );
    }

    #[test]
    fn fixture_c11_available() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let c11 = find_by_name(&items, "c11_available");
        assert_eq!(c11.kind, SymbolKind::Static);
    }

    // ── Gap 2: multi-variable declarations ────────────────────────

    #[test]
    fn multi_var_init_all_extracted() {
        let items = parse_and_extract("int a = 1, b = 2, c = 3;");
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].name, "a");
        assert_eq!(items[1].name, "b");
        assert_eq!(items[2].name, "c");
    }

    #[test]
    fn multi_var_plain_all_extracted() {
        let items = parse_and_extract("int x, y, z;");
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].name, "x");
        assert_eq!(items[1].name, "y");
        assert_eq!(items[2].name, "z");
    }

    #[test]
    fn fixture_multi_a_b_c() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let _ = find_by_name(&items, "multi_a");
        let _ = find_by_name(&items, "multi_b");
        let _ = find_by_name(&items, "multi_c");
    }

    #[test]
    fn fixture_coord_x_y_z() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let _ = find_by_name(&items, "coord_x");
        let _ = find_by_name(&items, "coord_y");
        let _ = find_by_name(&items, "coord_z");
    }

    // ── Gap 3: volatile qualifier ─────────────────────────────────

    #[test]
    fn volatile_variable_has_attr() {
        let items = parse_and_extract("volatile int sensor;");
        assert_eq!(items.len(), 1);
        assert!(
            items[0]
                .metadata
                .attributes
                .contains(&"volatile".to_string()),
            "should have volatile attr: {:?}",
            items[0].metadata.attributes
        );
    }

    #[test]
    fn volatile_const_both_detected() {
        let items = parse_and_extract("volatile const int hw = 0x1234;");
        assert_eq!(items[0].kind, SymbolKind::Const);
        assert!(
            items[0]
                .metadata
                .attributes
                .contains(&"volatile".to_string())
        );
        assert!(items[0].metadata.attributes.contains(&"const".to_string()));
    }

    #[test]
    fn fixture_sensor_reading_volatile() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let sr = find_by_name(&items, "sensor_reading");
        assert!(
            sr.metadata.attributes.contains(&"volatile".to_string()),
            "sensor_reading should have volatile: {:?}",
            sr.metadata.attributes
        );
    }

    #[test]
    fn fixture_hw_status_reg_volatile_const() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let hw = find_by_name(&items, "HW_STATUS_REG");
        assert_eq!(hw.kind, SymbolKind::Const);
        assert!(hw.metadata.attributes.contains(&"volatile".to_string()));
        assert!(hw.metadata.attributes.contains(&"const".to_string()));
    }

    // ── Gap 4: register storage class ─────────────────────────────

    #[test]
    fn register_variable_has_attr() {
        let items = parse_and_extract("register int fast;");
        assert_eq!(items.len(), 1);
        assert!(
            items[0]
                .metadata
                .attributes
                .contains(&"register".to_string()),
            "should have register attr: {:?}",
            items[0].metadata.attributes
        );
    }

    #[test]
    fn fixture_fast_counter_register() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let fc = find_by_name(&items, "fast_counter");
        assert!(
            fc.metadata.attributes.contains(&"register".to_string()),
            "fast_counter should have register: {:?}",
            fc.metadata.attributes
        );
    }

    // ── Gap 5: __attribute__((…)) ─────────────────────────────────

    #[test]
    fn gcc_attribute_on_variable() {
        let items = parse_and_extract("__attribute__((unused)) static int x = 0;");
        assert_eq!(items.len(), 1);
        assert!(
            items[0]
                .metadata
                .attributes
                .iter()
                .any(|a| a.contains("__attribute__")),
            "should have __attribute__: {:?}",
            items[0].metadata.attributes
        );
    }

    #[test]
    fn gcc_attribute_on_function() {
        let items = parse_and_extract("__attribute__((noreturn)) void die(void);");
        assert_eq!(items.len(), 1);
        assert!(
            items[0]
                .metadata
                .attributes
                .iter()
                .any(|a| a.contains("__attribute__")),
            "should have __attribute__: {:?}",
            items[0].metadata.attributes
        );
    }

    #[test]
    fn fixture_attr_var() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let av = find_by_name(&items, "attr_var");
        assert!(
            av.metadata
                .attributes
                .iter()
                .any(|a| a.contains("__attribute__")),
            "attr_var should have __attribute__: {:?}",
            av.metadata.attributes
        );
    }

    #[test]
    fn fixture_panic_handler_attr() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let ph = find_by_name(&items, "panic_handler");
        assert!(
            ph.metadata
                .attributes
                .iter()
                .any(|a| a.contains("noreturn")),
            "panic_handler should have noreturn attribute: {:?}",
            ph.metadata.attributes
        );
    }

    // ── Gap 6: C11 qualifiers (_Noreturn, _Atomic) ────────────────

    #[test]
    fn noreturn_function_has_attr() {
        let items = parse_and_extract("_Noreturn void die(void);");
        assert_eq!(items.len(), 1);
        assert!(
            items[0]
                .metadata
                .attributes
                .contains(&"_Noreturn".to_string()),
            "should have _Noreturn: {:?}",
            items[0].metadata.attributes
        );
    }

    #[test]
    fn atomic_variable_has_attr() {
        let items = parse_and_extract("_Atomic int counter;");
        assert_eq!(items.len(), 1);
        assert!(
            items[0]
                .metadata
                .attributes
                .contains(&"_Atomic".to_string()),
            "should have _Atomic: {:?}",
            items[0].metadata.attributes
        );
    }

    #[test]
    fn fixture_abort_with_message_noreturn() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let awm = find_by_name(&items, "abort_with_message");
        assert!(
            awm.metadata.attributes.contains(&"_Noreturn".to_string()),
            "abort_with_message should have _Noreturn: {:?}",
            awm.metadata.attributes
        );
    }

    #[test]
    fn fixture_atomic_counter() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let ac = find_by_name(&items, "atomic_counter");
        assert!(
            ac.metadata.attributes.contains(&"_Atomic".to_string()),
            "atomic_counter should have _Atomic: {:?}",
            ac.metadata.attributes
        );
    }

    // ── Gap 7: anonymous struct/union in fields ───────────────────

    #[test]
    fn anonymous_union_field() {
        let items = parse_and_extract("struct TV { int tag; union { int i; float f; }; };");
        assert_eq!(items.len(), 1);
        assert!(
            items[0]
                .metadata
                .fields
                .contains(&"(anonymous union)".to_string()),
            "should have anonymous union field: {:?}",
            items[0].metadata.fields
        );
    }

    #[test]
    fn anonymous_struct_field() {
        let items = parse_and_extract("struct Outer { struct { int x; int y; }; int z; };");
        assert!(
            items[0]
                .metadata
                .fields
                .contains(&"(anonymous struct)".to_string()),
            "should have anonymous struct field: {:?}",
            items[0].metadata.fields
        );
    }

    #[test]
    fn anonymous_union_does_not_hide_named_fields() {
        let items = parse_and_extract("struct TV { int tag; union { int i; float f; }; };");
        assert!(
            items[0].metadata.fields.contains(&"tag".to_string()),
            "should still have 'tag' field: {:?}",
            items[0].metadata.fields
        );
    }

    #[test]
    fn fixture_tagged_value_anonymous_union() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let tv = find_by_name(&items, "TaggedValue");
        assert_eq!(tv.kind, SymbolKind::Struct);
        assert!(
            tv.metadata
                .fields
                .contains(&"(anonymous union)".to_string()),
            "TaggedValue should have anonymous union: {:?}",
            tv.metadata.fields
        );
        assert!(
            tv.metadata.fields.contains(&"tag".to_string()),
            "TaggedValue should have 'tag': {:?}",
            tv.metadata.fields
        );
    }

    // ── Pointer-to-pointer ────────────────────────────────────────

    #[test]
    fn pointer_to_pointer_extracted() {
        let items = parse_and_extract("char **envp;");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "envp");
    }

    #[test]
    fn fixture_environment_ptr_ptr() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let env = find_by_name(&items, "environment");
        assert_eq!(env.kind, SymbolKind::Static);
    }

    // ── Gap 1 extended: nested #if / #elif / #else ──────────────────

    #[test]
    fn preproc_if_nested_ifdef_inside() {
        let src = "#if A\n#ifdef B\nint inner = 1;\n#endif\n#endif\n";
        let items = parse_and_extract(src);
        let inner = find_by_name(&items, "inner");
        assert_eq!(inner.kind, SymbolKind::Static);
    }

    #[test]
    fn preproc_if_with_defined() {
        let src = "#if defined(__GNUC__)\nint gcc = 1;\n#endif\n";
        let items = parse_and_extract(src);
        let gcc = find_by_name(&items, "gcc");
        assert_eq!(gcc.kind, SymbolKind::Static);
    }

    #[test]
    fn preproc_elif_condition_name() {
        let items =
            parse_and_extract("#if X\nint a=1;\n#elif Y\nint b=2;\n#else\nint c=3;\n#endif\n");
        let elif = items
            .iter()
            .find(|i| i.metadata.attributes.contains(&"#elif".to_string()))
            .expect("should find #elif");
        assert!(
            elif.signature.contains("#elif"),
            "signature should contain #elif: {:?}",
            elif.signature
        );
    }

    #[test]
    fn preproc_else_does_not_create_macro_item() {
        let items = parse_and_extract("#if X\nint a=1;\n#else\nint fallback=1;\n#endif\n");
        // #else has no condition so no macro item for it; just its children
        let fallback = find_by_name(&items, "fallback");
        assert_eq!(fallback.kind, SymbolKind::Static);
    }

    #[test]
    fn preproc_if_struct_inside() {
        let src = "#if 1\nstruct IfStruct { int field; };\n#endif\n";
        let items = parse_and_extract(src);
        let s = find_by_name(&items, "IfStruct");
        assert_eq!(s.kind, SymbolKind::Struct);
        assert!(s.metadata.fields.contains(&"field".to_string()));
    }

    // ── Gap 2 extended: multi-variable edge cases ─────────────────

    #[test]
    fn multi_var_const_all_extracted() {
        let items = parse_and_extract("const int CA = 1, CB = 2;");
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].kind, SymbolKind::Const);
        assert_eq!(items[1].kind, SymbolKind::Const);
    }

    #[test]
    fn multi_var_two_items() {
        let items = parse_and_extract("float p = 1.0, q = 2.0;");
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].name, "p");
        assert_eq!(items[1].name, "q");
    }

    #[test]
    fn multi_var_static_visibility() {
        let items = parse_and_extract("static int sa = 1, sb = 2;");
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].visibility, Visibility::Private);
        assert_eq!(items[1].visibility, Visibility::Private);
    }

    // ── Gap 3 extended: volatile combinations ─────────────────────

    #[test]
    fn volatile_static_combined() {
        let items = parse_and_extract("static volatile int flag = 0;");
        assert_eq!(items.len(), 1);
        assert!(
            items[0]
                .metadata
                .attributes
                .contains(&"volatile".to_string())
        );
        assert!(items[0].metadata.attributes.contains(&"static".to_string()));
        assert_eq!(items[0].visibility, Visibility::Private);
    }

    #[test]
    fn volatile_return_type_in_variable() {
        let items = parse_and_extract("volatile int reg;");
        assert!(
            items[0]
                .metadata
                .return_type
                .as_deref()
                .is_some_and(|rt| rt.contains("int")),
            "return_type should include int: {:?}",
            items[0].metadata.return_type
        );
    }

    // ── Gap 4 extended: register edge cases ───────────────────────

    #[test]
    fn register_with_init() {
        let items = parse_and_extract("register int r = 42;");
        assert_eq!(items.len(), 1);
        assert!(
            items[0]
                .metadata
                .attributes
                .contains(&"register".to_string())
        );
    }

    #[test]
    fn register_not_static_visibility() {
        let items = parse_and_extract("register int r;");
        assert_eq!(items[0].visibility, Visibility::Public);
    }

    // ── Gap 5 extended: __attribute__ variations ──────────────────

    #[test]
    fn gcc_attribute_deprecated() {
        let items = parse_and_extract("__attribute__((deprecated)) int old_api(void);");
        assert_eq!(items.len(), 1);
        assert!(
            items[0]
                .metadata
                .attributes
                .iter()
                .any(|a| a.contains("deprecated")),
            "should contain deprecated: {:?}",
            items[0].metadata.attributes
        );
    }

    #[test]
    fn gcc_attribute_preserved_text() {
        let items = parse_and_extract("__attribute__((unused)) static int x = 0;");
        let attr_text: Vec<_> = items[0]
            .metadata
            .attributes
            .iter()
            .filter(|a| a.contains("__attribute__"))
            .collect();
        assert!(
            !attr_text.is_empty(),
            "should preserve __attribute__ text: {:?}",
            items[0].metadata.attributes
        );
    }

    // ── Gap 6 extended: C11 qualifier variations ──────────────────

    #[test]
    fn restrict_variable_has_attr() {
        let items = parse_and_extract("restrict int *ptr;");
        assert_eq!(items.len(), 1);
        assert!(
            items[0]
                .metadata
                .attributes
                .contains(&"restrict".to_string()),
            "should have restrict: {:?}",
            items[0].metadata.attributes
        );
    }

    #[test]
    fn noreturn_definition() {
        let items = parse_and_extract("_Noreturn void die(void) { while(1); }");
        assert_eq!(items[0].kind, SymbolKind::Function);
        assert!(
            items[0]
                .metadata
                .attributes
                .contains(&"_Noreturn".to_string())
        );
    }

    #[test]
    fn atomic_with_init() {
        let items = parse_and_extract("_Atomic int shared = 0;");
        assert_eq!(items[0].kind, SymbolKind::Static);
        assert!(
            items[0]
                .metadata
                .attributes
                .contains(&"_Atomic".to_string())
        );
    }

    // ── Gap 7 extended: anonymous aggregates ──────────────────────

    #[test]
    fn anonymous_struct_named_field_preserved() {
        let items = parse_and_extract("struct Outer { struct { int x; int y; }; int z; };");
        assert!(
            items[0].metadata.fields.contains(&"z".to_string()),
            "should preserve named field 'z': {:?}",
            items[0].metadata.fields
        );
    }

    #[test]
    fn anonymous_union_in_typedef_struct() {
        let items =
            parse_and_extract("typedef struct { int tag; union { int i; double d; }; } Variant;");
        let v = find_by_name(&items, "Variant");
        assert_eq!(v.kind, SymbolKind::Struct);
        assert!(
            v.metadata.fields.contains(&"(anonymous union)".to_string()),
            "Variant should have anonymous union field: {:?}",
            v.metadata.fields
        );
    }

    #[test]
    fn multiple_named_fields_with_anonymous() {
        let items = parse_and_extract("struct M { int a; union { int x; float y; }; int b; };");
        let m = find_by_name(&items, "M");
        assert!(m.metadata.fields.contains(&"a".to_string()));
        assert!(m.metadata.fields.contains(&"b".to_string()));
        assert!(m.metadata.fields.contains(&"(anonymous union)".to_string()));
    }

    // ── Additional coverage: typedef union ────────────────────────

    #[test]
    fn typedef_union_extracted() {
        let items = parse_and_extract("typedef union { int i; float f; } NumericValue;");
        let nv = find_by_name(&items, "NumericValue");
        assert_eq!(nv.kind, SymbolKind::Union);
        assert!(nv.metadata.attributes.contains(&"typedef".to_string()));
        assert!(nv.metadata.fields.len() >= 2);
    }

    // ── Additional coverage: signature quality ────────────────────

    #[test]
    fn function_signature_normalized() {
        let items = parse_and_extract("int   spaced_func(  int   x,  int   y  ) { return x+y; }");
        let f = find_by_name(&items, "spaced_func");
        // Signature should be whitespace-normalized
        assert!(
            !f.signature.contains("  "),
            "signature should not have double spaces: {:?}",
            f.signature
        );
    }

    #[test]
    fn prototype_signature_no_body() {
        let items = parse_and_extract("int proto_func(int x);");
        assert!(
            !items[0].signature.contains('{'),
            "prototype signature should not contain braces"
        );
    }

    // ── Additional coverage: multi-dimensional arrays ─────────────

    #[test]
    fn fixture_transform_matrix() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        let tm = find_by_name(&items, "transform_matrix");
        assert!(
            tm.metadata.attributes.contains(&"array".to_string()),
            "transform_matrix should have array attr: {:?}",
            tm.metadata.attributes
        );
    }

    // ── Updated fixture count ─────────────────────────────────────

    #[test]
    fn fixture_total_item_count() {
        let source = include_str!("../../tests/fixtures/sample.c");
        let items = parse_and_extract(source);
        assert!(
            items.len() >= 90,
            "expected 90+ items with new constructs, got {}",
            items.len()
        );
    }
}
