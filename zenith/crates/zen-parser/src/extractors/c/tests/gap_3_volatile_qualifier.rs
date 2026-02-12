use super::*;

// ── Gap 3: volatile qualifier ─────────────────────────────────

#[test]
fn volatile_variable_has_attr() {
    let items = parse_and_extract("volatile int sensor;");
    assert_eq!(items.len(), 1);
    assert!(
        items[0]
            .metadata
            .attributes
            .contains(&"volatile".to_string()),
        "should have volatile attr: {:?}",
        items[0].metadata.attributes
    );
}

#[test]
fn volatile_const_both_detected() {
    let items = parse_and_extract("volatile const int hw = 0x1234;");
    assert_eq!(items[0].kind, SymbolKind::Const);
    assert!(
        items[0]
            .metadata
            .attributes
            .contains(&"volatile".to_string())
    );
    assert!(items[0].metadata.attributes.contains(&"const".to_string()));
}

#[test]
fn fixture_sensor_reading_volatile() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let sr = find_by_name(&items, "sensor_reading");
    assert!(
        sr.metadata.attributes.contains(&"volatile".to_string()),
        "sensor_reading should have volatile: {:?}",
        sr.metadata.attributes
    );
}

#[test]
fn fixture_hw_status_reg_volatile_const() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let hw = find_by_name(&items, "HW_STATUS_REG");
    assert_eq!(hw.kind, SymbolKind::Const);
    assert!(hw.metadata.attributes.contains(&"volatile".to_string()));
    assert!(hw.metadata.attributes.contains(&"const".to_string()));
}
