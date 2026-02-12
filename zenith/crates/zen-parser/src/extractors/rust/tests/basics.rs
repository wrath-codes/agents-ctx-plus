use super::*;

#[test]
fn extract_from_fixture() {
    let source = include_str!("../../../../tests/fixtures/sample.rs");
    let items = parse_and_extract(source);
    let names: Vec<&str> = items.iter().map(|i| i.name.as_str()).collect();
    assert!(names.contains(&"process"), "missing 'process': {names:?}");
    assert!(
        names.contains(&"dangerous"),
        "missing 'dangerous': {names:?}"
    );
    assert!(names.contains(&"Config"), "missing 'Config': {names:?}");
    assert!(names.contains(&"Status"), "missing 'Status': {names:?}");
    assert!(names.contains(&"Handler"), "missing 'Handler': {names:?}");
    assert!(names.contains(&"MAX_SIZE"), "missing 'MAX_SIZE': {names:?}");
    assert!(names.contains(&"MyResult"), "missing 'MyResult': {names:?}");
}

#[test]
fn async_function_detected() {
    let source = include_str!("../../../../tests/fixtures/sample.rs");
    let items = parse_and_extract(source);
    let process = find_by_name(&items, "process");
    assert_eq!(process.kind, SymbolKind::Function);
    assert!(process.metadata.is_async);
    assert!(!process.metadata.is_unsafe);
    assert_eq!(process.visibility, Visibility::Public);
}

#[test]
fn unsafe_function_detected() {
    let source = include_str!("../../../../tests/fixtures/sample.rs");
    let items = parse_and_extract(source);
    let dangerous = find_by_name(&items, "dangerous");
    assert!(dangerous.metadata.is_unsafe);
    assert!(!dangerous.metadata.is_async);
    assert_eq!(dangerous.visibility, Visibility::Private);
}

#[test]
fn doc_comments_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.rs");
    let items = parse_and_extract(source);
    let process = find_by_name(&items, "process");
    assert!(
        process.doc_comment.contains("documented async function"),
        "doc_comment: {:?}",
        process.doc_comment
    );
    assert!(
        process.doc_comment.contains("Second line"),
        "doc_comment: {:?}",
        process.doc_comment
    );
}

#[test]
fn generics_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.rs");
    let items = parse_and_extract(source);
    let process = find_by_name(&items, "process");
    assert!(
        process.metadata.generics.is_some(),
        "generics should be Some"
    );
    let g = process.metadata.generics.as_deref().unwrap();
    assert!(g.contains('T'), "generics should contain T: {g}");
}

#[test]
fn return_type_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.rs");
    let items = parse_and_extract(source);
    let process = find_by_name(&items, "process");
    assert!(process.metadata.returns_result);
    let rt = process.metadata.return_type.as_deref().unwrap();
    assert!(rt.contains("Result"), "return_type: {rt}");
}

#[test]
fn signature_no_body_leak() {
    let source = include_str!("../../../../tests/fixtures/sample.rs");
    let items = parse_and_extract(source);
    for item in &items {
        if !item.signature.is_empty() {
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
    let source = include_str!("../../../../tests/fixtures/sample.rs");
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

// ── New fixture coverage tests ─────────────────────────────────
