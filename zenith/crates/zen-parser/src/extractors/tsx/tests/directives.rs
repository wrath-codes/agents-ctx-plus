use super::*;

// ── "use client" directive ─────────────────────────────────────

#[test]
fn use_client_directive_detected() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let items = parse_and_extract(source);
    let btn = find_by_name(&items, "Button");
    assert_eq!(
        btn.metadata.component_directive.as_deref(),
        Some("use client")
    );
}

#[test]
fn directive_applies_to_all_items() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let items = parse_and_extract(source);
    for item in &items {
        assert_eq!(
            item.metadata.component_directive.as_deref(),
            Some("use client"),
            "'{}' should have directive",
            item.name
        );
    }
}

#[test]
fn use_server_directive() {
    let src = "\"use server\";\nexport async function submitForm() {}";
    let items = parse_and_extract(src);
    let f = find_by_name(&items, "submitForm");
    assert_eq!(
        f.metadata.component_directive.as_deref(),
        Some("use server")
    );
}

#[test]
fn no_directive_when_absent() {
    let src = "export function Foo() { return <div/>; }";
    let items = parse_and_extract(src);
    let f = find_by_name(&items, "Foo");
    assert!(f.metadata.component_directive.is_none());
}
