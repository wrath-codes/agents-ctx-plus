use super::*;

// ── Fixture parsing ───────────────────────────────────────────

#[test]
fn fixture_parses_without_error() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    assert!(items.len() >= 40, "expected 40+ items, got {}", items.len());
}

#[test]
fn fixture_has_functions() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let functions = find_all_by_kind(&items, SymbolKind::Function);
    assert!(
        functions.len() >= 10,
        "expected 10+ functions, got {}",
        functions.len()
    );
}

#[test]
fn fixture_has_structs() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let structs = find_all_by_kind(&items, SymbolKind::Struct);
    assert!(
        structs.len() >= 4,
        "expected 4+ structs, got {}",
        structs.len()
    );
}

#[test]
fn fixture_has_enums() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let enums = find_all_by_kind(&items, SymbolKind::Enum);
    assert!(enums.len() >= 3, "expected 3+ enums, got {}", enums.len());
}

#[test]
fn fixture_has_unions() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let unions = find_all_by_kind(&items, SymbolKind::Union);
    assert!(
        unions.len() >= 2,
        "expected 2+ unions, got {}",
        unions.len()
    );
}

#[test]
fn fixture_has_typedefs() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let typedefs = items
        .iter()
        .filter(|i| i.kind == SymbolKind::TypeAlias)
        .count();
    assert!(typedefs >= 3, "expected 3+ typedefs, got {typedefs}",);
}

#[test]
fn fixture_has_modules() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let modules = find_all_by_kind(&items, SymbolKind::Module);
    assert!(
        modules.len() >= 5,
        "expected 5+ #include modules, got {}",
        modules.len()
    );
}

#[test]
fn fixture_has_macros() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let macros = find_all_by_kind(&items, SymbolKind::Macro);
    assert!(
        macros.len() >= 5,
        "expected 5+ macros, got {}",
        macros.len()
    );
}
