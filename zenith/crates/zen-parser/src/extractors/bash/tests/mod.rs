use ast_grep_language::{LanguageExt, SupportLang};

use super::*;
pub(super) use crate::types::{SymbolKind, Visibility};

mod commands;
mod control_flow;
mod declarations;
mod functions;
mod inline;
mod lists;
mod source_lines;
mod structures;

fn parse_and_extract(source: &str) -> Vec<ParsedItem> {
    let root = SupportLang::Bash.ast_grep(source);
    extract(&root, source).expect("extraction should succeed")
}

fn find_by_name<'a>(items: &'a [ParsedItem], name: &str) -> &'a ParsedItem {
    items.iter().find(|i| i.name == name).unwrap_or_else(|| {
        let names: Vec<_> = items
            .iter()
            .map(|i| format!("{}: {}", i.kind, &i.name))
            .collect();
        panic!("item '{name}' not found. Available: {names:?}")
    })
}

fn find_all_by_kind(items: &[ParsedItem], kind: SymbolKind) -> Vec<&ParsedItem> {
    items.iter().filter(|i| i.kind == kind).collect()
}

fn find_by_name_prefix<'a>(items: &'a [ParsedItem], prefix: &str) -> &'a ParsedItem {
    items
        .iter()
        .find(|i| i.name.starts_with(prefix))
        .unwrap_or_else(|| {
            let names: Vec<_> = items
                .iter()
                .map(|i| format!("{}: {}", i.kind, &i.name))
                .collect();
            panic!("item starting with '{prefix}' not found. Available: {names:?}")
        })
}

#[test]
fn bash_does_not_emit_member_only_kinds() {
    let source = "#!/usr/bin/env bash\nvalue=1\nrun() { echo hi; }\n";
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
