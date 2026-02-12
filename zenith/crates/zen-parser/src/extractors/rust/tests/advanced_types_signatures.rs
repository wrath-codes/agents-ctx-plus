use super::*;

#[test]
fn lifetimes_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.rs");
    let items = parse_and_extract(source);
    let transform = find_by_name(&items, "transform");
    assert!(
        !transform.metadata.lifetimes.is_empty(),
        "lifetimes should be non-empty"
    );
    assert!(
        transform.metadata.lifetimes.contains(&"'a".to_string()),
        "should contain 'a: {:?}",
        transform.metadata.lifetimes
    );
    assert!(
        transform.metadata.lifetimes.contains(&"'b".to_string()),
        "should contain 'b: {:?}",
        transform.metadata.lifetimes
    );
}

#[test]
fn where_clause_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.rs");
    let items = parse_and_extract(source);
    let transform = find_by_name(&items, "transform");
    assert!(
        transform.metadata.where_clause.is_some(),
        "should have where clause"
    );
    let wc = transform.metadata.where_clause.as_deref().unwrap();
    assert!(wc.contains("Clone"), "where clause: {wc}");
    assert!(wc.contains("Send"), "where clause: {wc}");
}

#[test]
fn const_fn_detected() {
    let source = include_str!("../../../../tests/fixtures/sample.rs");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "const_add");
    assert_eq!(f.kind, SymbolKind::Function);
    assert!(
        f.metadata.attributes.contains(&"const".to_string()),
        "should have const attribute: {:?}",
        f.metadata.attributes
    );
}

#[test]
fn const_generics_struct() {
    let source = include_str!("../../../../tests/fixtures/sample.rs");
    let items = parse_and_extract(source);
    let buf = find_by_name(&items, "Buffer");
    assert_eq!(buf.kind, SymbolKind::Struct);
    assert!(
        buf.metadata
            .generics
            .as_deref()
            .is_some_and(|g| g.contains("const N")),
        "generics: {:?}",
        buf.metadata.generics
    );
}

#[test]
fn impl_trait_return_in_signature() {
    let source = include_str!("../../../../tests/fixtures/sample.rs");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "make_iterator");
    assert!(
        f.metadata
            .return_type
            .as_deref()
            .is_some_and(|rt| rt.contains("impl") && rt.contains("Iterator")),
        "return_type: {:?}",
        f.metadata.return_type
    );
}

#[test]
fn hrtb_in_where_clause() {
    let source = include_str!("../../../../tests/fixtures/sample.rs");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "apply_fn");
    assert!(
        f.metadata
            .where_clause
            .as_deref()
            .is_some_and(|wc| wc.contains("for<'a>")),
        "where_clause: {:?}",
        f.metadata.where_clause
    );
}

#[test]
fn dyn_trait_param_in_signature() {
    let source = include_str!("../../../../tests/fixtures/sample.rs");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "process_dyn");
    assert!(
        f.signature.contains("dyn"),
        "signature should contain dyn: {}",
        f.signature
    );
}
