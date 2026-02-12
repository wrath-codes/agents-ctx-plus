use super::*;

#[test]
fn extracts_class_and_hybrid_type_kinds() {
    let items = fixture_items();

    let renderable = find_by_name(&items, "Renderable");
    assert_eq!(renderable.kind, SymbolKind::Trait);

    let status = find_by_name(&items, "Status");
    assert_eq!(status.kind, SymbolKind::Enum);
    assert!(status.metadata.variants.iter().any(|v| v == "New"));
    assert!(status.metadata.variants.iter().any(|v| v == "Ready"));
    assert!(status.metadata.variants.iter().any(|v| v == "Done"));

    let widget = find_by_name(&items, "Widget");
    assert_eq!(widget.kind, SymbolKind::Struct);
    assert!(
        widget
            .metadata
            .fields
            .iter()
            .any(|f| f.contains("widgetId"))
    );
    assert!(
        widget
            .metadata
            .fields
            .iter()
            .any(|f| f.contains("widgetName"))
    );

    let user_id = find_by_name(&items, "UserId");
    assert_eq!(user_id.kind, SymbolKind::TypeAlias);
}
