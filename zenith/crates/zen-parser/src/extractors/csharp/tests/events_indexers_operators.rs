use super::*;

#[test]
fn extracts_events_indexers_and_operators() {
    let items = fixture_items();

    let changed = find_by_name(&items, "Changed");
    assert_eq!(changed.kind, SymbolKind::Event);

    let indexer = find_by_name(&items, "this[]");
    assert_eq!(indexer.kind, SymbolKind::Indexer);

    let op = find_by_name(&items, "operator+");
    assert_eq!(op.kind, SymbolKind::Method);

    let conversion = find_by_name(&items, "operator int");
    assert_eq!(conversion.kind, SymbolKind::Method);
}
