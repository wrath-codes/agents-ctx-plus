use super::*;

#[test]
fn list_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    let l = items
        .iter()
        .find(|i| i.metadata.attributes.contains(&"list".to_string()))
        .expect("should find list (logical chain)");
    assert_eq!(l.kind, SymbolKind::Macro);
}

#[test]
fn list_doc_comment() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    let l = items
        .iter()
        .find(|i| i.metadata.attributes.contains(&"list".to_string()))
        .expect("should find list");
    assert!(
        l.doc_comment.contains("Conditional chain"),
        "expected doc comment: {:?}",
        l.doc_comment
    );
}

#[test]
fn inline_list_and() {
    let items = parse_and_extract("true && echo ok");
    let l = items
        .iter()
        .find(|i| i.metadata.attributes.contains(&"list".to_string()))
        .expect("should find && list");
    assert_eq!(l.kind, SymbolKind::Macro);
}

#[test]
fn inline_list_or() {
    let items = parse_and_extract("false || echo fallback");
    let l = items
        .iter()
        .find(|i| i.metadata.attributes.contains(&"list".to_string()))
        .expect("should find || list");
    assert_eq!(l.kind, SymbolKind::Macro);
}

#[test]
fn inline_list_chain() {
    let items = parse_and_extract("cmd1 && cmd2 || cmd3");
    let l = items
        .iter()
        .find(|i| i.metadata.attributes.contains(&"list".to_string()))
        .expect("should find chain list");
    assert!(
        l.source.as_deref().unwrap_or("").contains("&&"),
        "source should contain &&: {:?}",
        l.source
    );
}
