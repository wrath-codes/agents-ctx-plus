use super::*;

#[test]
fn signature_no_body_leak() {
    let source = include_str!("../../../../tests/fixtures/sample.js");
    let items = parse_and_extract(source);
    for item in &items {
        if !item.signature.is_empty() && item.kind != SymbolKind::Const {
            assert!(
                !item.signature.contains('{'),
                "signature for '{}' leaks body: {}",
                item.name,
                item.signature
            );
        }
    }
}

#[test]
fn line_numbers_are_one_based() {
    let source = include_str!("../../../../tests/fixtures/sample.js");
    let items = parse_and_extract(source);
    for item in &items {
        assert!(
            item.start_line >= 1,
            "'{}' start_line should be >= 1, got {}",
            item.name,
            item.start_line
        );
        assert!(
            item.end_line >= item.start_line,
            "'{}' end_line {} < start_line {}",
            item.name,
            item.end_line,
            item.start_line
        );
    }
}
