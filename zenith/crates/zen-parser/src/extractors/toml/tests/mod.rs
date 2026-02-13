use ast_grep_core::tree_sitter::LanguageExt;

use super::*;
use crate::types::{ParsedItem, SymbolKind};

mod conformance_corpus;
mod dependency_normalization;
mod inline_tables_and_arrays;
mod spec_edge_cases;
mod structure;

fn parse_and_extract(source: &str) -> Vec<ParsedItem> {
    let root = crate::parser::TomlLang.ast_grep(source);
    extract(&root).expect("extraction should succeed")
}

fn fixture_items() -> Vec<ParsedItem> {
    let source = include_str!("../../../../tests/fixtures/sample.toml");
    parse_and_extract(source)
}

fn edge_fixture_items() -> Vec<ParsedItem> {
    let source = include_str!("../../../../tests/fixtures/edge.toml");
    parse_and_extract(source)
}

fn conformance_items() -> Vec<ParsedItem> {
    let source = include_str!("../../../../tests/fixtures/conformance.toml");
    parse_and_extract(source)
}

fn dependency_fixture_items() -> Vec<ParsedItem> {
    let source = include_str!("../../../../tests/fixtures/dependencies.toml");
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
