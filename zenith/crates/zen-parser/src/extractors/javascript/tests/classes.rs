use super::*;

#[test]
fn class_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.js");
    let items = parse_and_extract(source);
    let c = find_by_name(&items, "Animal");
    assert_eq!(c.kind, SymbolKind::Class);
    assert_eq!(c.visibility, Visibility::Private);
}

#[test]
fn class_methods_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.js");
    let items = parse_and_extract(source);
    let c = find_by_name(&items, "Animal");
    assert!(
        c.metadata.methods.contains(&"constructor".to_string()),
        "methods: {:?}",
        c.metadata.methods
    );
    assert!(
        c.metadata.methods.contains(&"speak".to_string()),
        "methods: {:?}",
        c.metadata.methods
    );
    assert!(
        c.metadata.methods.contains(&"create".to_string()),
        "methods: {:?}",
        c.metadata.methods
    );
}

#[test]
fn class_getter_setter_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.js");
    let items = parse_and_extract(source);
    let c = find_by_name(&items, "Animal");
    // Getters/setters are method_definition nodes, captured by name
    let display_count = c
        .metadata
        .methods
        .iter()
        .filter(|m| *m == "displayName")
        .count();
    assert!(
        display_count >= 1,
        "should have at least one displayName method, methods: {:?}",
        c.metadata.methods
    );
}

#[test]
fn error_class_detected() {
    let source = include_str!("../../../../tests/fixtures/sample.js");
    let items = parse_and_extract(source);
    let c = find_by_name(&items, "ValidationError");
    assert_eq!(c.kind, SymbolKind::Class);
    assert!(c.metadata.is_error_type);
    assert!(
        c.metadata.base_classes.contains(&"Error".to_string()),
        "base_classes: {:?}",
        c.metadata.base_classes
    );
}
