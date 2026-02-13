use super::*;

#[test]
fn rails_model_dsl_symbols_are_extracted() {
    let items = fixture_items();

    let account = find_by_name(&items, "account");
    assert_eq!(account.kind, SymbolKind::Property);
    assert_eq!(
        account.metadata.owner_name.as_deref(),
        Some("Billing::Invoice")
    );
    assert!(account
        .metadata
        .attributes
        .iter()
        .any(|attribute| attribute == "rails:belongs_to"));

    let line_items = find_by_name(&items, "line_items");
    assert_eq!(line_items.kind, SymbolKind::Property);
    assert!(line_items
        .metadata
        .attributes
        .iter()
        .any(|attribute| attribute == "rails:has_many"));

    let recent = find_by_name(&items, "recent");
    assert_eq!(recent.kind, SymbolKind::Method);
    assert!(recent.metadata.is_static_member);
    assert!(recent
        .metadata
        .attributes
        .iter()
        .any(|attribute| attribute == "rails:scope"));

    let validates = find_by_name(&items, "validates");
    assert_eq!(validates.kind, SymbolKind::Module);
    assert!(validates
        .metadata
        .attributes
        .iter()
        .any(|attribute| attribute == "rails:validates"));
}

#[test]
fn rails_controller_callbacks_are_tagged() {
    let items = fixture_items();

    let before_action = find_by_name(&items, "before_action");
    assert_eq!(before_action.kind, SymbolKind::Module);
    assert_eq!(
        before_action.metadata.owner_name.as_deref(),
        Some("Billing::PaymentsController")
    );
    assert!(before_action
        .metadata
        .attributes
        .iter()
        .any(|attribute| attribute == "rails:before_action"));
}
