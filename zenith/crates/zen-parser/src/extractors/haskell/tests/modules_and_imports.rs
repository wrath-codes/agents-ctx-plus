use super::*;

#[test]
fn extracts_module_header_and_imports_as_modules() {
    let items = fixture_items();

    let module = find_by_name(&items, "Zenith.Sample");
    assert_eq!(module.kind, SymbolKind::Module);

    for import_name in [
        "Data.Text",
        "Data.Maybe",
        "Foreign.C.String",
        "Foreign.C.Types",
    ] {
        let import = find_by_name(&items, import_name);
        assert_eq!(import.kind, SymbolKind::Module);
    }

    let modules = find_all_by_kind(&items, SymbolKind::Module);
    assert!(modules.len() >= 5);
}
