use super::*;

// ════════════════════════════════════════════════════════════════
// 24. Access specifier tests
// ════════════════════════════════════════════════════════════════

#[test]
fn class_access_demo_exists() {
    let items = fixture_items();
    let ad = find_by_name(&items, "AccessDemo");
    assert_eq!(ad.kind, SymbolKind::Class);
}

#[test]
fn class_access_demo_has_public_members() {
    let items = fixture_items();
    let ad = find_by_name(&items, "AccessDemo");
    assert!(
        ad.metadata
            .attributes
            .contains(&"has_public_members".to_string()),
        "AccessDemo should track public members, got {:?}",
        ad.metadata.attributes
    );
}

#[test]
fn class_access_demo_has_private_members() {
    let items = fixture_items();
    let ad = find_by_name(&items, "AccessDemo");
    assert!(
        ad.metadata
            .attributes
            .contains(&"has_private_members".to_string()),
        "AccessDemo should track private members, got {:?}",
        ad.metadata.attributes
    );
}

#[test]
fn class_access_demo_has_protected_members() {
    let items = fixture_items();
    let ad = find_by_name(&items, "AccessDemo");
    assert!(
        ad.metadata
            .attributes
            .contains(&"has_protected_members".to_string()),
        "AccessDemo should track protected members, got {:?}",
        ad.metadata.attributes
    );
}

#[test]
fn class_access_demo_has_methods() {
    let items = fixture_items();
    let ad = find_by_name(&items, "AccessDemo");
    assert!(
        ad.metadata.methods.contains(&"pub_method".to_string()),
        "AccessDemo should have pub_method"
    );
}
