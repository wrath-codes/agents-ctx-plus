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

#[test]
fn constructor_and_property_emitted_as_members() {
    let source = include_str!("../../../../tests/fixtures/sample.js");
    let items = parse_and_extract(source);

    let ctor = items
        .iter()
        .find(|i| {
            i.kind == SymbolKind::Constructor
                && i.metadata.owner_name.as_deref() == Some("Animal")
                && i.name.contains("constructor")
        })
        .expect("should emit Animal constructor member");
    assert_eq!(ctor.kind, SymbolKind::Constructor);

    let prop = items
        .iter()
        .find(|i| {
            i.kind == SymbolKind::Property
                && i.metadata.owner_name.as_deref() == Some("Animal")
                && i.name.contains("displayName")
        })
        .expect("should emit Animal displayName property member");
    assert_eq!(prop.kind, SymbolKind::Property);
}

#[test]
fn getter_and_setter_emit_single_property_item() {
    let source = r#"
class Sample {
  get title() { return "a"; }
  set title(v) {}
}
"#;
    let items = parse_and_extract(source);
    let title_props: Vec<_> = items
        .iter()
        .filter(|i| i.kind == SymbolKind::Property && i.name == "title")
        .collect();
    assert_eq!(title_props.len(), 1, "property items: {title_props:?}");
}
