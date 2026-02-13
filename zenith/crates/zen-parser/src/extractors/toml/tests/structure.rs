use super::*;

#[test]
fn fixture_parses_without_error() {
    let items = fixture_items();
    assert!(!items.is_empty());
}

#[test]
fn root_document_item_exists() {
    let items = fixture_items();
    let root = find_by_name(&items, "$");
    assert_eq!(root.kind, SymbolKind::Module);
    assert!(root
        .metadata
        .attributes
        .iter()
        .any(|a| a == "toml:kind:document"));
}

#[test]
fn top_level_pairs_and_tables_are_extracted() {
    let items = fixture_items();

    assert_eq!(find_by_name(&items, "title").kind, SymbolKind::Property);
    assert_eq!(find_by_name(&items, "enabled").kind, SymbolKind::Property);
    assert_eq!(find_by_name(&items, "database").kind, SymbolKind::Module);
    assert_eq!(
        find_by_name(&items, "servers.alpha").kind,
        SymbolKind::Module
    );
    assert_eq!(
        find_by_name(&items, "servers.beta").kind,
        SymbolKind::Module
    );
}

#[test]
fn array_table_elements_are_indexed() {
    let items = fixture_items();
    let p0 = find_by_name(&items, "products[0]");
    let p1 = find_by_name(&items, "products[1]");
    assert_eq!(p0.kind, SymbolKind::Module);
    assert_eq!(p1.kind, SymbolKind::Module);
    assert!(p0
        .metadata
        .attributes
        .iter()
        .any(|a| a == "toml:kind:table_array"));
}
