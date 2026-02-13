use super::*;

#[test]
fn extracts_functions_and_table_methods_with_owner_metadata() {
    let items = fixture_items();

    let add = find_by_name(&items, "add");
    assert_eq!(add.kind, SymbolKind::Method);
    assert_eq!(add.metadata.owner_name.as_deref(), Some("M"));
    assert_eq!(add.metadata.owner_kind, Some(SymbolKind::Module));
    assert!(add.metadata.is_static_member);

    let greet = find_by_name(&items, "greet");
    assert_eq!(greet.kind, SymbolKind::Method);
    assert_eq!(greet.metadata.owner_name.as_deref(), Some("M"));
    assert_eq!(greet.metadata.owner_kind, Some(SymbolKind::Module));
    assert!(!greet.metadata.is_static_member);

    let helper = find_by_name(&items, "helper");
    assert_eq!(helper.kind, SymbolKind::Function);
    assert_eq!(helper.visibility, Visibility::Private);

    let scale = find_by_name(&items, "scale");
    assert_eq!(scale.kind, SymbolKind::Method);
    assert_eq!(scale.metadata.owner_name.as_deref(), Some("M"));

    let alias = find_by_name(&items, "alias");
    assert_eq!(alias.kind, SymbolKind::Method);
    assert_eq!(alias.metadata.owner_name.as_deref(), Some("M"));
    assert!(alias.metadata.is_static_member);

    let build = find_by_name(&items, "build");
    assert_eq!(build.kind, SymbolKind::Method);
    assert_eq!(build.metadata.owner_name.as_deref(), Some("Config"));

    let ping = find_by_name(&items, "ping");
    assert_eq!(ping.kind, SymbolKind::Method);
    assert_eq!(ping.metadata.owner_name.as_deref(), Some("GlobalTable"));
}
