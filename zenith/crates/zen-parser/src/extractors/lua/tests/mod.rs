use ast_grep_language::LanguageExt;

use super::*;
pub(super) use crate::types::{ParsedItem, SymbolKind, Visibility};

mod docs_signatures_lines;
mod functions_and_members;
mod variables_and_attributes;

fn parse_and_extract(source: &str) -> Vec<ParsedItem> {
    let root = SupportLang::Lua.ast_grep(source);
    extract(&root).expect("extraction should succeed")
}

fn find_by_name<'a>(items: &'a [ParsedItem], name: &str) -> &'a ParsedItem {
    items
        .iter()
        .find(|item| item.name == name)
        .unwrap_or_else(|| panic!("should find item named '{name}'"))
}

fn fixture_items() -> Vec<ParsedItem> {
    let source = include_str!("../../../../tests/fixtures/sample.lua");
    parse_and_extract(source)
}
