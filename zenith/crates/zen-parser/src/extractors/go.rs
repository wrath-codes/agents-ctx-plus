//! Go rich extractor — `KindMatcher`-first strategy.
//!
//! Extracts functions, methods, type declarations (struct, interface,
//! type alias, function type), constants, and variables with Go-specific
//! metadata including receiver types, exported detection, and doc comments.

use ast_grep_core::Node;
use ast_grep_core::matcher::KindMatcher;
use ast_grep_core::ops::Any;
use ast_grep_language::SupportLang;

use super::helpers;
use crate::types::{ParsedItem, SymbolKind, SymbolMetadata, Visibility};

const GO_TOP_KINDS: &[&str] = &[
    "function_declaration",
    "method_declaration",
    "type_declaration",
    "const_declaration",
    "var_declaration",
];

/// Extract all API symbols from a Go source file.
///
/// # Errors
/// Returns `ParserError` if parsing fails.
pub fn extract<D: ast_grep_core::Doc<Lang = SupportLang>>(
    root: &ast_grep_core::AstGrep<D>,
) -> Result<Vec<ParsedItem>, crate::error::ParserError> {
    let mut items = Vec::new();
    let matchers: Vec<KindMatcher> = GO_TOP_KINDS
        .iter()
        .map(|k| KindMatcher::new(k, SupportLang::Go))
        .collect();
    let matcher = Any::new(matchers);

    for node in root.root().find_all(&matcher) {
        let kind = node.kind();
        match kind.as_ref() {
            "function_declaration" => {
                if let Some(item) = process_function(&node) {
                    items.push(item);
                }
            }
            "method_declaration" => {
                if let Some(item) = process_method(&node) {
                    items.push(item);
                }
            }
            "type_declaration" => {
                items.extend(process_type_declaration(&node));
            }
            "const_declaration" => {
                items.extend(process_const_declaration(&node));
            }
            "var_declaration" => {
                items.extend(process_var_declaration(&node));
            }
            _ => {}
        }
    }
    Ok(items)
}

// ── Exported detection ────────────────────────────────────────────

/// Go visibility: a name starting with an uppercase letter is exported.
fn go_visibility(name: &str) -> Visibility {
    if name.starts_with(char::is_uppercase) {
        Visibility::Public
    } else {
        Visibility::Private
    }
}

// ── Doc comment extraction ────────────────────────────────────────

/// Extract Go doc comments by walking backward through sibling `comment` nodes.
///
/// Go convention: doc comments are `//` comments immediately preceding
/// a declaration, with no blank lines in between.
fn extract_go_doc<D: ast_grep_core::Doc>(node: &Node<D>) -> String {
    let mut comments = Vec::new();
    let mut current = node.prev();
    while let Some(sibling) = current {
        if sibling.kind().as_ref() == "comment" {
            let text = sibling.text().to_string();
            if let Some(stripped) = text.strip_prefix("//") {
                comments.push(stripped.trim().to_string());
            }
        } else {
            break;
        }
        current = sibling.prev();
    }
    comments.reverse();
    comments.join("\n")
}

// ── function_declaration ──────────────────────────────────────────

fn process_function<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<ParsedItem> {
    let name = node
        .children()
        .find(|c| c.kind().as_ref() == "identifier")
        .map(|n| n.text().to_string())?;

    let doc = extract_go_doc(node);
    let return_type = extract_go_return_type(node);
    let parameters = extract_go_parameters(node);
    let type_params = extract_go_type_parameters(node);

    Some(ParsedItem {
        kind: SymbolKind::Function,
        name: name.clone(),
        signature: helpers::extract_signature(node),
        source: helpers::extract_source(node, 50),
        doc_comment: doc,
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: go_visibility(&name),
        metadata: SymbolMetadata {
            return_type,
            parameters,
            type_parameters: type_params,
            ..Default::default()
        },
    })
}

// ── method_declaration ────────────────────────────────────────────

fn process_method<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<ParsedItem> {
    // Method name is a field_identifier child
    let name = node
        .children()
        .find(|c| c.kind().as_ref() == "field_identifier")
        .map(|n| n.text().to_string())?;

    let doc = extract_go_doc(node);
    let return_type = extract_go_return_type(node);
    let parameters = extract_go_method_parameters(node);
    let receiver = extract_go_receiver(node);

    Some(ParsedItem {
        kind: SymbolKind::Method,
        name: name.clone(),
        signature: helpers::extract_signature(node),
        source: helpers::extract_source(node, 50),
        doc_comment: doc,
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: go_visibility(&name),
        metadata: SymbolMetadata {
            return_type,
            parameters,
            for_type: receiver,
            ..Default::default()
        },
    })
}

// ── type_declaration ──────────────────────────────────────────────

fn process_type_declaration<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<ParsedItem> {
    let mut items = Vec::new();
    let doc = extract_go_doc(node);

    for child in node.children() {
        let k = child.kind();
        match k.as_ref() {
            "type_spec" => {
                if let Some(item) = process_type_spec(&child, &doc) {
                    items.push(item);
                }
            }
            "type_alias" => {
                if let Some(item) = process_type_alias(&child, &doc) {
                    items.push(item);
                }
            }
            _ => {}
        }
    }
    items
}

fn process_type_spec<D: ast_grep_core::Doc>(node: &Node<D>, doc: &str) -> Option<ParsedItem> {
    let name = node
        .children()
        .find(|c| c.kind().as_ref() == "type_identifier")
        .map(|n| n.text().to_string())?;

    // Determine the symbol kind based on the type body
    let (symbol_kind, metadata) = classify_type_spec(node, &name);

    Some(ParsedItem {
        kind: symbol_kind,
        name: name.clone(),
        signature: helpers::extract_signature(node),
        source: helpers::extract_source(node, 50),
        doc_comment: doc.to_string(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: go_visibility(&name),
        metadata,
    })
}

fn classify_type_spec<D: ast_grep_core::Doc>(
    node: &Node<D>,
    name: &str,
) -> (SymbolKind, SymbolMetadata) {
    for child in node.children() {
        let k = child.kind();
        match k.as_ref() {
            "struct_type" => {
                let fields = extract_struct_fields(&child);
                let is_error = helpers::is_error_type_by_name(name);
                return (
                    SymbolKind::Struct,
                    SymbolMetadata {
                        fields,
                        is_error_type: is_error,
                        type_parameters: extract_go_type_params_from_spec(node),
                        ..Default::default()
                    },
                );
            }
            "interface_type" => {
                let methods = extract_interface_methods(&child);
                return (
                    SymbolKind::Interface,
                    SymbolMetadata {
                        methods,
                        ..Default::default()
                    },
                );
            }
            "function_type" => {
                return (SymbolKind::TypeAlias, SymbolMetadata::default());
            }
            _ => {}
        }
    }
    // Bare type (e.g., `type Direction int`)
    (SymbolKind::TypeAlias, SymbolMetadata::default())
}

fn process_type_alias<D: ast_grep_core::Doc>(node: &Node<D>, doc: &str) -> Option<ParsedItem> {
    let name = node
        .children()
        .find(|c| c.kind().as_ref() == "type_identifier")
        .map(|n| n.text().to_string())?;

    Some(ParsedItem {
        kind: SymbolKind::TypeAlias,
        name: name.clone(),
        signature: helpers::extract_signature(node),
        source: helpers::extract_source(node, 50),
        doc_comment: doc.to_string(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: go_visibility(&name),
        metadata: SymbolMetadata::default(),
    })
}

// ── const_declaration ─────────────────────────────────────────────

fn process_const_declaration<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<ParsedItem> {
    let mut items = Vec::new();
    let doc = extract_go_doc(node);

    for child in node.children() {
        if child.kind().as_ref() == "const_spec"
            && let Some(item) = process_const_spec(&child, &doc)
        {
            items.push(item);
        }
    }
    items
}

fn process_const_spec<D: ast_grep_core::Doc>(
    node: &Node<D>,
    parent_doc: &str,
) -> Option<ParsedItem> {
    let name = node
        .children()
        .find(|c| c.kind().as_ref() == "identifier")
        .map(|n| n.text().to_string())?;

    // Use the const_spec's own doc comment if available, else parent's
    let own_doc = extract_go_doc(node);
    let doc = if own_doc.is_empty() {
        parent_doc.to_string()
    } else {
        own_doc
    };

    Some(ParsedItem {
        kind: SymbolKind::Const,
        name: name.clone(),
        signature: node.text().to_string(),
        source: Some(node.text().to_string()),
        doc_comment: doc,
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: go_visibility(&name),
        metadata: SymbolMetadata::default(),
    })
}

// ── var_declaration ───────────────────────────────────────────────

fn process_var_declaration<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<ParsedItem> {
    let mut items = Vec::new();
    let doc = extract_go_doc(node);

    for child in node.children() {
        let k = child.kind();
        if k.as_ref() == "var_spec" {
            if let Some(item) = process_var_spec(&child, &doc) {
                items.push(item);
            }
        } else if k.as_ref() == "var_spec_list" {
            // var ( ... ) block
            for spec in child.children() {
                if spec.kind().as_ref() == "var_spec"
                    && let Some(item) = process_var_spec(&spec, &doc)
                {
                    items.push(item);
                }
            }
        }
    }
    items
}

fn process_var_spec<D: ast_grep_core::Doc>(node: &Node<D>, parent_doc: &str) -> Option<ParsedItem> {
    let name = node
        .children()
        .find(|c| c.kind().as_ref() == "identifier")
        .map(|n| n.text().to_string())?;

    let own_doc = extract_go_doc(node);
    let doc = if own_doc.is_empty() {
        parent_doc.to_string()
    } else {
        own_doc
    };

    Some(ParsedItem {
        kind: SymbolKind::Static,
        name: name.clone(),
        signature: node.text().to_string(),
        source: Some(node.text().to_string()),
        doc_comment: doc,
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: go_visibility(&name),
        metadata: SymbolMetadata::default(),
    })
}

// ── Go-specific helpers ───────────────────────────────────────────

/// Extract return type from a Go function/method.
///
/// Go return type can be:
/// - A single `type_identifier` (e.g., `error`, `string`)
/// - A single `pointer_type` (e.g., `*Config`)
/// - A single `slice_type` (e.g., `[]U`)
/// - A `parameter_list` for multiple returns (e.g., `([]string, error)`)
fn extract_go_return_type<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<String> {
    let children: Vec<_> = node.children().collect();

    // Find block (body) position — return type is between params and block
    let block_idx = children.iter().position(|c| c.kind().as_ref() == "block");

    let block_idx = block_idx?;

    // Look at the child immediately before the block
    if block_idx > 0 {
        let prev = &children[block_idx - 1];
        let k = prev.kind();
        let kr = k.as_ref();
        // The return type is any type node before the block
        // but NOT a parameter_list that is the actual params
        if kr == "type_identifier"
            || kr == "pointer_type"
            || kr == "slice_type"
            || kr == "qualified_type"
            || kr == "map_type"
            || kr == "channel_type"
            || kr == "array_type"
            || kr == "function_type"
        {
            return Some(prev.text().to_string());
        }
        // Multiple return values: the last parameter_list before block
        if kr == "parameter_list" {
            // For functions: param_list(params) param_list(returns) block
            // For methods: param_list(receiver) name param_list(params) param_list(returns) block
            // We need to check this is NOT the params list
            // Count parameter_lists before block
            let param_lists: Vec<_> = children[..block_idx]
                .iter()
                .filter(|c| c.kind().as_ref() == "parameter_list")
                .collect();

            let is_function = node.kind().as_ref() == "function_declaration";
            let min_param_lists = if is_function { 1 } else { 2 }; // methods have receiver + params

            if param_lists.len() > min_param_lists {
                return Some(prev.text().to_string());
            }
        }
    }
    None
}

/// Extract parameter declarations from a Go function (not method).
fn extract_go_parameters<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    // For function_declaration: the first parameter_list is the params
    let Some(params) = node
        .children()
        .find(|c| c.kind().as_ref() == "parameter_list")
    else {
        return Vec::new();
    };
    extract_param_decls(&params)
}

/// Extract parameter declarations from a Go method (skip receiver).
fn extract_go_method_parameters<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    // For method_declaration: first param_list is receiver, second is params
    let param_lists: Vec<_> = node
        .children()
        .filter(|c| c.kind().as_ref() == "parameter_list")
        .collect();

    if param_lists.len() >= 2 {
        extract_param_decls(&param_lists[1])
    } else {
        Vec::new()
    }
}

/// Extract the receiver type from a Go method.
fn extract_go_receiver<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<String> {
    // First parameter_list is the receiver
    let receiver_list = node
        .children()
        .find(|c| c.kind().as_ref() == "parameter_list")?;

    // The receiver is a parameter_declaration inside
    for child in receiver_list.children() {
        if child.kind().as_ref() == "parameter_declaration" {
            // Extract the type part — could be `*Config` or `Config`
            for sub in child.children() {
                let k = sub.kind();
                let kr = k.as_ref();
                if kr == "pointer_type" || kr == "type_identifier" {
                    return Some(sub.text().to_string());
                }
            }
        }
    }
    None
}

/// Extract parameter declarations from a `parameter_list` node.
///
/// Includes both regular and variadic (`...`) parameter declarations.
fn extract_param_decls<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    node.children()
        .filter(|c| {
            let k = c.kind();
            k.as_ref() == "parameter_declaration" || k.as_ref() == "variadic_parameter_declaration"
        })
        .map(|c| c.text().to_string())
        .collect()
}

/// Extract struct field names from a `struct_type` node.
///
/// Handles three cases:
/// - Named fields: `field_declaration` with `field_identifier` child (e.g., `Name string`)
/// - Embedded types: `field_declaration` with only `type_identifier` (e.g., `Config`)
/// - Embedded pointer types: `field_declaration` with `*` + `type_identifier` (e.g., `*Logger`)
fn extract_struct_fields<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    let mut fields = Vec::new();
    for child in node.children() {
        if child.kind().as_ref() == "field_declaration_list" {
            for field in child.children() {
                if field.kind().as_ref() == "field_declaration" {
                    if let Some(name) = field
                        .children()
                        .find(|c| c.kind().as_ref() == "field_identifier")
                    {
                        // Named field: `Port int`
                        fields.push(name.text().to_string());
                    } else if let Some(type_id) = field
                        .children()
                        .find(|c| c.kind().as_ref() == "type_identifier")
                    {
                        // Embedded type: `Config` or `*Logger` (type_identifier is the name)
                        fields.push(type_id.text().to_string());
                    }
                }
            }
        }
    }
    fields
}

/// Extract interface method names from an `interface_type` node.
fn extract_interface_methods<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    let mut methods = Vec::new();
    for child in node.children() {
        let k = child.kind();
        if (k.as_ref() == "method_spec" || k.as_ref() == "method_elem")
            && let Some(name) = child
                .children()
                .find(|c| c.kind().as_ref() == "field_identifier")
        {
            methods.push(name.text().to_string());
        }
    }
    methods
}

/// Extract type parameters from a `type_spec` node (Go generics).
fn extract_go_type_params_from_spec<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<String> {
    node.children()
        .find(|c| c.kind().as_ref() == "type_parameter_list")
        .map(|tp| tp.text().to_string())
}

/// Extract type parameters from a `function_declaration` node (Go generics).
fn extract_go_type_parameters<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<String> {
    node.children()
        .find(|c| c.kind().as_ref() == "type_parameter_list")
        .map(|tp| tp.text().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::SymbolKind;
    use ast_grep_language::LanguageExt;
    use pretty_assertions::assert_eq;

    fn parse_and_extract(source: &str) -> Vec<ParsedItem> {
        let root = SupportLang::Go.ast_grep(source);
        extract(&root).expect("extraction should succeed")
    }

    fn find_by_name<'a>(items: &'a [ParsedItem], name: &str) -> &'a ParsedItem {
        items
            .iter()
            .find(|i| i.name == name)
            .unwrap_or_else(|| panic!("should find item named '{name}'"))
    }

    // ── Exported function ──────────────────────────────────────────

    #[test]
    fn exported_function_extracted() {
        let source = include_str!("../../tests/fixtures/sample.go");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "ProcessItems");
        assert_eq!(f.kind, SymbolKind::Function);
        assert_eq!(f.visibility, Visibility::Public);
    }

    #[test]
    fn exported_function_has_doc_comment() {
        let source = include_str!("../../tests/fixtures/sample.go");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "ProcessItems");
        assert!(
            f.doc_comment.contains("processes a slice"),
            "doc: {:?}",
            f.doc_comment
        );
    }

    #[test]
    fn exported_function_parameters() {
        let source = include_str!("../../tests/fixtures/sample.go");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "ProcessItems");
        assert!(
            f.metadata.parameters.iter().any(|p| p.contains("items")),
            "params: {:?}",
            f.metadata.parameters
        );
    }

    // ── Private function ───────────────────────────────────────────

    #[test]
    fn private_function_extracted() {
        let source = include_str!("../../tests/fixtures/sample.go");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "privateHelper");
        assert_eq!(f.kind, SymbolKind::Function);
        assert_eq!(f.visibility, Visibility::Private);
    }

    // ── Struct ─────────────────────────────────────────────────────

    #[test]
    fn struct_extracted() {
        let source = include_str!("../../tests/fixtures/sample.go");
        let items = parse_and_extract(source);
        let s = find_by_name(&items, "Config");
        assert_eq!(s.kind, SymbolKind::Struct);
        assert_eq!(s.visibility, Visibility::Public);
    }

    #[test]
    fn struct_fields_extracted() {
        let source = include_str!("../../tests/fixtures/sample.go");
        let items = parse_and_extract(source);
        let s = find_by_name(&items, "Config");
        assert!(
            s.metadata.fields.contains(&"Name".to_string()),
            "fields: {:?}",
            s.metadata.fields
        );
        assert!(
            s.metadata.fields.contains(&"Count".to_string()),
            "fields: {:?}",
            s.metadata.fields
        );
        assert!(
            s.metadata.fields.contains(&"Enabled".to_string()),
            "fields: {:?}",
            s.metadata.fields
        );
    }

    #[test]
    fn struct_has_doc_comment() {
        let source = include_str!("../../tests/fixtures/sample.go");
        let items = parse_and_extract(source);
        let s = find_by_name(&items, "Config");
        assert!(
            s.doc_comment.contains("application configuration"),
            "doc: {:?}",
            s.doc_comment
        );
    }

    // ── Method (pointer receiver) ──────────────────────────────────

    #[test]
    fn pointer_receiver_method_extracted() {
        let source = include_str!("../../tests/fixtures/sample.go");
        let items = parse_and_extract(source);
        let m = find_by_name(&items, "Run");
        assert_eq!(m.kind, SymbolKind::Method);
        assert_eq!(m.visibility, Visibility::Public);
        assert_eq!(m.metadata.for_type.as_deref(), Some("*Config"));
    }

    #[test]
    fn pointer_receiver_method_has_doc() {
        let source = include_str!("../../tests/fixtures/sample.go");
        let items = parse_and_extract(source);
        let m = find_by_name(&items, "Run");
        assert!(
            m.doc_comment.contains("executes"),
            "doc: {:?}",
            m.doc_comment
        );
    }

    // ── Method (value receiver) ────────────────────────────────────

    #[test]
    fn value_receiver_method_extracted() {
        let source = include_str!("../../tests/fixtures/sample.go");
        let items = parse_and_extract(source);
        let m = find_by_name(&items, "String");
        assert_eq!(m.kind, SymbolKind::Method);
        assert_eq!(m.metadata.for_type.as_deref(), Some("Config"));
    }

    // ── Interface ──────────────────────────────────────────────────

    #[test]
    fn interface_extracted() {
        let source = include_str!("../../tests/fixtures/sample.go");
        let items = parse_and_extract(source);
        let i = find_by_name(&items, "Handler");
        assert_eq!(i.kind, SymbolKind::Interface);
        assert_eq!(i.visibility, Visibility::Public);
    }

    #[test]
    fn interface_methods_extracted() {
        let source = include_str!("../../tests/fixtures/sample.go");
        let items = parse_and_extract(source);
        let i = find_by_name(&items, "Handler");
        assert!(
            i.metadata.methods.contains(&"Handle".to_string()),
            "methods: {:?}",
            i.metadata.methods
        );
        assert!(
            i.metadata.methods.contains(&"Name".to_string()),
            "methods: {:?}",
            i.metadata.methods
        );
    }

    #[test]
    fn interface_has_doc_comment() {
        let source = include_str!("../../tests/fixtures/sample.go");
        let items = parse_and_extract(source);
        let i = find_by_name(&items, "Handler");
        assert!(
            i.doc_comment.contains("request handler"),
            "doc: {:?}",
            i.doc_comment
        );
    }

    // ── Embedded interface ─────────────────────────────────────────

    #[test]
    fn embedded_interface_extracted() {
        let source = include_str!("../../tests/fixtures/sample.go");
        let items = parse_and_extract(source);
        let i = find_by_name(&items, "Reader");
        assert_eq!(i.kind, SymbolKind::Interface);
    }

    // ── Type alias ─────────────────────────────────────────────────

    #[test]
    fn type_alias_extracted() {
        let source = include_str!("../../tests/fixtures/sample.go");
        let items = parse_and_extract(source);
        let t = find_by_name(&items, "MyInt");
        assert_eq!(t.kind, SymbolKind::TypeAlias);
        assert_eq!(t.visibility, Visibility::Public);
    }

    // ── Function type ──────────────────────────────────────────────

    #[test]
    fn function_type_extracted() {
        let source = include_str!("../../tests/fixtures/sample.go");
        let items = parse_and_extract(source);
        let t = find_by_name(&items, "Callback");
        assert_eq!(t.kind, SymbolKind::TypeAlias);
        assert_eq!(t.visibility, Visibility::Public);
    }

    // ── Constants ──────────────────────────────────────────────────

    #[test]
    fn single_const_extracted() {
        let source = include_str!("../../tests/fixtures/sample.go");
        let items = parse_and_extract(source);
        let c = find_by_name(&items, "MaxRetries");
        assert_eq!(c.kind, SymbolKind::Const);
        assert_eq!(c.visibility, Visibility::Public);
    }

    #[test]
    fn single_const_has_doc() {
        let source = include_str!("../../tests/fixtures/sample.go");
        let items = parse_and_extract(source);
        let c = find_by_name(&items, "MaxRetries");
        assert!(
            c.doc_comment.contains("maximum number"),
            "doc: {:?}",
            c.doc_comment
        );
    }

    #[test]
    fn iota_const_group_extracted() {
        let source = include_str!("../../tests/fixtures/sample.go");
        let items = parse_and_extract(source);
        let ok = find_by_name(&items, "StatusOK");
        assert_eq!(ok.kind, SymbolKind::Const);
        let _ = find_by_name(&items, "StatusError");
        let _ = find_by_name(&items, "StatusPending");
    }

    #[test]
    fn direction_consts_extracted() {
        let source = include_str!("../../tests/fixtures/sample.go");
        let items = parse_and_extract(source);
        let _ = find_by_name(&items, "North");
        let _ = find_by_name(&items, "South");
        let _ = find_by_name(&items, "East");
        let _ = find_by_name(&items, "West");
    }

    // ── Variables ──────────────────────────────────────────────────

    #[test]
    fn single_var_extracted() {
        let source = include_str!("../../tests/fixtures/sample.go");
        let items = parse_and_extract(source);
        let v = find_by_name(&items, "DefaultTimeout");
        assert_eq!(v.kind, SymbolKind::Static);
        assert_eq!(v.visibility, Visibility::Public);
    }

    #[test]
    fn single_var_has_doc() {
        let source = include_str!("../../tests/fixtures/sample.go");
        let items = parse_and_extract(source);
        let v = find_by_name(&items, "DefaultTimeout");
        assert!(
            v.doc_comment.contains("default timeout"),
            "doc: {:?}",
            v.doc_comment
        );
    }

    #[test]
    fn var_group_extracted() {
        let source = include_str!("../../tests/fixtures/sample.go");
        let items = parse_and_extract(source);
        let _ = find_by_name(&items, "GlobalCount");
        let _ = find_by_name(&items, "GlobalName");
    }

    // ── Error type ─────────────────────────────────────────────────

    #[test]
    fn error_struct_detected() {
        let source = include_str!("../../tests/fixtures/sample.go");
        let items = parse_and_extract(source);
        let e = find_by_name(&items, "AppError");
        assert_eq!(e.kind, SymbolKind::Struct);
        assert!(e.metadata.is_error_type);
    }

    #[test]
    fn error_method_extracted() {
        let source = include_str!("../../tests/fixtures/sample.go");
        let items = parse_and_extract(source);
        // Error() method on *AppError
        let methods: Vec<_> = items
            .iter()
            .filter(|i| i.name == "Error" && i.kind == SymbolKind::Method)
            .collect();
        assert!(!methods.is_empty());
        let m = methods[0];
        assert_eq!(m.metadata.for_type.as_deref(), Some("*AppError"));
    }

    // ── Constructor function ───────────────────────────────────────

    #[test]
    fn constructor_function_extracted() {
        let source = include_str!("../../tests/fixtures/sample.go");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "NewConfig");
        assert_eq!(f.kind, SymbolKind::Function);
        assert_eq!(f.visibility, Visibility::Public);
    }

    // ── Generic types ──────────────────────────────────────────────

    #[test]
    fn generic_struct_extracted() {
        let source = include_str!("../../tests/fixtures/sample.go");
        let items = parse_and_extract(source);
        let p = find_by_name(&items, "Pair");
        assert_eq!(p.kind, SymbolKind::Struct);
        assert!(
            p.metadata.type_parameters.is_some(),
            "should have type params"
        );
    }

    #[test]
    fn generic_function_extracted() {
        let source = include_str!("../../tests/fixtures/sample.go");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "Map");
        assert_eq!(f.kind, SymbolKind::Function);
        assert!(
            f.metadata.type_parameters.is_some(),
            "should have type params"
        );
    }

    // ── Bare type declaration ──────────────────────────────────────

    #[test]
    fn bare_type_declaration_extracted() {
        let source = include_str!("../../tests/fixtures/sample.go");
        let items = parse_and_extract(source);
        let d = find_by_name(&items, "Direction");
        assert_eq!(d.kind, SymbolKind::TypeAlias);
    }

    // ── init function ──────────────────────────────────────────────

    #[test]
    fn init_function_private() {
        let source = include_str!("../../tests/fixtures/sample.go");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "init");
        assert_eq!(f.kind, SymbolKind::Function);
        assert_eq!(f.visibility, Visibility::Private);
    }

    // ── Line numbers ───────────────────────────────────────────────

    #[test]
    fn line_numbers_are_one_based() {
        let source = include_str!("../../tests/fixtures/sample.go");
        let items = parse_and_extract(source);
        for item in &items {
            assert!(item.start_line >= 1, "{} starts at 0", item.name);
            assert!(
                item.end_line >= item.start_line,
                "{}: end {} < start {}",
                item.name,
                item.end_line,
                item.start_line
            );
        }
    }

    // ── Signature ──────────────────────────────────────────────────

    #[test]
    fn signature_no_body_leak() {
        let source = include_str!("../../tests/fixtures/sample.go");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "ProcessItems");
        assert!(!f.signature.contains("return"), "sig: {:?}", f.signature);
    }

    // ── Return type ────────────────────────────────────────────────

    #[test]
    fn single_return_type_extracted() {
        let source = include_str!("../../tests/fixtures/sample.go");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "privateHelper");
        assert_eq!(f.metadata.return_type.as_deref(), Some("int"));
    }

    #[test]
    fn pointer_return_type_extracted() {
        let source = include_str!("../../tests/fixtures/sample.go");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "NewConfig");
        assert_eq!(f.metadata.return_type.as_deref(), Some("*Config"));
    }

    // ── Variadic function ──────────────────────────────────────────

    #[test]
    fn variadic_function_extracted() {
        let source = include_str!("../../tests/fixtures/sample.go");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "Printf");
        assert_eq!(f.kind, SymbolKind::Function);
        assert_eq!(f.visibility, Visibility::Public);
    }

    #[test]
    fn variadic_param_included() {
        let source = include_str!("../../tests/fixtures/sample.go");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "Printf");
        assert!(
            f.metadata.parameters.iter().any(|p| p.contains("...")),
            "params should include variadic: {:?}",
            f.metadata.parameters
        );
        assert!(
            f.metadata.parameters.iter().any(|p| p.contains("format")),
            "params should include format: {:?}",
            f.metadata.parameters
        );
    }

    // ── Named returns ──────────────────────────────────────────────

    #[test]
    fn named_returns_extracted() {
        let source = include_str!("../../tests/fixtures/sample.go");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "Divide");
        assert_eq!(f.kind, SymbolKind::Function);
        assert!(
            f.metadata
                .return_type
                .as_deref()
                .is_some_and(|rt| rt.contains("float64")),
            "return type: {:?}",
            f.metadata.return_type
        );
    }

    // ── Embedded struct fields ─────────────────────────────────────

    #[test]
    fn embedded_type_in_struct_fields() {
        let source = include_str!("../../tests/fixtures/sample.go");
        let items = parse_and_extract(source);
        let s = find_by_name(&items, "Server");
        assert_eq!(s.kind, SymbolKind::Struct);
        assert!(
            s.metadata.fields.contains(&"Config".to_string()),
            "should contain embedded Config: {:?}",
            s.metadata.fields
        );
    }

    #[test]
    fn embedded_pointer_type_in_struct_fields() {
        let source = include_str!("../../tests/fixtures/sample.go");
        let items = parse_and_extract(source);
        let s = find_by_name(&items, "Server");
        assert!(
            s.metadata.fields.contains(&"Logger".to_string()),
            "should contain embedded *Logger: {:?}",
            s.metadata.fields
        );
    }

    #[test]
    fn named_fields_still_work_with_embedded() {
        let source = include_str!("../../tests/fixtures/sample.go");
        let items = parse_and_extract(source);
        let s = find_by_name(&items, "Server");
        assert!(
            s.metadata.fields.contains(&"Port".to_string()),
            "fields: {:?}",
            s.metadata.fields
        );
        assert!(
            s.metadata.fields.contains(&"Host".to_string()),
            "fields: {:?}",
            s.metadata.fields
        );
    }

    // ── Type constraint interface ──────────────────────────────────

    #[test]
    fn type_constraint_interface_extracted() {
        let source = include_str!("../../tests/fixtures/sample.go");
        let items = parse_and_extract(source);
        let i = find_by_name(&items, "Number");
        assert_eq!(i.kind, SymbolKind::Interface);
        assert_eq!(i.visibility, Visibility::Public);
    }

    // ── Typed constant ─────────────────────────────────────────────

    #[test]
    fn typed_const_extracted() {
        let source = include_str!("../../tests/fixtures/sample.go");
        let items = parse_and_extract(source);
        let c = find_by_name(&items, "Pi");
        assert_eq!(c.kind, SymbolKind::Const);
        assert_eq!(c.visibility, Visibility::Public);
    }

    // ── Map type definition ────────────────────────────────────────

    #[test]
    fn map_type_extracted() {
        let source = include_str!("../../tests/fixtures/sample.go");
        let items = parse_and_extract(source);
        let t = find_by_name(&items, "StringMap");
        assert_eq!(t.kind, SymbolKind::TypeAlias);
    }

    // ── Channel type definition ────────────────────────────────────

    #[test]
    fn channel_type_extracted() {
        let source = include_str!("../../tests/fixtures/sample.go");
        let items = parse_and_extract(source);
        let t = find_by_name(&items, "EventChan");
        assert_eq!(t.kind, SymbolKind::TypeAlias);
    }

    // ── Method with variadic params ────────────────────────────────

    #[test]
    fn method_variadic_params() {
        let source = include_str!("../../tests/fixtures/sample.go");
        let items = parse_and_extract(source);
        let m = find_by_name(&items, "Listen");
        assert_eq!(m.kind, SymbolKind::Method);
        assert!(
            m.metadata.parameters.iter().any(|p| p.contains("...")),
            "method params should include variadic: {:?}",
            m.metadata.parameters
        );
    }
}
