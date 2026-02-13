use ast_grep_core::tree_sitter::LanguageExt;

use super::*;
use crate::types::{ParsedItem, SymbolKind};

mod blocks_and_tags;
mod structure;

fn parse_and_extract(source: &str) -> Vec<ParsedItem> {
    let root = crate::parser::SvelteLang.ast_grep(source);
    extract(&root).expect("svelte extraction should succeed")
}

fn fixture_items() -> Vec<ParsedItem> {
    let source = include_str!("../../../../tests/fixtures/sample.svelte");
    parse_and_extract(source)
}

fn find_by_name<'a>(items: &'a [ParsedItem], name: &str) -> &'a ParsedItem {
    items
        .iter()
        .find(|item| item.name == name)
        .unwrap_or_else(|| {
            let names: Vec<_> = items.iter().map(|i| i.name.as_str()).collect();
            panic!("missing item '{name}', available={names:?}")
        })
}

fn has_attr(item: &ParsedItem, attr: &str) -> bool {
    item.metadata.attributes.iter().any(|a| a == attr)
}
