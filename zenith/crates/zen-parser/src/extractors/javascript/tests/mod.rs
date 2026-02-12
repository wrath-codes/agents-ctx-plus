use ast_grep_language::LanguageExt;

use super::*;
pub(super) use crate::types::{SymbolKind, Visibility};

mod arrow_functions;
mod async_functions;
mod classes;
mod constants_vars;
mod exports;
mod generators;
mod regular_functions;
mod signatures_and_lines;

fn parse_and_extract(source: &str) -> Vec<ParsedItem> {
    let root = SupportLang::JavaScript.ast_grep(source);
    extract(&root).expect("extraction should succeed")
}

fn find_by_name<'a>(items: &'a [ParsedItem], name: &str) -> &'a ParsedItem {
    items
        .iter()
        .find(|i| i.name == name)
        .unwrap_or_else(|| panic!("should find item named '{name}'"))
}
