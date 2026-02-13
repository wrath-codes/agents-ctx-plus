use super::*;

#[test]
fn extracts_namespaces_imports_and_types() {
    let items = fixture_items();

    assert_eq!(find_by_name(&items, "App\\Core").kind, SymbolKind::Module);

    assert!(
        items
            .iter()
            .any(|i| i.kind == SymbolKind::Module && i.name.contains("Psr\\Log\\LoggerInterface"))
    );

    assert_eq!(find_by_name(&items, "Service").kind, SymbolKind::Class);
    assert_eq!(
        find_by_name(&items, "Renderable").kind,
        SymbolKind::Interface
    );
    assert_eq!(find_by_name(&items, "UsesHelpers").kind, SymbolKind::Trait);
    assert_eq!(find_by_name(&items, "Status").kind, SymbolKind::Enum);

    let status = find_by_name(&items, "Status");
    assert!(status.metadata.variants.iter().any(|v| v == "Ready"));
    assert!(status.metadata.variants.iter().any(|v| v == "Done"));
}
