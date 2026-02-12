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
    let mut items = super::c::extract(root, source)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use ast_grep_language::LanguageExt;

    fn parse_and_extract(source: &str) -> Vec<ParsedItem> {
        let root = SupportLang::Cpp.ast_grep(source);
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

    #[allow(dead_code)]
    fn find_by_name_prefix<'a>(items: &'a [ParsedItem], prefix: &str) -> Vec<&'a ParsedItem> {
        items
            .iter()
            .filter(|i| i.name.starts_with(prefix))
            .collect()
    }

    fn fixture_items() -> Vec<ParsedItem> {
        let source = include_str!("../../tests/fixtures/sample.cpp");
        parse_and_extract(source)
    }

    // ════════════════════════════════════════════════════════════════
    // 1. Smoke / fixture tests
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn fixture_parses_without_error() {
        let items = fixture_items();
        assert!(items.len() >= 40, "expected 40+ items, got {}", items.len());
    }

    #[test]
    fn fixture_has_classes() {
        let items = fixture_items();
        let classes = find_all_by_kind(&items, SymbolKind::Class);
        assert!(
            classes.len() >= 15,
            "expected 15+ classes, got {}",
            classes.len()
        );
    }

    #[test]
    fn fixture_has_functions() {
        let items = fixture_items();
        let funcs = find_all_by_kind(&items, SymbolKind::Function);
        assert!(
            funcs.len() >= 15,
            "expected 15+ functions, got {}",
            funcs.len()
        );
    }

    #[test]
    fn fixture_has_enums() {
        let items = fixture_items();
        let enums = find_all_by_kind(&items, SymbolKind::Enum);
        assert!(enums.len() >= 3, "expected 3+ enums, got {}", enums.len());
    }

    #[test]
    fn fixture_has_structs() {
        let items = fixture_items();
        let structs = find_all_by_kind(&items, SymbolKind::Struct);
        assert!(
            structs.len() >= 3,
            "expected 3+ structs, got {}",
            structs.len()
        );
    }

    #[test]
    fn fixture_has_modules() {
        let items = fixture_items();
        let mods = find_all_by_kind(&items, SymbolKind::Module);
        assert!(mods.len() >= 10, "expected 10+ modules, got {}", mods.len());
    }

    #[test]
    fn fixture_has_type_aliases() {
        let items = fixture_items();
        let aliases = find_all_by_kind(&items, SymbolKind::TypeAlias);
        assert!(
            aliases.len() >= 5,
            "expected 5+ type aliases, got {}",
            aliases.len()
        );
    }

    #[test]
    fn fixture_has_consts() {
        let items = fixture_items();
        let consts = find_all_by_kind(&items, SymbolKind::Const);
        assert!(
            consts.len() >= 5,
            "expected 5+ consts, got {}",
            consts.len()
        );
    }

    #[test]
    fn fixture_has_traits() {
        let items = fixture_items();
        let traits = find_all_by_kind(&items, SymbolKind::Trait);
        assert!(
            traits.len() >= 2,
            "expected 2+ traits (concepts), got {}",
            traits.len()
        );
    }

    #[test]
    fn fixture_has_macros() {
        let items = fixture_items();
        let macros = find_all_by_kind(&items, SymbolKind::Macro);
        assert!(
            macros.len() >= 3,
            "expected 3+ macros (static_assert + defines), got {}",
            macros.len()
        );
    }

    // ════════════════════════════════════════════════════════════════
    // 2. Include tests
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn includes_extracted() {
        let items = fixture_items();
        let includes: Vec<_> = items
            .iter()
            .filter(|i| i.kind == SymbolKind::Module && i.name.starts_with('<'))
            .collect();
        assert!(
            includes.len() >= 9,
            "expected 9+ includes, got {}",
            includes.len()
        );
    }

    #[test]
    fn include_iostream_present() {
        let items = fixture_items();
        assert!(
            items.iter().any(|i| i.name.contains("iostream")),
            "expected <iostream> include"
        );
    }

    #[test]
    fn include_concepts_present() {
        let items = fixture_items();
        assert!(
            items.iter().any(|i| i.name.contains("concepts")),
            "expected <concepts> include"
        );
    }

    // ════════════════════════════════════════════════════════════════
    // 3. Class tests
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn class_shape_exists() {
        let items = fixture_items();
        let shape = find_by_name(&items, "Shape");
        assert_eq!(shape.kind, SymbolKind::Class, "Shape should be a Class");
    }

    #[test]
    fn class_shape_is_abstract() {
        let items = fixture_items();
        let shape = find_by_name(&items, "Shape");
        assert!(
            shape.metadata.attributes.contains(&"abstract".to_string()),
            "Shape should have abstract attribute"
        );
    }

    #[test]
    fn class_shape_has_methods() {
        let items = fixture_items();
        let shape = find_by_name(&items, "Shape");
        assert!(
            shape.metadata.methods.contains(&"area".to_string()),
            "Shape should have area method"
        );
        assert!(
            shape.metadata.methods.contains(&"perimeter".to_string()),
            "Shape should have perimeter method"
        );
        assert!(
            shape.metadata.methods.contains(&"name".to_string()),
            "Shape should have name method"
        );
    }

    #[test]
    fn class_shape_has_destructor() {
        let items = fixture_items();
        let shape = find_by_name(&items, "Shape");
        assert!(
            shape.metadata.methods.iter().any(|m| m.contains('~')),
            "Shape should have destructor in methods"
        );
    }

    #[test]
    fn class_circle_exists() {
        let items = fixture_items();
        let circle = find_by_name(&items, "Circle");
        assert_eq!(circle.kind, SymbolKind::Class);
    }

    #[test]
    fn class_circle_base_class() {
        let items = fixture_items();
        let circle = find_by_name(&items, "Circle");
        assert!(
            circle.metadata.base_classes.contains(&"Shape".to_string()),
            "Circle should inherit from Shape, got {:?}",
            circle.metadata.base_classes
        );
    }

    #[test]
    fn class_circle_has_methods() {
        let items = fixture_items();
        let circle = find_by_name(&items, "Circle");
        assert!(
            circle.metadata.methods.contains(&"area".to_string()),
            "Circle should have area method"
        );
        assert!(
            circle.metadata.methods.contains(&"radius".to_string()),
            "Circle should have radius method"
        );
    }

    #[test]
    fn class_circle_has_private_field() {
        let items = fixture_items();
        let circle = find_by_name(&items, "Circle");
        assert!(
            circle.metadata.fields.contains(&"radius_".to_string()),
            "Circle should have radius_ field, got {:?}",
            circle.metadata.fields
        );
    }

    #[test]
    fn class_rectangle_base_class() {
        let items = fixture_items();
        let rect = find_by_name(&items, "Rectangle");
        assert_eq!(rect.kind, SymbolKind::Class);
        assert!(
            rect.metadata.base_classes.contains(&"Shape".to_string()),
            "Rectangle should inherit from Shape"
        );
    }

    #[test]
    fn class_rectangle_has_width_height() {
        let items = fixture_items();
        let rect = find_by_name(&items, "Rectangle");
        assert!(
            rect.metadata.methods.contains(&"width".to_string())
                && rect.metadata.methods.contains(&"height".to_string()),
            "Rectangle should have width/height methods"
        );
    }

    #[test]
    fn class_square_is_final() {
        let items = fixture_items();
        let sq = find_by_name(&items, "Square");
        assert_eq!(sq.kind, SymbolKind::Class);
        assert!(
            sq.metadata.attributes.contains(&"final".to_string()),
            "Square should be final"
        );
    }

    #[test]
    fn class_square_inherits_rectangle() {
        let items = fixture_items();
        let sq = find_by_name(&items, "Square");
        assert!(
            sq.metadata.base_classes.contains(&"Rectangle".to_string()),
            "Square should inherit from Rectangle"
        );
    }

    #[test]
    fn class_document_multiple_inheritance() {
        let items = fixture_items();
        let doc = find_by_name(&items, "Document");
        assert_eq!(doc.kind, SymbolKind::Class);
        assert!(
            doc.metadata.base_classes.len() >= 2,
            "Document should have 2+ base classes, got {:?}",
            doc.metadata.base_classes
        );
    }

    #[test]
    fn class_serializable_abstract() {
        let items = fixture_items();
        let s = find_by_name(&items, "Serializable");
        assert_eq!(s.kind, SymbolKind::Class);
        assert!(
            s.metadata.attributes.contains(&"abstract".to_string()),
            "Serializable should be abstract"
        );
    }

    #[test]
    fn class_printable_abstract() {
        let items = fixture_items();
        let p = find_by_name(&items, "Printable");
        assert_eq!(p.kind, SymbolKind::Class);
        assert!(
            p.metadata.attributes.contains(&"abstract".to_string()),
            "Printable should be abstract"
        );
    }

    #[test]
    fn class_outer_exists() {
        let items = fixture_items();
        let outer = find_by_name(&items, "Outer");
        assert_eq!(outer.kind, SymbolKind::Class);
    }

    #[test]
    fn class_int_wrapper_exists() {
        let items = fixture_items();
        let iw = find_by_name(&items, "IntWrapper");
        assert_eq!(iw.kind, SymbolKind::Class);
    }

    #[test]
    fn class_int_wrapper_has_conversion_operators() {
        let items = fixture_items();
        let iw = find_by_name(&items, "IntWrapper");
        assert!(
            iw.metadata.methods.iter().any(|m| m.contains("operator")),
            "IntWrapper should have conversion operators, got {:?}",
            iw.metadata.methods
        );
    }

    #[test]
    fn class_explicit_only_exists() {
        let items = fixture_items();
        let e = find_by_name(&items, "ExplicitOnly");
        assert_eq!(e.kind, SymbolKind::Class);
    }

    #[test]
    fn class_container_template() {
        let items = fixture_items();
        let c = find_by_name(&items, "Container");
        assert_eq!(c.kind, SymbolKind::Class);
        assert!(
            c.metadata.attributes.contains(&"template".to_string()),
            "Container should have template attribute"
        );
    }

    #[test]
    fn class_container_has_generics() {
        let items = fixture_items();
        let c = find_by_name(&items, "Container");
        assert!(
            c.metadata.generics.is_some(),
            "Container should have generics"
        );
    }

    #[test]
    fn class_container_void_specialization() {
        let items = fixture_items();
        let spec = items
            .iter()
            .find(|i| i.name.contains("Container") && i.name.contains("void"));
        assert!(
            spec.is_some(),
            "Container<void> specialization should exist"
        );
    }

    #[test]
    fn class_container_methods() {
        let items = fixture_items();
        let c = find_by_name(&items, "Container");
        assert!(
            c.metadata.methods.contains(&"get".to_string()),
            "Container should have get method"
        );
        assert!(
            c.metadata.methods.contains(&"set".to_string()),
            "Container should have set method"
        );
    }

    // ════════════════════════════════════════════════════════════════
    // 4. Template tests
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn template_generic_add_exists() {
        let items = fixture_items();
        let f = find_by_name(&items, "generic_add");
        assert_eq!(f.kind, SymbolKind::Function);
        assert!(
            f.metadata.attributes.contains(&"template".to_string()),
            "generic_add should be a template"
        );
    }

    #[test]
    fn template_generic_add_has_generics() {
        let items = fixture_items();
        let f = find_by_name(&items, "generic_add");
        assert!(
            f.metadata.generics.is_some(),
            "generic_add should have generics"
        );
    }

    #[test]
    fn template_print_all_variadic() {
        let items = fixture_items();
        let f = find_by_name(&items, "print_all");
        assert_eq!(f.kind, SymbolKind::Function);
        assert!(
            f.metadata.attributes.contains(&"template".to_string()),
            "print_all should be a template"
        );
        let generics = f.metadata.generics.as_deref().unwrap_or("");
        assert!(
            generics.contains("..."),
            "print_all should have variadic template params, got {generics}"
        );
    }

    #[test]
    fn template_pair_struct() {
        let items = fixture_items();
        let p = find_by_name(&items, "Pair");
        assert_eq!(p.kind, SymbolKind::Struct);
        assert!(
            p.metadata.attributes.contains(&"template".to_string()),
            "Pair should be a template"
        );
    }

    #[test]
    fn template_pair_has_generics() {
        let items = fixture_items();
        let p = find_by_name(&items, "Pair");
        assert!(p.metadata.generics.is_some(), "Pair should have generics");
    }

    #[test]
    fn template_list_node_struct() {
        let items = fixture_items();
        let ln = find_by_name(&items, "ListNode");
        assert_eq!(ln.kind, SymbolKind::Struct);
        assert!(
            ln.metadata.attributes.contains(&"template".to_string()),
            "ListNode should be a template"
        );
    }

    #[test]
    fn template_list_node_has_fields() {
        let items = fixture_items();
        let ln = find_by_name(&items, "ListNode");
        assert!(
            ln.metadata.fields.contains(&"data".to_string()),
            "ListNode should have data field, got {:?}",
            ln.metadata.fields
        );
    }

    #[test]
    fn template_constrained_add() {
        let items = fixture_items();
        let f = find_by_name(&items, "constrained_add");
        assert_eq!(f.kind, SymbolKind::Function);
        assert!(
            f.metadata.attributes.contains(&"template".to_string()),
            "constrained_add should be a template"
        );
    }

    #[test]
    fn template_constrained_add_has_generics() {
        let items = fixture_items();
        let f = find_by_name(&items, "constrained_add");
        assert!(
            f.metadata.generics.is_some(),
            "constrained_add should have generics"
        );
    }

    #[test]
    fn template_generic_add_parameters() {
        let items = fixture_items();
        let f = find_by_name(&items, "generic_add");
        assert!(
            f.metadata.parameters.len() >= 2,
            "generic_add should have 2+ parameters, got {:?}",
            f.metadata.parameters
        );
    }

    // ════════════════════════════════════════════════════════════════
    // 5. Namespace tests
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn namespace_math_exists() {
        let items = fixture_items();
        let m = find_by_name(&items, "math");
        assert_eq!(m.kind, SymbolKind::Module);
        assert!(
            m.metadata.attributes.contains(&"namespace".to_string()),
            "math should have namespace attribute"
        );
    }

    #[test]
    fn namespace_math_is_public() {
        let items = fixture_items();
        let m = find_by_name(&items, "math");
        assert_eq!(m.visibility, Visibility::Public);
    }

    #[test]
    fn namespace_nested_utils_string() {
        let items = fixture_items();
        let ns = items.iter().find(|i| {
            i.kind == SymbolKind::Module
                && i.metadata.attributes.contains(&"namespace".to_string())
                && (i.name.contains("utils") || i.name.contains("string"))
        });
        assert!(ns.is_some(), "nested namespace utils::string should exist");
    }

    #[test]
    fn namespace_anonymous_exists() {
        let items = fixture_items();
        let anon = find_by_name(&items, "(anonymous)");
        assert_eq!(anon.kind, SymbolKind::Module);
        assert_eq!(anon.visibility, Visibility::Private);
    }

    #[test]
    fn namespace_math_contains_abs() {
        let items = fixture_items();
        let abs_items: Vec<_> = items
            .iter()
            .filter(|i| i.name == "abs" && i.kind == SymbolKind::Function)
            .collect();
        assert!(
            !abs_items.is_empty(),
            "abs function in math namespace should exist"
        );
    }

    #[test]
    fn namespace_math_contains_square() {
        let items = fixture_items();
        let sq: Vec<_> = items
            .iter()
            .filter(|i| i.name == "square" && i.kind == SymbolKind::Function)
            .collect();
        assert!(
            !sq.is_empty(),
            "square function in math namespace should exist"
        );
    }

    #[test]
    fn namespace_math_contains_point_struct() {
        let items = fixture_items();
        let pts: Vec<_> = items
            .iter()
            .filter(|i| i.name == "Point" && i.kind == SymbolKind::Struct)
            .collect();
        assert!(
            !pts.is_empty(),
            "Point struct in math namespace should exist"
        );
    }

    #[test]
    fn namespace_utils_string_contains_trim() {
        let items = fixture_items();
        let trims: Vec<_> = items
            .iter()
            .filter(|i| i.name == "trim" && i.kind == SymbolKind::Function)
            .collect();
        assert!(
            !trims.is_empty(),
            "trim function in utils::string namespace should exist"
        );
    }

    // ════════════════════════════════════════════════════════════════
    // 6. Enum tests
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn enum_color_exists() {
        let items = fixture_items();
        let color = find_by_name(&items, "Color");
        assert_eq!(color.kind, SymbolKind::Enum);
    }

    #[test]
    fn enum_color_is_scoped() {
        let items = fixture_items();
        let color = find_by_name(&items, "Color");
        assert!(
            color
                .metadata
                .attributes
                .contains(&"scoped_enum".to_string()),
            "Color should be a scoped enum, got {:?}",
            color.metadata.attributes
        );
    }

    #[test]
    fn enum_color_has_variants() {
        let items = fixture_items();
        let color = find_by_name(&items, "Color");
        assert!(
            color.metadata.variants.contains(&"Red".to_string()),
            "Color should have Red variant"
        );
        assert!(
            color.metadata.variants.contains(&"Green".to_string()),
            "Color should have Green variant"
        );
        assert!(
            color.metadata.variants.contains(&"Blue".to_string()),
            "Color should have Blue variant"
        );
    }

    #[test]
    fn enum_status_code_scoped() {
        let items = fixture_items();
        let sc = find_by_name(&items, "StatusCode");
        assert_eq!(sc.kind, SymbolKind::Enum);
        assert!(
            sc.metadata.attributes.contains(&"scoped_enum".to_string()),
            "StatusCode should be a scoped enum"
        );
    }

    #[test]
    fn enum_status_code_has_variants() {
        let items = fixture_items();
        let sc = find_by_name(&items, "StatusCode");
        assert!(
            sc.metadata.variants.contains(&"OK".to_string()),
            "StatusCode should have OK variant"
        );
        assert!(
            sc.metadata.variants.contains(&"NotFound".to_string()),
            "StatusCode should have NotFound variant"
        );
        assert!(
            sc.metadata.variants.contains(&"InternalError".to_string()),
            "StatusCode should have InternalError variant"
        );
    }

    #[test]
    fn enum_log_level_unscoped() {
        let items = fixture_items();
        let ll = find_by_name(&items, "LogLevel");
        assert_eq!(ll.kind, SymbolKind::Enum);
        assert!(
            !ll.metadata.attributes.contains(&"scoped_enum".to_string()),
            "LogLevel should NOT be scoped"
        );
    }

    #[test]
    fn enum_log_level_has_variants() {
        let items = fixture_items();
        let ll = find_by_name(&items, "LogLevel");
        assert!(
            ll.metadata.variants.len() >= 4,
            "LogLevel should have 4+ variants, got {}",
            ll.metadata.variants.len()
        );
    }

    #[test]
    fn enum_color_has_doc_comment() {
        let items = fixture_items();
        let color = find_by_name(&items, "Color");
        assert!(
            !color.doc_comment.is_empty(),
            "Color should have a doc comment"
        );
    }

    // ════════════════════════════════════════════════════════════════
    // 7. Concept tests
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn concept_stream_insertable() {
        let items = fixture_items();
        let si = find_by_name(&items, "StreamInsertable");
        assert_eq!(si.kind, SymbolKind::Trait);
        assert!(
            si.metadata.attributes.contains(&"concept".to_string()),
            "StreamInsertable should have concept attribute"
        );
    }

    #[test]
    fn concept_stream_insertable_has_generics() {
        let items = fixture_items();
        let si = find_by_name(&items, "StreamInsertable");
        assert!(
            si.metadata.generics.is_some(),
            "StreamInsertable should have generics"
        );
    }

    #[test]
    fn concept_addable() {
        let items = fixture_items();
        let a = find_by_name(&items, "Addable");
        assert_eq!(a.kind, SymbolKind::Trait);
        assert!(
            a.metadata.attributes.contains(&"concept".to_string()),
            "Addable should have concept attribute"
        );
    }

    #[test]
    fn concept_addable_has_doc() {
        let items = fixture_items();
        let a = find_by_name(&items, "Addable");
        assert!(
            !a.doc_comment.is_empty(),
            "Addable should have a doc comment"
        );
    }

    // ════════════════════════════════════════════════════════════════
    // 8. Constexpr / consteval / constinit tests
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn constexpr_max_elements() {
        let items = fixture_items();
        let me = find_by_name(&items, "MAX_ELEMENTS");
        assert_eq!(me.kind, SymbolKind::Const, "MAX_ELEMENTS should be Const");
        assert!(
            me.metadata.attributes.contains(&"constexpr".to_string()),
            "MAX_ELEMENTS should have constexpr attribute"
        );
    }

    #[test]
    fn constexpr_pi() {
        let items = fixture_items();
        let pi = find_by_name(&items, "PI");
        assert_eq!(pi.kind, SymbolKind::Const, "PI should be Const");
    }

    #[test]
    fn constexpr_buffer_size() {
        let items = fixture_items();
        let bs = find_by_name(&items, "BUFFER_SIZE");
        assert_eq!(bs.kind, SymbolKind::Const, "BUFFER_SIZE should be Const");
    }

    #[test]
    fn constinit_global() {
        let items = fixture_items();
        let g = find_by_name(&items, "global_init_val");
        assert_eq!(g.kind, SymbolKind::Const, "global_init_val should be Const");
        assert!(
            g.metadata.attributes.contains(&"constinit".to_string()),
            "global_init_val should have constinit attribute"
        );
    }

    #[test]
    fn constexpr_factorial_function() {
        let items = fixture_items();
        let f = find_by_name(&items, "factorial");
        assert_eq!(f.kind, SymbolKind::Function);
        assert!(
            f.metadata.attributes.contains(&"constexpr".to_string()),
            "factorial should have constexpr attribute"
        );
    }

    #[test]
    fn constexpr_compile_time_square() {
        let items = fixture_items();
        let f = find_by_name(&items, "compile_time_square");
        assert_eq!(f.kind, SymbolKind::Function);
        assert!(
            f.metadata.attributes.contains(&"constexpr".to_string()),
            "compile_time_square should have constexpr attribute"
        );
    }

    #[test]
    fn consteval_compile_only_double() {
        let items = fixture_items();
        let f = find_by_name(&items, "compile_only_double");
        assert_eq!(f.kind, SymbolKind::Function);
        assert!(
            f.metadata.attributes.contains(&"consteval".to_string()),
            "compile_only_double should have consteval attribute"
        );
    }

    #[test]
    fn noexcept_safe_divide() {
        let items = fixture_items();
        let f = find_by_name(&items, "safe_divide");
        assert_eq!(f.kind, SymbolKind::Function);
        assert!(
            f.metadata.attributes.contains(&"noexcept".to_string()),
            "safe_divide should have noexcept attribute"
        );
    }

    // ════════════════════════════════════════════════════════════════
    // 9. Using / typedef tests
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn using_alias_string_vec() {
        let items = fixture_items();
        let sv = find_by_name(&items, "StringVec");
        assert_eq!(sv.kind, SymbolKind::TypeAlias);
        assert!(
            sv.metadata.attributes.contains(&"using".to_string()),
            "StringVec should have 'using' attribute"
        );
    }

    #[test]
    fn using_alias_callback() {
        let items = fixture_items();
        let cb = find_by_name(&items, "Callback");
        assert_eq!(cb.kind, SymbolKind::TypeAlias);
    }

    #[test]
    fn using_alias_size() {
        let items = fixture_items();
        let s = find_by_name(&items, "Size");
        assert_eq!(s.kind, SymbolKind::TypeAlias);
    }

    #[test]
    fn using_alias_compare_func() {
        let items = fixture_items();
        let cf = find_by_name(&items, "CompareFunc");
        assert_eq!(cf.kind, SymbolKind::TypeAlias);
    }

    #[test]
    fn typedef_old_callback() {
        let items = fixture_items();
        let oc = find_by_name(&items, "OldCallback");
        assert_eq!(oc.kind, SymbolKind::TypeAlias);
        assert!(
            oc.metadata.attributes.contains(&"typedef".to_string()),
            "OldCallback should have typedef attribute"
        );
    }

    #[test]
    fn using_declaration_cout() {
        let items = fixture_items();
        let cout = items
            .iter()
            .find(|i| i.name.contains("cout") && i.kind == SymbolKind::Module);
        assert!(cout.is_some(), "using std::cout should be extracted");
    }

    #[test]
    fn using_declaration_endl() {
        let items = fixture_items();
        let endl = items
            .iter()
            .find(|i| i.name.contains("endl") && i.kind == SymbolKind::Module);
        assert!(endl.is_some(), "using std::endl should be extracted");
    }

    #[test]
    fn using_declaration_has_attribute() {
        let items = fixture_items();
        let cout = items
            .iter()
            .find(|i| i.name.contains("cout") && i.kind == SymbolKind::Module)
            .expect("cout should exist");
        assert!(
            cout.metadata
                .attributes
                .contains(&"using_declaration".to_string()),
            "using std::cout should have using_declaration attribute"
        );
    }

    // ════════════════════════════════════════════════════════════════
    // 10. Static assert tests
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn static_assert_extracted() {
        let items = fixture_items();
        let asserts: Vec<_> = items
            .iter()
            .filter(|i| i.name == "static_assert" && i.kind == SymbolKind::Macro)
            .collect();
        assert!(
            asserts.len() >= 3,
            "expected 3+ static_asserts, got {}",
            asserts.len()
        );
    }

    #[test]
    fn static_assert_has_attribute() {
        let items = fixture_items();
        let sa = items
            .iter()
            .find(|i| i.name == "static_assert" && i.kind == SymbolKind::Macro)
            .expect("static_assert should exist");
        assert!(
            sa.metadata
                .attributes
                .contains(&"static_assert".to_string()),
            "static_assert should have static_assert attribute"
        );
    }

    #[test]
    fn static_assert_has_signature() {
        let items = fixture_items();
        let sa = items
            .iter()
            .find(|i| i.name == "static_assert" && i.kind == SymbolKind::Macro)
            .expect("static_assert should exist");
        assert!(
            !sa.signature.is_empty(),
            "static_assert should have a signature"
        );
    }

    // ════════════════════════════════════════════════════════════════
    // 11. Extern "C" tests
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn extern_c_block_extracted() {
        let items = fixture_items();
        let ext = items.iter().find(|i| {
            i.name.contains("extern")
                && i.metadata
                    .attributes
                    .contains(&"linkage_specification".to_string())
        });
        assert!(ext.is_some(), "extern \"C\" block should be extracted");
    }

    #[test]
    fn extern_c_block_has_extern_c_attr() {
        let items = fixture_items();
        let ext = items
            .iter()
            .find(|i| {
                i.metadata
                    .attributes
                    .contains(&"linkage_specification".to_string())
            })
            .expect("extern C block should exist");
        assert!(
            ext.metadata.attributes.contains(&"extern_c".to_string()),
            "extern C should have extern_c attribute"
        );
    }

    #[test]
    fn extern_c_c_init_prototype() {
        let items = fixture_items();
        let ci = items
            .iter()
            .find(|i| i.name == "c_init" && i.kind == SymbolKind::Function);
        assert!(ci.is_some(), "c_init prototype should be extracted");
    }

    #[test]
    fn extern_c_c_process_prototype() {
        let items = fixture_items();
        let cp = items
            .iter()
            .find(|i| i.name == "c_process" && i.kind == SymbolKind::Function);
        assert!(cp.is_some(), "c_process prototype should be extracted");
    }

    // ════════════════════════════════════════════════════════════════
    // 12. Function tests
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn function_fast_max_inline() {
        let items = fixture_items();
        let f = find_by_name(&items, "fast_max");
        assert_eq!(f.kind, SymbolKind::Function);
        assert!(
            f.metadata.attributes.contains(&"inline".to_string()),
            "fast_max should have inline attribute"
        );
    }

    #[test]
    fn function_internal_helper_static() {
        let items = fixture_items();
        let f = find_by_name(&items, "internal_helper");
        assert_eq!(f.kind, SymbolKind::Function);
        assert_eq!(
            f.visibility,
            Visibility::Private,
            "static function should be private"
        );
    }

    #[test]
    fn function_external_function_extern() {
        let items = fixture_items();
        let ef = find_by_name(&items, "external_function");
        assert_eq!(ef.kind, SymbolKind::Function);
        assert!(
            ef.metadata.attributes.contains(&"extern".to_string())
                || ef.metadata.attributes.contains(&"prototype".to_string()),
            "external_function should be extern or prototype"
        );
    }

    #[test]
    fn function_process_data_params() {
        let items = fixture_items();
        let pd = find_by_name(&items, "process_data");
        assert_eq!(pd.kind, SymbolKind::Function);
        assert!(
            pd.metadata.parameters.len() >= 3,
            "process_data should have 3+ params, got {:?}",
            pd.metadata.parameters
        );
    }

    #[test]
    fn function_trailing_return_auto() {
        let items = fixture_items();
        let tr = find_by_name(&items, "trailing_return");
        assert_eq!(tr.kind, SymbolKind::Function);
        assert!(
            tr.metadata.attributes.contains(&"auto".to_string()),
            "trailing_return should have auto attribute, got {:?}",
            tr.metadata.attributes
        );
    }

    #[test]
    fn function_make_adder_exists() {
        let items = fixture_items();
        let ma = find_by_name(&items, "make_adder");
        assert_eq!(ma.kind, SymbolKind::Function);
    }

    #[test]
    fn function_reveal_secret_exists() {
        let items = fixture_items();
        let rs = find_by_name(&items, "reveal_secret");
        assert_eq!(rs.kind, SymbolKind::Function);
    }

    #[test]
    fn function_safe_divide_return_type() {
        let items = fixture_items();
        let sd = find_by_name(&items, "safe_divide");
        assert!(
            sd.metadata
                .return_type
                .as_deref()
                .unwrap_or("")
                .contains("int"),
            "safe_divide should return int, got {:?}",
            sd.metadata.return_type
        );
    }

    #[test]
    fn function_increment_counter_in_anonymous_ns() {
        let items = fixture_items();
        let ic: Vec<_> = items
            .iter()
            .filter(|i| i.name == "increment_counter" && i.kind == SymbolKind::Function)
            .collect();
        assert!(
            !ic.is_empty(),
            "increment_counter in anonymous namespace should exist"
        );
    }

    #[test]
    fn function_factorial_has_doc() {
        let items = fixture_items();
        let f = find_by_name(&items, "factorial");
        assert!(
            !f.doc_comment.is_empty(),
            "factorial should have a doc comment"
        );
    }

    // ════════════════════════════════════════════════════════════════
    // 13. Lambda / variable tests
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn variable_doubler_exists() {
        let items = fixture_items();
        let d = find_by_name(&items, "doubler");
        assert!(
            d.kind == SymbolKind::Static || d.kind == SymbolKind::Const,
            "doubler should be Static or Const, got {:?}",
            d.kind
        );
    }

    #[test]
    fn variable_doubler_auto_attr() {
        let items = fixture_items();
        let d = find_by_name(&items, "doubler");
        assert!(
            d.metadata.attributes.contains(&"auto".to_string()),
            "doubler should have auto attribute, got {:?}",
            d.metadata.attributes
        );
    }

    #[test]
    fn variable_g_counter_exists() {
        let items = fixture_items();
        let g = find_by_name(&items, "g_counter");
        assert_eq!(g.kind, SymbolKind::Static, "g_counter should be Static");
    }

    #[test]
    fn variable_s_instance_count_static() {
        let items = fixture_items();
        let s = find_by_name(&items, "s_instance_count");
        assert_eq!(
            s.kind,
            SymbolKind::Static,
            "s_instance_count should be Static"
        );
    }

    #[test]
    fn variable_app_name_const() {
        let items = fixture_items();
        let a = find_by_name(&items, "APP_NAME");
        assert_eq!(a.kind, SymbolKind::Const, "APP_NAME should be Const");
    }

    // ════════════════════════════════════════════════════════════════
    // 14. Forward declaration tests
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn forward_decl_config_struct() {
        let items = fixture_items();
        let c = find_by_name(&items, "Config");
        assert_eq!(c.kind, SymbolKind::Struct, "Config should be Struct");
    }

    #[test]
    fn forward_decl_widget_class() {
        let items = fixture_items();
        let w = find_by_name(&items, "Widget");
        assert_eq!(w.kind, SymbolKind::Class, "Widget should be Class");
    }

    #[test]
    fn forward_decl_config_no_fields() {
        let items = fixture_items();
        let c = find_by_name(&items, "Config");
        assert!(
            c.metadata.fields.is_empty(),
            "forward decl Config should have no fields"
        );
    }

    // ════════════════════════════════════════════════════════════════
    // 15. Struct tests
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn struct_counter_exists() {
        let items = fixture_items();
        let c = find_by_name(&items, "Counter");
        assert_eq!(c.kind, SymbolKind::Struct, "Counter should be Struct");
    }

    #[test]
    fn struct_counter_has_value_field() {
        let items = fixture_items();
        let c = find_by_name(&items, "Counter");
        assert!(
            c.metadata.fields.contains(&"value".to_string()),
            "Counter should have value field, got {:?}",
            c.metadata.fields
        );
    }

    #[test]
    fn struct_counter_has_doc_comment() {
        let items = fixture_items();
        let c = find_by_name(&items, "Counter");
        assert!(
            !c.doc_comment.is_empty(),
            "Counter should have a doc comment"
        );
    }

    #[test]
    fn struct_point_in_namespace() {
        let items = fixture_items();
        let pts: Vec<_> = items
            .iter()
            .filter(|i| i.name == "Point" && i.kind == SymbolKind::Struct)
            .collect();
        assert!(!pts.is_empty(), "Point struct should exist");
        assert!(
            pts[0].metadata.fields.contains(&"x".to_string()),
            "Point should have x field"
        );
        assert!(
            pts[0].metadata.fields.contains(&"y".to_string()),
            "Point should have y field"
        );
    }

    #[test]
    fn struct_pair_has_fields() {
        let items = fixture_items();
        let p = find_by_name(&items, "Pair");
        assert!(
            p.metadata.fields.contains(&"first".to_string()),
            "Pair should have first field"
        );
        assert!(
            p.metadata.fields.contains(&"second".to_string()),
            "Pair should have second field"
        );
    }

    // ════════════════════════════════════════════════════════════════
    // 16. Doc comment tests
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn doc_comment_shape() {
        let items = fixture_items();
        let s = find_by_name(&items, "Shape");
        assert!(
            s.doc_comment.contains("Abstract") || s.doc_comment.contains("shape"),
            "Shape should have doc comment about abstract/shape, got {:?}",
            s.doc_comment
        );
    }

    #[test]
    fn doc_comment_circle() {
        let items = fixture_items();
        let c = find_by_name(&items, "Circle");
        assert!(
            !c.doc_comment.is_empty(),
            "Circle should have a doc comment"
        );
    }

    #[test]
    fn doc_comment_container() {
        let items = fixture_items();
        let c = find_by_name(&items, "Container");
        assert!(
            !c.doc_comment.is_empty(),
            "Container should have a doc comment"
        );
    }

    #[test]
    fn doc_comment_math_namespace() {
        let items = fixture_items();
        let m = find_by_name(&items, "math");
        assert!(
            !m.doc_comment.is_empty(),
            "math namespace should have a doc comment"
        );
    }

    #[test]
    fn doc_comment_safe_divide() {
        let items = fixture_items();
        let sd = find_by_name(&items, "safe_divide");
        assert!(
            !sd.doc_comment.is_empty(),
            "safe_divide should have a doc comment"
        );
    }

    // ════════════════════════════════════════════════════════════════
    // 17. Line number tests
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn all_start_lines_valid() {
        let items = fixture_items();
        for item in &items {
            assert!(
                item.start_line >= 1,
                "item {} should have start_line >= 1, got {}",
                item.name,
                item.start_line
            );
        }
    }

    #[test]
    fn all_end_lines_gte_start() {
        let items = fixture_items();
        for item in &items {
            assert!(
                item.end_line >= item.start_line,
                "item {} end_line ({}) < start_line ({})",
                item.name,
                item.end_line,
                item.start_line
            );
        }
    }

    // ════════════════════════════════════════════════════════════════
    // 18. Inheritance tests
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn inheritance_single_circle_from_shape() {
        let items = fixture_items();
        let c = find_by_name(&items, "Circle");
        assert_eq!(c.metadata.base_classes, vec!["Shape"]);
    }

    #[test]
    fn inheritance_single_rectangle_from_shape() {
        let items = fixture_items();
        let r = find_by_name(&items, "Rectangle");
        assert_eq!(r.metadata.base_classes, vec!["Shape"]);
    }

    #[test]
    fn inheritance_multiple_document() {
        let items = fixture_items();
        let d = find_by_name(&items, "Document");
        assert!(
            d.metadata
                .base_classes
                .contains(&"Serializable".to_string())
                && d.metadata.base_classes.contains(&"Printable".to_string()),
            "Document should have Serializable and Printable bases, got {:?}",
            d.metadata.base_classes
        );
    }

    #[test]
    fn inheritance_virtual_vleft() {
        let items = fixture_items();
        let vl = find_by_name(&items, "VLeft");
        assert_eq!(vl.kind, SymbolKind::Class);
        assert!(
            vl.metadata.base_classes.contains(&"VBase".to_string()),
            "VLeft should inherit from VBase"
        );
    }

    #[test]
    fn inheritance_virtual_vright() {
        let items = fixture_items();
        let vr = find_by_name(&items, "VRight");
        assert_eq!(vr.kind, SymbolKind::Class);
        assert!(
            vr.metadata.base_classes.contains(&"VBase".to_string()),
            "VRight should inherit from VBase"
        );
    }

    #[test]
    fn inheritance_diamond() {
        let items = fixture_items();
        let d = find_by_name(&items, "Diamond");
        assert_eq!(d.kind, SymbolKind::Class);
        assert!(
            d.metadata.base_classes.len() >= 2,
            "Diamond should have 2+ base classes, got {:?}",
            d.metadata.base_classes
        );
    }

    // ════════════════════════════════════════════════════════════════
    // 19. Preprocessor define tests
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn define_max_size() {
        let items = fixture_items();
        let ms = find_by_name(&items, "MAX_SIZE");
        assert_eq!(ms.kind, SymbolKind::Const, "MAX_SIZE should be Const");
    }

    #[test]
    fn define_app_version() {
        let items = fixture_items();
        let av = find_by_name(&items, "APP_VERSION");
        assert_eq!(av.kind, SymbolKind::Const, "APP_VERSION should be Const");
    }

    // ════════════════════════════════════════════════════════════════
    // 20. VBase tests
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn class_vbase_exists() {
        let items = fixture_items();
        let vb = find_by_name(&items, "VBase");
        assert_eq!(vb.kind, SymbolKind::Class);
    }

    #[test]
    fn class_vbase_has_destructor() {
        let items = fixture_items();
        let vb = find_by_name(&items, "VBase");
        assert!(
            vb.metadata.methods.iter().any(|m| m.contains('~')),
            "VBase should have destructor, got {:?}",
            vb.metadata.methods
        );
    }

    #[test]
    fn class_vbase_has_base_val_field() {
        let items = fixture_items();
        let vb = find_by_name(&items, "VBase");
        assert!(
            vb.metadata.fields.contains(&"base_val".to_string()),
            "VBase should have base_val field, got {:?}",
            vb.metadata.fields
        );
    }

    // ════════════════════════════════════════════════════════════════
    // 21. Misc edge cases
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn operator_km_literal_extracted() {
        let items = fixture_items();
        let ops: Vec<_> = items
            .iter()
            .filter(|i| i.kind == SymbolKind::Function && i.name.contains("operator"))
            .collect();
        assert!(
            !ops.is_empty(),
            "user-defined literal operator should be extracted as a Function"
        );
    }

    #[test]
    fn operator_km_literal_has_operator_attr() {
        let items = fixture_items();
        let op = items
            .iter()
            .find(|i| i.kind == SymbolKind::Function && i.name.contains("_km"))
            .expect("operator\"\"_km should exist");
        assert!(
            op.metadata.attributes.contains(&"operator".to_string()),
            "operator\"\"_km should have 'operator' attribute, got {:?}",
            op.metadata.attributes
        );
    }

    #[test]
    fn empty_source_returns_empty() {
        let items = parse_and_extract("");
        assert!(items.is_empty(), "empty source should produce no items");
    }

    #[test]
    fn minimal_class() {
        let items = parse_and_extract("class Foo {};");
        let f = find_by_name(&items, "Foo");
        assert_eq!(f.kind, SymbolKind::Class);
    }

    #[test]
    fn minimal_namespace() {
        let items = parse_and_extract("namespace bar {}");
        let b = find_by_name(&items, "bar");
        assert_eq!(b.kind, SymbolKind::Module);
        assert!(b.metadata.attributes.contains(&"namespace".to_string()));
    }

    #[test]
    fn minimal_template_class() {
        let items = parse_and_extract("template<typename T> class Box {};");
        let b = find_by_name(&items, "Box");
        assert_eq!(b.kind, SymbolKind::Class);
        assert!(b.metadata.attributes.contains(&"template".to_string()));
        assert!(b.metadata.generics.is_some());
    }

    #[test]
    fn minimal_concept() {
        let items = parse_and_extract(
            "template<typename T> concept Hashable = requires(T a) { a.hash(); };",
        );
        let h = find_by_name(&items, "Hashable");
        assert_eq!(h.kind, SymbolKind::Trait);
        assert!(h.metadata.attributes.contains(&"concept".to_string()));
    }

    #[test]
    fn minimal_using_alias() {
        let items = parse_and_extract("using MyInt = int;");
        let mi = find_by_name(&items, "MyInt");
        assert_eq!(mi.kind, SymbolKind::TypeAlias);
        assert!(mi.metadata.attributes.contains(&"using".to_string()));
    }

    #[test]
    fn minimal_static_assert() {
        let items = parse_and_extract("static_assert(true, \"ok\");");
        let sa = find_by_name(&items, "static_assert");
        assert_eq!(sa.kind, SymbolKind::Macro);
    }

    #[test]
    fn minimal_extern_c() {
        let items = parse_and_extract("extern \"C\" { void foo(); }");
        let ext: Vec<_> = items
            .iter()
            .filter(|i| {
                i.metadata
                    .attributes
                    .contains(&"linkage_specification".to_string())
            })
            .collect();
        assert!(!ext.is_empty(), "extern C block should be extracted");
    }

    #[test]
    fn minimal_enum_class() {
        let items = parse_and_extract("enum class Direction { Up, Down, Left, Right };");
        let d = find_by_name(&items, "Direction");
        assert_eq!(d.kind, SymbolKind::Enum);
        assert!(
            d.metadata.attributes.contains(&"scoped_enum".to_string()),
            "enum class should be scoped"
        );
        assert!(d.metadata.variants.contains(&"Up".to_string()));
    }

    #[test]
    fn minimal_final_class() {
        let items = parse_and_extract("class Sealed final {};");
        let s = find_by_name(&items, "Sealed");
        assert_eq!(s.kind, SymbolKind::Class);
        assert!(s.metadata.attributes.contains(&"final".to_string()));
    }

    #[test]
    fn minimal_class_inheritance() {
        let items = parse_and_extract("class A {}; class B : public A {};");
        let b = find_by_name(&items, "B");
        assert!(
            b.metadata.base_classes.contains(&"A".to_string()),
            "B should inherit from A"
        );
    }

    #[test]
    fn shared_value_extern() {
        let items = fixture_items();
        let sv = find_by_name(&items, "shared_value");
        assert!(
            sv.kind == SymbolKind::Static || sv.kind == SymbolKind::Const,
            "shared_value should be Static or Const"
        );
    }

    #[test]
    fn class_document_has_methods() {
        let items = fixture_items();
        let d = find_by_name(&items, "Document");
        assert!(
            d.metadata.methods.contains(&"serialize".to_string()),
            "Document should have serialize method"
        );
    }

    #[test]
    fn fixture_no_duplicate_classes() {
        let items = fixture_items();
        let classes = find_all_by_kind(&items, SymbolKind::Class);
        let mut names: Vec<_> = classes.iter().map(|c| &c.name).collect();
        let total = names.len();
        names.sort();
        names.dedup();
        // Allow some duplicates from specializations but not excessive
        assert!(
            names.len() >= total / 2,
            "too many duplicate class names: {total} total, {} unique",
            names.len()
        );
    }

    #[test]
    fn class_shape_class_attribute() {
        let items = fixture_items();
        let shape = find_by_name(&items, "Shape");
        assert!(
            shape.metadata.attributes.contains(&"class".to_string()),
            "Shape should have 'class' attribute"
        );
    }

    #[test]
    fn class_square_not_abstract() {
        let items = fixture_items();
        let sq = find_by_name(&items, "Square");
        assert!(
            !sq.metadata.attributes.contains(&"abstract".to_string()),
            "Square should NOT be abstract"
        );
    }

    #[test]
    fn minimal_abstract_class() {
        let items = parse_and_extract(
            "class IFoo {\npublic:\n    virtual void bar() = 0;\n    virtual ~IFoo() = default;\n};",
        );
        let f = find_by_name(&items, "IFoo");
        assert!(f.metadata.attributes.contains(&"abstract".to_string()));
    }

    #[test]
    fn minimal_multiple_inheritance() {
        let items = parse_and_extract("class X {}; class Y {}; class Z : public X, public Y {};");
        let z = find_by_name(&items, "Z");
        assert!(z.metadata.base_classes.len() >= 2);
    }

    #[test]
    fn class_document_has_title_method() {
        let items = fixture_items();
        let d = find_by_name(&items, "Document");
        assert!(
            d.metadata.methods.contains(&"title".to_string()),
            "Document should have title method, got {:?}",
            d.metadata.methods
        );
    }

    #[test]
    fn namespace_signature_format() {
        let items = fixture_items();
        let m = find_by_name(&items, "math");
        assert!(
            m.signature.contains("namespace"),
            "namespace signature should contain 'namespace', got {:?}",
            m.signature
        );
    }

    #[test]
    fn constexpr_pi_has_return_type() {
        let items = fixture_items();
        let pi = find_by_name(&items, "PI");
        assert!(
            pi.metadata
                .return_type
                .as_deref()
                .unwrap_or("")
                .contains("double"),
            "PI should have double return_type, got {:?}",
            pi.metadata.return_type
        );
    }

    // ════════════════════════════════════════════════════════════════
    // 22. Qualified identifier / out-of-class method tests
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn class_out_of_class_exists() {
        let items = fixture_items();
        let oc = find_by_name(&items, "OutOfClass");
        assert_eq!(oc.kind, SymbolKind::Class);
    }

    #[test]
    fn qualified_identifier_recursive() {
        let items = parse_and_extract("class Foo { public: void bar(); };\nvoid Foo::bar() {}");
        // The out-of-class definition should be captured with qualified name
        let has_qualified = items
            .iter()
            .any(|i| i.kind == SymbolKind::Function && i.name.contains("Foo"));
        // Either found as qualified name or as plain function
        assert!(
            has_qualified
                || items
                    .iter()
                    .any(|i| i.kind == SymbolKind::Function && i.name == "bar"),
            "out-of-class method should be extracted"
        );
    }

    #[test]
    fn minimal_qualified_identifier() {
        let items = parse_and_extract("class A {};\nvoid A::foo() {}");
        let funcs: Vec<_> = items
            .iter()
            .filter(|i| i.kind == SymbolKind::Function && i.name.contains("foo"))
            .collect();
        assert!(
            !funcs.is_empty(),
            "qualified function A::foo should be extracted"
        );
    }

    // ════════════════════════════════════════════════════════════════
    // 23. C++11 Attribute tests
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn attributed_nodiscard_function() {
        let items = parse_and_extract("[[nodiscard]] int foo() { return 1; }");
        let f = items
            .iter()
            .find(|i| i.kind == SymbolKind::Function && i.name == "foo");
        assert!(f.is_some(), "[[nodiscard]] function should be extracted");
    }

    #[test]
    fn attributed_deprecated_function() {
        let items = parse_and_extract("[[deprecated]] void old() {}");
        let f = items
            .iter()
            .find(|i| i.kind == SymbolKind::Function && i.name == "old");
        assert!(f.is_some(), "[[deprecated]] function should be extracted");
    }

    #[test]
    fn fixture_must_use_result_exists() {
        let items = fixture_items();
        let f = find_by_name(&items, "must_use_result");
        assert_eq!(f.kind, SymbolKind::Function);
    }

    #[test]
    fn fixture_old_api_exists() {
        let items = fixture_items();
        let f = find_by_name(&items, "old_api");
        assert_eq!(f.kind, SymbolKind::Function);
    }

    // ════════════════════════════════════════════════════════════════
    // 24. Access specifier tests
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn class_access_demo_exists() {
        let items = fixture_items();
        let ad = find_by_name(&items, "AccessDemo");
        assert_eq!(ad.kind, SymbolKind::Class);
    }

    #[test]
    fn class_access_demo_has_public_members() {
        let items = fixture_items();
        let ad = find_by_name(&items, "AccessDemo");
        assert!(
            ad.metadata
                .attributes
                .contains(&"has_public_members".to_string()),
            "AccessDemo should track public members, got {:?}",
            ad.metadata.attributes
        );
    }

    #[test]
    fn class_access_demo_has_private_members() {
        let items = fixture_items();
        let ad = find_by_name(&items, "AccessDemo");
        assert!(
            ad.metadata
                .attributes
                .contains(&"has_private_members".to_string()),
            "AccessDemo should track private members, got {:?}",
            ad.metadata.attributes
        );
    }

    #[test]
    fn class_access_demo_has_protected_members() {
        let items = fixture_items();
        let ad = find_by_name(&items, "AccessDemo");
        assert!(
            ad.metadata
                .attributes
                .contains(&"has_protected_members".to_string()),
            "AccessDemo should track protected members, got {:?}",
            ad.metadata.attributes
        );
    }

    #[test]
    fn class_access_demo_has_methods() {
        let items = fixture_items();
        let ad = find_by_name(&items, "AccessDemo");
        assert!(
            ad.metadata.methods.contains(&"pub_method".to_string()),
            "AccessDemo should have pub_method"
        );
    }

    // ════════════════════════════════════════════════════════════════
    // 25. Template alias tests
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn template_alias_shared_ptr() {
        let items = fixture_items();
        let sp = find_by_name(&items, "SharedPtr");
        assert_eq!(sp.kind, SymbolKind::TypeAlias);
        assert!(
            sp.metadata.attributes.contains(&"template".to_string()),
            "SharedPtr should have template attribute"
        );
        assert!(
            sp.metadata.attributes.contains(&"using".to_string()),
            "SharedPtr should have using attribute"
        );
    }

    #[test]
    fn template_alias_shared_ptr_has_generics() {
        let items = fixture_items();
        let sp = find_by_name(&items, "SharedPtr");
        assert!(
            sp.metadata.generics.is_some(),
            "SharedPtr should have generics"
        );
    }

    #[test]
    fn template_alias_map() {
        let items = fixture_items();
        let m = find_by_name(&items, "Map");
        assert_eq!(m.kind, SymbolKind::TypeAlias);
        assert!(
            m.metadata.attributes.contains(&"template".to_string()),
            "Map should have template attribute"
        );
    }

    #[test]
    fn minimal_template_alias() {
        let items = parse_and_extract("template<typename T> using Ptr = T*;");
        let p = find_by_name(&items, "Ptr");
        assert_eq!(p.kind, SymbolKind::TypeAlias);
        assert!(p.metadata.attributes.contains(&"template".to_string()));
        assert!(p.metadata.attributes.contains(&"using".to_string()));
        assert!(p.metadata.generics.is_some());
    }

    // ════════════════════════════════════════════════════════════════
    // 26. Namespace alias tests
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn namespace_alias_vln() {
        let items = fixture_items();
        let vln = find_by_name(&items, "vln");
        assert_eq!(vln.kind, SymbolKind::Module);
        assert!(
            vln.metadata
                .attributes
                .contains(&"namespace_alias".to_string()),
            "vln should have namespace_alias attribute"
        );
    }

    #[test]
    fn minimal_namespace_alias() {
        let items = parse_and_extract("namespace orig {} namespace alias = orig;");
        let a = find_by_name(&items, "alias");
        assert_eq!(a.kind, SymbolKind::Module);
        assert!(
            a.metadata
                .attributes
                .contains(&"namespace_alias".to_string())
        );
    }

    // ════════════════════════════════════════════════════════════════
    // 27. Friend declaration tests
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn class_secret_holder_has_friends() {
        let items = fixture_items();
        let sh = find_by_name(&items, "SecretHolder");
        let has_friend = sh
            .metadata
            .attributes
            .iter()
            .any(|a| a.starts_with("friend:"));
        assert!(
            has_friend,
            "SecretHolder should have friend attributes, got {:?}",
            sh.metadata.attributes
        );
    }

    #[test]
    fn class_friend_demo_has_friends() {
        let items = fixture_items();
        let fd = find_by_name(&items, "FriendDemo");
        let has_friend = fd
            .metadata
            .attributes
            .iter()
            .any(|a| a.starts_with("friend:"));
        assert!(
            has_friend,
            "FriendDemo should have friend attributes, got {:?}",
            fd.metadata.attributes
        );
    }

    #[test]
    fn minimal_friend_class() {
        let items = parse_and_extract("class A { friend class B; };");
        let a = find_by_name(&items, "A");
        assert!(
            a.metadata.attributes.iter().any(|a| a.contains("friend")),
            "A should have friend attribute"
        );
    }

    // ════════════════════════════════════════════════════════════════
    // 28. Nested types in class tests
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn class_nesting_demo_exists() {
        let items = fixture_items();
        let nd = find_by_name(&items, "NestingDemo");
        assert_eq!(nd.kind, SymbolKind::Class);
    }

    #[test]
    fn nested_enum_inner_status() {
        let items = fixture_items();
        let e = items
            .iter()
            .find(|i| i.kind == SymbolKind::Enum && i.name == "InnerStatus");
        assert!(e.is_some(), "nested enum InnerStatus should be extracted");
    }

    #[test]
    fn nested_struct_inner_config() {
        let items = fixture_items();
        let s = items
            .iter()
            .find(|i| i.kind == SymbolKind::Struct && i.name == "InnerConfig");
        assert!(s.is_some(), "nested struct InnerConfig should be extracted");
    }

    #[test]
    fn nested_class_inner() {
        let items = fixture_items();
        let inner = items
            .iter()
            .find(|i| i.kind == SymbolKind::Class && i.name == "Inner");
        assert!(inner.is_some(), "nested class Inner should be extracted");
    }

    // ════════════════════════════════════════════════════════════════
    // 29. Method qualifier tests
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn class_method_derived_exists() {
        let items = fixture_items();
        let md = find_by_name(&items, "MethodDerived");
        assert_eq!(md.kind, SymbolKind::Class);
    }

    #[test]
    fn class_method_derived_has_override() {
        let items = fixture_items();
        let md = find_by_name(&items, "MethodDerived");
        assert!(
            md.metadata.attributes.contains(&"has_override".to_string()),
            "MethodDerived should have has_override attribute, got {:?}",
            md.metadata.attributes
        );
    }

    #[test]
    fn class_method_derived_has_final_methods() {
        let items = fixture_items();
        let md = find_by_name(&items, "MethodDerived");
        assert!(
            md.metadata
                .attributes
                .contains(&"has_final_methods".to_string()),
            "MethodDerived should have has_final_methods attribute, got {:?}",
            md.metadata.attributes
        );
    }

    #[test]
    fn class_method_derived_has_deleted_members() {
        let items = fixture_items();
        let md = find_by_name(&items, "MethodDerived");
        assert!(
            md.metadata
                .attributes
                .contains(&"has_deleted_members".to_string()),
            "MethodDerived should have has_deleted_members, got {:?}",
            md.metadata.attributes
        );
    }

    #[test]
    fn class_method_derived_has_defaulted_members() {
        let items = fixture_items();
        let md = find_by_name(&items, "MethodDerived");
        assert!(
            md.metadata
                .attributes
                .contains(&"has_defaulted_members".to_string()),
            "MethodDerived should have has_defaulted_members, got {:?}",
            md.metadata.attributes
        );
    }

    #[test]
    fn class_resource_guard_has_deleted_members() {
        let items = fixture_items();
        let rg = find_by_name(&items, "ResourceGuard");
        assert!(
            rg.metadata
                .attributes
                .contains(&"has_deleted_members".to_string()),
            "ResourceGuard should have has_deleted_members, got {:?}",
            rg.metadata.attributes
        );
    }

    // ════════════════════════════════════════════════════════════════
    // 30. Inline namespace tests
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn inline_namespace_v2() {
        let items = fixture_items();
        let v2 = find_by_name(&items, "v2");
        assert_eq!(v2.kind, SymbolKind::Module);
        assert!(
            v2.metadata.attributes.contains(&"inline".to_string()),
            "v2 should have inline attribute, got {:?}",
            v2.metadata.attributes
        );
    }

    #[test]
    fn inline_namespace_signature() {
        let items = fixture_items();
        let v2 = find_by_name(&items, "v2");
        assert!(
            v2.signature.contains("inline"),
            "inline namespace signature should contain 'inline', got {:?}",
            v2.signature
        );
    }

    #[test]
    fn minimal_inline_namespace() {
        let items = parse_and_extract("inline namespace detail {}");
        let d = find_by_name(&items, "detail");
        assert_eq!(d.kind, SymbolKind::Module);
        assert!(d.metadata.attributes.contains(&"inline".to_string()));
    }

    // ════════════════════════════════════════════════════════════════
    // 31. Template instantiation tests
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn explicit_template_instantiation() {
        let items = fixture_items();
        let inst = items.iter().find(|i| {
            i.metadata
                .attributes
                .contains(&"explicit_instantiation".to_string())
        });
        assert!(
            inst.is_some(),
            "explicit template instantiation should be extracted"
        );
    }

    #[test]
    fn explicit_template_instantiation_name() {
        let items = fixture_items();
        let inst = items
            .iter()
            .find(|i| {
                i.metadata
                    .attributes
                    .contains(&"explicit_instantiation".to_string())
            })
            .expect("instantiation should exist");
        assert!(
            inst.name.contains("Container"),
            "instantiation should reference Container, got {:?}",
            inst.name
        );
    }

    // ════════════════════════════════════════════════════════════════
    // 32. Requires clause tests
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn requires_clause_checked_add() {
        let items = fixture_items();
        let f = find_by_name(&items, "checked_add");
        assert_eq!(f.kind, SymbolKind::Function);
        assert!(
            f.metadata.attributes.contains(&"template".to_string()),
            "checked_add should be a template"
        );
    }

    #[test]
    fn requires_clause_has_requires_attr() {
        let items = fixture_items();
        let f = find_by_name(&items, "checked_add");
        let has_requires = f
            .metadata
            .attributes
            .iter()
            .any(|a| a.starts_with("requires:"));
        assert!(
            has_requires,
            "checked_add should have requires: attribute, got {:?}",
            f.metadata.attributes
        );
    }

    // ════════════════════════════════════════════════════════════════
    // 33. Union inside namespace tests
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn namespace_geo_exists() {
        let items = fixture_items();
        let geo = find_by_name(&items, "geo");
        assert_eq!(geo.kind, SymbolKind::Module);
    }

    #[test]
    fn union_shape_data_in_namespace() {
        let items = fixture_items();
        let sd = items
            .iter()
            .find(|i| i.kind == SymbolKind::Union && i.name == "ShapeData");
        assert!(
            sd.is_some(),
            "ShapeData union inside geo namespace should be extracted"
        );
    }

    #[test]
    fn function_in_geo_namespace() {
        let items = fixture_items();
        let ac = items
            .iter()
            .find(|i| i.kind == SymbolKind::Function && i.name == "area_calc");
        assert!(
            ac.is_some(),
            "area_calc function in geo namespace should be extracted"
        );
    }

    // ════════════════════════════════════════════════════════════════
    // 34. Decltype return type tests
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn decltype_example_exists() {
        let items = fixture_items();
        let de = find_by_name(&items, "decltype_example");
        assert_eq!(de.kind, SymbolKind::Function);
    }

    #[test]
    fn decltype_in_return_type() {
        let items = parse_and_extract("auto foo(int a) -> decltype(a) { return a; }");
        let f = find_by_name(&items, "foo");
        assert_eq!(f.kind, SymbolKind::Function);
    }

    // ════════════════════════════════════════════════════════════════
    // 35. Using directive tests
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn using_directive_std() {
        let items = fixture_items();
        let ud = items.iter().find(|i| {
            i.kind == SymbolKind::Module
                && i.metadata
                    .attributes
                    .contains(&"using_directive".to_string())
        });
        assert!(ud.is_some(), "using namespace std should be extracted");
    }

    #[test]
    fn using_directive_name_is_std() {
        let items = fixture_items();
        let ud = items
            .iter()
            .find(|i| {
                i.kind == SymbolKind::Module
                    && i.metadata
                        .attributes
                        .contains(&"using_directive".to_string())
            })
            .expect("using directive should exist");
        assert!(
            ud.name.contains("std"),
            "using directive name should contain 'std', got {:?}",
            ud.name
        );
    }

    #[test]
    fn minimal_using_directive() {
        let items = parse_and_extract("using namespace std;");
        let ud = items.iter().find(|i| {
            i.metadata
                .attributes
                .contains(&"using_directive".to_string())
        });
        assert!(ud.is_some(), "using namespace std should be extracted");
    }

    // ════════════════════════════════════════════════════════════════
    // 36. Wrapper struct / deduction guide tests
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn template_wrapper_struct() {
        let items = fixture_items();
        let w = find_by_name(&items, "Wrapper");
        assert_eq!(w.kind, SymbolKind::Struct);
        assert!(
            w.metadata.attributes.contains(&"template".to_string()),
            "Wrapper should be a template struct"
        );
    }

    // ════════════════════════════════════════════════════════════════
    // 37. MustUseClass attributed class tests
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn must_use_class_exists() {
        let items = fixture_items();
        let mu = find_by_name(&items, "MustUseClass");
        assert_eq!(mu.kind, SymbolKind::Class);
    }

    // ════════════════════════════════════════════════════════════════
    // 38. very_long_namespace_name
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn very_long_namespace_exists() {
        let items = fixture_items();
        let ns = find_by_name(&items, "very_long_namespace_name");
        assert_eq!(ns.kind, SymbolKind::Module);
    }

    // ════════════════════════════════════════════════════════════════
    // 39. MethodBase tests
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn class_method_base_exists() {
        let items = fixture_items();
        let mb = find_by_name(&items, "MethodBase");
        assert_eq!(mb.kind, SymbolKind::Class);
    }

    #[test]
    fn class_method_base_has_methods() {
        let items = fixture_items();
        let mb = find_by_name(&items, "MethodBase");
        assert!(
            mb.metadata.methods.contains(&"normal_virtual".to_string()),
            "MethodBase should have normal_virtual method"
        );
    }

    // ════════════════════════════════════════════════════════════════
    // 40. TemplateMethodHost tests
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn class_template_method_host_exists() {
        let items = fixture_items();
        let tmh = find_by_name(&items, "TemplateMethodHost");
        assert_eq!(tmh.kind, SymbolKind::Class);
    }

    // ════════════════════════════════════════════════════════════════
    // 41. Structured binding tests
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn function_use_structured_bindings_exists() {
        let items = fixture_items();
        let usb = find_by_name(&items, "use_structured_bindings");
        assert_eq!(usb.kind, SymbolKind::Function);
    }

    // ════════════════════════════════════════════════════════════════
    // 42. Fixture count validation tests
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn fixture_has_unions() {
        let items = fixture_items();
        let unions = find_all_by_kind(&items, SymbolKind::Union);
        assert!(!unions.is_empty(), "expected at least 1 union (ShapeData)");
    }

    #[test]
    fn fixture_total_items_increased() {
        let items = fixture_items();
        assert!(
            items.len() >= 60,
            "expected 60+ items with new features, got {}",
            items.len()
        );
    }

    // ════════════════════════════════════════════════════════════════
    // 43. Minimal edge-case tests for new features
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn minimal_friend_function() {
        let items = parse_and_extract("class C { friend void f(); public: void g() {} };");
        let c = find_by_name(&items, "C");
        let has_friend = c.metadata.attributes.iter().any(|a| a.contains("friend"));
        assert!(has_friend, "class with friend function should track it");
    }

    #[test]
    fn minimal_nested_enum() {
        let items = parse_and_extract("class Outer { public: enum class E { A, B }; };");
        let e = items
            .iter()
            .find(|i| i.kind == SymbolKind::Enum && i.name == "E");
        assert!(e.is_some(), "nested enum should be extracted");
    }

    #[test]
    fn minimal_nested_struct() {
        let items = parse_and_extract("class Outer { public: struct S { int x; }; };");
        let s = items
            .iter()
            .find(|i| i.kind == SymbolKind::Struct && i.name == "S");
        assert!(s.is_some(), "nested struct should be extracted");
    }

    #[test]
    fn minimal_override_method() {
        let items = parse_and_extract(
            "class Base { public: virtual void f() {} };\nclass D : public Base { public: void f() override {} };",
        );
        let d = find_by_name(&items, "D");
        assert!(
            d.metadata.attributes.contains(&"has_override".to_string()),
            "D should have has_override attribute"
        );
    }

    #[test]
    fn minimal_deleted_constructor() {
        let items = parse_and_extract(
            "class NoCopy { public: NoCopy() = default; NoCopy(const NoCopy&) = delete; };",
        );
        let nc = find_by_name(&items, "NoCopy");
        assert!(
            nc.metadata
                .attributes
                .contains(&"has_deleted_members".to_string()),
            "NoCopy should have has_deleted_members, got {:?}",
            nc.metadata.attributes
        );
    }

    #[test]
    fn minimal_union_in_namespace() {
        let items = parse_and_extract("namespace ns { union U { int x; double y; }; }");
        let u = items
            .iter()
            .find(|i| i.kind == SymbolKind::Union && i.name == "U");
        assert!(u.is_some(), "union in namespace should be extracted");
    }

    #[test]
    fn minimal_template_instantiation() {
        let items = parse_and_extract("template<typename T> class V {}; template class V<int>;");
        let inst = items.iter().find(|i| {
            i.metadata
                .attributes
                .contains(&"explicit_instantiation".to_string())
        });
        assert!(
            inst.is_some(),
            "explicit template instantiation should be extracted"
        );
    }

    #[test]
    fn minimal_requires_clause() {
        let items =
            parse_and_extract("template<typename T> requires true T ident(T x) { return x; }");
        let f = find_by_name(&items, "ident");
        let has_requires = f
            .metadata
            .attributes
            .iter()
            .any(|a| a.starts_with("requires:"));
        assert!(
            has_requires,
            "function with requires clause should have requires attr, got {:?}",
            f.metadata.attributes
        );
    }
}
