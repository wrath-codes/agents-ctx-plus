use super::*;

#[test]
fn if_statement_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    assert!(
        items.iter().any(|i| i.name.starts_with("if ")),
        "should find at least one if statement"
    );
}

#[test]
fn if_has_elif() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    let if_stmt = items
        .iter()
        .find(|i| {
            i.name.starts_with("if ") && i.metadata.attributes.iter().any(|a| a.contains("elif"))
        })
        .expect("should find if with elif");
    assert_eq!(if_stmt.kind, SymbolKind::Enum);
}

#[test]
fn if_doc_comment() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    let if_stmt = items
        .iter()
        .find(|i| i.name.starts_with("if "))
        .expect("should find if statement");
    assert!(
        if_stmt.doc_comment.contains("Check if a file"),
        "expected doc comment: {:?}",
        if_stmt.doc_comment
    );
}

#[test]
fn case_statement_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    let cs = items
        .iter()
        .find(|i| i.name.starts_with("case "))
        .expect("should find case statement");
    assert_eq!(cs.kind, SymbolKind::Enum);
}

#[test]
fn case_has_patterns() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    let cs = items
        .iter()
        .find(|i| i.name.starts_with("case ") && !i.metadata.variants.is_empty())
        .expect("should find case with patterns");
    assert!(
        cs.metadata.variants.len() >= 3,
        "should find at least 3 patterns: {:?}",
        cs.metadata.variants
    );
}

#[test]
fn case_doc_comment() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    let cs = items
        .iter()
        .find(|i| i.name.starts_with("case "))
        .expect("should find case");
    assert!(
        cs.doc_comment.contains("command routing"),
        "expected doc comment: {:?}",
        cs.doc_comment
    );
}

#[test]
fn for_loop_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    let f = items
        .iter()
        .find(|i| i.name.starts_with("for i"))
        .expect("should find for loop");
    assert_eq!(f.kind, SymbolKind::Macro);
    assert!(
        f.metadata.attributes.contains(&"for".to_string()),
        "should have for attribute: {:?}",
        f.metadata.attributes
    );
}

#[test]
fn for_loop_has_variable() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    let f = items
        .iter()
        .find(|i| i.name.starts_with("for i"))
        .expect("should find for loop");
    assert!(
        f.metadata.parameters.contains(&"i".to_string()),
        "should have loop var 'i': {:?}",
        f.metadata.parameters
    );
}

#[test]
fn for_loop_doc_comment() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    let f = items
        .iter()
        .find(|i| i.name.starts_with("for i"))
        .expect("should find for loop");
    assert!(
        f.doc_comment.contains("Iterate over numbers"),
        "expected doc comment: {:?}",
        f.doc_comment
    );
}

#[test]
fn while_loop_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    let w = items
        .iter()
        .find(|i| i.name.starts_with("while "))
        .expect("should find while loop");
    assert_eq!(w.kind, SymbolKind::Macro);
}

#[test]
fn until_loop_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    let u = items
        .iter()
        .find(|i| i.name.starts_with("until "))
        .expect("should find until loop");
    assert_eq!(u.kind, SymbolKind::Macro);
    assert!(
        u.metadata.attributes.contains(&"until".to_string()),
        "should have until attribute: {:?}",
        u.metadata.attributes
    );
}

#[test]
fn select_statement_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    let s = items
        .iter()
        .find(|i| i.name.starts_with("select "))
        .expect("should find select statement");
    assert_eq!(s.kind, SymbolKind::Enum);
    assert!(
        s.metadata.attributes.contains(&"select".to_string()),
        "should have select attribute"
    );
}

#[test]
fn c_style_for_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    let f = items
        .iter()
        .find(|i| i.metadata.attributes.contains(&"c_style".to_string()))
        .expect("should find c-style for loop");
    assert_eq!(f.kind, SymbolKind::Macro);
}

#[test]
fn c_style_for_has_for_attr() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    let f = items
        .iter()
        .find(|i| i.metadata.attributes.contains(&"c_style".to_string()))
        .expect("should find c-style for");
    assert!(
        f.metadata.attributes.contains(&"for".to_string()),
        "should have 'for' attribute: {:?}",
        f.metadata.attributes
    );
}

#[test]
fn c_style_for_doc_comment() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    let f = items
        .iter()
        .find(|i| i.metadata.attributes.contains(&"c_style".to_string()))
        .expect("should find c-style for");
    assert!(
        f.doc_comment.contains("C-style for loop"),
        "expected doc comment: {:?}",
        f.doc_comment
    );
}

#[test]
fn c_style_for_count() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    let c_for_count = items
        .iter()
        .filter(|i| i.metadata.attributes.contains(&"c_style".to_string()))
        .count();
    assert_eq!(c_for_count, 2, "should find 2 c-style for loops");
}
