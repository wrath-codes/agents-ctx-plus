use super::*;

#[test]
fn closure_assignment_sets_callable_context_and_alias() {
    let source = r"
local cb = function() return 1 end
";

    let items = parse_and_extract(source);
    let cb = find_by_name(&items, "cb");

    assert_eq!(cb.kind, SymbolKind::Function);
    assert!(
        cb.metadata
            .attributes
            .iter()
            .any(|a| a == "callable_origin:assignment")
    );
    assert!(
        cb.metadata
            .attributes
            .iter()
            .any(|a| a == "callable_alias:cb")
    );
}

#[test]
fn callable_in_argument_sets_argument_context() {
    let source = r"
M.cb = function() return 1 end
consume(M.cb)
";

    let items = parse_and_extract(source);
    let cb = find_by_name(&items, "cb");
    assert_eq!(cb.kind, SymbolKind::Method);
    assert!(
        cb.metadata
            .attributes
            .iter()
            .any(|a| a == "callable_origin:table_field")
    );
}
