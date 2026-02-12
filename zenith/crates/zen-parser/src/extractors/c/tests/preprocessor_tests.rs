use super::*;

// ── Preprocessor tests ────────────────────────────────────────

#[test]
fn include_stdio() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let stdio = find_by_name(&items, "<stdio.h>");
    assert_eq!(stdio.kind, SymbolKind::Module);
    assert!(
        stdio.metadata.attributes.contains(&"system".to_string()),
        "should be a system include"
    );
}

#[test]
fn include_mylib() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let mylib = items
        .iter()
        .find(|i| i.name.contains("mylib"))
        .expect("should find mylib include");
    assert_eq!(mylib.kind, SymbolKind::Module);
    assert!(
        mylib.metadata.attributes.contains(&"local".to_string()),
        "should be a local include"
    );
}

#[test]
fn define_max_buffer() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let mb = find_by_name(&items, "MAX_BUFFER");
    assert_eq!(mb.kind, SymbolKind::Const);
    assert!(
        mb.signature.contains("1024"),
        "should have value 1024 in signature: {:?}",
        mb.signature
    );
}

#[test]
fn define_square_function_like() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let sq = find_by_name(&items, "SQUARE");
    assert_eq!(sq.kind, SymbolKind::Macro);
    assert!(
        sq.metadata
            .attributes
            .contains(&"function_like".to_string()),
        "should be function-like macro: {:?}",
        sq.metadata.attributes
    );
    assert!(
        sq.metadata.parameters.contains(&"x".to_string()),
        "should have parameter 'x': {:?}",
        sq.metadata.parameters
    );
}

#[test]
fn define_min_function_like() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let min = find_by_name(&items, "MIN");
    assert_eq!(min.kind, SymbolKind::Macro);
    assert_eq!(
        min.metadata.parameters.len(),
        2,
        "MIN should have 2 params: {:?}",
        min.metadata.parameters
    );
}

#[test]
fn define_debug_log_variadic_macro() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let dl = find_by_name(&items, "DEBUG_LOG");
    assert_eq!(dl.kind, SymbolKind::Macro);
    assert!(
        dl.metadata.parameters.len() >= 2,
        "DEBUG_LOG should have 2+ params: {:?}",
        dl.metadata.parameters
    );
}

#[test]
fn ifdef_debug_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let dbg = items
        .iter()
        .find(|i| i.name == "DEBUG" && i.metadata.attributes.contains(&"#ifdef".to_string()))
        .expect("should find #ifdef DEBUG");
    assert_eq!(dbg.kind, SymbolKind::Macro);
}

#[test]
fn ifndef_header_guard() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let hg = items
        .iter()
        .find(|i| i.name == "SAMPLE_H" && i.metadata.attributes.contains(&"#ifndef".to_string()))
        .expect("should find #ifndef SAMPLE_H header guard");
    assert_eq!(hg.kind, SymbolKind::Macro);
}

#[test]
fn pragma_once_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let pragma = items
        .iter()
        .find(|i| i.name == "once")
        .expect("should find #pragma once");
    assert_eq!(pragma.kind, SymbolKind::Macro);
}

#[test]
fn pragma_pack_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    assert!(
        items
            .iter()
            .filter(|i| i.metadata.attributes.contains(&"#pragma".to_string()))
            .count()
            >= 2,
        "should have at least 2 pragma directives"
    );
}
