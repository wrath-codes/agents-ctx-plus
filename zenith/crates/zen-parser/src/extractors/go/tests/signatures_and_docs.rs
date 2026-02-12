use super::*;

#[test]
fn line_numbers_are_one_based() {
    let source = include_str!("../../../../tests/fixtures/sample.go");
    let items = parse_and_extract(source);
    for item in &items {
        assert!(item.start_line >= 1, "{} starts at 0", item.name);
        assert!(
            item.end_line >= item.start_line,
            "{}: end {} < start {}",
            item.name,
            item.end_line,
            item.start_line
        );
    }
}

#[test]
fn signature_no_body_leak() {
    let source = include_str!("../../../../tests/fixtures/sample.go");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "ProcessItems");
    assert!(!f.signature.contains("return"), "sig: {:?}", f.signature);
}

#[test]
fn single_return_type_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.go");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "privateHelper");
    assert_eq!(f.metadata.return_type.as_deref(), Some("int"));
}

#[test]
fn pointer_return_type_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.go");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "NewConfig");
    assert_eq!(f.metadata.return_type.as_deref(), Some("*Config"));
}

#[test]
fn named_returns_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.go");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "Divide");
    assert_eq!(f.kind, SymbolKind::Function);
    assert!(
        f.metadata
            .return_type
            .as_deref()
            .is_some_and(|rt| rt.contains("float64")),
        "return type: {:?}",
        f.metadata.return_type
    );
}
