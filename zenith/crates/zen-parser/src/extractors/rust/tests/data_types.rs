use super::*;

#[test]
fn struct_fields_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.rs");
    let items = parse_and_extract(source);
    let config = find_by_name(&items, "Config");
    assert_eq!(config.kind, SymbolKind::Struct);
    assert!(
        config.metadata.fields.len() >= 3,
        "fields: {:?}",
        config.metadata.fields
    );
}

#[test]
fn struct_attributes_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.rs");
    let items = parse_and_extract(source);
    let config = find_by_name(&items, "Config");
    assert!(
        !config.metadata.attributes.is_empty(),
        "should have derive attributes"
    );
}

#[test]
fn enum_variants_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.rs");
    let items = parse_and_extract(source);
    let status = find_by_name(&items, "Status");
    assert_eq!(status.kind, SymbolKind::Enum);
    assert!(
        status.metadata.variants.iter().any(|v| v == "Active"),
        "variants: {:?}",
        status.metadata.variants
    );
    assert!(
        status
            .metadata
            .variants
            .iter()
            .any(|v| v.starts_with("Inactive")),
        "variants: {:?}",
        status.metadata.variants
    );
}

#[test]
fn const_item_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.rs");
    let items = parse_and_extract(source);
    let max = find_by_name(&items, "MAX_SIZE");
    assert_eq!(max.kind, SymbolKind::Const);
    assert_eq!(max.visibility, Visibility::Public);
}

#[test]
fn type_alias_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.rs");
    let items = parse_and_extract(source);
    let my_result = find_by_name(&items, "MyResult");
    assert_eq!(my_result.kind, SymbolKind::TypeAlias);
}

#[test]
fn static_item_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.rs");
    let items = parse_and_extract(source);
    let global = find_by_name(&items, "GLOBAL_NAME");
    assert_eq!(global.kind, SymbolKind::Static);
}

#[test]
fn union_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.rs");
    let items = parse_and_extract(source);
    let raw = find_by_name(&items, "RawValue");
    assert_eq!(raw.kind, SymbolKind::Union);
}

#[test]
fn tuple_struct_fields() {
    let source = include_str!("../../../../tests/fixtures/sample.rs");
    let items = parse_and_extract(source);
    let point = find_by_name(&items, "Point");
    assert_eq!(point.kind, SymbolKind::Struct);
    assert!(
        !point.metadata.fields.is_empty(),
        "tuple struct should have fields"
    );
    assert!(
        point.metadata.fields.iter().any(|f| f.contains("f64")),
        "fields: {:?}",
        point.metadata.fields
    );
}

#[test]
fn unit_struct_detected() {
    let source = include_str!("../../../../tests/fixtures/sample.rs");
    let items = parse_and_extract(source);
    let marker = find_by_name(&items, "Marker");
    assert_eq!(marker.kind, SymbolKind::Struct);
    assert!(
        marker.metadata.fields.is_empty(),
        "unit struct has no fields"
    );
}

#[test]
fn enum_variant_payloads_captured() {
    let source = include_str!("../../../../tests/fixtures/sample.rs");
    let items = parse_and_extract(source);
    let msg = find_by_name(&items, "Message");
    assert_eq!(msg.kind, SymbolKind::Enum);
    assert!(
        msg.metadata.variants.iter().any(|v| v == "Quit"),
        "variants: {:?}",
        msg.metadata.variants
    );
    assert!(
        msg.metadata
            .variants
            .iter()
            .any(|v| v.starts_with("Move") && v.contains('x')),
        "Move variant should have struct payload: {:?}",
        msg.metadata.variants
    );
    assert!(
        msg.metadata
            .variants
            .iter()
            .any(|v| v.starts_with("Write") && v.contains("String")),
        "Write variant should have tuple payload: {:?}",
        msg.metadata.variants
    );
}

#[test]
fn const_item_has_type() {
    let source = include_str!("../../../../tests/fixtures/sample.rs");
    let items = parse_and_extract(source);
    let max = find_by_name(&items, "MAX_SIZE");
    assert!(
        max.metadata.return_type.is_some(),
        "const should have return_type: {:?}",
        max.metadata.return_type
    );
}

#[test]
fn static_item_has_type() {
    let source = include_str!("../../../../tests/fixtures/sample.rs");
    let items = parse_and_extract(source);
    let global = find_by_name(&items, "GLOBAL_NAME");
    assert!(
        global.metadata.return_type.is_some(),
        "static should have return_type: {:?}",
        global.metadata.return_type
    );
}

#[test]
fn receiver_struct_detected() {
    let source = include_str!("../../../../tests/fixtures/sample.rs");
    let items = parse_and_extract(source);
    let r = find_by_name(&items, "Receiver");
    assert_eq!(r.kind, SymbolKind::Struct);
}
