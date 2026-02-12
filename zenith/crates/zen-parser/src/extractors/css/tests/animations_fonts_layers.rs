use super::*;

#[test]
fn keyframes_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.css");
    let items = parse_and_extract(source);
    let kf = find_all_by_at_rule(&items, "keyframes");
    assert_eq!(kf.len(), 3, "should find 3 @keyframes");
}

#[test]
fn keyframes_fade_in() {
    let source = include_str!("../../../../tests/fixtures/sample.css");
    let items = parse_and_extract(source);
    let k = find_by_name(&items, "@keyframes fadeIn");
    assert_eq!(k.kind, SymbolKind::Function);
    assert_eq!(k.metadata.at_rule_name.as_deref(), Some("keyframes"));
}

#[test]
fn keyframes_slide_in() {
    let source = include_str!("../../../../tests/fixtures/sample.css");
    let items = parse_and_extract(source);
    let k = find_by_name(&items, "@keyframes slideIn");
    assert_eq!(k.kind, SymbolKind::Function);
}

#[test]
fn keyframes_pulse() {
    let source = include_str!("../../../../tests/fixtures/sample.css");
    let items = parse_and_extract(source);
    let k = find_by_name(&items, "@keyframes pulse");
    assert_eq!(k.kind, SymbolKind::Function);
}

// ── @font-face tests ───────────────────────────────────────────

#[test]
fn font_face_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.css");
    let items = parse_and_extract(source);
    let ff = find_all_by_at_rule(&items, "font-face");
    assert_eq!(ff.len(), 2, "should find 2 @font-face declarations");
}

#[test]
fn font_face_kind() {
    let source = include_str!("../../../../tests/fixtures/sample.css");
    let items = parse_and_extract(source);
    let ff = find_all_by_at_rule(&items, "font-face");
    for f in &ff {
        assert_eq!(f.kind, SymbolKind::Struct);
    }
}

#[test]
fn font_face_has_properties() {
    let source = include_str!("../../../../tests/fixtures/sample.css");
    let items = parse_and_extract(source);
    let ff = find_all_by_at_rule(&items, "font-face");
    let first = ff.first().expect("should have at least one @font-face");
    assert!(
        first
            .metadata
            .css_properties
            .iter()
            .any(|p| p.contains("font-family")),
        "font-face should have font-family property: {:?}",
        first.metadata.css_properties
    );
}

// ── @layer tests ───────────────────────────────────────────────

#[test]
fn layer_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.css");
    let items = parse_and_extract(source);
    let layers = find_all_by_at_rule(&items, "layer");
    assert_eq!(layers.len(), 2, "should find 2 @layer declarations");
}

#[test]
fn layer_base() {
    let source = include_str!("../../../../tests/fixtures/sample.css");
    let items = parse_and_extract(source);
    let l = find_by_name(&items, "@layer base");
    assert_eq!(l.kind, SymbolKind::Module);
    assert_eq!(l.metadata.at_rule_name.as_deref(), Some("layer"));
}

#[test]
fn layer_utilities() {
    let source = include_str!("../../../../tests/fixtures/sample.css");
    let items = parse_and_extract(source);
    let l = find_by_name(&items, "@layer utilities");
    assert_eq!(l.kind, SymbolKind::Module);
}

#[test]
fn layer_nested_rules() {
    let source = include_str!("../../../../tests/fixtures/sample.css");
    let items = parse_and_extract(source);
    // Rules inside @layer should have parent context
    assert!(
        items
            .iter()
            .any(|i| i.name.contains("@base") || i.name.contains("@utilities")),
        "should find nested rules inside @layer"
    );
}

// ── @container tests ───────────────────────────────────────────

#[test]
fn simple_keyframes() {
    let items = parse_and_extract("@keyframes spin { to { transform: rotate(360deg); } }");
    let k = find_by_name(&items, "@keyframes spin");
    assert_eq!(k.kind, SymbolKind::Function);
}
