use super::*;

#[test]
fn line_numbers_are_one_based_and_ranges_valid() {
    let items = fixture_items();

    for item in &items {
        assert!(item.start_line >= 1, "start_line should be one-based");
        assert!(
            item.end_line >= item.start_line,
            "end_line must be >= start_line"
        );
    }
}

#[test]
fn signatures_are_present_for_key_symbols() {
    let items = fixture_items();

    for name in ["mkWidget", "classify", "onChange", "c_puts", "hs_render"] {
        let item = find_by_name(&items, name);
        assert!(
            !item.signature.trim().is_empty(),
            "signature should be non-empty for {name}"
        );
    }
}
