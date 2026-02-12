use ast_grep_language::LanguageExt;

use super::*;
pub(super) use crate::types::SymbolKind;

mod animations_fonts_layers;
mod custom_properties_universal;
mod imports_namespace_charset;
mod media_supports_container;
mod modern_selectors_nesting_scope;
mod page_property_counter_starting;
mod sanity_and_lines;
mod selectors_and_rules;

fn parse_and_extract(source: &str) -> Vec<ParsedItem> {
    let root = SupportLang::Css.ast_grep(source);
    extract(&root).expect("extraction should succeed")
}

fn find_by_name<'a>(items: &'a [ParsedItem], name: &str) -> &'a ParsedItem {
    items.iter().find(|i| i.name == name).unwrap_or_else(|| {
        let names: Vec<_> = items.iter().map(|i| &i.name).collect();
        panic!("should find item named '{name}', available: {names:?}")
    })
}

fn find_all_by_at_rule<'a>(items: &'a [ParsedItem], rule: &str) -> Vec<&'a ParsedItem> {
    items
        .iter()
        .filter(|i| i.metadata.at_rule_name.as_deref() == Some(rule))
        .collect()
}
