use super::*;

#[test]
fn variable_simple() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    let v = find_by_name(&items, "FOO");
    assert_eq!(v.kind, SymbolKind::Static);
    assert!(v.signature.contains("FOO="));
}

#[test]
fn variable_numeric() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    let v = find_by_name(&items, "BAZ");
    assert_eq!(v.kind, SymbolKind::Static);
}

#[test]
fn export_with_value() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    let v = find_by_name(&items, "DATABASE_URL");
    assert_eq!(v.kind, SymbolKind::Const);
    assert_eq!(v.visibility, Visibility::Export);
}

#[test]
fn export_doc_comment() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    let v = find_by_name(&items, "DATABASE_URL");
    assert!(
        v.doc_comment.contains("Database connection"),
        "expected doc comment, got: {:?}",
        v.doc_comment
    );
}

#[test]
fn export_api_key() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    let v = find_by_name(&items, "API_KEY");
    assert_eq!(v.visibility, Visibility::Export);
}

#[test]
fn readonly_const() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    let v = find_by_name(&items, "MAX_RETRIES");
    assert_eq!(v.kind, SymbolKind::Const);
    assert_eq!(v.visibility, Visibility::Public);
}

#[test]
fn readonly_app_name() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    let v = find_by_name(&items, "APP_NAME");
    assert_eq!(v.kind, SymbolKind::Const);
}

#[test]
fn local_variable() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    let v = find_by_name(&items, "COUNTER");
    assert_eq!(v.kind, SymbolKind::Static);
    assert_eq!(v.visibility, Visibility::Private);
}

#[test]
fn declare_exported() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    let v = find_by_name(&items, "EXPORTED_VAR");
    assert_eq!(v.kind, SymbolKind::Const);
    assert_eq!(v.visibility, Visibility::Export);
}

#[test]
fn declare_integer() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    let v = find_by_name(&items, "INTEGER_VAR");
    assert_eq!(v.kind, SymbolKind::Static);
}

#[test]
fn declare_readonly() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    let v = find_by_name(&items, "DECLARED_READONLY");
    assert_eq!(v.kind, SymbolKind::Const);
}

#[test]
fn indexed_array() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    let v = find_by_name(&items, "FRUITS");
    assert!(
        v.metadata
            .attributes
            .iter()
            .any(|a| a.contains("indexed_array")),
        "should have indexed_array attribute: {:?}",
        v.metadata.attributes
    );
}

#[test]
fn indexed_array_doc() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    let v = find_by_name(&items, "FRUITS");
    assert!(
        v.doc_comment.contains("Indexed array"),
        "expected doc comment: {:?}",
        v.doc_comment
    );
}

#[test]
fn associative_array() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    let v = find_by_name(&items, "CONFIG");
    assert!(
        v.metadata
            .attributes
            .iter()
            .any(|a| a.contains("associative_array")),
        "should have associative_array attribute: {:?}",
        v.metadata.attributes
    );
}
