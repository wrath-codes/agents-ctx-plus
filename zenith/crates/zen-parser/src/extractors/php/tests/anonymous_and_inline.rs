use super::*;

#[test]
fn extracts_arrow_closure_and_anonymous_class_symbols() {
    let items = fixture_items();

    let arrow = items
        .iter()
        .find(|i| i.kind == SymbolKind::Function && i.name.starts_with("<arrow@"))
        .expect("expected arrow function symbol");
    assert!(
        arrow
            .metadata
            .parameters
            .iter()
            .any(|p| p.contains("x: int"))
    );

    let closure = items
        .iter()
        .find(|i| i.kind == SymbolKind::Function && i.name.starts_with("<closure@"))
        .expect("expected closure symbol");
    assert!(
        closure
            .metadata
            .attributes
            .iter()
            .any(|a| a.contains("closure_use"))
    );
    assert!(
        closure
            .metadata
            .attributes
            .iter()
            .any(|a| a.starts_with("callable_alias:"))
    );
    assert!(
        closure
            .metadata
            .attributes
            .iter()
            .any(|a| a == "callable_origin:assignment")
    );

    let anon_class = items
        .iter()
        .find(|i| i.kind == SymbolKind::Class && i.name.starts_with("<anonymous_class@"))
        .expect("expected anonymous class symbol");
    assert!(!anon_class.name.is_empty());

    let anon_method = items
        .iter()
        .find(|i| i.kind == SymbolKind::Method && i.name == "value")
        .expect("expected anonymous class method symbol");
    assert!(
        anon_method
            .metadata
            .owner_name
            .as_deref()
            .is_some_and(|o| o.starts_with("<anonymous_class@"))
    );
}
