use super::*;

#[test]
fn extracts_namespace_use_clauses_with_aliases_and_kinds() {
    let items = fixture_items();

    let logger = items
        .iter()
        .find(|i| i.kind == SymbolKind::Module && i.name.contains("Psr\\Log\\LoggerInterface"))
        .expect("expected class import item");
    assert!(
        logger
            .metadata
            .attributes
            .iter()
            .any(|a| a == "import_kind:class")
    );

    let func_import = items
        .iter()
        .find(|i| i.kind == SymbolKind::Module && i.name.contains("strlen as str_len"))
        .expect("expected function import item");
    assert!(
        func_import
            .metadata
            .attributes
            .iter()
            .any(|a| a == "import_kind:function")
    );
    assert!(
        func_import
            .metadata
            .attributes
            .iter()
            .any(|a| a == "import_alias:str_len")
    );

    let const_import = items
        .iter()
        .find(|i| i.kind == SymbolKind::Module && i.name.contains("PHP_VERSION_ID"))
        .expect("expected const import item");
    assert!(
        const_import
            .metadata
            .attributes
            .iter()
            .any(|a| a == "import_kind:const")
    );

    assert!(items.iter().any(
        |i| i.kind == SymbolKind::Module && i.name.contains("Vendor\\Tools\\Formatter as Fmt")
    ));
    assert!(
        items
            .iter()
            .any(|i| i.kind == SymbolKind::Module && i.name.contains("Vendor\\Tools\\Runner"))
    );
}

#[test]
fn handles_grouped_and_clause_level_import_kind_overrides() {
    let source = r"
<?php
namespace Demo;
use function Vendor\Utils\{trim as t, clean};
use Vendor\Lib\{const A as A_CONST, function run as run_fn, Item};
";

    let items = parse_and_extract(source);

    let t = items
        .iter()
        .find(|i| i.kind == SymbolKind::Module && i.name.contains("trim as t"))
        .expect("expected grouped function import");
    assert!(
        t.metadata
            .attributes
            .iter()
            .any(|a| a == "import_kind:function")
    );

    let a_const = items
        .iter()
        .find(|i| i.kind == SymbolKind::Module && i.name.contains("A as A_CONST"))
        .expect("expected clause-level const import");
    assert!(
        a_const
            .metadata
            .attributes
            .iter()
            .any(|a| a == "import_kind:const")
    );

    let run_fn = items
        .iter()
        .find(|i| i.kind == SymbolKind::Module && i.name.contains("run as run_fn"))
        .expect("expected clause-level function import");
    assert!(
        run_fn
            .metadata
            .attributes
            .iter()
            .any(|a| a == "import_kind:function")
    );
}
