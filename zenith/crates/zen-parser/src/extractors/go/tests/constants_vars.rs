use super::*;

#[test]
fn single_const_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.go");
    let items = parse_and_extract(source);
    let c = find_by_name(&items, "MaxRetries");
    assert_eq!(c.kind, SymbolKind::Const);
    assert_eq!(c.visibility, Visibility::Public);
}

#[test]
fn single_const_has_doc() {
    let source = include_str!("../../../../tests/fixtures/sample.go");
    let items = parse_and_extract(source);
    let c = find_by_name(&items, "MaxRetries");
    assert!(
        c.doc_comment.contains("maximum number"),
        "doc: {:?}",
        c.doc_comment
    );
}

#[test]
fn iota_const_group_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.go");
    let items = parse_and_extract(source);
    let ok = find_by_name(&items, "StatusOK");
    assert_eq!(ok.kind, SymbolKind::Const);
    let _ = find_by_name(&items, "StatusError");
    let _ = find_by_name(&items, "StatusPending");
}

#[test]
fn direction_consts_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.go");
    let items = parse_and_extract(source);
    let _ = find_by_name(&items, "North");
    let _ = find_by_name(&items, "South");
    let _ = find_by_name(&items, "East");
    let _ = find_by_name(&items, "West");
}

#[test]
fn single_var_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.go");
    let items = parse_and_extract(source);
    let v = find_by_name(&items, "DefaultTimeout");
    assert_eq!(v.kind, SymbolKind::Static);
    assert_eq!(v.visibility, Visibility::Public);
}

#[test]
fn single_var_has_doc() {
    let source = include_str!("../../../../tests/fixtures/sample.go");
    let items = parse_and_extract(source);
    let v = find_by_name(&items, "DefaultTimeout");
    assert!(
        v.doc_comment.contains("default timeout"),
        "doc: {:?}",
        v.doc_comment
    );
}

#[test]
fn var_group_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.go");
    let items = parse_and_extract(source);
    let _ = find_by_name(&items, "GlobalCount");
    let _ = find_by_name(&items, "GlobalName");
}

#[test]
fn typed_const_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.go");
    let items = parse_and_extract(source);
    let c = find_by_name(&items, "Pi");
    assert_eq!(c.kind, SymbolKind::Const);
    assert_eq!(c.visibility, Visibility::Public);
}
