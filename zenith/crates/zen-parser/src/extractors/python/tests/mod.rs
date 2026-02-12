use ast_grep_language::LanguageExt;

use super::*;

mod basics;
mod classes;
mod decorators_and_regressions;
mod docstrings;
mod errors;
mod functions;
mod module_features;
mod visibility_exports;

fn parse_and_extract(source: &str) -> Vec<ParsedItem> {
    let root = SupportLang::Python.ast_grep(source);
    extract(&root).expect("extraction should succeed")
}

fn find_by_name<'a>(items: &'a [ParsedItem], name: &str) -> &'a ParsedItem {
    items
        .iter()
        .find(|i| i.name == name)
        .unwrap_or_else(|| panic!("no item named '{name}' found"))
}
