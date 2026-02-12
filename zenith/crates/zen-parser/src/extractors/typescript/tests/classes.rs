use super::*;

#[test]
fn default_export_class() {
    let source = include_str!("../../../../tests/fixtures/sample.ts");
    let items = parse_and_extract(source);
    let c = find_by_name(&items, "EventEmitter");
    assert_eq!(c.kind, SymbolKind::Class);
    assert_eq!(c.visibility, Visibility::Export);
    assert!(c.metadata.is_default_export);
    assert!(c.metadata.is_exported);
}

#[test]
fn class_methods_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.ts");
    let items = parse_and_extract(source);
    let c = find_by_name(&items, "EventEmitter");
    assert!(c.metadata.methods.contains(&"on".to_string()));
    assert!(c.metadata.methods.contains(&"emit".to_string()));
    assert!(c.metadata.methods.contains(&"cleanup".to_string()));
}

#[test]
fn error_class_detected() {
    let source = include_str!("../../../../tests/fixtures/sample.ts");
    let items = parse_and_extract(source);
    let c = find_by_name(&items, "HttpError");
    assert_eq!(c.kind, SymbolKind::Class);
    assert!(c.metadata.is_error_type);
    assert!(c.metadata.base_classes.contains(&"Error".to_string()));
}

#[test]
fn abstract_class_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.ts");
    let items = parse_and_extract(source);
    let c = find_by_name(&items, "Shape");
    assert_eq!(c.kind, SymbolKind::Class);
    assert_eq!(c.visibility, Visibility::Export);
    assert!(c.metadata.is_unsafe, "is_unsafe used for abstract");
}

#[test]
fn abstract_class_methods_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.ts");
    let items = parse_and_extract(source);
    let c = find_by_name(&items, "Shape");
    assert!(
        c.metadata.methods.contains(&"area".to_string()),
        "methods: {:?}",
        c.metadata.methods
    );
    assert!(
        c.metadata.methods.contains(&"perimeter".to_string()),
        "methods: {:?}",
        c.metadata.methods
    );
}

#[test]
fn class_constructor_member_emitted() {
    let source = include_str!("../../../../tests/fixtures/sample.ts");
    let items = parse_and_extract(source);
    let ctor = items
        .iter()
        .find(|i| {
            i.kind == SymbolKind::Constructor
                && i.metadata.owner_name.as_deref() == Some("HttpError")
        })
        .expect("should emit HttpError constructor member");
    assert_eq!(ctor.kind, SymbolKind::Constructor);
}

#[test]
fn class_event_and_indexer_members_emitted() {
    let source = r"
class Store {
  onChange: (ev: Event) => void;
  [name: string]: unknown;
}
";
    let items = parse_and_extract(source);

    let event_member = items
        .iter()
        .find(|i| {
            i.kind == SymbolKind::Event
                && i.metadata.owner_name.as_deref() == Some("Store")
                && i.name.contains("onChange")
        })
        .expect("should emit Store event member");
    assert_eq!(event_member.kind, SymbolKind::Event);

    let indexer = find_by_name(&items, "Store[]");
    assert_eq!(indexer.kind, SymbolKind::Indexer);
}
