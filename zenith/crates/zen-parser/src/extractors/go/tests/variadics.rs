use super::*;

#[test]
fn variadic_function_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.go");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "Printf");
    assert_eq!(f.kind, SymbolKind::Function);
    assert_eq!(f.visibility, Visibility::Public);
}

#[test]
fn variadic_param_included() {
    let source = include_str!("../../../../tests/fixtures/sample.go");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "Printf");
    assert!(
        f.metadata.parameters.iter().any(|p| p.contains("...")),
        "params should include variadic: {:?}",
        f.metadata.parameters
    );
    assert!(
        f.metadata.parameters.iter().any(|p| p.contains("format")),
        "params should include format: {:?}",
        f.metadata.parameters
    );
}

#[test]
fn method_variadic_params() {
    let source = include_str!("../../../../tests/fixtures/sample.go");
    let items = parse_and_extract(source);
    let m = find_by_name(&items, "Listen");
    assert_eq!(m.kind, SymbolKind::Method);
    assert!(
        m.metadata.parameters.iter().any(|p| p.contains("...")),
        "method params should include variadic: {:?}",
        m.metadata.parameters
    );
}
