//! Function definition processing for Python extraction.

use ast_grep_core::Node;

use crate::extractors::helpers;
use crate::types::{ParsedItem, PythonMetadataExt, SymbolKind, SymbolMetadata};

use super::super::doc::{extract_docstring, parse_python_doc_sections};
use super::super::pyhelpers::{
    decorator_matches, detect_generator, extract_python_parameters, python_visibility,
};

pub fn process_function<D: ast_grep_core::Doc>(
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

    let kind = if name == "__init__" {
        SymbolKind::Constructor
    } else if is_property {
        SymbolKind::Property
    } else {
        SymbolKind::Function
    };

    Some(ParsedItem {
        kind,
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
