use super::*;

// ════════════════════════════════════════════════════════════════
// 18. Inheritance tests
// ════════════════════════════════════════════════════════════════

#[test]
fn inheritance_single_circle_from_shape() {
    let items = fixture_items();
    let c = find_by_name(&items, "Circle");
    assert_eq!(c.metadata.base_classes, vec!["Shape"]);
}

#[test]
fn inheritance_single_rectangle_from_shape() {
    let items = fixture_items();
    let r = find_by_name(&items, "Rectangle");
    assert_eq!(r.metadata.base_classes, vec!["Shape"]);
}

#[test]
fn inheritance_multiple_document() {
    let items = fixture_items();
    let d = find_by_name(&items, "Document");
    assert!(
        d.metadata
            .base_classes
            .contains(&"Serializable".to_string())
            && d.metadata.base_classes.contains(&"Printable".to_string()),
        "Document should have Serializable and Printable bases, got {:?}",
        d.metadata.base_classes
    );
}

#[test]
fn inheritance_virtual_vleft() {
    let items = fixture_items();
    let vl = find_by_name(&items, "VLeft");
    assert_eq!(vl.kind, SymbolKind::Class);
    assert!(
        vl.metadata.base_classes.contains(&"VBase".to_string()),
        "VLeft should inherit from VBase"
    );
}

#[test]
fn inheritance_virtual_vright() {
    let items = fixture_items();
    let vr = find_by_name(&items, "VRight");
    assert_eq!(vr.kind, SymbolKind::Class);
    assert!(
        vr.metadata.base_classes.contains(&"VBase".to_string()),
        "VRight should inherit from VBase"
    );
}

#[test]
fn inheritance_diamond() {
    let items = fixture_items();
    let d = find_by_name(&items, "Diamond");
    assert_eq!(d.kind, SymbolKind::Class);
    assert!(
        d.metadata.base_classes.len() >= 2,
        "Diamond should have 2+ base classes, got {:?}",
        d.metadata.base_classes
    );
}
