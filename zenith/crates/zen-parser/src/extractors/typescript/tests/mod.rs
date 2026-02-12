use ast_grep_language::LanguageExt;

use super::*;
pub(super) use crate::types::{SymbolKind, Visibility};

mod ambient_declarations;
mod arrow_functions;
mod classes;
mod constants_vars;
mod enums;
mod functions;
mod interfaces;
mod jsdoc;
mod namespaces;
mod overloads;
mod signatures_and_lines;
mod tsx_compat;
mod type_aliases;

fn parse_and_extract(source: &str) -> Vec<ParsedItem> {
    let root = SupportLang::TypeScript.ast_grep(source);
    extract(&root, SupportLang::TypeScript).expect("extraction should succeed")
}

fn find_by_name<'a>(items: &'a [ParsedItem], name: &str) -> &'a ParsedItem {
    items
        .iter()
        .find(|i| i.name == name)
        .unwrap_or_else(|| panic!("should find item named '{name}'"))
}
