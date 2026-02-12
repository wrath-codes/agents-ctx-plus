use super::*;

#[test]
fn extern_c_fn_detected() {
    let source = include_str!("../../../../tests/fixtures/sample.rs");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "c_callback");
    assert_eq!(f.kind, SymbolKind::Function);
    assert_eq!(f.metadata.abi.as_deref(), Some("C"), "should have ABI 'C'");
}

#[test]
fn repr_c_in_attributes() {
    let source = include_str!("../../../../tests/fixtures/sample.rs");
    let items = parse_and_extract(source);
    let ffi = find_by_name(&items, "FfiPoint");
    assert_eq!(ffi.kind, SymbolKind::Struct);
    assert!(
        ffi.metadata.attributes.iter().any(|a| a.contains("repr")),
        "attributes: {:?}",
        ffi.metadata.attributes
    );
}

#[test]
fn extern_block_functions_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.rs");
    let items = parse_and_extract(source);
    let ext_func = find_by_name(&items, "external_func");
    assert_eq!(ext_func.kind, SymbolKind::Function);
    assert_eq!(ext_func.metadata.abi.as_deref(), Some("C"));
    assert!(
        ext_func.metadata.attributes.contains(&"extern".to_string()),
        "attrs: {:?}",
        ext_func.metadata.attributes
    );
}

#[test]
fn extern_block_statics_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.rs");
    let items = parse_and_extract(source);
    let ext_var = find_by_name(&items, "EXTERNAL_VAR");
    assert_eq!(ext_var.kind, SymbolKind::Static);
    assert_eq!(ext_var.metadata.abi.as_deref(), Some("C"));
}

#[test]
fn pub_super_visibility() {
    let source = include_str!("../../../../tests/fixtures/sample.rs");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "super_visible");
    assert_eq!(f.visibility, Visibility::Protected);
}

#[test]
fn pub_in_path_visibility() {
    let source = include_str!("../../../../tests/fixtures/sample.rs");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "path_visible");
    assert_eq!(f.visibility, Visibility::Protected);
}
