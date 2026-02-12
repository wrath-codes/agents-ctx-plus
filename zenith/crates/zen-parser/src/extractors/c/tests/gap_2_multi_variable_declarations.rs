use super::*;

// ── Gap 2: multi-variable declarations ────────────────────────

#[test]
fn multi_var_init_all_extracted() {
    let items = parse_and_extract("int a = 1, b = 2, c = 3;");
    assert_eq!(items.len(), 3);
    assert_eq!(items[0].name, "a");
    assert_eq!(items[1].name, "b");
    assert_eq!(items[2].name, "c");
}

#[test]
fn multi_var_plain_all_extracted() {
    let items = parse_and_extract("int x, y, z;");
    assert_eq!(items.len(), 3);
    assert_eq!(items[0].name, "x");
    assert_eq!(items[1].name, "y");
    assert_eq!(items[2].name, "z");
}

#[test]
fn fixture_multi_a_b_c() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let _ = find_by_name(&items, "multi_a");
    let _ = find_by_name(&items, "multi_b");
    let _ = find_by_name(&items, "multi_c");
}

#[test]
fn fixture_coord_x_y_z() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let _ = find_by_name(&items, "coord_x");
    let _ = find_by_name(&items, "coord_y");
    let _ = find_by_name(&items, "coord_z");
}
