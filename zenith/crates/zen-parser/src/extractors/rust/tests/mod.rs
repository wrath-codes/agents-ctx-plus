use ast_grep_language::LanguageExt;

use super::*;
pub(super) use crate::types::{SymbolKind, Visibility};

mod advanced_types_signatures;
mod basics;
mod data_types;
mod docs_and_attributes;
mod modules_macros_imports;
mod traits_and_impls;
mod visibility_and_ffi;
fn parse_and_extract(source: &str) -> Vec<ParsedItem> {
    let root = SupportLang::Rust.ast_grep(source);
    extract(&root, source).expect("extraction should succeed")
}

fn find_by_name<'a>(items: &'a [ParsedItem], name: &str) -> &'a ParsedItem {
    items
        .iter()
        .find(|i| i.name == name)
        .unwrap_or_else(|| panic!("no item named '{name}' found"))
}
