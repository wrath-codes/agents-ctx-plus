use super::*;

#[test]
fn exported_function_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.js");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "formatDate");
    assert_eq!(f.kind, SymbolKind::Function);
    assert_eq!(f.visibility, Visibility::Export);
    assert!(f.metadata.is_exported);
}

#[test]
fn exported_class_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.js");
    let items = parse_and_extract(source);
    let c = find_by_name(&items, "EventBus");
    assert_eq!(c.kind, SymbolKind::Class);
    assert_eq!(c.visibility, Visibility::Export);
    assert!(c.metadata.is_exported);
}

#[test]
fn exported_class_methods_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.js");
    let items = parse_and_extract(source);
    let c = find_by_name(&items, "EventBus");
    assert!(
        c.metadata.methods.contains(&"emit".to_string()),
        "methods: {:?}",
        c.metadata.methods
    );
}

#[test]
fn exported_const_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.js");
    let items = parse_and_extract(source);
    let c = find_by_name(&items, "VERSION");
    assert_eq!(c.kind, SymbolKind::Const);
    assert_eq!(c.visibility, Visibility::Export);
}

#[test]
fn exported_arrow_function_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.js");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "processItems");
    assert_eq!(f.kind, SymbolKind::Function);
    assert_eq!(f.visibility, Visibility::Export);
    assert!(f.metadata.is_async);
}

#[test]
fn default_export_function() {
    let source = include_str!("../../../../tests/fixtures/sample.js");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "main");
    assert_eq!(f.kind, SymbolKind::Function);
    assert_eq!(f.visibility, Visibility::Export);
    assert!(f.metadata.is_default_export);
    assert!(f.metadata.is_exported);
}
