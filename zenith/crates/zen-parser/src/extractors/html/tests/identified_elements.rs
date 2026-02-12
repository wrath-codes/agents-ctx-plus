use super::*;

#[test]
fn element_with_id_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.html");
    let items = parse_and_extract(source);
    let h = find_by_name(&items, "main-header");
    assert_eq!(h.metadata.tag_name.as_deref(), Some("header"));
    assert_eq!(h.metadata.element_id.as_deref(), Some("main-header"));
}

#[test]
fn element_classes_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.html");
    let items = parse_and_extract(source);
    let h = find_by_name(&items, "main-header");
    assert!(
        h.metadata.class_names.contains(&"site-header".to_string()),
        "classes: {:?}",
        h.metadata.class_names
    );
    assert!(
        h.metadata.class_names.contains(&"sticky".to_string()),
        "classes: {:?}",
        h.metadata.class_names
    );
}

#[test]
fn nav_with_id_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.html");
    let items = parse_and_extract(source);
    let nav = find_by_name(&items, "main-nav");
    assert_eq!(nav.metadata.tag_name.as_deref(), Some("nav"));
}

#[test]
fn content_main_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.html");
    let items = parse_and_extract(source);
    let m = find_by_name(&items, "content");
    assert_eq!(m.metadata.tag_name.as_deref(), Some("main"));
}

#[test]
fn section_with_id_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.html");
    let items = parse_and_extract(source);
    let s = find_by_name(&items, "hero");
    assert_eq!(s.metadata.tag_name.as_deref(), Some("section"));
}

#[test]
fn article_with_id_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.html");
    let items = parse_and_extract(source);
    let a = find_by_name(&items, "feature-1");
    assert_eq!(a.metadata.tag_name.as_deref(), Some("article"));
}

#[test]
fn article_data_attribute() {
    let source = include_str!("../../../../tests/fixtures/sample.html");
    let items = parse_and_extract(source);
    let a = find_by_name(&items, "feature-1");
    assert!(
        a.metadata
            .html_attributes
            .iter()
            .any(|(n, v)| n == "data-category" && v.as_deref() == Some("core")),
        "attrs: {:?}",
        a.metadata.html_attributes
    );
}

// ── Form tests ─────────────────────────────────────────────────

#[test]
fn footer_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.html");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "main-footer");
    assert_eq!(f.metadata.tag_name.as_deref(), Some("footer"));
}

#[test]
fn aside_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.html");
    let items = parse_and_extract(source);
    let a = find_by_name(&items, "sidebar");
    assert_eq!(a.metadata.tag_name.as_deref(), Some("aside"));
}

// ── Signature tests ────────────────────────────────────────────
