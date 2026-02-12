//! C++ rich extractor.
//!
//! Delegates to the [`c`](super::c) extractor for shared C constructs
//! (functions, structs, unions, enums, typedefs, variables, preprocessor
//! directives), then layers C++-specific processing: classes (with
//! inheritance, access specifiers, virtual/override/final, constructors,
//! destructors), templates, namespaces, concepts, operator overloading,
//! using declarations/aliases, constexpr/consteval/constinit, `static_assert`,
//! RAII patterns, and extern "C" linkage.

use crate::types::{CppMetadataExt, ParsedItem, SymbolKind, SymbolMetadata, Visibility};
use ast_grep_core::Node;
use ast_grep_language::SupportLang;

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

// ── Lightweight C-node handlers for namespace interiors ─────────────

fn process_c_function_definition<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
) {
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

    let return_type = extract_return_type_from_children(&children);
    let parameters = extract_parameters_from_declarator(func_decl);

    let mut attrs = Vec::new();
    for child in &children {
        match child.kind().as_ref() {
            "storage_class_specifier" => {
                let t = child.text();
                match t.as_ref() {
                    "static" => attrs.push("static".to_string()),
                    "inline" => attrs.push("inline".to_string()),
                    "extern" => attrs.push("extern".to_string()),
                    _ => {}
                }
            }
            "type_qualifier" => {
                let t = child.text();
                match t.as_ref() {
                    "constexpr" => attrs.push("constexpr".to_string()),
                    "consteval" => attrs.push("consteval".to_string()),
                    _ => {}
                }
            }
            _ => {}
        }
    }

    let visibility = if attrs.contains(&"static".to_string()) {
        Visibility::Private
    } else {
        Visibility::Public
    };

    let mut metadata = SymbolMetadata::default();
    metadata.set_return_type(return_type);
    metadata.set_parameters(parameters);
    for attr in attrs {
        metadata.push_attribute(attr);
    }

    items.push(ParsedItem {
        kind: SymbolKind::Function,
        name,
        signature: extract_signature(node),
        source: extract_source_limited(node, 30),
        doc_comment: doc_comment.to_string(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility,
        metadata,
    });
}

fn process_c_declaration<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
) {
    let children: Vec<_> = node.children().collect();

    // Function prototype
    if let Some(func_decl) = children
        .iter()
        .find(|c| c.kind().as_ref() == "function_declarator")
    {
        let name = find_identifier_recursive(func_decl);
        if !name.is_empty() {
            let return_type = extract_return_type_from_children(&children);
            let parameters = extract_parameters_from_declarator(func_decl);
            let mut metadata = SymbolMetadata::default();
            metadata.set_return_type(return_type);
            metadata.set_parameters(parameters);
            metadata.push_attribute("prototype");

            items.push(ParsedItem {
                kind: SymbolKind::Function,
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
        return;
    }

    // Variable declaration
    let init_decls: Vec<_> = children
        .iter()
        .filter(|c| c.kind().as_ref() == "init_declarator")
        .collect();
    for init_decl in &init_decls {
        let name = find_identifier_recursive(init_decl);
        if name.is_empty() {
            continue;
        }
        let return_type = extract_return_type_from_children(&children);
        let is_const = children
            .iter()
            .any(|c| c.kind().as_ref() == "type_qualifier" && c.text().as_ref() == "const")
            || children
                .iter()
                .any(|c| c.kind().as_ref() == "type_qualifier" && c.text().as_ref() == "constexpr");

        let kind = if is_const {
            SymbolKind::Const
        } else {
            SymbolKind::Static
        };
        let mut metadata = SymbolMetadata::default();
        metadata.set_return_type(return_type);

        items.push(ParsedItem {
            kind,
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

    // Plain identifiers
    if init_decls.is_empty() {
        for child in &children {
            if child.kind().as_ref() == "identifier" {
                items.push(ParsedItem {
                    kind: SymbolKind::Static,
                    name: child.text().to_string(),
                    signature: extract_signature(node),
                    source: Some(node.text().to_string()),
                    doc_comment: doc_comment.to_string(),
                    start_line: node.start_pos().line() as u32 + 1,
                    end_line: node.end_pos().line() as u32 + 1,
                    visibility: Visibility::Public,
                    metadata: SymbolMetadata::default(),
                });
            }
        }
    }
}

fn process_c_struct<D: ast_grep_core::Doc>(
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
        .any(|c| c.kind().as_ref() == "field_declaration_list");
    if has_body {
        let fields = extract_field_names(node);
        let methods = extract_method_names(node);
        let mut metadata = SymbolMetadata::default();
        metadata.set_fields(fields);
        metadata.set_methods(methods);

        items.push(ParsedItem {
            kind: SymbolKind::Struct,
            name,
            signature: extract_signature(node),
            source: extract_source_limited(node, 30),
            doc_comment: doc_comment.to_string(),
            start_line: node.start_pos().line() as u32 + 1,
            end_line: node.end_pos().line() as u32 + 1,
            visibility: Visibility::Public,
            metadata,
        });
    }
}

fn process_c_enum<D: ast_grep_core::Doc>(
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
    let is_scoped = node.children().any(|c| c.kind().as_ref() == "class");
    let variants = extract_enum_variants(node);
    let mut metadata = SymbolMetadata::default();
    metadata.set_variants(variants);
    if is_scoped {
        metadata.push_attribute("scoped_enum");
    }
    items.push(ParsedItem {
        kind: SymbolKind::Enum,
        name,
        signature: extract_signature(node),
        source: extract_source_limited(node, 30),
        doc_comment: doc_comment.to_string(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata,
    });
}

fn process_c_typedef<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
) {
    let children: Vec<_> = node.children().collect();
    let name = children
        .iter()
        .filter(|c| c.kind().as_ref() == "type_identifier")
        .last()
        .map_or_else(String::new, |n| n.text().to_string());
    if name.is_empty() {
        return;
    }
    let mut metadata = SymbolMetadata::default();
    metadata.push_attribute("typedef");

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

fn process_c_union<D: ast_grep_core::Doc>(
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
    let fields = extract_field_names(node);
    let mut metadata = SymbolMetadata::default();
    metadata.set_fields(fields);

    items.push(ParsedItem {
        kind: SymbolKind::Union,
        name,
        signature: extract_signature(node),
        source: extract_source_limited(node, 30),
        doc_comment: doc_comment.to_string(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata,
    });
}

// ── Class processing ───────────────────────────────────────────────

#[allow(clippy::too_many_lines)]
fn process_class<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
    template_params: Option<&str>,
) {
    let children: Vec<_> = node.children().collect();

    let name = children
        .iter()
        .find(|c| c.kind().as_ref() == "type_identifier" || c.kind().as_ref() == "template_type")
        .map_or_else(String::new, |n| n.text().to_string());
    if name.is_empty() {
        return;
    }

    let is_final = children
        .iter()
        .any(|c| c.kind().as_ref() == "virtual_specifier" && c.text().as_ref() == "final");

    let base_classes = extract_base_classes(&children);
    let (methods, fields, has_pure_virtual, access_sections) = extract_class_members(node);
    let is_abstract = has_pure_virtual;

    let is_error_type = name.ends_with("Error")
        || name.ends_with("Exception")
        || base_classes.iter().any(|b| b.contains("exception"));

    let mut attrs = vec!["class".to_string()];
    if is_final {
        attrs.push("final".to_string());
    }
    if is_abstract {
        attrs.push("abstract".to_string());
    }
    if template_params.is_some() {
        attrs.push("template".to_string());
    }
    attrs.extend(access_sections);

    let mut metadata = SymbolMetadata::default();
    metadata.set_base_classes(base_classes);
    metadata.set_methods(methods);
    metadata.set_fields(fields);
    if is_abstract {
        metadata.mark_unsafe();
    }
    if is_error_type {
        metadata.mark_error_type();
    }
    metadata.set_generics(template_params.map(String::from));
    for attr in attrs {
        metadata.push_attribute(attr);
    }

    items.push(ParsedItem {
        kind: SymbolKind::Class,
        name,
        signature: extract_signature(node),
        source: extract_source_limited(node, 40),
        doc_comment: doc_comment.to_string(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata,
    });

    // Emit nested types (nested classes, structs, enums, aliases) as
    // separate ParsedItems.
    extract_nested_types(node, items);
}

/// Walk a class/struct body and emit separate `ParsedItem`s for nested
/// type definitions (classes, structs, enums, aliases, templates).
fn extract_nested_types<D: ast_grep_core::Doc>(node: &Node<D>, items: &mut Vec<ParsedItem>) {
    let Some(body) = node
        .children()
        .find(|c| c.kind().as_ref() == "field_declaration_list")
    else {
        return;
    };
    for child in body.children() {
        dispatch_nested_type(&child, items);
    }
}

/// Dispatch a single child from a `field_declaration_list` to the
/// appropriate nested-type handler.  Also handles `field_declaration`
/// nodes that wrap type specifiers (tree-sitter-cpp wraps nested
/// `enum class E { … };` inside a `field_declaration`).
fn dispatch_nested_type<D: ast_grep_core::Doc>(node: &Node<D>, items: &mut Vec<ParsedItem>) {
    match node.kind().as_ref() {
        "class_specifier" => {
            process_class(node, items, "", None);
        }
        "struct_specifier" => {
            process_c_struct(node, items, "");
        }
        "enum_specifier" => {
            process_c_enum(node, items, "");
        }
        "alias_declaration" => {
            process_alias_declaration(node, items, "");
        }
        "template_declaration" => {
            process_template_declaration(node, items, "", "");
        }
        "field_declaration" => {
            // field_declaration may wrap a nested type specifier, e.g.:
            //   field_declaration: "enum class E { A, B };"
            //     enum_specifier: "enum class E { A, B }"
            for inner in node.children() {
                let k = inner.kind();
                if k.as_ref() == "enum_specifier"
                    || k.as_ref() == "class_specifier"
                    || k.as_ref() == "struct_specifier"
                    || k.as_ref() == "union_specifier"
                {
                    dispatch_nested_type(&inner, items);
                }
            }
        }
        _ => {}
    }
}

fn extract_base_classes<D: ast_grep_core::Doc>(children: &[Node<D>]) -> Vec<String> {
    let Some(clause) = children
        .iter()
        .find(|c| c.kind().as_ref() == "base_class_clause")
    else {
        return Vec::new();
    };
    let clause_children: Vec<_> = clause.children().collect();
    clause_children
        .iter()
        .filter(|c| {
            let k = c.kind();
            k.as_ref() == "type_identifier"
                || k.as_ref() == "qualified_identifier"
                || k.as_ref() == "template_type"
        })
        .map(|c| c.text().to_string())
        .collect()
}

/// Extract methods, fields, and detect pure virtuals from a class body.
///
/// Returns `(methods, fields, has_pure_virtual, access_sections)`.
/// `access_sections` contains markers like `"has_public_members"`,
/// `"has_protected_members"`, `"has_private_members"` for each access
/// level that contains at least one member, plus friend declarations
/// (as `"friend:Name"`) and method qualifier markers
/// (`"has_override"`, `"has_final_methods"`, `"has_deleted_members"`,
/// `"has_defaulted_members"`).
#[allow(clippy::too_many_lines)]
fn extract_class_members<D: ast_grep_core::Doc>(
    node: &Node<D>,
) -> (Vec<String>, Vec<String>, bool, Vec<String>) {
    let mut methods = Vec::new();
    let mut fields = Vec::new();
    let mut has_pure_virtual = false;
    let mut has_public = false;
    let mut has_protected = false;
    let mut has_private = false;
    let mut friends: Vec<String> = Vec::new();
    let mut has_override = false;
    let mut has_final_methods = false;
    let mut has_deleted_members = false;
    let mut has_defaulted_members = false;
    // Default access for `class` is private
    let mut current_access = "private";

    let Some(body) = node
        .children()
        .find(|c| c.kind().as_ref() == "field_declaration_list")
    else {
        return (methods, fields, has_pure_virtual, Vec::new());
    };

    let body_children: Vec<_> = body.children().collect();
    for child in &body_children {
        match child.kind().as_ref() {
            "access_specifier" => {
                let text = child.text().to_string();
                let trimmed = text.trim().trim_end_matches(':').trim();
                match trimmed {
                    "public" => current_access = "public",
                    "protected" => current_access = "protected",
                    "private" => current_access = "private",
                    _ => {}
                }
            }
            "function_definition" => {
                // Detect method qualifiers on function_definition
                detect_method_qualifiers(
                    child,
                    &mut has_override,
                    &mut has_final_methods,
                    &mut has_deleted_members,
                    &mut has_defaulted_members,
                );
                if let Some(name) = extract_method_name(child) {
                    match current_access {
                        "public" => has_public = true,
                        "protected" => has_protected = true,
                        _ => has_private = true,
                    }
                    methods.push(name);
                }
            }
            "field_declaration" => {
                // Could be: pure virtual method, regular field, or friend
                let fc_children: Vec<_> = child.children().collect();

                // Detect method qualifiers on field declarations with function_declarators
                detect_method_qualifiers(
                    child,
                    &mut has_override,
                    &mut has_final_methods,
                    &mut has_deleted_members,
                    &mut has_defaulted_members,
                );

                // Pure virtual: has function_declarator + "= 0"
                let has_func_decl = fc_children
                    .iter()
                    .any(|c| c.kind().as_ref() == "function_declarator");
                let has_zero = fc_children
                    .iter()
                    .any(|c| c.kind().as_ref() == "number_literal" && c.text().as_ref() == "0");

                if has_func_decl && has_zero {
                    has_pure_virtual = true;
                    if let Some(name) = extract_field_decl_method_name(&fc_children) {
                        match current_access {
                            "public" => has_public = true,
                            "protected" => has_protected = true,
                            _ => has_private = true,
                        }
                        methods.push(name);
                    }
                } else if has_func_decl {
                    // Non-pure virtual method declaration
                    if let Some(name) = extract_field_decl_method_name(&fc_children) {
                        match current_access {
                            "public" => has_public = true,
                            "protected" => has_protected = true,
                            _ => has_private = true,
                        }
                        methods.push(name);
                    }
                } else {
                    // Regular field — look for field_identifier
                    for fc in &fc_children {
                        if fc.kind().as_ref() == "field_identifier" {
                            match current_access {
                                "public" => has_public = true,
                                "protected" => has_protected = true,
                                _ => has_private = true,
                            }
                            fields.push(fc.text().to_string());
                        }
                    }
                    // Also check for class_specifier inside (nested class as field_declaration)
                }
            }
            "friend_declaration" => {
                // Extract the friend name from friend declarations.
                // tree-sitter-cpp wraps `friend void f();` as:
                //   friend_declaration > declaration > function_declarator > identifier
                // and `friend class B;` may have type_identifier directly.
                // We look at non-"friend" children, trying direct matches first,
                // then recursing into `declaration` children.
                let fc: Vec<_> = child.children().collect();
                let friend_name = fc
                    .iter()
                    .filter(|c| c.kind().as_ref() != "friend" && c.kind().as_ref() != ";")
                    .find_map(|c| {
                        let k = c.kind();
                        if k.as_ref() == "type_identifier" || k.as_ref() == "identifier" {
                            Some(c.text().to_string())
                        } else if k.as_ref() == "function_declarator" {
                            let name = find_identifier_recursive(c);
                            if name.is_empty() { None } else { Some(name) }
                        } else if k.as_ref() == "declaration" {
                            // friend void f(); → declaration contains
                            // function_declarator with the actual name
                            let name = find_identifier_recursive(c);
                            if name.is_empty() { None } else { Some(name) }
                        } else {
                            None
                        }
                    });
                if let Some(name) = friend_name
                    && !name.is_empty()
                {
                    friends.push(name);
                }
            }
            _ => {}
        }
    }

    let mut access_sections = Vec::new();
    if has_public {
        access_sections.push("has_public_members".to_string());
    }
    if has_protected {
        access_sections.push("has_protected_members".to_string());
    }
    if has_private {
        access_sections.push("has_private_members".to_string());
    }
    for friend in &friends {
        access_sections.push(format!("friend:{friend}"));
    }
    if has_override {
        access_sections.push("has_override".to_string());
    }
    if has_final_methods {
        access_sections.push("has_final_methods".to_string());
    }
    if has_deleted_members {
        access_sections.push("has_deleted_members".to_string());
    }
    if has_defaulted_members {
        access_sections.push("has_defaulted_members".to_string());
    }

    (methods, fields, has_pure_virtual, access_sections)
}

/// Detect method qualifiers (override, final, = delete, = default) on a node.
fn detect_method_qualifiers<D: ast_grep_core::Doc>(
    node: &Node<D>,
    has_override: &mut bool,
    has_final_methods: &mut bool,
    has_deleted_members: &mut bool,
    has_defaulted_members: &mut bool,
) {
    for child in node.children() {
        match child.kind().as_ref() {
            "virtual_specifier" => {
                let text = child.text();
                match text.as_ref() {
                    "override" => *has_override = true,
                    "final" => *has_final_methods = true,
                    _ => {}
                }
            }
            "default_method_clause" => *has_defaulted_members = true,
            "delete_method_clause" => *has_deleted_members = true,
            "function_declarator" => {
                // Recurse into function_declarator to find qualifiers
                for fc in child.children() {
                    if fc.kind().as_ref() == "virtual_specifier" {
                        let text = fc.text();
                        match text.as_ref() {
                            "override" => *has_override = true,
                            "final" => *has_final_methods = true,
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

fn extract_method_name<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<String> {
    let children: Vec<_> = node.children().collect();

    // Check for destructor
    for child in &children {
        if child.kind().as_ref() == "function_declarator" {
            let dc: Vec<_> = child.children().collect();
            for d in &dc {
                if d.kind().as_ref() == "destructor_name" {
                    return Some(d.text().to_string());
                }
                if d.kind().as_ref() == "operator_name" {
                    return Some(d.text().to_string());
                }
                if d.kind().as_ref() == "field_identifier" {
                    return Some(d.text().to_string());
                }
                if d.kind().as_ref() == "identifier" {
                    return Some(d.text().to_string());
                }
            }
        }
        // operator_cast: `operator float() const`
        if child.kind().as_ref() == "operator_cast" {
            return Some(child.text().split('(').next()?.trim().to_string());
        }
        // reference_declarator wrapping function_declarator (e.g., `Vec2& operator++()`)
        if child.kind().as_ref() == "reference_declarator" {
            let rc: Vec<_> = child.children().collect();
            for r in &rc {
                if r.kind().as_ref() == "function_declarator" {
                    let fc: Vec<_> = r.children().collect();
                    for f in &fc {
                        if f.kind().as_ref() == "operator_name" {
                            return Some(f.text().to_string());
                        }
                        if f.kind().as_ref() == "field_identifier" {
                            return Some(f.text().to_string());
                        }
                    }
                }
            }
        }
    }
    None
}

fn extract_field_decl_method_name<D: ast_grep_core::Doc>(children: &[Node<D>]) -> Option<String> {
    for child in children {
        if child.kind().as_ref() == "function_declarator" {
            let dc: Vec<_> = child.children().collect();
            for d in &dc {
                if d.kind().as_ref() == "field_identifier"
                    || d.kind().as_ref() == "identifier"
                    || d.kind().as_ref() == "operator_name"
                    || d.kind().as_ref() == "destructor_name"
                {
                    return Some(d.text().to_string());
                }
            }
        }
    }
    None
}

// ── Template processing ────────────────────────────────────────────

#[allow(clippy::only_used_in_recursion, clippy::too_many_lines)]
fn process_template_declaration<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    source: &str,
    doc_comment: &str,
) {
    let children: Vec<_> = node.children().collect();

    let template_params = children
        .iter()
        .find(|c| c.kind().as_ref() == "template_parameter_list")
        .map(|c| c.text().to_string());

    // Detect requires clause on the template declaration itself
    let requires_constraint = children
        .iter()
        .find(|c| c.kind().as_ref() == "requires_clause")
        .map(|c| c.text().to_string());

    let items_before = items.len();

    for child in &children {
        match child.kind().as_ref() {
            "class_specifier" => {
                process_class(child, items, doc_comment, template_params.as_deref());
            }
            "struct_specifier" => {
                // Template struct — emit with template attribute
                let name = child
                    .children()
                    .find(|c| c.kind().as_ref() == "type_identifier")
                    .map_or_else(String::new, |n| n.text().to_string());
                if !name.is_empty() {
                    let fields = extract_field_names(child);
                    let methods = extract_method_names(child);
                    let mut metadata = SymbolMetadata::default();
                    metadata.set_fields(fields);
                    metadata.set_methods(methods);
                    metadata.set_generics(template_params.clone());
                    metadata.push_attribute("template");

                    items.push(ParsedItem {
                        kind: SymbolKind::Struct,
                        name,
                        signature: extract_signature(node),
                        source: extract_source_limited(node, 30),
                        doc_comment: doc_comment.to_string(),
                        start_line: node.start_pos().line() as u32 + 1,
                        end_line: node.end_pos().line() as u32 + 1,
                        visibility: Visibility::Public,
                        metadata,
                    });
                }
            }
            "function_definition" => {
                // Template function — emit directly
                process_template_function(
                    node,
                    child,
                    items,
                    doc_comment,
                    template_params.as_deref(),
                );
            }
            "declaration" => {
                // Could be template variable or template function prototype
                process_template_function_decl(
                    node,
                    child,
                    items,
                    doc_comment,
                    template_params.as_deref(),
                );
            }
            "alias_declaration" => {
                // Template alias: template<typename T> using Ptr = T*;
                let alias_name = child
                    .children()
                    .find(|c| c.kind().as_ref() == "type_identifier")
                    .map_or_else(String::new, |n| n.text().to_string());
                if !alias_name.is_empty() {
                    let mut metadata = SymbolMetadata::default();
                    metadata.set_generics(template_params.clone());
                    metadata.push_attribute("template");
                    metadata.push_attribute("using");

                    items.push(ParsedItem {
                        kind: SymbolKind::TypeAlias,
                        name: alias_name,
                        signature: extract_signature(node),
                        source: Some(node.text().to_string()),
                        doc_comment: doc_comment.to_string(),
                        start_line: node.start_pos().line() as u32 + 1,
                        end_line: node.end_pos().line() as u32 + 1,
                        visibility: Visibility::Public,
                        metadata,
                    });
                }
            }
            "concept_definition" => {
                process_concept(child, items, doc_comment, template_params.as_deref());
            }
            "template_declaration" => {
                // Nested template — recurse
                let inner_doc = doc_comment.to_string();
                process_template_declaration(child, items, source, &inner_doc);
            }
            _ => {}
        }
    }

    // Annotate newly emitted items with the requires clause if present
    if let Some(ref constraint) = requires_constraint {
        for item in items.iter_mut().skip(items_before) {
            let attr = format!("requires:{constraint}");
            if !item.metadata.attributes.contains(&attr) {
                item.metadata.attributes.push(attr);
            }
        }
    }
}

fn process_template_instantiation<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
) {
    let sig = extract_signature(node);

    // tree-sitter-cpp wraps `template class V<int>;` as:
    //   template_instantiation
    //     class_specifier (or struct_specifier)
    //       template_type: "V<int>"
    // So we first look inside class_specifier/struct_specifier children,
    // then fall back to direct children of the node.
    let name = node
        .children()
        .find(|c| {
            let k = c.kind();
            k.as_ref() == "class_specifier" || k.as_ref() == "struct_specifier"
        })
        .and_then(|spec| {
            spec.children()
                .find(|c| c.kind().as_ref() == "template_type")
                .map(|n| n.text().to_string())
                .or_else(|| {
                    spec.children()
                        .find(|c| c.kind().as_ref() == "type_identifier")
                        .map(|n| n.text().to_string())
                })
        })
        .or_else(|| {
            node.children()
                .find(|c| c.kind().as_ref() == "template_type")
                .map(|n| n.text().to_string())
        })
        .or_else(|| {
            node.children()
                .find(|c| c.kind().as_ref() == "type_identifier")
                .map(|n| n.text().to_string())
        })
        .unwrap_or_default();
    if name.is_empty() {
        return;
    }
    let mut metadata = SymbolMetadata::default();
    metadata.push_attribute("explicit_instantiation");

    items.push(ParsedItem {
        kind: SymbolKind::Class,
        name,
        signature: sig,
        source: Some(node.text().to_string()),
        doc_comment: doc_comment.to_string(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata,
    });
}

fn process_template_function<D: ast_grep_core::Doc>(
    template_node: &Node<D>,
    func_node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
    template_params: Option<&str>,
) {
    let children: Vec<_> = func_node.children().collect();
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

    let return_type = extract_return_type_from_children(&children);
    let parameters = extract_parameters_from_declarator(func_decl);

    let mut attrs = vec!["template".to_string()];
    for child in &children {
        match child.kind().as_ref() {
            "storage_class_specifier" => {
                attrs.push(child.text().to_string());
            }
            "requires_clause" => {
                attrs.push(format!("requires:{}", child.text()));
            }
            _ => {}
        }
    }

    let mut metadata = SymbolMetadata::default();
    metadata.set_return_type(return_type);
    metadata.set_parameters(parameters);
    metadata.set_generics(template_params.map(String::from));
    for attr in attrs {
        metadata.push_attribute(attr);
    }

    items.push(ParsedItem {
        kind: SymbolKind::Function,
        name,
        signature: extract_signature(template_node),
        source: extract_source_limited(template_node, 30),
        doc_comment: doc_comment.to_string(),
        start_line: template_node.start_pos().line() as u32 + 1,
        end_line: template_node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata,
    });
}

fn process_template_function_decl<D: ast_grep_core::Doc>(
    template_node: &Node<D>,
    decl_node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
    template_params: Option<&str>,
) {
    let children: Vec<_> = decl_node.children().collect();
    let Some(func_decl) = children
        .iter()
        .find(|c| c.kind().as_ref() == "function_declarator")
    else {
        // Template variable — look for init_declarator
        for child in &children {
            if child.kind().as_ref() == "init_declarator" {
                let name = find_identifier_recursive(child);
                if !name.is_empty() {
                    let mut metadata = SymbolMetadata::default();
                    metadata.set_generics(template_params.map(String::from));
                    metadata.push_attribute("template");

                    items.push(ParsedItem {
                        kind: SymbolKind::Static,
                        name,
                        signature: extract_signature(template_node),
                        source: Some(template_node.text().to_string()),
                        doc_comment: doc_comment.to_string(),
                        start_line: template_node.start_pos().line() as u32 + 1,
                        end_line: template_node.end_pos().line() as u32 + 1,
                        visibility: Visibility::Public,
                        metadata,
                    });
                }
            }
        }
        return;
    };

    let name = find_identifier_recursive(func_decl);
    if name.is_empty() {
        return;
    }
    let return_type = extract_return_type_from_children(&children);
    let parameters = extract_parameters_from_declarator(func_decl);

    let mut metadata = SymbolMetadata::default();
    metadata.set_return_type(return_type);
    metadata.set_parameters(parameters);
    metadata.set_generics(template_params.map(String::from));
    metadata.push_attribute("template");
    metadata.push_attribute("prototype");

    items.push(ParsedItem {
        kind: SymbolKind::Function,
        name,
        signature: extract_signature(template_node),
        source: Some(template_node.text().to_string()),
        doc_comment: doc_comment.to_string(),
        start_line: template_node.start_pos().line() as u32 + 1,
        end_line: template_node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata,
    });
}

fn process_concept<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
    template_params: Option<&str>,
) {
    let name = node
        .children()
        .find(|c| c.kind().as_ref() == "identifier")
        .map_or_else(String::new, |n| n.text().to_string());
    if name.is_empty() {
        return;
    }

    let mut metadata = SymbolMetadata::default();
    metadata.set_generics(template_params.map(String::from));
    metadata.push_attribute("concept");

    items.push(ParsedItem {
        kind: SymbolKind::Trait,
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

// ── Enrichment pass ────────────────────────────────────────────────

fn enrich_items<D: ast_grep_core::Doc>(root: &Node<D>, items: &mut Vec<ParsedItem>) {
    enrich_recursive(root, items);
}

fn enrich_recursive<D: ast_grep_core::Doc>(node: &Node<D>, items: &mut Vec<ParsedItem>) {
    let kind = node.kind();
    let start_line = node.start_pos().line() as u32 + 1;

    match kind.as_ref() {
        "enum_specifier" => {
            // Detect scoped enum (enum class)
            let is_scoped = node.children().any(|c| c.kind().as_ref() == "class");
            if is_scoped
                && let Some(item) = items
                    .iter_mut()
                    .find(|i| i.kind == SymbolKind::Enum && i.start_line == start_line)
                && !item
                    .metadata
                    .attributes
                    .contains(&"scoped_enum".to_string())
            {
                item.metadata.attributes.push("scoped_enum".to_string());
            }
        }
        "function_definition" | "declaration" => {
            let children: Vec<_> = node.children().collect();
            let mut cpp_attrs = Vec::new();

            for child in &children {
                match child.kind().as_ref() {
                    "type_qualifier" => {
                        let t = child.text();
                        match t.as_ref() {
                            "constexpr" => cpp_attrs.push("constexpr".to_string()),
                            "consteval" => cpp_attrs.push("consteval".to_string()),
                            "constinit" => cpp_attrs.push("constinit".to_string()),
                            _ => {}
                        }
                    }
                    "function_declarator" => {
                        let fc: Vec<_> = child.children().collect();
                        for f in &fc {
                            if f.kind().as_ref() == "noexcept" {
                                cpp_attrs.push("noexcept".to_string());
                            }
                            if f.kind().as_ref() == "trailing_return_type" {
                                let rt_text = f
                                    .text()
                                    .to_string()
                                    .trim_start_matches("->")
                                    .trim()
                                    .to_string();
                                if let Some(item) = items.iter_mut().find(|i| {
                                    i.start_line == start_line
                                        && (i.kind == SymbolKind::Function
                                            || i.kind == SymbolKind::Const
                                            || i.kind == SymbolKind::Static)
                                }) {
                                    item.metadata.return_type = Some(rt_text);
                                }
                            }
                        }
                    }
                    "placeholder_type_specifier" => {
                        cpp_attrs.push("auto".to_string());
                    }
                    _ => {}
                }
            }

            if !cpp_attrs.is_empty()
                && let Some(item) = items.iter_mut().find(|i| i.start_line == start_line)
            {
                for attr in &cpp_attrs {
                    if !item.metadata.attributes.contains(attr) {
                        item.metadata.attributes.push(attr.clone());
                    }
                }
                // constexpr/consteval/constinit variables should be Const
                if (item.kind == SymbolKind::Static)
                    && (cpp_attrs.contains(&"constexpr".to_string())
                        || cpp_attrs.contains(&"constinit".to_string()))
                {
                    item.kind = SymbolKind::Const;
                }
            }
        }
        "attributed_declaration" | "attributed_statement" => {
            // Annotate items with C++11 [[...]] attributes from this wrapper
            let attr_start_line = start_line;
            for attr_child in node.children() {
                if attr_child.kind().as_ref() == "attribute_declaration" {
                    let attr_text = attr_child.text().to_string();
                    if let Some(item) = items.iter_mut().find(|i| i.start_line == attr_start_line)
                        && !item.metadata.attributes.contains(&attr_text)
                    {
                        item.metadata.attributes.push(attr_text);
                    }
                }
            }
        }
        _ => {}
    }

    let children: Vec<_> = node.children().collect();
    for child in &children {
        enrich_recursive(child, items);
    }
}

// ── Shared extraction helpers ──────────────────────────────────────

fn find_identifier_recursive<D: ast_grep_core::Doc>(node: &Node<D>) -> String {
    let children: Vec<_> = node.children().collect();
    for child in &children {
        match child.kind().as_ref() {
            // qualified_identifier preserves `::` separators (e.g. MyClass::method)
            "identifier"
            | "field_identifier"
            | "operator_name"
            | "destructor_name"
            | "qualified_identifier" => {
                return child.text().to_string();
            }
            "structured_binding_declarator" => {
                // Extract all identifiers and join as "[x, y]"
                let ids: Vec<String> = child
                    .children()
                    .filter(|c| c.kind().as_ref() == "identifier")
                    .map(|c| c.text().to_string())
                    .collect();
                if !ids.is_empty() {
                    return format!("[{}]", ids.join(", "));
                }
            }
            "pointer_declarator"
            | "reference_declarator"
            | "init_declarator"
            | "function_declarator"
            | "parenthesized_declarator"
            | "attributed_declarator" => {
                let name = find_identifier_recursive(child);
                if !name.is_empty() {
                    return name;
                }
            }
            _ => {}
        }
    }
    String::new()
}

fn extract_return_type_from_children<D: ast_grep_core::Doc>(
    children: &[Node<D>],
) -> Option<String> {
    let mut parts = Vec::new();
    for child in children {
        match child.kind().as_ref() {
            "primitive_type"
            | "type_identifier"
            | "sized_type_specifier"
            | "type_qualifier"
            | "struct_specifier"
            | "qualified_identifier"
            | "template_type"
            | "decltype" => {
                parts.push(child.text().to_string());
            }
            "placeholder_type_specifier" => {
                parts.push("auto".to_string());
            }
            "function_declarator"
            | "init_declarator"
            | "identifier"
            | "array_declarator"
            | "pointer_declarator"
            | "reference_declarator"
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

fn extract_parameters_from_declarator<D: ast_grep_core::Doc>(func_decl: &Node<D>) -> Vec<String> {
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
            k.as_ref() == "parameter_declaration"
                || k.as_ref() == "variadic_parameter"
                || k.as_ref() == "optional_parameter_declaration"
        })
        .map(|c| {
            c.text()
                .to_string()
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect()
}

fn extract_field_names<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    let mut fields = Vec::new();
    let Some(body) = node
        .children()
        .find(|c| c.kind().as_ref() == "field_declaration_list")
    else {
        return fields;
    };
    let body_children: Vec<_> = body.children().collect();
    for child in &body_children {
        if child.kind().as_ref() == "field_declaration" {
            let fc: Vec<_> = child.children().collect();
            // Skip if it has a function_declarator (it's a method, not a field)
            if fc
                .iter()
                .any(|c| c.kind().as_ref() == "function_declarator")
            {
                continue;
            }
            for f in &fc {
                if f.kind().as_ref() == "field_identifier" {
                    fields.push(f.text().to_string());
                }
            }
        }
    }
    fields
}

fn extract_method_names<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    let mut methods = Vec::new();
    let Some(body) = node
        .children()
        .find(|c| c.kind().as_ref() == "field_declaration_list")
    else {
        return methods;
    };
    let body_children: Vec<_> = body.children().collect();
    for child in &body_children {
        if child.kind().as_ref() == "function_definition"
            && let Some(name) = extract_method_name(child)
        {
            methods.push(name);
        }
    }
    methods
}

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

// ── Doc comment helpers (adapted from C extractor) ─────────────────

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
