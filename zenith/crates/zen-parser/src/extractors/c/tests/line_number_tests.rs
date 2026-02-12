use super::*;

// ── Line number tests ─────────────────────────────────────────

#[test]
fn line_numbers_are_positive() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    for item in &items {
        assert!(
            item.start_line > 0,
            "start_line should be > 0 for {}: got {}",
            item.name,
            item.start_line
        );
        assert!(
            item.end_line >= item.start_line,
            "end_line should be >= start_line for {}",
            item.name
        );
    }
}

#[test]
fn function_definition_spans_multiple_lines() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let pd = items
        .iter()
        .find(|i| {
            i.name == "process_data" && !i.metadata.attributes.contains(&"prototype".to_string())
        })
        .expect("should find process_data definition");
    assert!(
        pd.end_line > pd.start_line,
        "process_data should span multiple lines"
    );
}
