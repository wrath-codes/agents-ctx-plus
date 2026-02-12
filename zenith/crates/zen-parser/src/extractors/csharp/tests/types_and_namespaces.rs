use super::*;

#[test]
fn extracts_top_level_types_and_namespace() {
    let items = fixture_items();

    assert_eq!(
        find_by_name(&items, "Zenith.Sample").kind,
        SymbolKind::Module
    );
    assert_eq!(find_by_name(&items, "Widget").kind, SymbolKind::Class);
    assert_eq!(find_by_name(&items, "IData").kind, SymbolKind::Interface);
    assert_eq!(find_by_name(&items, "DataPoint").kind, SymbolKind::Struct);
    assert_eq!(find_by_name(&items, "Status").kind, SymbolKind::Enum);
    assert_eq!(
        find_by_name(&items, "Transformer").kind,
        SymbolKind::TypeAlias
    );
}

#[test]
fn extracts_file_scoped_namespace() {
    let source = "namespace Core.Tools; public class Runner { }";
    let items = parse_and_extract(source);
    assert_eq!(find_by_name(&items, "Core.Tools").kind, SymbolKind::Module);
}
