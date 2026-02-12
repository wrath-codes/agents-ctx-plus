use super::*;

#[test]
fn defimpl_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.ex");
    let items = parse_and_extract(source);
    let i = find_by_name(&items, "Sample.Renderable.BitString");
    assert_eq!(i.kind, SymbolKind::Trait);
}

#[test]
fn defimpl_methods_listed() {
    let source = include_str!("../../../../tests/fixtures/sample.ex");
    let items = parse_and_extract(source);
    let i = find_by_name(&items, "Sample.Renderable.BitString");
    assert!(
        i.metadata.methods.contains(&"render".to_string()),
        "methods: {:?}",
        i.metadata.methods
    );
}
