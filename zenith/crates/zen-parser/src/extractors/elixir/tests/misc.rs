use super::*;

#[test]
fn defmodule_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.ex");
    let items = parse_and_extract(source);
    let m = find_by_name(&items, "Sample.Processor");
    assert_eq!(m.kind, SymbolKind::Module);
    assert_eq!(m.visibility, Visibility::Public);
}

#[test]
fn moduledoc_heredoc_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.ex");
    let items = parse_and_extract(source);
    let m = find_by_name(&items, "Sample.Processor");
    assert!(
        m.doc_comment.contains("A sample processor module."),
        "doc: {:?}",
        m.doc_comment
    );
    assert!(
        m.doc_comment.contains("configurable strategies"),
        "doc should contain full text: {:?}",
        m.doc_comment
    );
}

#[test]
fn moduledoc_inline_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.ex");
    let items = parse_and_extract(source);
    let m = find_by_name(&items, "Sample.Config");
    assert_eq!(m.doc_comment, "Configuration struct.");
}

#[test]
fn moduledoc_false_is_empty() {
    let source = include_str!("../../../../tests/fixtures/sample.ex");
    let items = parse_and_extract(source);
    let m = find_by_name(&items, "Sample.Internal");
    assert_eq!(m.doc_comment, "");
}

#[test]
fn module_methods_listed() {
    let source = include_str!("../../../../tests/fixtures/sample.ex");
    let items = parse_and_extract(source);
    let m = find_by_name(&items, "Sample.Processor");
    assert!(
        m.metadata.methods.contains(&"process".to_string()),
        "methods: {:?}",
        m.metadata.methods
    );
    assert!(
        m.metadata.methods.contains(&"process_one".to_string()),
        "methods: {:?}",
        m.metadata.methods
    );
}

#[test]
fn all_modules_found() {
    let source = include_str!("../../../../tests/fixtures/sample.ex");
    let items = parse_and_extract(source);
    let module_names: Vec<_> = items
        .iter()
        .filter(|i| i.kind == SymbolKind::Module)
        .map(|i| i.name.as_str())
        .collect();
    assert!(module_names.contains(&"Sample.Processor"));
    assert!(module_names.contains(&"Sample.Config"));
    assert!(module_names.contains(&"Sample.Worker"));
    assert!(module_names.contains(&"Sample.Behaviour"));
    assert!(module_names.contains(&"Sample.Guards"));
    assert!(module_names.contains(&"Sample.Types"));
    assert!(module_names.contains(&"Sample.Constants"));
    assert!(module_names.contains(&"Sample.AppError"));
    assert!(module_names.contains(&"Sample.CustomGuards"));
    assert!(module_names.contains(&"Sample.Delegator"));
    assert!(module_names.contains(&"Sample.Internal"));
}
