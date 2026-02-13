use super::*;

#[test]
fn extracts_locals_globals_and_const_attributes() {
    let items = fixture_items();

    let answer = find_by_name(&items, "answer");
    assert_eq!(answer.kind, SymbolKind::Const);
    assert_eq!(answer.visibility, Visibility::Private);
    assert!(
        answer
            .metadata
            .attributes
            .iter()
            .any(|attr| attr == "const")
    );

    let temp = find_by_name(&items, "temp");
    assert_eq!(temp.kind, SymbolKind::Static);
    assert_eq!(temp.visibility, Visibility::Private);
    assert!(temp.metadata.attributes.iter().any(|attr| attr == "close"));

    let global_counter = find_by_name(&items, "global_counter");
    assert_eq!(global_counter.kind, SymbolKind::Static);
    assert_eq!(global_counter.visibility, Visibility::Public);

    let version = find_by_name(&items, "version");
    assert_eq!(version.kind, SymbolKind::Field);
    assert_eq!(version.metadata.owner_name.as_deref(), Some("M"));

    let mode = find_by_name(&items, "mode");
    assert_eq!(mode.kind, SymbolKind::Field);
    assert_eq!(mode.metadata.owner_name.as_deref(), Some("Config"));

    let enabled = find_by_name(&items, "enabled");
    assert_eq!(enabled.kind, SymbolKind::Field);
    assert_eq!(enabled.metadata.owner_name.as_deref(), Some("Config"));

    let level = find_by_name(&items, "level");
    assert_eq!(level.kind, SymbolKind::Field);
    assert_eq!(level.metadata.owner_name.as_deref(), Some("GlobalTable"));
}
