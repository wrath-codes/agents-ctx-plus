use super::*;

// ── Additional coverage: multi-dimensional arrays ─────────────

#[test]
fn fixture_transform_matrix() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let tm = find_by_name(&items, "transform_matrix");
    assert!(
        tm.metadata.attributes.contains(&"array".to_string()),
        "transform_matrix should have array attr: {:?}",
        tm.metadata.attributes
    );
}
