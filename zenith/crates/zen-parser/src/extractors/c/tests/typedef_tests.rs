use super::*;

// ── Typedef tests ─────────────────────────────────────────────

#[test]
fn typedef_byte() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let byte = find_by_name(&items, "Byte");
    assert_eq!(byte.kind, SymbolKind::TypeAlias);
}

#[test]
fn typedef_size() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let size = find_by_name(&items, "Size");
    assert_eq!(size.kind, SymbolKind::TypeAlias);
}

#[test]
fn typedef_point2d() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let p2d = find_by_name(&items, "Point2D");
    assert_eq!(p2d.kind, SymbolKind::TypeAlias);
}

#[test]
fn typedef_comparator_function_pointer() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let cmp = find_by_name(&items, "Comparator");
    assert_eq!(cmp.kind, SymbolKind::TypeAlias);
    assert!(
        cmp.metadata
            .attributes
            .contains(&"function_pointer".to_string()),
        "Comparator should be a function pointer typedef: {:?}",
        cmp.metadata.attributes
    );
}

#[test]
fn typedef_event_callback() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let ec = find_by_name(&items, "EventCallback");
    assert_eq!(ec.kind, SymbolKind::TypeAlias);
    assert!(
        ec.metadata
            .attributes
            .contains(&"function_pointer".to_string()),
        "EventCallback should be a function pointer typedef"
    );
}

#[test]
fn typedef_allocator() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let alloc = find_by_name(&items, "Allocator");
    assert_eq!(alloc.kind, SymbolKind::TypeAlias);
}
