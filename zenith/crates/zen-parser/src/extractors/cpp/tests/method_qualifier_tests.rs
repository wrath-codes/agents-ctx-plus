use super::*;

// ════════════════════════════════════════════════════════════════
// 29. Method qualifier tests
// ════════════════════════════════════════════════════════════════

#[test]
fn class_method_derived_exists() {
    let items = fixture_items();
    let md = find_by_name(&items, "MethodDerived");
    assert_eq!(md.kind, SymbolKind::Class);
}

#[test]
fn class_method_derived_has_override() {
    let items = fixture_items();
    let md = find_by_name(&items, "MethodDerived");
    assert!(
        md.metadata.attributes.contains(&"has_override".to_string()),
        "MethodDerived should have has_override attribute, got {:?}",
        md.metadata.attributes
    );
}

#[test]
fn class_method_derived_has_final_methods() {
    let items = fixture_items();
    let md = find_by_name(&items, "MethodDerived");
    assert!(
        md.metadata
            .attributes
            .contains(&"has_final_methods".to_string()),
        "MethodDerived should have has_final_methods attribute, got {:?}",
        md.metadata.attributes
    );
}

#[test]
fn class_method_derived_has_deleted_members() {
    let items = fixture_items();
    let md = find_by_name(&items, "MethodDerived");
    assert!(
        md.metadata
            .attributes
            .contains(&"has_deleted_members".to_string()),
        "MethodDerived should have has_deleted_members, got {:?}",
        md.metadata.attributes
    );
}

#[test]
fn class_method_derived_has_defaulted_members() {
    let items = fixture_items();
    let md = find_by_name(&items, "MethodDerived");
    assert!(
        md.metadata
            .attributes
            .contains(&"has_defaulted_members".to_string()),
        "MethodDerived should have has_defaulted_members, got {:?}",
        md.metadata.attributes
    );
}

#[test]
fn class_resource_guard_has_deleted_members() {
    let items = fixture_items();
    let rg = find_by_name(&items, "ResourceGuard");
    assert!(
        rg.metadata
            .attributes
            .contains(&"has_deleted_members".to_string()),
        "ResourceGuard should have has_deleted_members, got {:?}",
        rg.metadata.attributes
    );
}
