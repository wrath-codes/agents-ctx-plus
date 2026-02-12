use super::*;

// ── forwardRef component ───────────────────────────────────────

#[test]
fn fancyinput_is_forward_ref() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let items = parse_and_extract(source);
    let fi = find_by_name(&items, "FancyInput");
    assert!(fi.metadata.is_forward_ref);
    assert!(fi.metadata.is_component);
    assert_eq!(fi.kind, SymbolKind::Component);
}

#[test]
fn fancyinput_has_jsx() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let items = parse_and_extract(source);
    let fi = find_by_name(&items, "FancyInput");
    assert!(
        fi.metadata.jsx_elements.contains(&"input".to_string()),
        "jsx: {:?}",
        fi.metadata.jsx_elements
    );
}

// ── React.memo ─────────────────────────────────────────────────

#[test]
fn memo_card_is_memo_component() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let items = parse_and_extract(source);
    let mc = find_by_name(&items, "MemoCard");
    assert!(mc.metadata.is_memo);
    assert!(mc.metadata.is_component);
    assert_eq!(mc.kind, SymbolKind::Component);
}

#[test]
fn memo_card_has_jsx() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let items = parse_and_extract(source);
    let mc = find_by_name(&items, "MemoCard");
    assert!(
        mc.metadata.jsx_elements.contains(&"div".to_string()),
        "jsx: {:?}",
        mc.metadata.jsx_elements
    );
}

#[test]
fn memo_avatar_is_memo_component() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let items = parse_and_extract(source);
    let ma = find_by_name(&items, "MemoAvatar");
    assert!(ma.metadata.is_memo);
    assert!(ma.metadata.is_component);
    assert_eq!(ma.kind, SymbolKind::Component);
}

#[test]
fn memo_avatar_has_jsx() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let items = parse_and_extract(source);
    let ma = find_by_name(&items, "MemoAvatar");
    assert!(
        ma.metadata.jsx_elements.contains(&"img".to_string()),
        "jsx: {:?}",
        ma.metadata.jsx_elements
    );
}

#[test]
fn memo_not_forward_ref() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let items = parse_and_extract(source);
    let mc = find_by_name(&items, "MemoCard");
    assert!(!mc.metadata.is_forward_ref);
}

// ── React.lazy ─────────────────────────────────────────────────

#[test]
fn lazy_settings_is_lazy() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let items = parse_and_extract(source);
    let ls = find_by_name(&items, "LazySettings");
    assert!(ls.metadata.is_lazy);
    assert!(
        !ls.metadata.is_component,
        "lazy import itself is not a component"
    );
}
