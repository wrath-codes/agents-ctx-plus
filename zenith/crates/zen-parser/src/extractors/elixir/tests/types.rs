use super::*;

#[test]
fn type_attr_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.ex");
    let items = parse_and_extract(source);
    let t = find_by_name(&items, "direction");
    assert_eq!(t.kind, SymbolKind::TypeAlias);
    assert_eq!(t.visibility, Visibility::Public);
}

#[test]
fn type_attr_signature() {
    let source = include_str!("../../../../tests/fixtures/sample.ex");
    let items = parse_and_extract(source);
    let t = find_by_name(&items, "direction");
    assert!(t.signature.starts_with("@type"), "sig: {:?}", t.signature);
}

#[test]
fn parametric_type_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.ex");
    let items = parse_and_extract(source);
    let t = find_by_name(&items, "result");
    assert_eq!(t.kind, SymbolKind::TypeAlias);
    assert_eq!(t.visibility, Visibility::Public);
}

#[test]
fn typep_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.ex");
    let items = parse_and_extract(source);
    let t = find_by_name(&items, "internal_state");
    assert_eq!(t.kind, SymbolKind::TypeAlias);
    assert_eq!(t.visibility, Visibility::Private);
}

#[test]
fn opaque_type_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.ex");
    let items = parse_and_extract(source);
    let t = find_by_name(&items, "wrapped");
    assert_eq!(t.kind, SymbolKind::TypeAlias);
    assert_eq!(t.visibility, Visibility::Public);
}

#[test]
fn opaque_type_signature() {
    let source = include_str!("../../../../tests/fixtures/sample.ex");
    let items = parse_and_extract(source);
    let t = find_by_name(&items, "wrapped");
    assert!(t.signature.starts_with("@opaque"), "sig: {:?}", t.signature);
}

#[test]
fn struct_type_t_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.ex");
    let items = parse_and_extract(source);
    // The Sample.Config module has @type t :: %__MODULE__{}
    assert!(
        items
            .iter()
            .any(|i| i.name == "t" && i.kind == SymbolKind::TypeAlias),
        "should extract @type t"
    );
}
