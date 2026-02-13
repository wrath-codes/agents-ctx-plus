use super::*;

#[test]
fn emits_root_and_top_level_properties() {
    let items = fixture_items();

    let root = find_by_name(&items, "$");
    assert_eq!(root.kind, SymbolKind::Module);
    assert_eq!(root.metadata.return_type.as_deref(), Some("object"));

    let app_name = find_by_name(&items, "app.name");
    assert_eq!(app_name.kind, SymbolKind::Property);
    assert_eq!(app_name.metadata.owner_name.as_deref(), Some("app"));
}
