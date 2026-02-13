use super::*;

#[test]
fn class_and_module_names_are_extracted() {
    let items = fixture_items();

    let billing = find_by_name(&items, "Billing");
    assert_eq!(billing.kind, SymbolKind::Module);

    let invoice = find_by_name(&items, "Billing::Invoice");
    assert_eq!(invoice.kind, SymbolKind::Class);
    assert!(
        invoice
            .metadata
            .attributes
            .iter()
            .any(|attribute| attribute == "rails:kind:model")
    );

    let payments_controller = find_by_name(&items, "Billing::PaymentsController");
    assert_eq!(payments_controller.kind, SymbolKind::Class);
    assert!(
        payments_controller
            .metadata
            .attributes
            .iter()
            .any(|attribute| attribute == "rails:kind:controller")
    );
}
