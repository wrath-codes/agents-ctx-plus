use super::*;

#[test]
fn extracts_functions_and_dedupes_signature_definition_pairs() {
    let items = fixture_items();

    for fn_name in ["version", "mkWidget", "classify", "onChange"] {
        let item = find_by_name(&items, fn_name);
        assert_eq!(item.kind, SymbolKind::Function);
        assert!(
            item.signature.contains(fn_name),
            "signature should mention {fn_name}"
        );
    }

    for fn_name in ["mkWidget", "classify", "onChange"] {
        let count = items
            .iter()
            .filter(|i| i.kind == SymbolKind::Function && i.name == fn_name)
            .count();
        assert_eq!(count, 1, "expected one merged symbol for {fn_name}");
    }
}
