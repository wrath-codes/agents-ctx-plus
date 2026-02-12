use super::*;

#[test]
fn defguard_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.ex");
    let items = parse_and_extract(source);
    let g = find_by_name(&items, "is_pos_integer");
    assert_eq!(g.kind, SymbolKind::Macro);
    assert_eq!(g.visibility, Visibility::Public);
}

#[test]
fn defguard_has_doc() {
    let source = include_str!("../../../../tests/fixtures/sample.ex");
    let items = parse_and_extract(source);
    let g = find_by_name(&items, "is_pos_integer");
    assert!(
        g.doc_comment.contains("positive integer"),
        "doc: {:?}",
        g.doc_comment
    );
}

#[test]
fn defguard_params_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.ex");
    let items = parse_and_extract(source);
    let g = find_by_name(&items, "is_pos_integer");
    assert_eq!(g.metadata.parameters, vec!["value"]);
}

#[test]
fn defguard_guard_clause() {
    let source = include_str!("../../../../tests/fixtures/sample.ex");
    let items = parse_and_extract(source);
    let g = find_by_name(&items, "is_pos_integer");
    assert!(
        g.metadata.where_clause.is_some(),
        "should have guard clause"
    );
    assert!(
        g.metadata
            .where_clause
            .as_deref()
            .unwrap()
            .contains("is_integer"),
        "guard: {:?}",
        g.metadata.where_clause
    );
}

#[test]
fn defguard_signature_format() {
    let source = include_str!("../../../../tests/fixtures/sample.ex");
    let items = parse_and_extract(source);
    let g = find_by_name(&items, "is_pos_integer");
    assert!(
        g.signature.starts_with("defguard is_pos_integer"),
        "sig: {:?}",
        g.signature
    );
}

#[test]
fn defguardp_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.ex");
    let items = parse_and_extract(source);
    let g = find_by_name(&items, "is_internal");
    assert_eq!(g.kind, SymbolKind::Macro);
    assert_eq!(g.visibility, Visibility::Private);
}

#[test]
fn defguardp_signature_format() {
    let source = include_str!("../../../../tests/fixtures/sample.ex");
    let items = parse_and_extract(source);
    let g = find_by_name(&items, "is_internal");
    assert!(
        g.signature.starts_with("defguardp is_internal"),
        "sig: {:?}",
        g.signature
    );
}
