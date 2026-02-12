use ast_grep_language::LanguageExt;

use super::*;
pub(super) use crate::types::SymbolKind;

mod controls_and_slots;
mod custom_elements;
mod forms_templates_dialog;
mod identified_elements;
mod media_embeds;
mod resources_meta;
mod signatures_and_lines;

fn parse_and_extract(source: &str) -> Vec<ParsedItem> {
    let root = SupportLang::Html.ast_grep(source);
    extract(&root).expect("extraction should succeed")
}

fn find_by_name<'a>(items: &'a [ParsedItem], name: &str) -> &'a ParsedItem {
    items
        .iter()
        .find(|i| i.name == name)
        .unwrap_or_else(|| panic!("should find item named '{name}'"))
}

fn find_all_by_tag<'a>(items: &'a [ParsedItem], tag: &str) -> Vec<&'a ParsedItem> {
    items
        .iter()
        .filter(|i| i.metadata.tag_name.as_deref() == Some(tag))
        .collect()
}

#[test]
fn html_does_not_emit_member_only_kinds() {
    let items = parse_and_extract("<div id=\"app\"><button>Run</button></div>");
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
