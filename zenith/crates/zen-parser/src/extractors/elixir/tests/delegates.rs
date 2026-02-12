use super::*;

#[test]
fn defdelegate_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.ex");
    let items = parse_and_extract(source);
    // There are multiple "process" functions — find the delegate one
    let delegates: Vec<_> = items
        .iter()
        .filter(|i| i.metadata.for_type.as_deref() == Some("Sample.Processor"))
        .collect();
    assert!(!delegates.is_empty(), "should find delegated process");
    assert_eq!(delegates[0].kind, SymbolKind::Function);
    assert_eq!(delegates[0].visibility, Visibility::Public);
}

#[test]
fn defdelegate_target_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.ex");
    let items = parse_and_extract(source);
    let delegates: Vec<_> = items
        .iter()
        .filter(|i| i.metadata.for_type.as_deref() == Some("Sample.Config"))
        .collect();
    assert!(!delegates.is_empty(), "should find delegated new");
    assert_eq!(
        delegates[0].metadata.for_type.as_deref(),
        Some("Sample.Config")
    );
}

#[test]
fn defdelegate_signature_format() {
    let source = include_str!("../../../../tests/fixtures/sample.ex");
    let items = parse_and_extract(source);
    let delegates: Vec<_> = items
        .iter()
        .filter(|i| i.metadata.for_type.as_deref() == Some("Sample.Processor"))
        .collect();
    assert!(!delegates.is_empty());
    assert!(
        delegates[0].signature.starts_with("defdelegate"),
        "sig: {:?}",
        delegates[0].signature
    );
}

#[test]
fn delegator_module_methods_include_delegates() {
    let source = include_str!("../../../../tests/fixtures/sample.ex");
    let items = parse_and_extract(source);
    let m = find_by_name(&items, "Sample.Delegator");
    assert!(
        m.metadata.methods.contains(&"process".to_string()),
        "methods should include delegated 'process': {:?}",
        m.metadata.methods
    );
}
