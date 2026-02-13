use super::*;

#[test]
fn inline_table_expands_nested_properties() {
    let items = fixture_items();

    let x = find_by_name(&items, "point.x");
    let y = find_by_name(&items, "point.y");

    assert_eq!(x.kind, SymbolKind::Property);
    assert_eq!(y.kind, SymbolKind::Property);
    assert_eq!(x.metadata.owner_name.as_deref(), Some("point"));
    assert_eq!(y.metadata.owner_name.as_deref(), Some("point"));
}

#[test]
fn arrays_include_shape_and_elements() {
    let items = fixture_items();

    let ports = find_by_name(&items, "database.ports");
    assert!(
        ports
            .metadata
            .attributes
            .iter()
            .any(|a| a == "toml:array_count:3")
    );
    assert!(
        ports
            .metadata
            .attributes
            .iter()
            .any(|a| a == "toml:array_elements:integer")
    );

    assert_eq!(
        find_by_name(&items, "database.ports[0]").kind,
        SymbolKind::Property
    );
    assert_eq!(
        find_by_name(&items, "database.ports[1]").kind,
        SymbolKind::Property
    );
    assert_eq!(
        find_by_name(&items, "database.ports[2]").kind,
        SymbolKind::Property
    );
}

#[test]
fn preserves_owner_for_nested_table_pairs() {
    let items = fixture_items();
    let ip = find_by_name(&items, "servers.alpha.ip");
    assert_eq!(ip.metadata.owner_name.as_deref(), Some("servers.alpha"));
    assert_eq!(ip.metadata.owner_kind, Some(SymbolKind::Module));
}

#[test]
fn array_table_pairs_are_scoped_to_indexed_element() {
    let items = fixture_items();
    let names = find_all_by_name(&items, "products[0].name");
    assert_eq!(names.len(), 1);
    let second = find_all_by_name(&items, "products[1].name");
    assert_eq!(second.len(), 1);
}
