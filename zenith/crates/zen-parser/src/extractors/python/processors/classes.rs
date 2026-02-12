//! Class definition processing for Python extraction.

use ast_grep_core::Node;
use std::collections::HashSet;

use crate::extractors::helpers;
use crate::types::{ParsedItem, PythonMetadataExt, SymbolKind, SymbolMetadata, Visibility};

use super::super::doc::{extract_docstring, parse_python_doc_sections};
use super::super::pyhelpers::{decorator_matches_any, is_exception_subclass, python_visibility};

pub fn process_class<D: ast_grep_core::Doc>(
    node: &Node<D>,
    decorators: &[String],
) -> Option<ParsedItem> {
    let name = node.field("name").map(|n| n.text().to_string())?;

    let base_classes = extract_base_classes(node);
    let docstring = extract_docstring(node);
    let doc_sections = parse_python_doc_sections(&docstring);

    let is_dataclass = decorator_matches_any(decorators, &["dataclass"]);
    let is_pydantic = base_classes.iter().any(|b| b.contains("BaseModel"));
    let is_protocol = base_classes.iter().any(|b| b == "Protocol");
    let is_enum = base_classes
        .iter()
        .any(|b| b == "Enum" || b == "IntEnum" || b == "StrEnum");
    let is_namedtuple = base_classes.iter().any(|b| b == "NamedTuple");
    let is_typed_dict = base_classes.iter().any(|b| b == "TypedDict");
    let is_error_type =
        helpers::is_error_type_by_name(&name) || is_exception_subclass(&base_classes);
    let is_generic = base_classes
        .iter()
        .any(|b| b.starts_with("Generic[") || b == "Generic");

    let (methods, fields) = extract_class_members(node);

    // Map to appropriate SymbolKind
    let symbol_kind = if is_enum {
        SymbolKind::Enum
    } else if is_protocol {
        SymbolKind::Interface
    } else {
        SymbolKind::Class
    };

    let visibility = python_visibility(&name);

    // Detect generics from base classes (e.g., Generic[T])
    let generics = base_classes
        .iter()
        .find(|b| b.starts_with("Generic["))
        .map(|b| {
            b.trim_start_matches("Generic")
                .trim_start_matches('[')
                .trim_end_matches(']')
                .to_string()
        });

    // Extract enum variants from fields for enum classes
    let variants = if is_enum { fields.clone() } else { Vec::new() };

    let mut metadata = SymbolMetadata {
        base_classes,
        decorators: decorators.to_vec(),
        methods,
        fields,
        variants,
        doc_sections,
        is_error_type,
        generics,
        ..Default::default()
    };

    if is_dataclass {
        metadata.mark_dataclass();
    }
    if is_pydantic {
        metadata.mark_pydantic();
    }
    if is_protocol {
        metadata.mark_protocol();
    }
    if is_enum {
        metadata.mark_enum();
    }

    Some(ParsedItem {
        kind: symbol_kind,
        name,
        signature: helpers::extract_signature_python(node),
        source: helpers::extract_source(node, 50),
        doc_comment: docstring,
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility,
        metadata,
    })
    .map(|mut item| {
        // Store extra detection in attributes for features without dedicated fields
        if is_namedtuple {
            item.metadata.push_attribute("namedtuple");
        }
        if is_typed_dict {
            item.metadata.push_attribute("typed_dict");
        }
        if is_generic {
            item.metadata.push_attribute("generic");
        }
        item
    })
}

pub fn process_class_member_items<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<ParsedItem> {
    let Some(owner_name) = node.field("name").map(|n| n.text().to_string()) else {
        return Vec::new();
    };
    let Some(body) = node.field("body") else {
        return Vec::new();
    };

    let mut items = Vec::new();
    let mut seen = HashSet::new();
    for child in body.children() {
        match child.kind().as_ref() {
            "function_definition" => {
                if let Some(member) = build_function_member_item(&child, &owner_name, &[]) {
                    push_member_if_new(&mut items, &mut seen, member);
                }
            }
            "decorated_definition" => {
                let decorators: Vec<String> = child
                    .children()
                    .filter(|c| c.kind().as_ref() == "decorator")
                    .map(|c| c.text().trim_start_matches('@').trim().to_string())
                    .collect();
                if let Some(func) = child
                    .children()
                    .find(|c| c.kind().as_ref() == "function_definition")
                    && let Some(member) =
                        build_function_member_item(&func, &owner_name, &decorators)
                {
                    push_member_if_new(&mut items, &mut seen, member);
                }
            }
            "expression_statement" => {
                if let Some(field_item) = build_field_member_item(&child, &owner_name) {
                    push_member_if_new(&mut items, &mut seen, field_item);
                }
            }
            _ => {}
        }
    }

    items
}

/// Extract base classes, filtering out metaclass keyword arguments.
fn extract_base_classes<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    let Some(superclasses) = node.field("superclasses") else {
        return Vec::new();
    };
    superclasses
        .children()
        .filter(|c| {
            let k = c.kind();
            k.as_ref() != "(" && k.as_ref() != ")" && k.as_ref() != ","
        })
        .map(|c| c.text().to_string())
        // Filter out keyword arguments like metaclass=ABCMeta
        .filter(|text| !text.contains("metaclass="))
        .collect()
}

fn extract_class_members<D: ast_grep_core::Doc>(node: &Node<D>) -> (Vec<String>, Vec<String>) {
    let mut methods = Vec::new();
    let mut fields = Vec::new();

    let Some(body) = node.field("body") else {
        return (methods, fields);
    };

    for child in body.children() {
        let k = child.kind();
        match k.as_ref() {
            "function_definition" => {
                if let Some(name) = child.field("name") {
                    methods.push(name.text().to_string());
                }
                // Extract instance variables from __init__ body
                if let Some(name_node) = child.field("name")
                    && name_node.text().as_ref() == "__init__"
                {
                    extract_instance_vars(&child, &mut fields);
                }
            }
            "decorated_definition" => {
                let inner = child
                    .children()
                    .find(|c| c.kind().as_ref() == "function_definition");
                if let Some(func) = inner
                    && let Some(name) = func.field("name")
                {
                    methods.push(name.text().to_string());
                }
            }
            "expression_statement" => {
                let text = child.text().to_string();
                let trimmed = text.trim();
                // Skip docstrings
                if trimmed.starts_with("\"\"\"")
                    || trimmed.starts_with("'''")
                    || trimmed.starts_with('"')
                    || trimmed.starts_with('\'')
                {
                    continue;
                }
                if (trimmed.contains('=') || trimmed.contains(':'))
                    && let Some(var_name) = trimmed.split([':', '=']).next()
                {
                    let var_name = var_name.trim();
                    if !var_name.is_empty() {
                        fields.push(var_name.to_string());
                    }
                }
            }
            _ => {}
        }
    }
    (methods, fields)
}

/// Extract `self.x` assignments from `__init__` method body.
fn extract_instance_vars<D: ast_grep_core::Doc>(init_node: &Node<D>, fields: &mut Vec<String>) {
    let Some(body) = init_node.field("body") else {
        return;
    };
    for child in body.children() {
        if child.kind().as_ref() != "expression_statement" {
            continue;
        }
        let text = child.text().to_string();
        let trimmed = text.trim();
        // Match self.x = ... patterns
        if let Some(attr) = trimmed.strip_prefix("self.")
            && let Some(var_name) = attr.split(['=', ':']).next()
        {
            let var_name = var_name.trim();
            if !var_name.is_empty() && !fields.contains(&var_name.to_string()) {
                fields.push(var_name.to_string());
            }
        }
    }
}

fn build_function_member_item<D: ast_grep_core::Doc>(
    node: &Node<D>,
    owner_name: &str,
    decorators: &[String],
) -> Option<ParsedItem> {
    let name = node.field("name").map(|n| n.text().to_string())?;
    let metadata = SymbolMetadata {
        owner_name: Some(owner_name.to_string()),
        owner_kind: Some(SymbolKind::Class),
        parameters: super::super::pyhelpers::extract_python_parameters(node),
        return_type: node.field("return_type").map(|rt| rt.text().to_string()),
        ..Default::default()
    };

    let kind = if name == "__init__" {
        SymbolKind::Constructor
    } else if super::super::pyhelpers::decorator_matches_any(
        decorators,
        &["property", "cached_property"],
    ) {
        SymbolKind::Property
    } else {
        SymbolKind::Method
    };

    Some(ParsedItem {
        kind,
        name: format!("{owner_name}::{name}"),
        signature: helpers::extract_signature_python(node),
        source: helpers::extract_source(node, 30),
        doc_comment: extract_docstring(node),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: python_visibility(&name),
        metadata,
    })
}

fn build_field_member_item<D: ast_grep_core::Doc>(
    expr_stmt: &Node<D>,
    owner_name: &str,
) -> Option<ParsedItem> {
    let text = expr_stmt.text().to_string();
    let trimmed = text.trim();
    if trimmed.starts_with("\"\"\"")
        || trimmed.starts_with("'''")
        || trimmed.starts_with('"')
        || trimmed.starts_with('\'')
    {
        return None;
    }

    let raw_name = trimmed.split([':', '=']).next()?.trim();
    if raw_name.is_empty() {
        return None;
    }

    let metadata = SymbolMetadata {
        owner_name: Some(owner_name.to_string()),
        owner_kind: Some(SymbolKind::Class),
        is_static_member: true,
        ..Default::default()
    };

    Some(ParsedItem {
        kind: SymbolKind::Field,
        name: format!("{owner_name}::{raw_name}"),
        signature: text.clone(),
        source: Some(text),
        doc_comment: String::new(),
        start_line: expr_stmt.start_pos().line() as u32 + 1,
        end_line: expr_stmt.end_pos().line() as u32 + 1,
        visibility: Visibility::Private,
        metadata,
    })
}

fn push_member_if_new(items: &mut Vec<ParsedItem>, seen: &mut HashSet<String>, item: ParsedItem) {
    let key = if item.kind == SymbolKind::Property {
        format!("{}:{}", item.kind, item.name)
    } else {
        format!("{}:{}:{}", item.kind, item.name, item.start_line)
    };
    if seen.insert(key) {
        items.push(item);
    }
}
