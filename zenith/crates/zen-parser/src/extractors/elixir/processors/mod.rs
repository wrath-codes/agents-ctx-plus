//! Elixir extraction processors: modules, functions, macros, protocols,
//! structs, guards, delegates, and type attributes.

mod definitions;
mod types;

use crate::types::{ParsedItem, SymbolKind};

pub(super) use definitions::{
    process_def, process_defdelegate, process_defguard, process_defmacro, process_defmodule,
};
pub(super) use types::{
    process_defexception, process_defimpl, process_defprotocol, process_defstruct,
    try_extract_type_attr,
};

/// Deduplicate multi-clause functions.
///
/// Elixir allows multiple function clauses (e.g., `def classify(x) when is_integer(x)`
/// and `def classify(x) when is_float(x)`). We keep only the first clause per name+kind
/// **within the same scope** (determined by line proximity — clauses within 20 lines
/// of each other are considered the same function).
pub(super) fn dedup_multi_clause(items: &mut Vec<ParsedItem>) {
    // Map from (kind, name) → first occurrence line number
    let mut seen: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
    items.retain(|item| {
        // Only dedup functions and macros (modules, protocols, etc. are unique)
        if matches!(item.kind, SymbolKind::Function | SymbolKind::Macro) {
            let key = format!("{}:{}", item.kind, item.name);
            if let Some(&first_line) = seen.get(&key) {
                // Only dedup if within 20 lines of the first clause
                // (same module scope). Different modules will be far apart.
                item.start_line.abs_diff(first_line) > 20
            } else {
                seen.insert(key, item.start_line);
                true
            }
        } else {
            true
        }
    });
}
