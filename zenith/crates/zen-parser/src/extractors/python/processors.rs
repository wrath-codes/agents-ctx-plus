use ast_grep_core::Node;

use crate::extractors::helpers;
use crate::types::{ParsedItem, PythonMetadataExt, SymbolKind, SymbolMetadata};

use super::doc::{extract_docstring, parse_python_doc_sections};
use super::pyhelpers::{
    decorator_matches, decorator_matches_any, detect_generator, extract_decorators,
    extract_python_parameters, is_exception_subclass, python_visibility,
};

pub(super) fn extract_dunder_all<D: ast_grep_core::Doc>(root: &Node<D>) -> Option<Vec<String>> {
    for child in root.children() {
        if child.kind().as_ref() != "expression_statement" {
            continue;
        }
        let text = child.text().to_string();
        let trimmed = text.trim();
        if !trimmed.starts_with("__all__") {
            continue;
        }
        // Extract names from __all__ = ["name1", "name2", ...]
        let mut names = Vec::new();
        if let Some(bracket_start) = text.find('[')
            && let Some(bracket_end) = text.rfind(']')
        {
            let inner = &text[bracket_start + 1..bracket_end];
            for part in inner.split(',') {
                let part = part.trim().trim_matches('"').trim_matches('\'');
                if !part.is_empty() {
                    names.push(part.to_string());
                }
            }
        }
        if !names.is_empty() {
            return Some(names);
        }
    }
    None
}

pub(super) fn process_decorated<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<ParsedItem> {
    let decorators = extract_decorators(node);
    let inner = node.children().find(|c| {
        let k = c.kind();
        k.as_ref() == "class_definition" || k.as_ref() == "function_definition"
    })?;

    match inner.kind().as_ref() {
        "class_definition" => process_class(&inner, &decorators),
        "function_definition" => process_function(&inner, &decorators),
        _ => None,
    }
}

pub(super) fn process_class<D: ast_grep_core::Doc>(
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

pub(super) fn process_function<D: ast_grep_core::Doc>(
    node: &Node<D>,
    decorators: &[String],
) -> Option<ParsedItem> {
    let name = node.field("name").map(|n| n.text().to_string())?;

    // Trim leading whitespace before checking for "async " prefix
    let is_async = node.text().trim_start().starts_with("async ");
    let return_type = node.field("return_type").map(|rt| rt.text().to_string());
    let parameters = extract_python_parameters(node);
    let docstring = extract_docstring(node);
    let doc_sections = parse_python_doc_sections(&docstring);
    let is_generator = detect_generator(node);

    let is_property = decorator_matches(decorators, "property")
        || decorator_matches(decorators, "cached_property");
    let is_classmethod = decorator_matches(decorators, "classmethod");
    let is_staticmethod = decorator_matches(decorators, "staticmethod");
    let is_overload = decorator_matches(decorators, "overload");
    let is_context_manager = decorator_matches(decorators, "contextmanager")
        || decorator_matches(decorators, "asynccontextmanager");
    let is_abstract = decorator_matches(decorators, "abstractmethod");

    let visibility = python_visibility(&name);
    let returns_result = helpers::returns_result(return_type.as_deref());

    // Store semantic flags in attributes
    let mut attributes = Vec::new();
    if is_overload {
        attributes.push("overload".to_string());
    }
    if is_context_manager {
        attributes.push("contextmanager".to_string());
    }
    if is_abstract {
        attributes.push("abstractmethod".to_string());
    }

    let mut metadata = SymbolMetadata {
        return_type,
        parameters,
        decorators: decorators.to_vec(),
        attributes,
        returns_result,
        doc_sections,
        ..Default::default()
    };

    if is_async {
        metadata.is_async = true;
    }
    if is_property {
        metadata.mark_property();
    }
    if is_classmethod {
        metadata.mark_classmethod();
    }
    if is_staticmethod {
        metadata.mark_staticmethod();
    }
    if is_generator {
        metadata.mark_generator();
    }

    Some(ParsedItem {
        kind: SymbolKind::Function,
        name,
        signature: helpers::extract_signature_python(node),
        source: helpers::extract_source(node, 50),
        doc_comment: docstring,
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility,
        metadata,
    })
}

pub(super) fn process_module_assignment<D: ast_grep_core::Doc>(
    expr_stmt: &Node<D>,
) -> Option<ParsedItem> {
    let assignment = expr_stmt
        .children()
        .find(|c| c.kind().as_ref() == "assignment")?;

    let name_node = assignment
        .children()
        .find(|c| c.kind().as_ref() == "identifier")?;
    let name = name_node.text().to_string();
    if name.is_empty() {
        return None;
    }

    // Skip __all__ (handled separately) and docstrings
    if name == "__all__" {
        return None;
    }

    let type_annotation = assignment
        .children()
        .find(|c| c.kind().as_ref() == "type")
        .map(|t| t.text().to_string());

    // Detect TypeAlias annotation
    let is_type_alias = type_annotation
        .as_ref()
        .is_some_and(|t| t.contains("TypeAlias"));

    let symbol_kind = if is_type_alias {
        SymbolKind::TypeAlias
    } else {
        SymbolKind::Const
    };

    let visibility = python_visibility(&name);

    Some(ParsedItem {
        kind: symbol_kind,
        name,
        signature: assignment.text().to_string(),
        source: Some(assignment.text().to_string()),
        doc_comment: String::new(),
        start_line: expr_stmt.start_pos().line() as u32 + 1,
        end_line: expr_stmt.end_pos().line() as u32 + 1,
        visibility,
        metadata: SymbolMetadata {
            return_type: type_annotation,
            ..Default::default()
        },
    })
}
