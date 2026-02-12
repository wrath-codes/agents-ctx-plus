use super::*;

#[test]
fn extracts_constructor_method_property_and_fields() {
    let items = fixture_items();

    let ctor = items
        .iter()
        .find(|i| {
            i.kind == SymbolKind::Constructor && i.metadata.owner_name.as_deref() == Some("Widget")
        })
        .expect("should find Widget constructor");
    assert_eq!(ctor.kind, SymbolKind::Constructor);
    assert_eq!(ctor.metadata.owner_name.as_deref(), Some("Widget"));

    let render = find_by_name(&items, "Render");
    assert_eq!(render.kind, SymbolKind::Method);
    assert_eq!(render.metadata.owner_name.as_deref(), Some("Widget"));

    let title = find_by_name(&items, "Title");
    assert_eq!(title.kind, SymbolKind::Property);
    assert_eq!(title.metadata.owner_name.as_deref(), Some("Widget"));

    let max_size = find_by_name(&items, "MaxSize");
    assert_eq!(max_size.kind, SymbolKind::Const);
    assert_eq!(max_size.metadata.owner_name.as_deref(), Some("Widget"));

    let count = find_by_name(&items, "_count");
    assert_eq!(count.kind, SymbolKind::Field);
    assert!(count.metadata.is_static_member);
}
