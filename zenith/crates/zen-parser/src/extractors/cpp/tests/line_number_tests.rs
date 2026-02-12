use super::*;

// ════════════════════════════════════════════════════════════════
// 17. Line number tests
// ════════════════════════════════════════════════════════════════

#[test]
fn all_start_lines_valid() {
    let items = fixture_items();
    for item in &items {
        assert!(
            item.start_line >= 1,
            "item {} should have start_line >= 1, got {}",
            item.name,
            item.start_line
        );
    }
}

#[test]
fn all_end_lines_gte_start() {
    let items = fixture_items();
    for item in &items {
        assert!(
            item.end_line >= item.start_line,
            "item {} end_line ({}) < start_line ({})",
            item.name,
            item.end_line,
            item.start_line
        );
    }
}
