use super::*;

#[test]
fn does_not_duplicate_type_member_items() {
    let source = r"package demo
type Reader interface { Close() error }
";

    let items = parse_and_extract(source);
    assert_eq!(
        items
            .iter()
            .filter(|i| i.kind == SymbolKind::Method && i.name == "Reader::Close")
            .count(),
        1
    );
}
