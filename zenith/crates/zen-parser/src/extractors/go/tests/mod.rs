use ast_grep_language::LanguageExt;

use super::*;
pub(super) use crate::types::{SymbolKind, Visibility};

mod constants_vars;
mod dedupe_edge_cases;
mod embedded_types_edge_cases;
mod errors;
mod functions_private;
mod functions_public;
mod generics;
mod interfaces;
mod metadata_contract_edge_cases;
mod methods;
mod misc;
mod package_import_edge_cases;
mod receiver_owner_edge_cases;
mod signatures_and_docs;
mod structs;
mod type_canonicalization_edge_cases;
mod types_aliases;
mod variadics;

fn parse_and_extract(source: &str) -> Vec<ParsedItem> {
    let root = SupportLang::Go.ast_grep(source);
    extract(&root).expect("extraction should succeed")
}

fn find_by_name<'a>(items: &'a [ParsedItem], name: &str) -> &'a ParsedItem {
    items
        .iter()
        .find(|i| i.name == name)
        .unwrap_or_else(|| panic!("should find item named '{name}'"))
}

// ── Exported function ──────────────────────────────────────────
