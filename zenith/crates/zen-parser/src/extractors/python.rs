//! Python rich extractor — classes, functions, decorators, docstrings.
//!
//! Extracts from `function_definition`, `class_definition`,
//! `decorated_definition`, and module-level typed assignments.

use ast_grep_core::matcher::KindMatcher;
use ast_grep_core::ops::Any;
use ast_grep_core::Node;
use ast_grep_language::SupportLang;

use super::helpers;
use crate::types::{DocSections, ParsedItem, SymbolKind, SymbolMetadata, Visibility};

const PYTHON_TOP_KINDS: &[&str] = &[
    "function_definition",
    "class_definition",
    "decorated_definition",
];

/// Extract all API symbols from a Python source file.
///
/// # Errors
/// Returns `ParserError` if parsing fails.
pub fn extract<D: ast_grep_core::Doc<Lang = SupportLang>>(
    root: &ast_grep_core::AstGrep<D>,
) -> Result<Vec<ParsedItem>, crate::error::ParserError> {
    let mut items = Vec::new();
    let matchers: Vec<KindMatcher> = PYTHON_TOP_KINDS
        .iter()
        .map(|k| KindMatcher::new(k, SupportLang::Python))
        .collect();
    let matcher = Any::new(matchers);

    for node in root.root().find_all(&matcher) {
        let kind = node.kind();
        match kind.as_ref() {
            "decorated_definition" => {
                if let Some(item) = process_decorated(&node) {
                    items.push(item);
                }
            }
            "class_definition" => {
                if let Some(item) = process_class(&node, &[]) {
                    items.push(item);
                }
            }
            "function_definition" => {
                if let Some(item) = process_function(&node, &[], false) {
                    items.push(item);
                }
            }
            _ => {}
        }
    }

    // Module-level typed assignments (e.g., `MAX_RETRIES: int = 3`)
    for child in root.root().children() {
        if child.kind().as_ref() == "expression_statement"
            && let Some(item) = process_module_assignment(&child)
        {
            items.push(item);
        }
    }

    Ok(items)
}

// ── decorated_definition ───────────────────────────────────────────

fn process_decorated<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<ParsedItem> {
    let decorators = extract_decorators(node);
    let inner = node
        .children()
        .find(|c| {
            let k = c.kind();
            k.as_ref() == "class_definition" || k.as_ref() == "function_definition"
        })?;

    match inner.kind().as_ref() {
        "class_definition" => process_class(&inner, &decorators),
        "function_definition" => process_function(&inner, &decorators, false),
        _ => None,
    }
}

fn extract_decorators<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    node.children()
        .filter(|c| c.kind().as_ref() == "decorator")
        .map(|c| {
            let text = c.text().to_string();
            text.trim_start_matches('@').trim().to_string()
        })
        .collect()
}

// ── class_definition ───────────────────────────────────────────────

fn process_class<D: ast_grep_core::Doc>(
    node: &Node<D>,
    decorators: &[String],
) -> Option<ParsedItem> {
    let name = node.field("name").map(|n| n.text().to_string())?;

    let base_classes = extract_base_classes(node);
    let docstring = extract_docstring(node);
    let doc_sections = parse_python_doc_sections(&docstring);

    let is_dataclass = decorators.iter().any(|d| d == "dataclass");
    let is_pydantic = base_classes.iter().any(|b| b.contains("BaseModel"));
    let is_protocol = base_classes.iter().any(|b| b == "Protocol");
    let is_enum = base_classes
        .iter()
        .any(|b| b == "Enum" || b == "IntEnum" || b == "StrEnum");
    let is_error_type = helpers::is_error_type_by_name(&name)
        || is_exception_subclass(&base_classes);

    let (methods, fields) = extract_class_members(node, decorators);

    let visibility = if name.starts_with('_') {
        Visibility::Private
    } else {
        Visibility::Public
    };

    Some(ParsedItem {
        kind: SymbolKind::Class,
        name,
        signature: helpers::extract_signature_python(node),
        source: helpers::extract_source(node, 50),
        doc_comment: docstring,
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility,
        metadata: SymbolMetadata {
            base_classes,
            decorators: decorators.to_vec(),
            is_dataclass,
            is_pydantic,
            is_protocol,
            is_enum,
            methods,
            fields,
            doc_sections,
            is_error_type,
            ..Default::default()
        },
    })
}

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
        .collect()
}

const EXCEPTION_BASE_CLASSES: &[&str] = &[
    "Exception",
    "BaseException",
    "ValueError",
    "TypeError",
    "RuntimeError",
    "IOError",
    "OSError",
    "KeyError",
    "IndexError",
    "AttributeError",
    "NotImplementedError",
    "StopIteration",
    "ArithmeticError",
    "LookupError",
    "EnvironmentError",
];

fn is_exception_subclass(base_classes: &[String]) -> bool {
    base_classes
        .iter()
        .any(|b| EXCEPTION_BASE_CLASSES.contains(&b.as_str()) || b.ends_with("Error"))
}

fn extract_class_members<D: ast_grep_core::Doc>(
    node: &Node<D>,
    _class_decorators: &[String],
) -> (Vec<String>, Vec<String>) {
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
            }
            "decorated_definition" => {
                let inner = child.children().find(|c| {
                    c.kind().as_ref() == "function_definition"
                });
                if let Some(func) = inner
                    && let Some(name) = func.field("name")
                {
                    methods.push(name.text().to_string());
                }
            }
            "expression_statement" => {
                let text = child.text().to_string();
                let trimmed = text.trim();
                if (trimmed.contains('=') || trimmed.contains(':'))
                    && let Some(var_name) = trimmed.split([':', '=']).next()
                {
                    let var_name = var_name.trim();
                    if !var_name.starts_with('"')
                        && !var_name.starts_with('\'')
                        && !var_name.is_empty()
                    {
                        fields.push(var_name.to_string());
                    }
                }
            }
            _ => {}
        }
    }
    (methods, fields)
}

// ── function_definition ────────────────────────────────────────────

fn process_function<D: ast_grep_core::Doc>(
    node: &Node<D>,
    decorators: &[String],
    _is_method: bool,
) -> Option<ParsedItem> {
    let name = node.field("name").map(|n| n.text().to_string())?;

    let is_async = node.text().starts_with("async ");
    let return_type = node.field("return_type").map(|rt| rt.text().to_string());
    let parameters = extract_python_parameters(node);
    let docstring = extract_docstring(node);
    let doc_sections = parse_python_doc_sections(&docstring);
    let is_generator = detect_generator(node);

    let is_property = decorators.iter().any(|d| d == "property");
    let is_classmethod = decorators.iter().any(|d| d == "classmethod");
    let is_staticmethod = decorators.iter().any(|d| d == "staticmethod");

    let visibility = if name.starts_with('_') {
        Visibility::Private
    } else {
        Visibility::Public
    };

    Some(ParsedItem {
        kind: SymbolKind::Function,
        name,
        signature: helpers::extract_signature_python(node),
        source: helpers::extract_source(node, 50),
        doc_comment: docstring,
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility,
        metadata: SymbolMetadata {
            is_async,
            return_type,
            parameters,
            decorators: decorators.to_vec(),
            is_property,
            is_classmethod,
            is_staticmethod,
            is_generator,
            returns_result: false,
            doc_sections,
            ..Default::default()
        },
    })
}

// ── module-level assignments ───────────────────────────────────────

fn process_module_assignment<D: ast_grep_core::Doc>(
    expr_stmt: &Node<D>,
) -> Option<ParsedItem> {
    // expression_statement → assignment with identifier + type
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

    let type_annotation = assignment
        .children()
        .find(|c| c.kind().as_ref() == "type")
        .map(|t| t.text().to_string());

    let visibility = if name.starts_with('_') {
        Visibility::Private
    } else {
        Visibility::Public
    };

    Some(ParsedItem {
        kind: SymbolKind::Const,
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

fn extract_python_parameters<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    let Some(params) = node.field("parameters") else {
        return Vec::new();
    };
    params
        .children()
        .filter(|c| {
            let k = c.kind();
            k.as_ref() != "(" && k.as_ref() != ")" && k.as_ref() != ","
        })
        .map(|c| c.text().to_string())
        .collect()
}

fn detect_generator<D: ast_grep_core::Doc>(node: &Node<D>) -> bool {
    let Some(body) = node.field("body") else {
        return false;
    };
    let yield_matcher = KindMatcher::new("yield", SupportLang::Python);
    body.find(yield_matcher).is_some()
}

// ── Docstring extraction ───────────────────────────────────────────

fn extract_docstring<D: ast_grep_core::Doc>(node: &Node<D>) -> String {
    let Some(body) = node.field("body") else {
        return String::new();
    };

    let Some(first_child) = body.children().next() else {
        return String::new();
    };

    if first_child.kind().as_ref() != "expression_statement" {
        return String::new();
    }

    let Some(s) = first_child
        .children()
        .find(|c| c.kind().as_ref() == "string")
    else {
        return String::new();
    };

    let text = s.text().to_string();
    text.trim_start_matches("\"\"\"")
        .trim_end_matches("\"\"\"")
        .trim_start_matches("'''")
        .trim_end_matches("'''")
        .trim()
        .to_string()
}

// ── Docstring section parsing ──────────────────────────────────────

fn parse_python_doc_sections(doc: &str) -> DocSections {
    if doc.is_empty() {
        return DocSections::default();
    }

    // Try Google-style first, then Sphinx-style
    let mut sections = parse_google_style(doc);
    if sections_empty(&sections) {
        sections = parse_sphinx_style(doc);
    }
    sections
}

fn sections_empty(s: &DocSections) -> bool {
    s.args.is_empty()
        && s.returns.is_none()
        && s.raises.is_empty()
        && s.yields.is_none()
}

fn parse_google_style(doc: &str) -> DocSections {
    let mut sections = DocSections::default();
    let mut current_section: Option<&str> = None;
    let mut current_content = String::new();

    for line in doc.lines() {
        let trimmed = line.trim();

        if trimmed == "Args:" || trimmed == "Arguments:" || trimmed == "Parameters:" {
            flush_google_section(&mut sections, current_section, &current_content);
            current_section = Some("args");
            current_content.clear();
        } else if trimmed == "Returns:" || trimmed == "Return:" {
            flush_google_section(&mut sections, current_section, &current_content);
            current_section = Some("returns");
            current_content.clear();
        } else if trimmed == "Raises:" || trimmed == "Exceptions:" {
            flush_google_section(&mut sections, current_section, &current_content);
            current_section = Some("raises");
            current_content.clear();
        } else if trimmed == "Yields:" || trimmed == "Yield:" {
            flush_google_section(&mut sections, current_section, &current_content);
            current_section = Some("yields");
            current_content.clear();
        } else if trimmed == "Examples:" || trimmed == "Example:" {
            flush_google_section(&mut sections, current_section, &current_content);
            current_section = Some("examples");
            current_content.clear();
        } else if trimmed == "Notes:" || trimmed == "Note:" {
            flush_google_section(&mut sections, current_section, &current_content);
            current_section = Some("notes");
            current_content.clear();
        } else if current_section.is_some() {
            if !current_content.is_empty() {
                current_content.push('\n');
            }
            current_content.push_str(trimmed);
        }
    }
    flush_google_section(&mut sections, current_section, &current_content);
    sections
}

fn flush_google_section(
    sections: &mut DocSections,
    heading: Option<&str>,
    content: &str,
) {
    let content = content.trim();
    if content.is_empty() {
        return;
    }
    match heading {
        Some("args") => {
            for param in parse_param_block(content) {
                sections.args.insert(param.0, param.1);
            }
        }
        Some("returns") => sections.returns = Some(content.to_string()),
        Some("raises") => {
            for exc in parse_param_block(content) {
                sections.raises.insert(exc.0, exc.1);
            }
        }
        Some("yields") => sections.yields = Some(content.to_string()),
        Some("examples") => sections.examples = Some(content.to_string()),
        Some("notes") => sections.notes = Some(content.to_string()),
        _ => {}
    }
}

fn parse_param_block(block: &str) -> Vec<(String, String)> {
    let mut params = Vec::new();
    let mut current_name = String::new();
    let mut current_desc = String::new();

    for line in block.lines() {
        let trimmed = line.trim();
        if let Some((name, desc)) = trimmed.split_once(':') {
            if !current_name.is_empty() {
                params.push((current_name.clone(), current_desc.trim().to_string()));
            }
            current_name = name.trim().to_string();
            current_desc = desc.trim().to_string();
        } else if !current_name.is_empty() {
            current_desc.push(' ');
            current_desc.push_str(trimmed);
        }
    }
    if !current_name.is_empty() {
        params.push((current_name, current_desc.trim().to_string()));
    }
    params
}

fn parse_sphinx_style(doc: &str) -> DocSections {
    let mut sections = DocSections::default();

    for line in doc.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(":param ") {
            if let Some((name, desc)) = rest.split_once(':') {
                sections
                    .args
                    .insert(name.trim().to_string(), desc.trim().to_string());
            }
        } else if let Some(rest) = trimmed.strip_prefix(":returns:") {
            sections.returns = Some(rest.trim().to_string());
        } else if let Some(rest) = trimmed.strip_prefix(":return:") {
            sections.returns = Some(rest.trim().to_string());
        } else if let Some(rest) = trimmed.strip_prefix(":raises ")
            && let Some((exc, desc)) = rest.split_once(':')
        {
            sections
                .raises
                .insert(exc.trim().to_string(), desc.trim().to_string());
        }
    }
    sections
}

#[cfg(test)]
mod tests {
    use super::*;
    use ast_grep_language::LanguageExt;
    use pretty_assertions::assert_eq;

    fn parse_and_extract(source: &str) -> Vec<ParsedItem> {
        let root = SupportLang::Python.ast_grep(source);
        extract(&root).expect("extraction should succeed")
    }

    fn find_by_name<'a>(items: &'a [ParsedItem], name: &str) -> &'a ParsedItem {
        items
            .iter()
            .find(|i| i.name == name)
            .unwrap_or_else(|| panic!("no item named '{name}' found"))
    }

    #[test]
    fn extract_from_fixture() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        let names: Vec<&str> = items.iter().map(|i| i.name.as_str()).collect();
        assert!(names.contains(&"BaseProcessor"), "names: {names:?}");
        assert!(names.contains(&"Config"), "names: {names:?}");
        assert!(names.contains(&"fetch_data"), "names: {names:?}");
        assert!(names.contains(&"Validator"), "names: {names:?}");
    }

    #[test]
    fn class_docstring_extracted() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        let base = find_by_name(&items, "BaseProcessor");
        assert_eq!(base.kind, SymbolKind::Class);
        assert!(
            base.doc_comment.contains("base processor"),
            "doc: {:?}",
            base.doc_comment
        );
    }

    #[test]
    fn class_methods_listed() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        let base = find_by_name(&items, "BaseProcessor");
        assert!(
            base.metadata.methods.contains(&"process".to_string()),
            "methods: {:?}",
            base.metadata.methods
        );
        assert!(
            base.metadata.methods.contains(&"helper".to_string()),
            "methods: {:?}",
            base.metadata.methods
        );
    }

    #[test]
    fn dataclass_detected() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        let config = find_by_name(&items, "Config");
        assert_eq!(config.kind, SymbolKind::Class);
        assert!(config.metadata.is_dataclass);
        assert!(
            config.metadata.decorators.iter().any(|d| d == "dataclass"),
            "decorators: {:?}",
            config.metadata.decorators
        );
    }

    #[test]
    fn protocol_detected() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        let validator = find_by_name(&items, "Validator");
        assert!(validator.metadata.is_protocol);
        assert!(
            validator.metadata.base_classes.contains(&"Protocol".to_string()),
            "base_classes: {:?}",
            validator.metadata.base_classes
        );
    }

    #[test]
    fn async_function_detected() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        let fetch = find_by_name(&items, "fetch_data");
        assert!(fetch.metadata.is_async);
        assert_eq!(fetch.kind, SymbolKind::Function);
    }

    #[test]
    fn function_docstring_extracted() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        let fetch = find_by_name(&items, "fetch_data");
        assert!(
            fetch.doc_comment.contains("Fetch data"),
            "doc: {:?}",
            fetch.doc_comment
        );
    }

    #[test]
    fn function_return_type_extracted() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        let fetch = find_by_name(&items, "fetch_data");
        assert_eq!(fetch.metadata.return_type.as_deref(), Some("bytes"));
    }

    #[test]
    fn function_parameters_extracted() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        let fetch = find_by_name(&items, "fetch_data");
        assert!(
            !fetch.metadata.parameters.is_empty(),
            "should have parameters"
        );
    }

    #[test]
    fn google_style_args_parsed() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        let base = find_by_name(&items, "BaseProcessor");
        let process_method = base.metadata.methods.iter().find(|m| *m == "process");
        assert!(process_method.is_some(), "should have process method");
    }

    #[test]
    fn sphinx_style_docstring_parsed() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        let transform = find_by_name(&items, "transform");
        assert!(
            !transform.metadata.doc_sections.args.is_empty(),
            "sphinx :param should be parsed: {:?}",
            transform.metadata.doc_sections.args
        );
        assert!(
            transform.metadata.doc_sections.returns.is_some(),
            "sphinx :returns: should be parsed"
        );
        assert!(
            !transform.metadata.doc_sections.raises.is_empty(),
            "sphinx :raises: should be parsed: {:?}",
            transform.metadata.doc_sections.raises
        );
    }

    #[test]
    fn generator_detected() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        let generator = find_by_name(&items, "generate_items");
        assert!(
            generator.metadata.is_generator,
            "should detect yield as generator"
        );
    }

    #[test]
    fn google_style_yields_parsed() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        let generator = find_by_name(&items, "generate_items");
        assert!(
            generator.metadata.doc_sections.yields.is_some(),
            "Yields: section should be parsed"
        );
    }

    #[test]
    fn private_function_visibility() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        let private = find_by_name(&items, "_private_helper");
        assert_eq!(private.visibility, Visibility::Private);
    }

    #[test]
    fn variadic_params_extracted() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        let transform = find_by_name(&items, "transform");
        let params_str = transform.metadata.parameters.join(", ");
        assert!(
            params_str.contains("args"),
            "params: {:?}",
            transform.metadata.parameters
        );
        assert!(
            params_str.contains("kwargs"),
            "params: {:?}",
            transform.metadata.parameters
        );
    }

    #[test]
    fn signature_no_body_leak() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        for item in &items {
            if !item.signature.is_empty() {
                assert!(
                    !item.signature.contains("\n    pass")
                        && !item.signature.contains("\n    return")
                        && !item.signature.contains("\"\"\""),
                    "signature for '{}' leaks body: {}",
                    item.name,
                    item.signature
                );
            }
        }
    }

    // ── New fixture coverage tests ─────────────────────────────────

    #[test]
    fn pydantic_model_detected() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        let settings = find_by_name(&items, "UserSettings");
        assert_eq!(settings.kind, SymbolKind::Class);
        assert!(settings.metadata.is_pydantic);
        assert!(
            settings.metadata.base_classes.contains(&"BaseModel".to_string()),
            "base_classes: {:?}",
            settings.metadata.base_classes
        );
    }

    #[test]
    fn enum_class_detected() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        let status = find_by_name(&items, "Status");
        assert_eq!(status.kind, SymbolKind::Class);
        assert!(status.metadata.is_enum);
        assert!(
            status.metadata.base_classes.contains(&"Enum".to_string()),
            "base_classes: {:?}",
            status.metadata.base_classes
        );
    }

    #[test]
    fn int_enum_detected() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        let priority = find_by_name(&items, "Priority");
        assert!(priority.metadata.is_enum);
    }

    #[test]
    fn exception_subclass_is_error_type() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        let proc_err = find_by_name(&items, "ProcessingError");
        assert!(
            proc_err.metadata.is_error_type,
            "ProcessingError(Exception) should be error type"
        );
        assert!(
            proc_err.metadata.base_classes.contains(&"Exception".to_string()),
            "base_classes: {:?}",
            proc_err.metadata.base_classes
        );
    }

    #[test]
    fn value_error_subclass_is_error_type() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        let val_err = find_by_name(&items, "ValidationError");
        assert!(
            val_err.metadata.is_error_type,
            "ValidationError(ValueError) should be error type"
        );
    }

    #[test]
    fn exception_docstring_extracted() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        let proc_err = find_by_name(&items, "ProcessingError");
        assert!(
            proc_err.doc_comment.contains("processing fails"),
            "doc: {:?}",
            proc_err.doc_comment
        );
    }

    #[test]
    fn module_level_const_extracted() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        let max_retries = find_by_name(&items, "MAX_RETRIES");
        assert_eq!(max_retries.kind, SymbolKind::Const);
        assert_eq!(max_retries.visibility, Visibility::Public);
        assert_eq!(
            max_retries.metadata.return_type.as_deref(),
            Some("int"),
        );
    }

    #[test]
    fn module_level_float_const_extracted() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        let timeout = find_by_name(&items, "DEFAULT_TIMEOUT");
        assert_eq!(timeout.kind, SymbolKind::Const);
        assert_eq!(
            timeout.metadata.return_type.as_deref(),
            Some("float"),
        );
    }

    #[test]
    fn private_module_const_visibility() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        let cache = find_by_name(&items, "_internal_cache");
        assert_eq!(cache.kind, SymbolKind::Const);
        assert_eq!(cache.visibility, Visibility::Private);
    }

    #[test]
    fn enum_docstring_extracted() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        let status = find_by_name(&items, "Status");
        assert!(
            status.doc_comment.contains("enumeration"),
            "doc: {:?}",
            status.doc_comment
        );
    }

    #[test]
    fn pydantic_docstring_extracted() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        let settings = find_by_name(&items, "UserSettings");
        assert!(
            settings.doc_comment.contains("user settings"),
            "doc: {:?}",
            settings.doc_comment
        );
    }
}
