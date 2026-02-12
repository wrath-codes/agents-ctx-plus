use ast_grep_language::LanguageExt;

use super::*;
pub(super) use crate::types::{SymbolKind, Visibility};

// Re-export helper functions needed by unit tests
pub(super) use super::tsx_helpers::{
    extract_props_from_type_annotation, is_component_name, is_component_return_type, is_hoc_name,
    is_hook_name,
};

mod class_components;
mod components;
mod directives;
mod forward_ref_memo_lazy;
mod hoc;
mod hooks;
mod interfaces_types;
mod misc;
mod naming_conventions;

fn parse_and_extract(source: &str) -> Vec<ParsedItem> {
    let root = SupportLang::Tsx.ast_grep(source);
    extract(&root, SupportLang::Tsx).expect("extraction should succeed")
}

fn find_by_name<'a>(items: &'a [ParsedItem], name: &str) -> &'a ParsedItem {
    items.iter().find(|i| i.name == name).unwrap_or_else(|| {
        let names: Vec<_> = items
            .iter()
            .map(|i| format!("{}({})", i.name, i.kind))
            .collect();
        panic!("should find item named '{name}', available: {names:?}")
    })
}
