use super::*;

// ════════════════════════════════════════════════════════════════
// 8. Constexpr / consteval / constinit tests
// ════════════════════════════════════════════════════════════════

#[test]
fn constexpr_max_elements() {
    let items = fixture_items();
    let me = find_by_name(&items, "MAX_ELEMENTS");
    assert_eq!(me.kind, SymbolKind::Const, "MAX_ELEMENTS should be Const");
    assert!(
        me.metadata.attributes.contains(&"constexpr".to_string()),
        "MAX_ELEMENTS should have constexpr attribute"
    );
}

#[test]
fn constexpr_pi() {
    let items = fixture_items();
    let pi = find_by_name(&items, "PI");
    assert_eq!(pi.kind, SymbolKind::Const, "PI should be Const");
}

#[test]
fn constexpr_buffer_size() {
    let items = fixture_items();
    let bs = find_by_name(&items, "BUFFER_SIZE");
    assert_eq!(bs.kind, SymbolKind::Const, "BUFFER_SIZE should be Const");
}

#[test]
fn constinit_global() {
    let items = fixture_items();
    let g = find_by_name(&items, "global_init_val");
    assert_eq!(g.kind, SymbolKind::Const, "global_init_val should be Const");
    assert!(
        g.metadata.attributes.contains(&"constinit".to_string()),
        "global_init_val should have constinit attribute"
    );
}

#[test]
fn constexpr_factorial_function() {
    let items = fixture_items();
    let f = find_by_name(&items, "factorial");
    assert_eq!(f.kind, SymbolKind::Function);
    assert!(
        f.metadata.attributes.contains(&"constexpr".to_string()),
        "factorial should have constexpr attribute"
    );
}

#[test]
fn constexpr_compile_time_square() {
    let items = fixture_items();
    let f = find_by_name(&items, "compile_time_square");
    assert_eq!(f.kind, SymbolKind::Function);
    assert!(
        f.metadata.attributes.contains(&"constexpr".to_string()),
        "compile_time_square should have constexpr attribute"
    );
}

#[test]
fn consteval_compile_only_double() {
    let items = fixture_items();
    let f = find_by_name(&items, "compile_only_double");
    assert_eq!(f.kind, SymbolKind::Function);
    assert!(
        f.metadata.attributes.contains(&"consteval".to_string()),
        "compile_only_double should have consteval attribute"
    );
}

#[test]
fn noexcept_safe_divide() {
    let items = fixture_items();
    let f = find_by_name(&items, "safe_divide");
    assert_eq!(f.kind, SymbolKind::Function);
    assert!(
        f.metadata.attributes.contains(&"noexcept".to_string()),
        "safe_divide should have noexcept attribute"
    );
}
