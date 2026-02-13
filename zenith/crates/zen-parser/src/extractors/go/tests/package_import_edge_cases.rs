use super::*;

#[test]
fn extracts_package_clause_as_module() {
    let source = "package demo\nfunc x() {}";
    let items = parse_and_extract(source);
    let pkg = find_by_name(&items, "demo");
    assert_eq!(pkg.kind, SymbolKind::Module);
}

#[test]
fn extracts_import_specs_with_alias_tags() {
    let source = r#"package demo
import (
    f "fmt"
    . "strings"
    _ "net/http/pprof"
)
"#;
    let items = parse_and_extract(source);

    let fmt = items
        .iter()
        .find(|i| i.kind == SymbolKind::Module && i.name.contains("fmt as f"))
        .expect("expected aliased fmt import");
    assert!(
        fmt.metadata
            .attributes
            .iter()
            .any(|a| a == "go:import_alias:f")
    );

    assert!(items.iter().any(|i| {
        i.kind == SymbolKind::Module
            && i.metadata
                .attributes
                .iter()
                .any(|a| a == "go:import_alias:.")
    }));
    assert!(items.iter().any(|i| {
        i.kind == SymbolKind::Module
            && i.metadata
                .attributes
                .iter()
                .any(|a| a == "go:import_alias:_")
    }));
}
