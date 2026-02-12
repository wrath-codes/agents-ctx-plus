use super::*;

#[test]
fn iframe_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.html");
    let items = parse_and_extract(source);
    let i = find_by_name(&items, "embed-frame");
    assert_eq!(i.kind, SymbolKind::Static);
    assert_eq!(i.metadata.tag_name.as_deref(), Some("iframe"));
}

#[test]
fn iframe_src_in_attributes() {
    let source = include_str!("../../../../tests/fixtures/sample.html");
    let items = parse_and_extract(source);
    let i = find_by_name(&items, "embed-frame");
    assert!(
        i.metadata
            .html_attributes
            .iter()
            .any(|(n, v)| n == "src" && v.as_deref() == Some("https://example.com")),
        "attrs: {:?}",
        i.metadata.html_attributes
    );
}

// ── Object / Embed tests ───────────────────────────────────────

#[test]
fn object_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.html");
    let items = parse_and_extract(source);
    let o = find_by_name(&items, "flash-obj");
    assert_eq!(o.kind, SymbolKind::Static);
    assert_eq!(o.metadata.tag_name.as_deref(), Some("object"));
}

#[test]
fn embed_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.html");
    let items = parse_and_extract(source);
    let e = find_by_name(&items, "pdf-embed");
    assert_eq!(e.kind, SymbolKind::Static);
    assert_eq!(e.metadata.tag_name.as_deref(), Some("embed"));
}

// ── Video / Audio tests ────────────────────────────────────────

#[test]
fn video_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.html");
    let items = parse_and_extract(source);
    let v = find_by_name(&items, "intro-video");
    assert_eq!(v.kind, SymbolKind::Static);
    assert_eq!(v.metadata.tag_name.as_deref(), Some("video"));
}

#[test]
fn video_attributes() {
    let source = include_str!("../../../../tests/fixtures/sample.html");
    let items = parse_and_extract(source);
    let v = find_by_name(&items, "intro-video");
    assert!(
        v.metadata
            .html_attributes
            .iter()
            .any(|(n, _)| n == "controls"),
        "attrs: {:?}",
        v.metadata.html_attributes
    );
    assert!(
        v.metadata
            .html_attributes
            .iter()
            .any(|(n, _)| n == "autoplay"),
        "attrs: {:?}",
        v.metadata.html_attributes
    );
}

#[test]
fn audio_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.html");
    let items = parse_and_extract(source);
    let a = find_by_name(&items, "bg-music");
    assert_eq!(a.kind, SymbolKind::Static);
    assert_eq!(a.metadata.tag_name.as_deref(), Some("audio"));
}

// ── Picture / Canvas tests ─────────────────────────────────────

#[test]
fn picture_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.html");
    let items = parse_and_extract(source);
    let p = find_by_name(&items, "hero-picture");
    assert_eq!(p.kind, SymbolKind::Static);
    assert_eq!(p.metadata.tag_name.as_deref(), Some("picture"));
}

#[test]
fn canvas_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.html");
    let items = parse_and_extract(source);
    let c = find_by_name(&items, "game-canvas");
    assert_eq!(c.kind, SymbolKind::Static);
    assert_eq!(c.metadata.tag_name.as_deref(), Some("canvas"));
}

#[test]
fn canvas_dimensions() {
    let source = include_str!("../../../../tests/fixtures/sample.html");
    let items = parse_and_extract(source);
    let c = find_by_name(&items, "game-canvas");
    assert!(
        c.metadata
            .html_attributes
            .iter()
            .any(|(n, v)| n == "width" && v.as_deref() == Some("800")),
        "attrs: {:?}",
        c.metadata.html_attributes
    );
}

// ── Fieldset / Select / Output tests ───────────────────────────
