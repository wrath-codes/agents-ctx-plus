use ast_grep_language::LanguageExt;

use super::*;
pub(super) use crate::types::{ParsedItem, SymbolKind};

mod duplicate_and_empty;
mod metadata;
mod nested_paths;
mod path_edge_cases;
mod structure;
mod top_level_variants;

fn parse_and_extract(source: &str) -> Vec<ParsedItem> {
    let root = SupportLang::Json.ast_grep(source);
    extract(&root).expect("extraction should succeed")
}

fn fixture_items() -> Vec<ParsedItem> {
    let source = include_str!("../../../../tests/fixtures/sample.json");
    parse_and_extract(source)
}

fn find_by_name<'a>(items: &'a [ParsedItem], name: &str) -> &'a ParsedItem {
    items
        .iter()
        .find(|item| item.name == name)
        .unwrap_or_else(|| {
            let names: Vec<_> = items.iter().map(|item| item.name.as_str()).collect();
            panic!("should find item named '{name}', available: {names:?}")
        })
}

fn find_all_by_name<'a>(items: &'a [ParsedItem], name: &str) -> Vec<&'a ParsedItem> {
    items.iter().filter(|item| item.name == name).collect()
}
