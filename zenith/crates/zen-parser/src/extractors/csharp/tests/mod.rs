use ast_grep_language::LanguageExt;

use super::*;
pub(super) use crate::types::{SymbolKind, Visibility};

mod docs_signatures_lines;
mod events_indexers_operators;
mod members;
mod types_and_namespaces;
mod using_directives;
mod visibility_modifiers;

fn parse_and_extract(source: &str) -> Vec<ParsedItem> {
    let root = SupportLang::CSharp.ast_grep(source);
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
    let source = include_str!("../../../../tests/fixtures/sample.cs");
    parse_and_extract(source)
}
