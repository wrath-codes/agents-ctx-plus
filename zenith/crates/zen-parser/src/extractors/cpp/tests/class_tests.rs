use super::*;

// ════════════════════════════════════════════════════════════════
// 3. Class tests
// ════════════════════════════════════════════════════════════════

#[test]
fn class_shape_exists() {
    let items = fixture_items();
    let shape = find_by_name(&items, "Shape");
    assert_eq!(shape.kind, SymbolKind::Class, "Shape should be a Class");
}

#[test]
fn class_shape_is_abstract() {
    let items = fixture_items();
    let shape = find_by_name(&items, "Shape");
    assert!(
        shape.metadata.attributes.contains(&"abstract".to_string()),
        "Shape should have abstract attribute"
    );
}

#[test]
fn class_shape_has_methods() {
    let items = fixture_items();
    let shape = find_by_name(&items, "Shape");
    assert!(
        shape.metadata.methods.contains(&"area".to_string()),
        "Shape should have area method"
    );
    assert!(
        shape.metadata.methods.contains(&"perimeter".to_string()),
        "Shape should have perimeter method"
    );
    assert!(
        shape.metadata.methods.contains(&"name".to_string()),
        "Shape should have name method"
    );
}

#[test]
fn class_shape_has_destructor() {
    let items = fixture_items();
    let shape = find_by_name(&items, "Shape");
    assert!(
        shape.metadata.methods.iter().any(|m| m.contains('~')),
        "Shape should have destructor in methods"
    );
}

#[test]
fn class_circle_exists() {
    let items = fixture_items();
    let circle = find_by_name(&items, "Circle");
    assert_eq!(circle.kind, SymbolKind::Class);
}

#[test]
fn class_circle_base_class() {
    let items = fixture_items();
    let circle = find_by_name(&items, "Circle");
    assert!(
        circle.metadata.base_classes.contains(&"Shape".to_string()),
        "Circle should inherit from Shape, got {:?}",
        circle.metadata.base_classes
    );
}

#[test]
fn class_circle_has_methods() {
    let items = fixture_items();
    let circle = find_by_name(&items, "Circle");
    assert!(
        circle.metadata.methods.contains(&"area".to_string()),
        "Circle should have area method"
    );
    assert!(
        circle.metadata.methods.contains(&"radius".to_string()),
        "Circle should have radius method"
    );
}

#[test]
fn class_circle_has_private_field() {
    let items = fixture_items();
    let circle = find_by_name(&items, "Circle");
    assert!(
        circle.metadata.fields.contains(&"radius_".to_string()),
        "Circle should have radius_ field, got {:?}",
        circle.metadata.fields
    );
}

#[test]
fn class_rectangle_base_class() {
    let items = fixture_items();
    let rect = find_by_name(&items, "Rectangle");
    assert_eq!(rect.kind, SymbolKind::Class);
    assert!(
        rect.metadata.base_classes.contains(&"Shape".to_string()),
        "Rectangle should inherit from Shape"
    );
}

#[test]
fn class_rectangle_has_width_height() {
    let items = fixture_items();
    let rect = find_by_name(&items, "Rectangle");
    assert!(
        rect.metadata.methods.contains(&"width".to_string())
            && rect.metadata.methods.contains(&"height".to_string()),
        "Rectangle should have width/height methods"
    );
}

#[test]
fn class_square_is_final() {
    let items = fixture_items();
    let sq = find_by_name(&items, "Square");
    assert_eq!(sq.kind, SymbolKind::Class);
    assert!(
        sq.metadata.attributes.contains(&"final".to_string()),
        "Square should be final"
    );
}

#[test]
fn class_square_inherits_rectangle() {
    let items = fixture_items();
    let sq = find_by_name(&items, "Square");
    assert!(
        sq.metadata.base_classes.contains(&"Rectangle".to_string()),
        "Square should inherit from Rectangle"
    );
}

#[test]
fn class_document_multiple_inheritance() {
    let items = fixture_items();
    let doc = find_by_name(&items, "Document");
    assert_eq!(doc.kind, SymbolKind::Class);
    assert!(
        doc.metadata.base_classes.len() >= 2,
        "Document should have 2+ base classes, got {:?}",
        doc.metadata.base_classes
    );
}

#[test]
fn class_serializable_abstract() {
    let items = fixture_items();
    let s = find_by_name(&items, "Serializable");
    assert_eq!(s.kind, SymbolKind::Class);
    assert!(
        s.metadata.attributes.contains(&"abstract".to_string()),
        "Serializable should be abstract"
    );
}

#[test]
fn class_printable_abstract() {
    let items = fixture_items();
    let p = find_by_name(&items, "Printable");
    assert_eq!(p.kind, SymbolKind::Class);
    assert!(
        p.metadata.attributes.contains(&"abstract".to_string()),
        "Printable should be abstract"
    );
}

#[test]
fn class_outer_exists() {
    let items = fixture_items();
    let outer = find_by_name(&items, "Outer");
    assert_eq!(outer.kind, SymbolKind::Class);
}

#[test]
fn class_int_wrapper_exists() {
    let items = fixture_items();
    let iw = find_by_name(&items, "IntWrapper");
    assert_eq!(iw.kind, SymbolKind::Class);
}

#[test]
fn class_int_wrapper_has_conversion_operators() {
    let items = fixture_items();
    let iw = find_by_name(&items, "IntWrapper");
    assert!(
        iw.metadata.methods.iter().any(|m| m.contains("operator")),
        "IntWrapper should have conversion operators, got {:?}",
        iw.metadata.methods
    );
}

#[test]
fn class_explicit_only_exists() {
    let items = fixture_items();
    let e = find_by_name(&items, "ExplicitOnly");
    assert_eq!(e.kind, SymbolKind::Class);
}

#[test]
fn class_container_template() {
    let items = fixture_items();
    let c = find_by_name(&items, "Container");
    assert_eq!(c.kind, SymbolKind::Class);
    assert!(
        c.metadata.attributes.contains(&"template".to_string()),
        "Container should have template attribute"
    );
}

#[test]
fn class_container_has_generics() {
    let items = fixture_items();
    let c = find_by_name(&items, "Container");
    assert!(
        c.metadata.generics.is_some(),
        "Container should have generics"
    );
}

#[test]
fn class_container_void_specialization() {
    let items = fixture_items();
    let spec = items
        .iter()
        .find(|i| i.name.contains("Container") && i.name.contains("void"));
    assert!(
        spec.is_some(),
        "Container<void> specialization should exist"
    );
}

#[test]
fn class_container_methods() {
    let items = fixture_items();
    let c = find_by_name(&items, "Container");
    assert!(
        c.metadata.methods.contains(&"get".to_string()),
        "Container should have get method"
    );
    assert!(
        c.metadata.methods.contains(&"set".to_string()),
        "Container should have set method"
    );
}
