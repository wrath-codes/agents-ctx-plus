use super::*;

// ── Non-component items stay unchanged ─────────────────────────

#[test]
fn format_date_not_component() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "formatDate");
    assert_eq!(f.kind, SymbolKind::Function);
    assert!(!f.metadata.is_component);
    assert!(!f.metadata.is_hook);
}

#[test]
fn api_url_not_component() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let items = parse_and_extract(source);
    let c = find_by_name(&items, "API_URL");
    assert_eq!(c.kind, SymbolKind::Const);
    assert!(!c.metadata.is_component);
}

#[test]
fn theme_type_alias_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let items = parse_and_extract(source);
    let t = find_by_name(&items, "Theme");
    assert_eq!(t.kind, SymbolKind::TypeAlias);
}

#[test]
fn status_enum_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let items = parse_and_extract(source);
    let e = find_by_name(&items, "Status");
    assert_eq!(e.kind, SymbolKind::Enum);
    assert!(e.metadata.variants.contains(&"Active".to_string()));
    assert!(e.metadata.variants.contains(&"Inactive".to_string()));
}

// ── Props via type alias ───────────────────────────────────────

#[test]
fn card_props_type_alias_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let items = parse_and_extract(source);
    let t = find_by_name(&items, "CardProps");
    assert_eq!(t.kind, SymbolKind::TypeAlias);
}

// ── Interfaces extracted ───────────────────────────────────────

#[test]
fn button_props_interface_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let items = parse_and_extract(source);
    let i = find_by_name(&items, "ButtonProps");
    assert_eq!(i.kind, SymbolKind::Interface);
}

#[test]
fn user_card_props_interface_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let items = parse_and_extract(source);
    let i = find_by_name(&items, "UserCardProps");
    assert_eq!(i.kind, SymbolKind::Interface);
}

#[test]
fn list_props_interface_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let items = parse_and_extract(source);
    let i = find_by_name(&items, "ListProps");
    assert_eq!(i.kind, SymbolKind::Interface);
}

#[test]
fn input_props_interface_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let items = parse_and_extract(source);
    let i = find_by_name(&items, "InputProps");
    assert_eq!(i.kind, SymbolKind::Interface);
}

#[test]
fn avatar_props_interface_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let items = parse_and_extract(source);
    let i = find_by_name(&items, "AvatarProps");
    assert_eq!(i.kind, SymbolKind::Interface);
}

#[test]
fn eb_props_interface_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let items = parse_and_extract(source);
    let i = find_by_name(&items, "EBProps");
    assert_eq!(i.kind, SymbolKind::Interface);
}

#[test]
fn eb_state_interface_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let items = parse_and_extract(source);
    let i = find_by_name(&items, "EBState");
    assert_eq!(i.kind, SymbolKind::Interface);
}

#[test]
fn theme_context_value_interface() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let items = parse_and_extract(source);
    let i = find_by_name(&items, "ThemeContextValue");
    assert_eq!(i.kind, SymbolKind::Interface);
}

#[test]
fn counter_class_props_interface() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let items = parse_and_extract(source);
    let i = find_by_name(&items, "CounterClassProps");
    assert_eq!(i.kind, SymbolKind::Interface);
}

#[test]
fn todo_state_interface_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let items = parse_and_extract(source);
    let i = find_by_name(&items, "TodoState");
    assert_eq!(i.kind, SymbolKind::Interface);
}

// ── Context const ──────────────────────────────────────────────

#[test]
fn theme_context_const_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let items = parse_and_extract(source);
    let c = find_by_name(&items, "ThemeContext");
    assert_eq!(c.kind, SymbolKind::Const);
    assert!(!c.metadata.is_component);
}
