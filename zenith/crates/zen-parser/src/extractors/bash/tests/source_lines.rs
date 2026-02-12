use super::*;

#[test]
fn source_command() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    let s = find_by_name(&items, "./lib/utils.sh");
    assert_eq!(s.kind, SymbolKind::Module);
    assert!(s.signature.starts_with("source"));
}

#[test]
fn dot_command() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    let d = find_by_name(&items, "./lib/helpers.sh");
    assert_eq!(d.kind, SymbolKind::Module);
    assert!(d.signature.starts_with(". "));
}

#[test]
fn source_doc_comment() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    let s = find_by_name(&items, "./lib/utils.sh");
    assert!(
        s.doc_comment.contains("Load utility"),
        "expected doc comment: {:?}",
        s.doc_comment
    );
}

#[test]
fn line_numbers_are_one_based() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    for item in &items {
        assert!(
            item.start_line >= 1,
            "start_line should be >= 1: {} ({})",
            item.name,
            item.start_line
        );
        assert!(
            item.end_line >= item.start_line,
            "end_line should be >= start_line: {} ({} > {})",
            item.name,
            item.start_line,
            item.end_line
        );
    }
}

#[test]
fn shebang_at_line_1() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    let shebang = find_by_name(&items, "shebang");
    assert_eq!(shebang.start_line, 1, "shebang should be on line 1");
}
