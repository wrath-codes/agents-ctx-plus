use super::*;

#[test]
fn exported_enum_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.ts");
    let items = parse_and_extract(source);
    let e = find_by_name(&items, "Color");
    assert_eq!(e.kind, SymbolKind::Enum);
    assert_eq!(e.visibility, Visibility::Export);
}

#[test]
fn non_exported_enum_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.ts");
    let items = parse_and_extract(source);
    let e = find_by_name(&items, "InternalStatus");
    assert_eq!(e.kind, SymbolKind::Enum);
    assert_eq!(e.visibility, Visibility::Private);
}

#[test]
fn enum_variants_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.ts");
    let items = parse_and_extract(source);
    let e = find_by_name(&items, "Color");
    assert!(
        e.metadata.variants.contains(&"Red".to_string()),
        "variants: {:?}",
        e.metadata.variants
    );
    assert!(
        e.metadata.variants.contains(&"Green".to_string()),
        "variants: {:?}",
        e.metadata.variants
    );
    assert!(
        e.metadata.variants.contains(&"Blue".to_string()),
        "variants: {:?}",
        e.metadata.variants
    );
}

#[test]
fn enum_with_string_values_variants_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.ts");
    let items = parse_and_extract(source);
    let e = find_by_name(&items, "Direction");
    assert_eq!(e.kind, SymbolKind::Enum);
    assert!(
        e.metadata.variants.contains(&"Up".to_string()),
        "variants: {:?}",
        e.metadata.variants
    );
    assert!(
        e.metadata.variants.contains(&"Down".to_string()),
        "variants: {:?}",
        e.metadata.variants
    );
    assert!(
        e.metadata.variants.contains(&"Left".to_string()),
        "variants: {:?}",
        e.metadata.variants
    );
    assert!(
        e.metadata.variants.contains(&"Right".to_string()),
        "variants: {:?}",
        e.metadata.variants
    );
}
