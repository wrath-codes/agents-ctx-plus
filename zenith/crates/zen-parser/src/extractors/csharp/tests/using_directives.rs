use super::*;

#[test]
fn extracts_using_directives_as_modules() {
    let items = fixture_items();
    let modules = find_all_by_kind(&items, SymbolKind::Module);

    assert!(modules.iter().any(|m| m.name == "System"));
    assert!(
        modules
            .iter()
            .any(|m| m.name == "System.Collections.Generic")
    );
}
