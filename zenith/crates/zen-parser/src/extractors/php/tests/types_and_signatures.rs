use super::*;

#[test]
fn normalizes_union_types_and_parameter_qualifiers() {
    let items = fixture_items();

    let render = items
        .iter()
        .find(|i| i.name == "render" && i.metadata.owner_name.as_deref() == Some("Service"))
        .expect("expected Service::render");

    assert_eq!(render.metadata.return_type.as_deref(), Some("int|string"));
    assert!(
        render
            .metadata
            .parameters
            .iter()
            .any(|p| p.contains("suffix: int|string") && p.contains("default"))
    );
}

#[test]
fn captures_property_hook_metadata() {
    let items = fixture_items();
    let slug = find_by_name(&items, "slug");

    assert_eq!(slug.kind, SymbolKind::Property);
    assert!(slug.metadata.methods.iter().any(|m| m.contains("hook:get")));
    assert!(slug.metadata.methods.iter().any(|m| m.contains("hook:set")));
    assert!(
        slug.metadata
            .attributes
            .iter()
            .any(|a| a.starts_with("property_hook:name:"))
    );
}

#[test]
fn canonicalizes_equivalent_union_types() {
    let source = r"
<?php
function a(int|string $x): string|int { return $x; }
function b(string|int $x): int|string { return $x; }
";

    let items = parse_and_extract(source);
    let a = find_by_name(&items, "a");
    let b = find_by_name(&items, "b");

    assert_eq!(a.metadata.return_type, b.metadata.return_type);
    assert_eq!(a.metadata.parameters, b.metadata.parameters);
}
