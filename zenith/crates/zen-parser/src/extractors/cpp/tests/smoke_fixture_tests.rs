use super::*;

// ════════════════════════════════════════════════════════════════
// 1. Smoke / fixture tests
// ════════════════════════════════════════════════════════════════

#[test]
fn fixture_parses_without_error() {
    let items = fixture_items();
    assert!(items.len() >= 40, "expected 40+ items, got {}", items.len());
}

#[test]
fn fixture_has_classes() {
    let items = fixture_items();
    let classes = find_all_by_kind(&items, SymbolKind::Class);
    assert!(
        classes.len() >= 15,
        "expected 15+ classes, got {}",
        classes.len()
    );
}

#[test]
fn fixture_has_functions() {
    let items = fixture_items();
    let funcs = find_all_by_kind(&items, SymbolKind::Function);
    assert!(
        funcs.len() >= 15,
        "expected 15+ functions, got {}",
        funcs.len()
    );
}

#[test]
fn fixture_has_enums() {
    let items = fixture_items();
    let enums = find_all_by_kind(&items, SymbolKind::Enum);
    assert!(enums.len() >= 3, "expected 3+ enums, got {}", enums.len());
}

#[test]
fn fixture_has_structs() {
    let items = fixture_items();
    let structs = find_all_by_kind(&items, SymbolKind::Struct);
    assert!(
        structs.len() >= 3,
        "expected 3+ structs, got {}",
        structs.len()
    );
}

#[test]
fn fixture_has_modules() {
    let items = fixture_items();
    let mods = find_all_by_kind(&items, SymbolKind::Module);
    assert!(mods.len() >= 10, "expected 10+ modules, got {}", mods.len());
}

#[test]
fn fixture_has_type_aliases() {
    let items = fixture_items();
    let aliases = find_all_by_kind(&items, SymbolKind::TypeAlias);
    assert!(
        aliases.len() >= 5,
        "expected 5+ type aliases, got {}",
        aliases.len()
    );
}

#[test]
fn fixture_has_consts() {
    let items = fixture_items();
    let consts = find_all_by_kind(&items, SymbolKind::Const);
    assert!(
        consts.len() >= 5,
        "expected 5+ consts, got {}",
        consts.len()
    );
}

#[test]
fn fixture_has_traits() {
    let items = fixture_items();
    let traits = find_all_by_kind(&items, SymbolKind::Trait);
    assert!(
        traits.len() >= 2,
        "expected 2+ traits (concepts), got {}",
        traits.len()
    );
}

#[test]
fn fixture_has_macros() {
    let items = fixture_items();
    let macros = find_all_by_kind(&items, SymbolKind::Macro);
    assert!(
        macros.len() >= 3,
        "expected 3+ macros (static_assert + defines), got {}",
        macros.len()
    );
}
