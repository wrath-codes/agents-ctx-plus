use super::*;

#[test]
fn declare_function_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.ts");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "fetchExternal");
    assert_eq!(f.kind, SymbolKind::Function);
}

#[test]
fn declare_const_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.ts");
    let items = parse_and_extract(source);
    let c = find_by_name(&items, "API_VERSION");
    assert_eq!(c.kind, SymbolKind::Const);
}

#[test]
fn declare_class_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.ts");
    let items = parse_and_extract(source);
    let c = find_by_name(&items, "ExternalLib");
    assert_eq!(c.kind, SymbolKind::Class);
}

#[test]
fn declare_module_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.ts");
    let items = parse_and_extract(source);
    let m = find_by_name(&items, "my-module");
    assert_eq!(m.kind, SymbolKind::Module);
    assert!(
        m.signature.contains("declare module"),
        "sig: {:?}",
        m.signature
    );
}
