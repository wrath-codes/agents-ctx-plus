use super::*;

#[test]
fn local_const_and_close_attributes_are_mapped() {
    let source = r#"
local a<const> = 1
local b<close> = io.open("x.txt", "r")
local c<const>, d = 2, 3
local f<const> = function(x) return x end
"#;

    let items = parse_and_extract(source);

    let const_local = find_by_name(&items, "a");
    assert_eq!(const_local.kind, SymbolKind::Const);
    assert_eq!(const_local.visibility, Visibility::Private);
    assert!(
        const_local
            .metadata
            .attributes
            .iter()
            .any(|x| x == "local_attr:const")
    );

    let close_local = find_by_name(&items, "b");
    assert_eq!(close_local.kind, SymbolKind::Static);
    assert!(
        close_local
            .metadata
            .attributes
            .iter()
            .any(|x| x == "local_attr:close")
    );

    let const_multi = find_by_name(&items, "c");
    assert_eq!(const_multi.kind, SymbolKind::Const);

    let static_multi = find_by_name(&items, "d");
    assert_eq!(static_multi.kind, SymbolKind::Static);

    let function_local = find_by_name(&items, "f");
    assert_eq!(function_local.kind, SymbolKind::Function);
    assert!(
        function_local
            .metadata
            .attributes
            .iter()
            .any(|x| x == "local_attr:const")
    );
    assert!(
        function_local
            .metadata
            .attributes
            .iter()
            .any(|x| x == "callable_origin:assignment")
    );
}
