use ast_grep_language::LanguageExt;

use super::*;
pub(super) use crate::types::{ParsedItem, SymbolKind, Visibility};

mod concern_edge_cases;
mod docs_signatures_lines;
mod members_and_visibility;
mod rails_dsl;
mod types_and_modules;

fn parse_and_extract(source: &str) -> Vec<ParsedItem> {
    let root = SupportLang::Ruby.ast_grep(source);
    extract(&root).expect("extraction should succeed")
}

fn find_by_name<'a>(items: &'a [ParsedItem], name: &str) -> &'a ParsedItem {
    items
        .iter()
        .find(|i| i.name == name)
        .unwrap_or_else(|| panic!("should find item named '{name}'"))
}

fn fixture_items() -> Vec<ParsedItem> {
    let source = include_str!("../../../../tests/fixtures/sample.rb");
    parse_and_extract(source)
}
