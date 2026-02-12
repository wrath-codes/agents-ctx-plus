//! Python rich extractor — classes, functions, decorators, docstrings.
//!
//! Extracts from `function_definition`, `class_definition`,
//! `decorated_definition`, and module-level typed/untyped assignments.
//!
//! Walks only top-level children of the module to avoid duplicate extraction
//! of nested classes and methods (which are captured in class metadata).

use crate::types::{ParsedItem, PythonMetadataExt, SymbolKind, SymbolMetadata, Visibility};

#[path = "../python/doc.rs"]
mod doc;
#[path = "../python/processors/mod.rs"]
mod processors;
#[path = "../python/helpers.rs"]
mod pyhelpers;

use ast_grep_language::SupportLang;
use doc::extract_module_docstring;
use processors::{
    extract_dunder_all, process_class, process_class_member_items, process_decorated,
    process_function, process_module_assignment,
};

#[cfg(test)]
use doc::parse_numpy_style;
#[cfg(test)]
use pyhelpers::{decorator_matches, python_visibility};

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
                items.extend(process_decorated(&child));
            }
            "class_definition" => {
                if let Some(item) = process_class(&child, &[]) {
                    items.push(item);
                }
                items.extend(process_class_member_items(&child));
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
                item.metadata.mark_exported();
            }
        }
    }

    Ok(items)
}

#[cfg(test)]
#[path = "../python/tests/mod.rs"]
mod tests;
