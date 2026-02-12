use ast_grep_language::LanguageExt;

use super::*;
pub(super) use crate::types::{SymbolKind, Visibility};

mod behaviours;
mod callbacks;
mod dedup;
mod delegates;
mod exceptions;
mod functions_misc;
mod functions_private;
mod functions_public;
mod guards;
mod macros;
mod misc;
mod modules;
mod protocol_impls;
mod protocols;
mod signatures_and_lines;
mod structs;
mod types;

fn parse_and_extract(source: &str) -> Vec<ParsedItem> {
    let root = SupportLang::Elixir.ast_grep(source);
    extract(&root).expect("extraction should succeed")
}

fn find_by_name<'a>(items: &'a [ParsedItem], name: &str) -> &'a ParsedItem {
    items.iter().find(|i| i.name == name).unwrap_or_else(|| {
        let names: Vec<_> = items
            .iter()
            .map(|i| format!("{}:{}", i.kind, i.name))
            .collect();
        panic!("no item named '{name}', available: {names:?}");
    })
}

fn find_all_by_name<'a>(items: &'a [ParsedItem], name: &str) -> Vec<&'a ParsedItem> {
    items.iter().filter(|i| i.name == name).collect()
}

// ── Module extraction ──────────────────────────────────────────

#[test]
fn elixir_does_not_emit_member_only_kinds() {
    let source = "defmodule Demo do\n  def run, do: :ok\nend\n";
    let items = parse_and_extract(source);
    assert!(items.iter().all(|item| {
        !matches!(
            item.kind,
            SymbolKind::Constructor
                | SymbolKind::Field
                | SymbolKind::Property
                | SymbolKind::Event
                | SymbolKind::Indexer
        )
    }));
}
