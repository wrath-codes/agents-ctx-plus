use super::*;

#[test]
fn script_with_src_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.html");
    let items = parse_and_extract(source);
    let s = find_by_name(&items, "vendor.js");
    assert_eq!(s.kind, SymbolKind::Module);
    assert_eq!(s.metadata.tag_name.as_deref(), Some("script"));
}

#[test]
fn script_module_with_src_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.html");
    let items = parse_and_extract(source);
    let s = find_by_name(&items, "app.js");
    assert_eq!(s.kind, SymbolKind::Module);
}

#[test]
fn inline_module_script_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.html");
    let items = parse_and_extract(source);
    let s = find_by_name(&items, "inline-module");
    assert_eq!(s.kind, SymbolKind::Module);
}

// ── Link tests ─────────────────────────────────────────────────

#[test]
fn link_elements_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.html");
    let items = parse_and_extract(source);
    let links = find_all_by_tag(&items, "link");
    assert!(
        links.len() >= 2,
        "should find at least 2 link elements, found {}",
        links.len()
    );
}

// ── Meta tests ─────────────────────────────────────────────────

#[test]
fn meta_tags_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.html");
    let items = parse_and_extract(source);
    let metas = find_all_by_tag(&items, "meta");
    assert!(!metas.is_empty(), "should find meta elements");
}

// ── Semantic landmark tests ────────────────────────────────────

#[test]
fn inline_style_extraction() {
    let source = "<style>.card { display: flex; }</style>";
    let items = parse_and_extract(source);
    let s = find_by_name(&items, "inline-style");
    assert_eq!(s.kind, SymbolKind::Module);
    assert_eq!(s.metadata.tag_name.as_deref(), Some("style"));
}

// ── Self-closing test ──────────────────────────────────────────

#[test]
fn self_closing_element_detected() {
    let source = include_str!("../../../../tests/fixtures/sample.html");
    let items = parse_and_extract(source);
    // meta tags are self-closing
    let metas = find_all_by_tag(&items, "meta");
    for meta in &metas {
        assert!(
            meta.metadata.is_self_closing,
            "meta should be self-closing: {}",
            meta.name
        );
    }
}

// ── Table tests ────────────────────────────────────────────────
