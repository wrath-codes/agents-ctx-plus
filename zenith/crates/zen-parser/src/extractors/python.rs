//! Python rich extractor — classes, functions, decorators, docstrings.
//!
//! Extracts from `function_definition`, `class_definition`,
//! `decorated_definition`, and module-level typed/untyped assignments.
//!
//! Walks only top-level children of the module to avoid duplicate extraction
//! of nested classes and methods (which are captured in class metadata).

use ast_grep_core::Node;
use ast_grep_language::SupportLang;

use super::helpers;
use crate::types::{DocSections, ParsedItem, SymbolKind, SymbolMetadata, Visibility};

/// Extract all API symbols from a Python source file.
///
/// Walks only top-level children of the module root to prevent duplicate
/// extraction of methods/nested classes (which are captured as class metadata).
///
/// # Errors
/// Returns `ParserError` if parsing fails.
pub fn extract<D: ast_grep_core::Doc<Lang = SupportLang>>(
    root: &ast_grep_core::AstGrep<D>,
) -> Result<Vec<ParsedItem>, crate::error::ParserError> {
    let mut items = Vec::new();

    // Detect __all__ for export visibility
    let all_exports = extract_dunder_all(&root.root());

    // Module docstring (first expression_statement containing a string)
    if let Some(module_doc) = extract_module_docstring(&root.root()) {
        items.push(ParsedItem {
            kind: SymbolKind::Module,
            name: "<module>".to_string(),
            signature: String::new(),
            source: None,
            doc_comment: module_doc,
            start_line: 1,
            end_line: root.root().end_pos().line() as u32 + 1,
            visibility: Visibility::Public,
            metadata: SymbolMetadata::default(),
        });
    }

    // Walk only top-level children (no recursive find_all)
    for child in root.root().children() {
        let kind = child.kind();
        match kind.as_ref() {
            "decorated_definition" => {
                if let Some(item) = process_decorated(&child) {
                    items.push(item);
                }
            }
            "class_definition" => {
                if let Some(item) = process_class(&child, &[]) {
                    items.push(item);
                }
            }
            "function_definition" => {
                if let Some(item) = process_function(&child, &[]) {
                    items.push(item);
                }
            }
            "expression_statement" => {
                if let Some(item) = process_module_assignment(&child) {
                    items.push(item);
                }
            }
            _ => {}
        }
    }

    // Apply __all__ export visibility
    if let Some(ref exports) = all_exports {
        for item in &mut items {
            if exports.contains(&item.name) {
                item.visibility = Visibility::Export;
                item.metadata.is_exported = true;
            }
        }
    }

    Ok(items)
}

/// Extract `__all__` list from module top level.
fn extract_dunder_all<D: ast_grep_core::Doc>(root: &Node<D>) -> Option<Vec<String>> {
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

/// Extract the module-level docstring (first string expression in module body).
fn extract_module_docstring<D: ast_grep_core::Doc>(root: &Node<D>) -> Option<String> {
    let first = root.children().next()?;
    if first.kind().as_ref() != "expression_statement" {
        return None;
    }
    let string_node = first.children().find(|c| c.kind().as_ref() == "string")?;
    let text = string_node.text().to_string();
    let doc = text
        .trim_start_matches("\"\"\"")
        .trim_end_matches("\"\"\"")
        .trim_start_matches("'''")
        .trim_end_matches("'''")
        .trim();
    if doc.is_empty() {
        return None;
    }
    Some(doc.to_string())
}

// ── decorated_definition ───────────────────────────────────────────

fn process_decorated<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<ParsedItem> {
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

    Some(ParsedItem {
        kind: symbol_kind,
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
            variants,
            doc_sections,
            is_error_type,
            generics,
            // NamedTuple/TypedDict not in SymbolMetadata yet but tracked via base_classes
            // and is_dataclass covers dataclass; generic tracked via generics field
            ..Default::default()
        },
    })
    .map(|mut item| {
        // Store extra detection in attributes for features without dedicated fields
        if is_namedtuple {
            item.metadata.attributes.push("namedtuple".to_string());
        }
        if is_typed_dict {
            item.metadata.attributes.push("typed_dict".to_string());
        }
        if is_generic {
            item.metadata.attributes.push("generic".to_string());
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

/// Check if any decorator matches the given suffixes (handles dotted paths).
fn decorator_matches_any(decorators: &[String], suffixes: &[&str]) -> bool {
    decorators.iter().any(|d| {
        let base = d.split('(').next().unwrap_or(d);
        suffixes
            .iter()
            .any(|s| base == *s || base.ends_with(&format!(".{s}")))
    })
}

fn decorator_matches(decorators: &[String], suffix: &str) -> bool {
    decorator_matches_any(decorators, &[suffix])
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

// ── function_definition ────────────────────────────────────────────

fn process_function<D: ast_grep_core::Doc>(
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
            attributes,
            is_property,
            is_classmethod,
            is_staticmethod,
            is_generator,
            returns_result,
            doc_sections,
            ..Default::default()
        },
    })
}

/// Python visibility rules:
/// - `__dunder__` (starts AND ends with `__`) → Public
/// - `__name_mangled` (starts with `__`, no trailing `__`) → Private  
/// - `_protected` (starts with single `_`) → Protected
/// - everything else → Public
fn python_visibility(name: &str) -> Visibility {
    if name.starts_with("__") && name.ends_with("__") && name.len() > 4 {
        Visibility::Public
    } else if name.starts_with("__") {
        Visibility::Private
    } else if name.starts_with('_') {
        Visibility::Protected
    } else {
        Visibility::Public
    }
}

// ── module-level assignments ───────────────────────────────────────

fn process_module_assignment<D: ast_grep_core::Doc>(expr_stmt: &Node<D>) -> Option<ParsedItem> {
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

/// Detect if a function is a generator by looking for `yield` statements,
/// but only in the function's own body — not in nested function definitions.
fn detect_generator<D: ast_grep_core::Doc>(node: &Node<D>) -> bool {
    let Some(body) = node.field("body") else {
        return false;
    };
    has_yield_shallow(&body)
}

/// Shallow recursive search for yield, stopping at nested function boundaries.
fn has_yield_shallow<D: ast_grep_core::Doc>(node: &Node<D>) -> bool {
    for child in node.children() {
        let k = child.kind();
        let kind = k.as_ref();
        // Stop at nested function definitions
        if kind == "function_definition" || kind == "decorated_definition" {
            continue;
        }
        if kind == "yield" || kind == "yield_expression" {
            return true;
        }
        if has_yield_shallow(&child) {
            return true;
        }
    }
    false
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

    // Try Google-style first, then Sphinx-style, then NumPy-style
    let mut sections = parse_google_style(doc);
    if sections_empty(&sections) {
        sections = parse_sphinx_style(doc);
    }
    if sections_empty(&sections) {
        sections = parse_numpy_style(doc);
    }
    sections
}

fn sections_empty(s: &DocSections) -> bool {
    s.args.is_empty() && s.returns.is_none() && s.raises.is_empty() && s.yields.is_none()
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

fn flush_google_section(sections: &mut DocSections, heading: Option<&str>, content: &str) {
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

/// Parse NumPy-style docstrings.
///
/// `NumPy` uses section headers underlined with dashes:
/// ```text
/// Parameters
/// ----------
/// x : float
///     Description.
/// ```
fn parse_numpy_style(doc: &str) -> DocSections {
    let mut sections = DocSections::default();
    let lines: Vec<&str> = doc.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let trimmed = lines[i].trim();

        // Check if next line is a dash underline (NumPy section marker)
        let is_section_header = i + 1 < lines.len()
            && !trimmed.is_empty()
            && lines[i + 1].trim().chars().all(|c| c == '-')
            && !lines[i + 1].trim().is_empty();

        if is_section_header {
            let section_name = trimmed;
            i += 2; // Skip header + underline

            // Collect section content until next section or end
            let mut content = String::new();
            while i < lines.len() {
                let line = lines[i].trim();
                // Check if this is another section header
                if i + 1 < lines.len()
                    && !line.is_empty()
                    && lines[i + 1].trim().chars().all(|c| c == '-')
                    && !lines[i + 1].trim().is_empty()
                {
                    break;
                }
                if !content.is_empty() {
                    content.push('\n');
                }
                content.push_str(line);
                i += 1;
            }

            let content = content.trim().to_string();
            match section_name {
                "Parameters" | "Params" | "Args" => {
                    for (name, desc) in parse_numpy_param_block(&content) {
                        sections.args.insert(name, desc);
                    }
                }
                "Returns" | "Return" => sections.returns = Some(content),
                "Raises" | "Exceptions" => {
                    for (name, desc) in parse_numpy_param_block(&content) {
                        sections.raises.insert(name, desc);
                    }
                }
                "Yields" | "Yield" => sections.yields = Some(content),
                "Examples" | "Example" => sections.examples = Some(content),
                "Notes" | "Note" => sections.notes = Some(content),
                _ => {}
            }
        } else {
            i += 1;
        }
    }
    sections
}

/// Parse a NumPy-style parameter block (entries like `name : type\n    desc`).
fn parse_numpy_param_block(block: &str) -> Vec<(String, String)> {
    let mut params = Vec::new();
    let mut current_name = String::new();
    let mut current_desc = String::new();

    for line in block.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        // NumPy param lines: "name : type" or just "name"
        if !line.starts_with(' ') && !line.starts_with('\t') {
            if !current_name.is_empty() {
                params.push((current_name.clone(), current_desc.trim().to_string()));
            }
            current_name = trimmed
                .split(':')
                .next()
                .unwrap_or(trimmed)
                .trim()
                .to_string();
            current_desc = String::new();
        } else {
            if !current_desc.is_empty() {
                current_desc.push(' ');
            }
            current_desc.push_str(trimmed);
        }
    }
    if !current_name.is_empty() {
        params.push((current_name, current_desc.trim().to_string()));
    }
    params
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
            validator
                .metadata
                .base_classes
                .contains(&"Protocol".to_string()),
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
        assert_eq!(private.visibility, Visibility::Protected);
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
            settings
                .metadata
                .base_classes
                .contains(&"BaseModel".to_string()),
            "base_classes: {:?}",
            settings.metadata.base_classes
        );
    }

    #[test]
    fn enum_class_detected() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        let status = find_by_name(&items, "Status");
        assert_eq!(status.kind, SymbolKind::Enum);
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
            proc_err
                .metadata
                .base_classes
                .contains(&"Exception".to_string()),
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
        // MAX_RETRIES is in __all__, so it gets Export visibility
        assert_eq!(max_retries.visibility, Visibility::Export);
        assert_eq!(max_retries.metadata.return_type.as_deref(), Some("int"),);
    }

    #[test]
    fn module_level_float_const_extracted() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        let timeout = find_by_name(&items, "DEFAULT_TIMEOUT");
        assert_eq!(timeout.kind, SymbolKind::Const);
        assert_eq!(timeout.metadata.return_type.as_deref(), Some("float"),);
    }

    #[test]
    fn private_module_const_visibility() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        let cache = find_by_name(&items, "_internal_cache");
        assert_eq!(cache.kind, SymbolKind::Const);
        assert_eq!(cache.visibility, Visibility::Protected);
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

    // ── Critical bug fix tests ─────────────────────────────────────

    #[test]
    fn no_duplicate_decorated_items() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        // Config has @dataclass — should appear exactly once
        let config_count = items.iter().filter(|i| i.name == "Config").count();
        assert_eq!(config_count, 1, "decorated class should not be duplicated");
    }

    #[test]
    fn no_methods_as_top_level_functions() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        // "process" is a method of BaseProcessor, not a top-level function
        let process_items: Vec<_> = items.iter().filter(|i| i.name == "process").collect();
        assert!(
            process_items.is_empty(),
            "methods should not be extracted as top-level items: {:?}",
            process_items.iter().map(|i| &i.name).collect::<Vec<_>>()
        );
    }

    #[test]
    fn no_nested_functions_as_top_level() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        // inner_function is nested inside outer_function
        let inner = items.iter().find(|i| i.name == "inner_function");
        assert!(
            inner.is_none(),
            "nested functions should not be top-level items"
        );
    }

    #[test]
    fn no_nested_classes_as_top_level() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        // Inner is nested inside Outer
        let inner = items.iter().find(|i| i.name == "Inner");
        assert!(
            inner.is_none(),
            "nested classes should not be top-level items"
        );
    }

    // ── Visibility tests ───────────────────────────────────────────

    #[test]
    fn dunder_methods_are_public() {
        // __init__, __len__, etc. should be Public, not Private
        let vis = super::python_visibility("__init__");
        assert_eq!(vis, Visibility::Public);
        let vis = super::python_visibility("__len__");
        assert_eq!(vis, Visibility::Public);
    }

    #[test]
    fn name_mangled_is_private() {
        let vis = super::python_visibility("__private_method");
        assert_eq!(vis, Visibility::Private);
    }

    #[test]
    fn single_underscore_is_protected() {
        let vis = super::python_visibility("_protected_method");
        assert_eq!(vis, Visibility::Protected);
    }

    #[test]
    fn regular_name_is_public() {
        let vis = super::python_visibility("public_method");
        assert_eq!(vis, Visibility::Public);
    }

    #[test]
    fn all_exports_applied() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        // Items in __all__ should have Export visibility
        let base = find_by_name(&items, "BaseProcessor");
        assert_eq!(base.visibility, Visibility::Export);
        let config = find_by_name(&items, "Config");
        assert_eq!(config.visibility, Visibility::Export);
        let fetch = find_by_name(&items, "fetch_data");
        assert_eq!(fetch.visibility, Visibility::Export);
        let status = find_by_name(&items, "Status");
        assert_eq!(status.visibility, Visibility::Export);
    }

    #[test]
    fn non_exported_stays_public() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        // Color is not in __all__, should stay Public
        let color = find_by_name(&items, "Color");
        assert_eq!(color.visibility, Visibility::Public);
    }

    // ── Module-level features ──────────────────────────────────────

    #[test]
    fn module_docstring_extracted() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        let module = find_by_name(&items, "<module>");
        assert_eq!(module.kind, SymbolKind::Module);
        assert!(
            module.doc_comment.contains("Module docstring"),
            "doc: {:?}",
            module.doc_comment
        );
    }

    #[test]
    fn dunder_version_extracted() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        let version = find_by_name(&items, "__version__");
        assert_eq!(version.kind, SymbolKind::Const);
        assert_eq!(version.visibility, Visibility::Public);
    }

    #[test]
    fn untyped_const_extracted() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        let version = find_by_name(&items, "VERSION");
        assert_eq!(version.kind, SymbolKind::Const);
        let debug = find_by_name(&items, "DEBUG");
        assert_eq!(debug.kind, SymbolKind::Const);
    }

    #[test]
    fn type_alias_detected() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        let json = find_by_name(&items, "JsonValue");
        assert_eq!(json.kind, SymbolKind::TypeAlias);
    }

    // ── Class feature tests ────────────────────────────────────────

    #[test]
    fn protocol_is_interface_kind() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        let validator = find_by_name(&items, "Validator");
        assert_eq!(validator.kind, SymbolKind::Interface);
        assert!(validator.metadata.is_protocol);
    }

    #[test]
    fn enum_is_enum_kind() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        let status = find_by_name(&items, "Status");
        assert_eq!(status.kind, SymbolKind::Enum);
        let priority = find_by_name(&items, "Priority");
        assert_eq!(priority.kind, SymbolKind::Enum);
    }

    #[test]
    fn strenum_detected() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        let http = find_by_name(&items, "HttpMethod");
        assert_eq!(http.kind, SymbolKind::Enum);
        assert!(http.metadata.is_enum);
    }

    #[test]
    fn namedtuple_detected() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        let point = find_by_name(&items, "Point");
        assert_eq!(point.kind, SymbolKind::Class);
        assert!(
            point
                .metadata
                .attributes
                .contains(&"namedtuple".to_string()),
            "attrs: {:?}",
            point.metadata.attributes
        );
        assert!(
            point
                .metadata
                .base_classes
                .contains(&"NamedTuple".to_string()),
            "base: {:?}",
            point.metadata.base_classes
        );
    }

    #[test]
    fn typed_dict_detected() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        let profile = find_by_name(&items, "UserProfile");
        assert_eq!(profile.kind, SymbolKind::Class);
        assert!(
            profile
                .metadata
                .attributes
                .contains(&"typed_dict".to_string()),
            "attrs: {:?}",
            profile.metadata.attributes
        );
    }

    #[test]
    fn metaclass_filtered_from_base_classes() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        let handler = find_by_name(&items, "AbstractHandler");
        assert!(
            !handler
                .metadata
                .base_classes
                .iter()
                .any(|b| b.contains("metaclass")),
            "base_classes should not contain metaclass: {:?}",
            handler.metadata.base_classes
        );
    }

    #[test]
    fn multiple_inheritance_base_classes() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        let multi = find_by_name(&items, "MultiBase");
        assert!(
            multi
                .metadata
                .base_classes
                .contains(&"BaseProcessor".to_string()),
            "base: {:?}",
            multi.metadata.base_classes
        );
        assert!(
            multi
                .metadata
                .base_classes
                .contains(&"AbstractHandler".to_string()),
            "base: {:?}",
            multi.metadata.base_classes
        );
    }

    #[test]
    fn generic_class_detected() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        let stack = find_by_name(&items, "Stack");
        assert!(
            stack.metadata.attributes.contains(&"generic".to_string()),
            "attrs: {:?}",
            stack.metadata.attributes
        );
        assert!(
            stack.metadata.generics.is_some(),
            "should have generics: {:?}",
            stack.metadata.generics
        );
    }

    #[test]
    fn instance_vars_from_init() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        let slotted = find_by_name(&items, "SlottedClass");
        // __init__ sets self.x, self.y, self.name
        assert!(
            slotted.metadata.fields.contains(&"x".to_string()),
            "fields: {:?}",
            slotted.metadata.fields
        );
        assert!(
            slotted.metadata.fields.contains(&"y".to_string()),
            "fields: {:?}",
            slotted.metadata.fields
        );
    }

    #[test]
    fn enum_variants_extracted() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        let status = find_by_name(&items, "Status");
        assert!(
            !status.metadata.variants.is_empty(),
            "enum should have variants"
        );
        assert!(
            status.metadata.variants.contains(&"ACTIVE".to_string()),
            "variants: {:?}",
            status.metadata.variants
        );
    }

    #[test]
    fn enum_with_methods() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        let direction = find_by_name(&items, "Direction");
        assert_eq!(direction.kind, SymbolKind::Enum);
        assert!(
            direction.metadata.methods.contains(&"opposite".to_string()),
            "methods: {:?}",
            direction.metadata.methods
        );
    }

    #[test]
    fn dataclass_with_decorators() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        let immutable = find_by_name(&items, "ImmutableConfig");
        assert!(immutable.metadata.is_dataclass);
        assert!(!immutable.metadata.fields.is_empty(), "should have fields");
    }

    #[test]
    fn outer_class_has_methods() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        let outer = find_by_name(&items, "Outer");
        assert!(
            outer.metadata.methods.contains(&"outer_method".to_string()),
            "methods: {:?}",
            outer.metadata.methods
        );
    }

    #[test]
    fn container_dunder_methods() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        let container = find_by_name(&items, "Container");
        assert!(container.metadata.methods.contains(&"__init__".to_string()));
        assert!(container.metadata.methods.contains(&"__len__".to_string()));
        assert!(
            container
                .metadata
                .methods
                .contains(&"__getitem__".to_string())
        );
        assert!(container.metadata.methods.contains(&"__iter__".to_string()));
        assert!(container.metadata.methods.contains(&"__repr__".to_string()));
        assert!(
            container
                .metadata
                .methods
                .contains(&"__enter__".to_string())
        );
        assert!(container.metadata.methods.contains(&"__exit__".to_string()));
    }

    #[test]
    fn container_instance_vars() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        let container = find_by_name(&items, "Container");
        assert!(
            container.metadata.fields.contains(&"items".to_string()),
            "fields: {:?}",
            container.metadata.fields
        );
    }

    // ── Function feature tests ─────────────────────────────────────

    #[test]
    fn overload_detected() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        let overloads: Vec<_> = items.iter().filter(|i| i.name == "parse_input").collect();
        // Should have the overloaded versions
        assert!(
            overloads.len() >= 2,
            "should have overloaded parse_input: found {}",
            overloads.len()
        );
        let has_overload_attr = overloads
            .iter()
            .any(|i| i.metadata.attributes.contains(&"overload".to_string()));
        assert!(
            has_overload_attr,
            "at least one should have overload attribute"
        );
    }

    #[test]
    fn context_manager_detected() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        let managed = find_by_name(&items, "managed_resource");
        assert!(
            managed
                .metadata
                .attributes
                .contains(&"contextmanager".to_string()),
            "attrs: {:?}",
            managed.metadata.attributes
        );
    }

    #[test]
    fn multiple_decorators_extracted() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        let multi = find_by_name(&items, "multi_decorated");
        assert!(
            multi.metadata.decorators.len() >= 2,
            "decorators: {:?}",
            multi.metadata.decorators
        );
    }

    #[test]
    fn async_generator() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        let ag = find_by_name(&items, "async_generate");
        assert!(ag.metadata.is_async, "should be async");
        assert!(ag.metadata.is_generator, "should be generator");
    }

    #[test]
    fn generator_not_false_positive_on_nested() {
        // outer_function contains inner_function but no yield itself
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        let outer = find_by_name(&items, "outer_function");
        assert!(
            !outer.metadata.is_generator,
            "outer_function should not be a generator (yield is only in nested)"
        );
    }

    #[test]
    fn mixed_params_extracted() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        let mixed = find_by_name(&items, "mixed_params");
        assert!(
            !mixed.metadata.parameters.is_empty(),
            "should have parameters"
        );
    }

    // ── Docstring parsing tests ────────────────────────────────────

    #[test]
    fn numpy_style_args_parsed() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        let numpy = find_by_name(&items, "numpy_documented");
        assert!(
            !numpy.metadata.doc_sections.args.is_empty(),
            "NumPy args should be parsed: {:?}",
            numpy.metadata.doc_sections.args
        );
        assert!(
            numpy.metadata.doc_sections.args.contains_key("x"),
            "should have 'x' param: {:?}",
            numpy.metadata.doc_sections.args
        );
    }

    #[test]
    fn numpy_style_returns_parsed() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        let numpy = find_by_name(&items, "numpy_documented");
        assert!(
            numpy.metadata.doc_sections.returns.is_some(),
            "NumPy Returns should be parsed"
        );
    }

    #[test]
    fn numpy_style_raises_parsed() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        let numpy = find_by_name(&items, "numpy_documented");
        assert!(
            !numpy.metadata.doc_sections.raises.is_empty(),
            "NumPy Raises should be parsed: {:?}",
            numpy.metadata.doc_sections.raises
        );
    }

    #[test]
    fn numpy_style_examples_parsed() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        let numpy = find_by_name(&items, "numpy_documented");
        assert!(
            numpy.metadata.doc_sections.examples.is_some(),
            "NumPy Examples should be parsed"
        );
    }

    #[test]
    fn numpy_style_notes_parsed() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        let numpy = find_by_name(&items, "numpy_documented");
        assert!(
            numpy.metadata.doc_sections.notes.is_some(),
            "NumPy Notes should be parsed"
        );
    }

    // ── Decorator semantics tests ──────────────────────────────────

    #[test]
    fn decorator_matches_dotted_path() {
        let decorators = vec!["dataclasses.dataclass".to_string()];
        assert!(super::decorator_matches(&decorators, "dataclass"));
    }

    #[test]
    fn decorator_matches_with_args() {
        let decorators = vec!["dataclass(frozen=True)".to_string()];
        assert!(super::decorator_matches(&decorators, "dataclass"));
    }

    #[test]
    fn decorator_matches_exact() {
        let decorators = vec!["staticmethod".to_string()];
        assert!(super::decorator_matches(&decorators, "staticmethod"));
    }

    // ── Property tests ─────────────────────────────────────────────

    #[test]
    fn cached_property_detected() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        let cached = find_by_name(&items, "CachedExample");
        assert!(
            cached.metadata.methods.contains(&"expensive".to_string()),
            "methods: {:?}",
            cached.metadata.methods
        );
    }

    #[test]
    fn property_example_methods() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        let prop = find_by_name(&items, "PropertyExample");
        assert!(
            prop.metadata.methods.contains(&"__init__".to_string()),
            "methods: {:?}",
            prop.metadata.methods
        );
        // value appears as property getter (and setter/deleter with same name)
        assert!(
            prop.metadata.methods.contains(&"value".to_string()),
            "methods: {:?}",
            prop.metadata.methods
        );
    }

    // ── Async resource class ───────────────────────────────────────

    #[test]
    fn async_resource_methods() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        let resource = find_by_name(&items, "AsyncResource");
        assert!(
            resource
                .metadata
                .methods
                .contains(&"__aenter__".to_string())
        );
        assert!(resource.metadata.methods.contains(&"__aexit__".to_string()));
        assert!(resource.metadata.methods.contains(&"fetch".to_string()));
    }

    // ── Visibility example class ───────────────────────────────────

    #[test]
    fn visibility_example_class() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        let vis = find_by_name(&items, "VisibilityExample");
        assert!(
            vis.metadata.methods.contains(&"public_method".to_string()),
            "methods: {:?}",
            vis.metadata.methods
        );
        assert!(
            vis.metadata
                .methods
                .contains(&"_protected_method".to_string()),
            "methods: {:?}",
            vis.metadata.methods
        );
        assert!(
            vis.metadata
                .methods
                .contains(&"__private_method".to_string()),
            "methods: {:?}",
            vis.metadata.methods
        );
        assert!(
            vis.metadata
                .methods
                .contains(&"__dunder_method__".to_string()),
            "methods: {:?}",
            vis.metadata.methods
        );
    }

    // ── Unit tests for python_visibility ────────────────────────────

    #[test]
    fn visibility_all_cases() {
        assert_eq!(super::python_visibility("foo"), Visibility::Public);
        assert_eq!(super::python_visibility("_foo"), Visibility::Protected);
        assert_eq!(super::python_visibility("__foo"), Visibility::Private);
        assert_eq!(super::python_visibility("__foo__"), Visibility::Public);
        assert_eq!(super::python_visibility("__init__"), Visibility::Public);
        assert_eq!(super::python_visibility("__version__"), Visibility::Public);
    }

    // ── Unit tests for parse_numpy_style ────────────────────────────

    #[test]
    fn numpy_parse_basic() {
        let doc = "Summary.\n\nParameters\n----------\nx : float\n    The x.\ny : int\n    The y.\n\nReturns\n-------\nfloat\n    The result.";
        let sections = super::parse_numpy_style(doc);
        assert!(sections.args.contains_key("x"), "args: {:?}", sections.args);
        assert!(sections.args.contains_key("y"), "args: {:?}", sections.args);
        assert!(sections.returns.is_some());
    }

    // ── Error type tests ───────────────────────────────────────────

    #[test]
    fn error_type_by_exception_name() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        let proc_err = find_by_name(&items, "ProcessingError");
        assert!(proc_err.metadata.is_error_type);
        let val_err = find_by_name(&items, "ValidationError");
        assert!(val_err.metadata.is_error_type);
    }

    // ── Signature tests ────────────────────────────────────────────

    #[test]
    fn async_function_signature_prefix() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        let fetch = find_by_name(&items, "fetch_data");
        assert!(
            fetch.signature.starts_with("async def"),
            "sig: {:?}",
            fetch.signature
        );
    }

    #[test]
    fn class_signature_format() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        let base = find_by_name(&items, "BaseProcessor");
        assert!(
            base.signature.starts_with("class BaseProcessor"),
            "sig: {:?}",
            base.signature
        );
    }

    // ── Total item count sanity ────────────────────────────────────

    #[test]
    fn reasonable_item_count() {
        let source = include_str!("../../tests/fixtures/sample.py");
        let items = parse_and_extract(source);
        // Should have a reasonable number of items (not inflated by duplicates)
        // Module + classes + functions + constants
        let names: Vec<&str> = items.iter().map(|i| i.name.as_str()).collect();
        assert!(
            items.len() >= 25,
            "should have at least 25 items, got {}: {:?}",
            items.len(),
            names
        );
        assert!(
            items.len() <= 60,
            "should not exceed 60 items (no duplicates), got {}: {:?}",
            items.len(),
            names
        );
    }
}
