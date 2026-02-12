use super::*;

#[test]
fn private_function_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.ex");
    let items = parse_and_extract(source);
    // Find private "transform" — there may be a public one in Types module too
    assert!(
        items
            .iter()
            .any(|i| i.name == "transform" && i.visibility == Visibility::Private),
        "should find private transform function"
    );
}

#[test]
fn private_function_with_default_params() {
    let source = include_str!("../../../../tests/fixtures/sample.ex");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "validate");
    assert_eq!(f.kind, SymbolKind::Function);
    assert_eq!(f.visibility, Visibility::Private);
    assert!(
        f.metadata.parameters.len() >= 2,
        "params: {:?}",
        f.metadata.parameters
    );
}
