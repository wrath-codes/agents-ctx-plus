use super::*;

// ════════════════════════════════════════════════════════════════
// 25. Template alias tests
// ════════════════════════════════════════════════════════════════

#[test]
fn template_alias_shared_ptr() {
    let items = fixture_items();
    let sp = find_by_name(&items, "SharedPtr");
    assert_eq!(sp.kind, SymbolKind::TypeAlias);
    assert!(
        sp.metadata.attributes.contains(&"template".to_string()),
        "SharedPtr should have template attribute"
    );
    assert!(
        sp.metadata.attributes.contains(&"using".to_string()),
        "SharedPtr should have using attribute"
    );
}

#[test]
fn template_alias_shared_ptr_has_generics() {
    let items = fixture_items();
    let sp = find_by_name(&items, "SharedPtr");
    assert!(
        sp.metadata.generics.is_some(),
        "SharedPtr should have generics"
    );
}

#[test]
fn template_alias_map() {
    let items = fixture_items();
    let m = find_by_name(&items, "Map");
    assert_eq!(m.kind, SymbolKind::TypeAlias);
    assert!(
        m.metadata.attributes.contains(&"template".to_string()),
        "Map should have template attribute"
    );
}

#[test]
fn minimal_template_alias() {
    let items = parse_and_extract("template<typename T> using Ptr = T*;");
    let p = find_by_name(&items, "Ptr");
    assert_eq!(p.kind, SymbolKind::TypeAlias);
    assert!(p.metadata.attributes.contains(&"template".to_string()));
    assert!(p.metadata.attributes.contains(&"using".to_string()));
    assert!(p.metadata.generics.is_some());
}
