use super::*;

#[test]
fn alias_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    let a = find_by_name(&items, "ll");
    assert_eq!(a.kind, SymbolKind::Static);
    assert!(a.signature.contains("alias"));
}

#[test]
fn alias_doc_comment() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    let a = find_by_name(&items, "ll");
    assert!(
        a.doc_comment.contains("List files"),
        "expected doc comment on alias: {:?}",
        a.doc_comment
    );
}

#[test]
fn alias_count() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    let alias_count = items
        .iter()
        .filter(|i| i.metadata.attributes.contains(&"alias".to_string()))
        .count();
    assert_eq!(alias_count, 3, "should find 3 aliases: ll, gs, gp");
}

#[test]
fn trap_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    let traps: Vec<_> = items
        .iter()
        .filter(|i| i.name.starts_with("trap "))
        .collect();
    assert!(
        traps.len() >= 2,
        "should find at least 2 traps, got {}",
        traps.len()
    );
}

#[test]
fn trap_has_signals() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    let trap = items
        .iter()
        .find(|i| i.name.contains("EXIT"))
        .expect("should find trap for EXIT signal");
    assert_eq!(trap.kind, SymbolKind::Function);
    assert!(
        trap.metadata.attributes.contains(&"EXIT".to_string()),
        "should have EXIT signal: {:?}",
        trap.metadata.attributes
    );
}

#[test]
fn trap_doc_comment() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    let trap = items
        .iter()
        .find(|i| i.name.contains("EXIT"))
        .expect("should find EXIT trap");
    assert!(
        trap.doc_comment.contains("Clean up on exit"),
        "expected doc comment: {:?}",
        trap.doc_comment
    );
}

#[test]
fn negated_command_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    let n = items
        .iter()
        .find(|i| i.metadata.attributes.contains(&"negated".to_string()))
        .expect("should find negated command");
    assert_eq!(n.kind, SymbolKind::Macro);
    assert!(n.name.starts_with("! "));
}

#[test]
fn negated_command_doc_comment() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    let n = items
        .iter()
        .find(|i| i.metadata.attributes.contains(&"negated".to_string()))
        .expect("should find negated command");
    assert!(
        n.doc_comment.contains("Negate grep"),
        "expected doc comment: {:?}",
        n.doc_comment
    );
}

#[test]
fn test_command_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    let t = items
        .iter()
        .find(|i| i.metadata.attributes.contains(&"test".to_string()))
        .expect("should find test command");
    assert_eq!(t.kind, SymbolKind::Macro);
    assert!(t.name.starts_with("test "));
}

#[test]
fn test_command_doc_comment() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    let t = items
        .iter()
        .find(|i| i.metadata.attributes.contains(&"test".to_string()))
        .expect("should find test command");
    assert!(
        t.doc_comment.contains("Standalone test"),
        "expected doc comment: {:?}",
        t.doc_comment
    );
}

#[test]
fn unset_variable_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    let u = find_by_name(&items, "unset TEMP_VAR");
    assert_eq!(u.kind, SymbolKind::Static);
    assert!(
        u.metadata.attributes.contains(&"unset".to_string()),
        "should have unset attribute: {:?}",
        u.metadata.attributes
    );
}

#[test]
fn unset_variable_doc_comment() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    let u = find_by_name(&items, "unset TEMP_VAR");
    assert!(
        u.doc_comment.contains("Remove a variable"),
        "expected doc comment: {:?}",
        u.doc_comment
    );
}

#[test]
fn unset_function_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    let u = find_by_name(&items, "unset old_func");
    assert_eq!(u.kind, SymbolKind::Function);
    assert!(
        u.metadata.attributes.contains(&"-f".to_string()),
        "should have -f flag: {:?}",
        u.metadata.attributes
    );
}

#[test]
fn unset_function_doc_comment() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    let u = find_by_name(&items, "unset old_func");
    assert!(
        u.doc_comment.contains("Remove a function"),
        "expected doc comment: {:?}",
        u.doc_comment
    );
}

#[test]
fn unset_count() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    let unset_count = items
        .iter()
        .filter(|i| i.name.starts_with("unset "))
        .count();
    assert_eq!(unset_count, 2, "should find 2 unset commands");
}
