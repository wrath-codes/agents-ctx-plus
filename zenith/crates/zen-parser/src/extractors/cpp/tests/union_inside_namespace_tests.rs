use super::*;

// ════════════════════════════════════════════════════════════════
// 33. Union inside namespace tests
// ════════════════════════════════════════════════════════════════

#[test]
fn namespace_geo_exists() {
    let items = fixture_items();
    let geo = find_by_name(&items, "geo");
    assert_eq!(geo.kind, SymbolKind::Module);
}

#[test]
fn union_shape_data_in_namespace() {
    let items = fixture_items();
    let sd = items
        .iter()
        .find(|i| i.kind == SymbolKind::Union && i.name == "ShapeData");
    assert!(
        sd.is_some(),
        "ShapeData union inside geo namespace should be extracted"
    );
}

#[test]
fn function_in_geo_namespace() {
    let items = fixture_items();
    let ac = items
        .iter()
        .find(|i| i.kind == SymbolKind::Function && i.name == "area_calc");
    assert!(
        ac.is_some(),
        "area_calc function in geo namespace should be extracted"
    );
}
