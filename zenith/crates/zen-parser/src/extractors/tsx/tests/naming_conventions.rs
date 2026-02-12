use super::*;

// ── Naming convention helpers ───────────────────────────────────

#[test]
fn hook_name_detection() {
    assert!(is_hook_name("useState"));
    assert!(is_hook_name("useEffect"));
    assert!(is_hook_name("useCustom"));
    assert!(is_hook_name("useReducer"));
    assert!(!is_hook_name("use"));
    assert!(!is_hook_name("useless"));
    assert!(!is_hook_name("User"));
}

#[test]
fn hoc_name_detection() {
    assert!(is_hoc_name("withLoading"));
    assert!(is_hoc_name("withAuth"));
    assert!(!is_hoc_name("with"));
    assert!(!is_hoc_name("without"));
    assert!(!is_hoc_name("Widget"));
}

#[test]
fn component_name_detection() {
    assert!(is_component_name("Button"));
    assert!(is_component_name("App"));
    assert!(!is_component_name("formatDate"));
    assert!(!is_component_name("useState"));
}

// ── Return type detection ──────────────────────────────────────

#[test]
fn component_return_type_detection() {
    assert!(is_component_return_type(Some("JSX.Element")));
    assert!(is_component_return_type(Some("ReactNode")));
    assert!(is_component_return_type(Some("ReactElement")));
    assert!(is_component_return_type(Some("React.FC<Props>")));
    assert!(!is_component_return_type(Some("string")));
    assert!(!is_component_return_type(None));
}

// ── Props extraction from type annotations ─────────────────────

#[test]
fn extract_props_from_fc_annotation() {
    assert_eq!(
        extract_props_from_type_annotation(Some("React.FC<UserCardProps>")),
        Some("UserCardProps".to_string())
    );
}

#[test]
fn extract_props_from_function_component_annotation() {
    assert_eq!(
        extract_props_from_type_annotation(Some("React.FunctionComponent<ButtonProps>")),
        Some("ButtonProps".to_string())
    );
}

#[test]
fn extract_props_no_angle_brackets() {
    assert_eq!(extract_props_from_type_annotation(Some("string")), None);
}
