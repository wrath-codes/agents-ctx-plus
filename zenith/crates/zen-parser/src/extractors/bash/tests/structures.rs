use super::*;

#[test]
fn heredoc_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    let hd = items
        .iter()
        .find(|i| i.name.starts_with("heredoc "))
        .expect("should find heredoc");
    assert_eq!(hd.kind, SymbolKind::Const);
}

#[test]
fn heredoc_delimiter() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    let hd = find_by_name(&items, "heredoc EOF");
    assert!(
        hd.metadata.attributes.contains(&"heredoc".to_string()),
        "should have heredoc attribute: {:?}",
        hd.metadata.attributes
    );
}

#[test]
fn heredoc_indented() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    let hd = find_by_name(&items, "heredoc INDENTED");
    assert!(
        hd.signature.contains("<<-"),
        "should contain indented heredoc operator: {:?}",
        hd.signature
    );
}

#[test]
fn heredoc_count() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    let heredocs: Vec<_> = items
        .iter()
        .filter(|i| i.name.starts_with("heredoc "))
        .collect();
    assert!(
        heredocs.len() >= 2,
        "should find at least 2 heredocs, got {}",
        heredocs.len()
    );
}

#[test]
fn subshell_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    let sub = items
        .iter()
        .find(|i| i.name.starts_with("subshell "))
        .expect("should find subshell");
    assert_eq!(sub.kind, SymbolKind::Macro);
}

#[test]
fn command_group_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    let cg = items
        .iter()
        .find(|i| i.name.starts_with("command_group "))
        .expect("should find command group");
    assert_eq!(cg.kind, SymbolKind::Macro);
}

#[test]
fn pipeline_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    let pipes: Vec<_> = items
        .iter()
        .filter(|i| i.metadata.attributes.contains(&"pipeline".to_string()))
        .collect();
    assert!(
        pipes.len() >= 2,
        "should find at least 2 pipelines, got {}",
        pipes.len()
    );
}

#[test]
fn pipeline_has_commands() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    let pipe = items
        .iter()
        .find(|i| i.metadata.attributes.contains(&"pipeline".to_string()))
        .expect("should find pipeline");
    assert!(
        pipe.metadata.parameters.len() >= 2,
        "pipeline should have at least 2 commands: {:?}",
        pipe.metadata.parameters
    );
}

#[test]
fn command_substitution_in_variable() {
    let source = include_str!("../../../../tests/fixtures/sample.sh");
    let items = parse_and_extract(source);
    let v = find_by_name(&items, "CURRENT_DATE");
    assert_eq!(v.kind, SymbolKind::Static);
    assert!(
        v.source.as_deref().unwrap_or("").contains("$("),
        "should contain command substitution: {:?}",
        v.source
    );
}
