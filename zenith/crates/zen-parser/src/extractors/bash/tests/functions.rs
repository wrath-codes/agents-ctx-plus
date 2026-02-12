use super::*;

#[test]
fn fixture_parses_without_error() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    assert!(
        !items.is_empty(),
        "should extract at least one item from fixture"
    );
}

#[test]
fn fixture_item_count() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    // The fixture has many constructs — verify a reasonable minimum
    assert!(
        items.len() >= 25,
        "expected at least 25 items, got {}",
        items.len()
    );
}

#[test]
fn shebang_detected() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    let shebang = find_by_name(&items, "shebang");
    assert_eq!(shebang.kind, SymbolKind::Macro);
    assert!(shebang.signature.contains("#!/bin/bash"));
}

#[test]
fn shebang_env_style() {
    let items = parse_and_extract("#!/usr/bin/env bash\necho hello");
    let shebang = find_by_name(&items, "shebang");
    assert!(
        shebang
            .metadata
            .attributes
            .iter()
            .any(|a| a.contains("env bash")),
        "should detect env-style shebang: {:?}",
        shebang.metadata.attributes
    );
}

#[test]
fn no_shebang_no_item() {
    let items = parse_and_extract("echo hello");
    assert!(
        items.iter().all(|i| i.name != "shebang"),
        "should not emit shebang for scripts without one"
    );
}

#[test]
fn function_parens_style() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "greet");
    assert_eq!(f.kind, SymbolKind::Function);
    assert!(f.signature.contains("greet()"));
}

#[test]
fn function_keyword_style() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "cleanup");
    assert_eq!(f.kind, SymbolKind::Function);
    assert!(f.signature.contains("function cleanup"));
}

#[test]
fn function_both_keyword_and_parens() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "deploy");
    assert_eq!(f.kind, SymbolKind::Function);
    assert!(f.signature.contains("function deploy()"));
}

#[test]
fn function_doc_comment() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "greet");
    assert!(
        f.doc_comment.contains("Greet a user"),
        "expected doc comment, got: {:?}",
        f.doc_comment
    );
}

#[test]
fn function_multi_line_doc() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "greet");
    assert!(
        f.doc_comment.contains('\n'),
        "expected multi-line doc comment: {:?}",
        f.doc_comment
    );
}

#[test]
fn function_has_function_keyword_attribute() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "cleanup");
    assert!(
        f.metadata
            .attributes
            .contains(&"function_keyword".to_string()),
        "should have function_keyword attribute: {:?}",
        f.metadata.attributes
    );
}

#[test]
fn function_inline_oneliner() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "say_hi");
    assert_eq!(f.kind, SymbolKind::Function);
}

#[test]
fn function_count() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    let fns = find_all_by_kind(&items, SymbolKind::Function);
    // greet, cleanup, deploy, say_hi, process_data + trap items
    assert!(
        fns.len() >= 5,
        "expected at least 5 function-kind items, got {}",
        fns.len()
    );
}

#[test]
fn process_data_function() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "process_data");
    assert_eq!(f.kind, SymbolKind::Function);
    assert!(
        f.doc_comment.contains("Process data"),
        "expected doc comment: {:?}",
        f.doc_comment
    );
}
