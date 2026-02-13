use super::*;

#[test]
fn nested_owners_are_preserved() {
    let source = r#"
A.B.fn = function() return 1 end
A["B"].g = function() return 2 end
function M:run() return self end
function M.stop() return true end
"#;

    let items = parse_and_extract(source);

    let fn_item = find_by_name(&items, "fn");
    assert_eq!(fn_item.kind, SymbolKind::Method);
    assert_eq!(fn_item.metadata.owner_name.as_deref(), Some("A.B"));
    assert_eq!(fn_item.metadata.owner_kind, Some(SymbolKind::Module));

    let g = find_by_name(&items, "g");
    assert_eq!(g.kind, SymbolKind::Method);
    assert!(g.metadata.owner_name.is_some());

    let run = find_by_name(&items, "run");
    assert_eq!(run.metadata.owner_name.as_deref(), Some("M"));
    assert!(!run.metadata.is_static_member);

    let stop = find_by_name(&items, "stop");
    assert_eq!(stop.metadata.owner_name.as_deref(), Some("M"));
    assert!(stop.metadata.is_static_member);
}
