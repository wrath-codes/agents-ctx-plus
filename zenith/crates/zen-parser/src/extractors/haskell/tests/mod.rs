use ast_grep_language::LanguageExt;

use super::*;
pub(super) use crate::types::{ParsedItem, SymbolKind};

mod foreign_symbols;
mod functions_and_signatures;
mod lines_and_signatures;
mod modules_and_imports;
mod types_and_classes;

fn parse_and_extract(source: &str) -> Vec<ParsedItem> {
    let root = SupportLang::Haskell.ast_grep(source);
    extract(&root).expect("extraction should succeed")
}

fn find_by_name<'a>(items: &'a [ParsedItem], name: &str) -> &'a ParsedItem {
    items
        .iter()
        .find(|i| i.name == name)
        .unwrap_or_else(|| panic!("should find item named '{name}'"))
}

fn find_all_by_kind(items: &[ParsedItem], kind: SymbolKind) -> Vec<&ParsedItem> {
    items.iter().filter(|i| i.kind == kind).collect()
}

fn fixture_items() -> Vec<ParsedItem> {
    let source = include_str!("../../../../tests/fixtures/sample.hs");
    parse_and_extract(source)
}
