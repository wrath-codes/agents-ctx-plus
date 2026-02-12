//! C++ rich extractor.
//!
//! Delegates to the [`c`](crate::extractors::c) extractor for shared C constructs
//! (functions, structs, unions, enums, typedefs, variables, preprocessor
//! directives), then layers C++-specific processing: classes (with
//! inheritance, access specifiers, virtual/override/final, constructors,
//! destructors), templates, namespaces, concepts, operator overloading,
//! using declarations/aliases, constexpr/consteval/constinit, `static_assert`,
//! RAII patterns, and extern "C" linkage.

mod c_nodes;
mod classes;
mod enrichment;
mod helpers;
mod templates;

use ast_grep_core::Node;
use ast_grep_language::SupportLang;

use crate::types::{CppMetadataExt, ParsedItem, SymbolKind, SymbolMetadata, Visibility};

use c_nodes::{
    process_c_declaration, process_c_enum, process_c_function_definition, process_c_struct,
    process_c_typedef, process_c_union,
};
use classes::process_class;
use enrichment::enrich_items;
use helpers::{
    extract_parameters_from_declarator, extract_return_type_from_children,
    find_identifier_recursive,
};
use templates::{process_template_declaration, process_template_instantiation};

/// Extract all significant elements from a C++ source file.
///
/// First runs the C extractor for shared constructs, then walks the
/// AST for C++-specific nodes: classes, templates, namespaces, concepts,
/// using declarations, `static_assert`, and extern "C" linkage.
///
/// # Errors
/// Returns `ParserError` if parsing fails.
pub fn extract<D: ast_grep_core::Doc<Lang = SupportLang>>(
    root: &ast_grep_core::AstGrep<D>,
    source: &str,
) -> Result<Vec<ParsedItem>, crate::error::ParserError> {
    let mut items = crate::extractors::c::extract(root, source)?;
    let root_node = root.root();
    collect_cpp_nodes(&root_node, &mut items, source);
    enrich_items(&root_node, &mut items);
    Ok(items)
}

// ── Top-level C++ node dispatcher ──────────────────────────────────

fn collect_cpp_nodes<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    source: &str,
) {
    let children: Vec<_> = node.children().collect();
    for (idx, child) in children.iter().enumerate() {
        dispatch_cpp_node(child, &children, idx, items, source);
    }
}

fn dispatch_cpp_node<D: ast_grep_core::Doc>(
    child: &Node<D>,
    siblings: &[Node<D>],
    idx: usize,
    items: &mut Vec<ParsedItem>,
    source: &str,
) {
    let kind = child.kind();
    match kind.as_ref() {
        "namespace_definition" => {
            let doc = collect_doc_comment(siblings, idx, source);
            process_namespace(child, items, source, &doc);
        }
        "class_specifier" => {
            let doc = collect_doc_comment(siblings, idx, source);
            process_class(child, items, &doc, None);
        }
        "template_declaration" => {
            let doc = collect_doc_comment(siblings, idx, source);
            process_template_declaration(child, items, source, &doc);
        }
        "alias_declaration" => {
            let doc = collect_doc_comment(siblings, idx, source);
            process_alias_declaration(child, items, &doc);
        }
        "using_declaration" => {
            process_using_declaration(child, items);
        }
        "static_assert_declaration" => {
            let doc = collect_doc_comment(siblings, idx, source);
            process_static_assert(child, items, &doc);
        }
        "linkage_specification" => {
            process_linkage_spec(child, items, source);
        }
        "function_definition" => {
            // Catch C++ operator functions (operator overloads, user-defined
            // literals) that the C extractor skips because its name extraction
            // only looks for plain `identifier` nodes.
            maybe_process_operator_function(child, siblings, idx, items, source);
        }
        "namespace_alias_definition" => {
            let doc = collect_doc_comment(siblings, idx, source);
            process_namespace_alias(child, items, &doc);
        }
        "template_instantiation" => {
            let doc = collect_doc_comment(siblings, idx, source);
            process_template_instantiation(child, items, &doc);
        }
        "attributed_statement" | "attributed_declaration" => {
            // Unwrap: these wrap another node with [[...]] attributes
            let inner_children: Vec<_> = child.children().collect();
            for ic in &inner_children {
                if ic.kind().as_ref() != "attribute_declaration" {
                    dispatch_cpp_node(ic, siblings, idx, items, source);
                    dispatch_c_node(ic, siblings, idx, items, source);
                }
            }
            // Extract attribute text and annotate the emitted item
            for ic in &inner_children {
                if ic.kind().as_ref() == "attribute_declaration" {
                    let attr_text = ic.text().to_string();
                    let start_line = child.start_pos().line() as u32 + 1;
                    if let Some(item) = items.iter_mut().find(|i| i.start_line == start_line)
                        && !item.metadata.attributes.contains(&attr_text)
                    {
                        item.metadata.attributes.push(attr_text);
                    }
                }
            }
        }
        _ => {}
    }
}

/// Emit a `ParsedItem` for top-level operator function definitions that
/// the C extractor could not name (user-defined literals, free operator
/// overloads).  Skips functions already present in `items` by start line.
fn maybe_process_operator_function<D: ast_grep_core::Doc>(
    node: &Node<D>,
    siblings: &[Node<D>],
    idx: usize,
    items: &mut Vec<ParsedItem>,
    source: &str,
) {
    let start_line = node.start_pos().line() as u32 + 1;

    // If the C extractor already emitted this function, nothing to do.
    if items
        .iter()
        .any(|i| i.start_line == start_line && i.kind == SymbolKind::Function)
    {
        return;
    }

    let children: Vec<_> = node.children().collect();
    let Some(func_decl) = children
        .iter()
        .find(|c| c.kind().as_ref() == "function_declarator")
    else {
        return;
    };

    let name = find_identifier_recursive(func_decl);
    if name.is_empty() {
        return;
    }

    let doc = collect_doc_comment(siblings, idx, source);
    let return_type = extract_return_type_from_children(&children);
    let parameters = extract_parameters_from_declarator(func_decl);

    let mut metadata = SymbolMetadata::default();
    metadata.set_return_type(return_type);
    metadata.set_parameters(parameters);
    metadata.push_attribute("operator");

    items.push(ParsedItem {
        kind: SymbolKind::Function,
        name,
        signature: extract_signature(node),
        source: extract_source_limited(node, 30),
        doc_comment: doc,
        start_line,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata,
    });
}

// ── Namespace processing ───────────────────────────────────────────

fn process_namespace<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    source: &str,
    doc_comment: &str,
) {
    let children: Vec<_> = node.children().collect();

    // Detect inline namespace
    let is_inline = children.iter().any(|c| c.text().as_ref() == "inline");

    // Name: namespace_identifier, nested_namespace_specifier, or anonymous
    let name = children
        .iter()
        .find(|c| c.kind().as_ref() == "nested_namespace_specifier")
        .map(|c| c.text().to_string())
        .or_else(|| {
            children
                .iter()
                .find(|c| c.kind().as_ref() == "namespace_identifier")
                .map(|c| c.text().to_string())
        })
        .unwrap_or_else(|| "(anonymous)".to_string());

    let mut metadata = SymbolMetadata::default();
    metadata.push_attribute("namespace");
    if is_inline {
        metadata.push_attribute("inline");
    }

    items.push(ParsedItem {
        kind: SymbolKind::Module,
        name: name.clone(),
        signature: if is_inline {
            format!("inline namespace {name}")
        } else {
            format!("namespace {name}")
        },
        source: extract_source_limited(node, 5),
        doc_comment: doc_comment.to_string(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: if name == "(anonymous)" {
            Visibility::Private
        } else {
            Visibility::Public
        },
        metadata,
    });

    // Recurse into declaration_list for items inside the namespace
    let Some(decl_list) = children
        .iter()
        .find(|c| c.kind().as_ref() == "declaration_list")
    else {
        return;
    };

    let inner_children: Vec<_> = decl_list.children().collect();
    for (idx, child) in inner_children.iter().enumerate() {
        // Dispatch both C-style and C++ nodes inside namespaces
        dispatch_cpp_node(child, &inner_children, idx, items, source);
        dispatch_c_node(child, &inner_children, idx, items, source);
    }
}

fn process_namespace_alias<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
) {
    let children: Vec<_> = node.children().collect();
    let name = children
        .iter()
        .find(|c| c.kind().as_ref() == "namespace_identifier")
        .map_or_else(String::new, |n| n.text().to_string());
    if name.is_empty() {
        return;
    }
    let mut metadata = SymbolMetadata::default();
    metadata.push_attribute("namespace_alias");

    items.push(ParsedItem {
        kind: SymbolKind::Module,
        name,
        signature: extract_signature(node),
        source: Some(node.text().to_string()),
        doc_comment: doc_comment.to_string(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata,
    });
}

/// Dispatch C-style nodes that the C extractor would handle at top level
/// but won't see inside namespace blocks.
fn dispatch_c_node<D: ast_grep_core::Doc>(
    child: &Node<D>,
    siblings: &[Node<D>],
    idx: usize,
    items: &mut Vec<ParsedItem>,
    source: &str,
) {
    let kind = child.kind();
    match kind.as_ref() {
        "function_definition" => {
            let doc = collect_doc_comment(siblings, idx, source);
            process_c_function_definition(child, items, &doc);
        }
        "declaration" => {
            let doc = collect_doc_comment(siblings, idx, source);
            process_c_declaration(child, items, &doc);
        }
        "struct_specifier" => {
            let doc = collect_doc_comment(siblings, idx, source);
            process_c_struct(child, items, &doc);
        }
        "enum_specifier" => {
            let doc = collect_doc_comment(siblings, idx, source);
            process_c_enum(child, items, &doc);
        }
        "type_definition" => {
            let doc = collect_doc_comment(siblings, idx, source);
            process_c_typedef(child, items, &doc);
        }
        "union_specifier" => {
            let doc = collect_doc_comment(siblings, idx, source);
            process_c_union(child, items, &doc);
        }
        "attributed_statement" | "attributed_declaration" => {
            // Unwrap: these wrap another node with [[...]] attributes
            let inner_children: Vec<_> = child.children().collect();
            for ic in &inner_children {
                if ic.kind().as_ref() != "attribute_declaration" {
                    dispatch_c_node(ic, siblings, idx, items, source);
                }
            }
            // Extract attribute text and annotate the emitted item
            for ic in &inner_children {
                if ic.kind().as_ref() == "attribute_declaration" {
                    let attr_text = ic.text().to_string();
                    let start_line = child.start_pos().line() as u32 + 1;
                    if let Some(item) = items.iter_mut().find(|i| i.start_line == start_line)
                        && !item.metadata.attributes.contains(&attr_text)
                    {
                        item.metadata.attributes.push(attr_text);
                    }
                }
            }
        }
        _ => {}
    }
}

// ── Using / alias / static_assert / linkage ────────────────────────

fn process_alias_declaration<D: ast_grep_core::Doc>(
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
    let mut metadata = SymbolMetadata::default();
    metadata.push_attribute("using");

    items.push(ParsedItem {
        kind: SymbolKind::TypeAlias,
        name,
        signature: extract_signature(node),
        source: Some(node.text().to_string()),
        doc_comment: doc_comment.to_string(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata,
    });
}

fn process_using_declaration<D: ast_grep_core::Doc>(node: &Node<D>, items: &mut Vec<ParsedItem>) {
    let children: Vec<_> = node.children().collect();

    // Detect "using namespace X;" pattern
    let is_using_namespace = children.iter().any(|c| c.kind().as_ref() == "namespace");

    let name = children
        .iter()
        .find(|c| {
            let k = c.kind();
            k.as_ref() == "qualified_identifier" || k.as_ref() == "identifier"
        })
        .map_or_else(String::new, |n| n.text().to_string());
    if name.is_empty() {
        return;
    }

    let attr = if is_using_namespace {
        "using_directive".to_string()
    } else {
        "using_declaration".to_string()
    };

    let mut metadata = SymbolMetadata::default();
    metadata.push_attribute(attr);

    items.push(ParsedItem {
        kind: SymbolKind::Module,
        name,
        signature: node.text().to_string().trim().to_string(),
        source: Some(node.text().to_string()),
        doc_comment: String::new(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata,
    });
}

fn process_static_assert<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
) {
    let text = node.text().to_string();
    let sig = text.trim_end_matches(';').trim().to_string();

    let mut metadata = SymbolMetadata::default();
    metadata.push_attribute("static_assert");

    items.push(ParsedItem {
        kind: SymbolKind::Macro,
        name: "static_assert".to_string(),
        signature: sig,
        source: Some(text),
        doc_comment: doc_comment.to_string(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata,
    });
}

fn process_linkage_spec<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    source: &str,
) {
    // extern "C" { ... } or extern "C" single_decl
    let children: Vec<_> = node.children().collect();

    // Emit the linkage_specification itself
    let mut metadata = SymbolMetadata::default();
    metadata.push_attribute("linkage_specification");
    metadata.push_attribute("extern_c");

    items.push(ParsedItem {
        kind: SymbolKind::Module,
        name: "extern \"C\"".to_string(),
        signature: "extern \"C\"".to_string(),
        source: extract_source_limited(node, 5),
        doc_comment: String::new(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata,
    });

    // Process declarations inside the block
    for child in &children {
        match child.kind().as_ref() {
            "declaration_list" => {
                let inner: Vec<_> = child.children().collect();
                for (idx, ic) in inner.iter().enumerate() {
                    dispatch_c_node(ic, &inner, idx, items, source);
                }
            }
            "declaration" => {
                process_c_declaration(child, items, "");
            }
            "function_definition" => {
                process_c_function_definition(child, items, "");
            }
            _ => {}
        }
    }
}

// ── Doc comment helpers ────────────────────────────────────────────

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

fn strip_comment(text: &str) -> String {
    let text = text.trim();
    if let Some(rest) = text.strip_prefix("//") {
        return rest.trim().to_string();
    }
    let inner = text
        .strip_prefix("/**")
        .or_else(|| text.strip_prefix("/*"))
        .unwrap_or(text);
    let inner = inner.strip_suffix("*/").unwrap_or(inner);
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

// ── Signature / source helpers ─────────────────────────────────────

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
