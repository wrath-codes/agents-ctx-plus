use super::*;

#[test]
fn signature_includes_tag_and_attrs() {
    let source = include_str!("../../../../tests/fixtures/sample.html");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "contact-form");
    assert!(f.signature.starts_with("<form"), "sig: {}", f.signature);
    assert!(f.signature.contains("action="), "sig: {}", f.signature);
}

#[test]
fn custom_element_signature() {
    let source = include_str!("../../../../tests/fixtures/sample.html");
    let items = parse_and_extract(source);
    let c = find_by_name(&items, "my-component");
    assert!(
        c.signature.starts_with("<my-component"),
        "sig: {}",
        c.signature
    );
}

// ── Line number tests ──────────────────────────────────────────

#[test]
fn line_numbers_are_one_based() {
    let source = include_str!("../../../../tests/fixtures/sample.html");
    let items = parse_and_extract(source);
    for item in &items {
        assert!(
            item.start_line >= 1,
            "'{}' start_line should be >= 1, got {}",
            item.name,
            item.start_line
        );
        assert!(
            item.end_line >= item.start_line,
            "'{}' end_line {} < start_line {}",
            item.name,
            item.end_line,
            item.start_line
        );
    }
}

// ── Style element test ─────────────────────────────────────────
