use super::*;

// ── Struct tests ──────────────────────────────────────────────

#[test]
fn struct_point_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let point = find_by_name(&items, "Point");
    assert_eq!(point.kind, SymbolKind::Struct);
}

#[test]
fn struct_point_has_fields() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let point = find_by_name(&items, "Point");
    assert!(
        point.metadata.fields.len() >= 2,
        "Point should have at least 2 fields: {:?}",
        point.metadata.fields
    );
    assert!(
        point.metadata.fields.contains(&"x".to_string()),
        "Point should have field x: {:?}",
        point.metadata.fields
    );
    assert!(
        point.metadata.fields.contains(&"y".to_string()),
        "Point should have field y: {:?}",
        point.metadata.fields
    );
}

#[test]
fn struct_point_doc_comment() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let point = find_by_name(&items, "Point");
    assert!(
        point.doc_comment.contains("2D point"),
        "expected doc about 2D point, got: {:?}",
        point.doc_comment
    );
}

#[test]
fn struct_rectangle_typedef() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let rect = find_by_name(&items, "Rectangle");
    assert_eq!(rect.kind, SymbolKind::Struct);
    assert!(
        rect.metadata.attributes.contains(&"typedef".to_string()),
        "Rectangle should be a typedef: {:?}",
        rect.metadata.attributes
    );
}

#[test]
fn struct_rectangle_has_fields() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let rect = find_by_name(&items, "Rectangle");
    assert!(
        rect.metadata.fields.len() >= 3,
        "Rectangle should have at least 3 fields: {:?}",
        rect.metadata.fields
    );
}

#[test]
fn struct_node_has_fields() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let nodes: Vec<_> = items
        .iter()
        .filter(|i| i.name == "Node" && !i.metadata.fields.is_empty())
        .collect();
    assert!(!nodes.is_empty(), "should find Node struct with fields");
    let node = nodes[0];
    assert!(
        node.metadata.fields.contains(&"value".to_string()),
        "Node should have 'value' field: {:?}",
        node.metadata.fields
    );
}

#[test]
fn struct_hardware_register_bitfields() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let hw = find_by_name(&items, "HardwareRegister");
    assert_eq!(hw.kind, SymbolKind::Struct);
    assert!(
        hw.metadata
            .fields
            .iter()
            .filter(|f| f.contains("bitfield"))
            .count()
            >= 3,
        "HardwareRegister should have 3+ bitfields: {:?}",
        hw.metadata.fields
    );
}

#[test]
fn struct_config_has_many_fields() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let config = find_by_name(&items, "Config");
    assert_eq!(config.kind, SymbolKind::Struct);
    assert!(
        config.metadata.fields.len() >= 5,
        "Config should have 5+ fields: {:?}",
        config.metadata.fields
    );
}

#[test]
fn struct_forward_declaration_node() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let fwd = items
        .iter()
        .find(|i| {
            i.name == "Node"
                && i.metadata
                    .attributes
                    .contains(&"forward_declaration".to_string())
        })
        .expect("should find Node forward declaration");
    assert_eq!(fwd.kind, SymbolKind::Struct);
}

#[test]
fn struct_forward_declaration_opaque() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let fwd = items
        .iter()
        .find(|i| {
            i.name == "OpaqueHandle"
                && i.metadata
                    .attributes
                    .contains(&"forward_declaration".to_string())
        })
        .expect("should find OpaqueHandle forward declaration");
    assert_eq!(fwd.kind, SymbolKind::Struct);
}
