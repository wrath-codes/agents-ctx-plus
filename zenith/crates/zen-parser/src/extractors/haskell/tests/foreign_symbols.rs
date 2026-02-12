use super::*;

#[test]
fn extracts_foreign_import_and_export_as_functions() {
    let items = fixture_items();

    let c_puts = find_by_name(&items, "c_puts");
    assert_eq!(c_puts.kind, SymbolKind::Function);

    let hs_render = find_by_name(&items, "hs_render");
    assert_eq!(hs_render.kind, SymbolKind::Function);

    let hs_render_count = items
        .iter()
        .filter(|i| i.kind == SymbolKind::Function && i.name == "hs_render")
        .count();
    assert_eq!(hs_render_count, 1);
}
