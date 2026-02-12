use super::*;

// ── Enum tests ────────────────────────────────────────────────

#[test]
fn enum_color_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let color = find_by_name(&items, "Color");
    assert_eq!(color.kind, SymbolKind::Enum);
}

#[test]
fn enum_color_has_variants() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let color = find_by_name(&items, "Color");
    assert!(
        color.metadata.variants.len() >= 5,
        "Color should have 5 variants: {:?}",
        color.metadata.variants
    );
    assert!(
        color.metadata.variants.contains(&"COLOR_RED".to_string()),
        "should have COLOR_RED"
    );
}

#[test]
fn enum_color_doc_comment() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let color = find_by_name(&items, "Color");
    assert!(
        color.doc_comment.contains("Color constants"),
        "expected doc about color constants, got: {:?}",
        color.doc_comment
    );
}

#[test]
fn enum_status_code_typedef() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let sc = find_by_name(&items, "StatusCode");
    assert_eq!(sc.kind, SymbolKind::Enum);
    assert!(
        sc.metadata.attributes.contains(&"typedef".to_string()),
        "StatusCode should be typedef: {:?}",
        sc.metadata.attributes
    );
}

#[test]
fn enum_status_code_has_variants() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let sc = find_by_name(&items, "StatusCode");
    assert!(
        sc.metadata.variants.len() >= 4,
        "StatusCode should have 4+ variants: {:?}",
        sc.metadata.variants
    );
}

#[test]
fn enum_log_level_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let ll = find_by_name(&items, "LogLevel");
    assert_eq!(ll.kind, SymbolKind::Enum);
    assert!(
        ll.metadata.variants.len() >= 6,
        "LogLevel should have 6 variants: {:?}",
        ll.metadata.variants
    );
}

#[test]
fn enum_forward_declaration() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let fwd = items
        .iter()
        .find(|i| {
            i.name == "Status"
                && i.metadata
                    .attributes
                    .contains(&"forward_declaration".to_string())
        })
        .expect("should find Status forward declaration");
    assert_eq!(fwd.kind, SymbolKind::Enum);
}
