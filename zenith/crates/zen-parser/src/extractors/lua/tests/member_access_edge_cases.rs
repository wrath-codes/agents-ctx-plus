use super::*;

#[test]
fn dot_colon_bracket_member_access_are_tagged() {
    let source = r#"
function M.stop() return true end
function M:run() return self end
M["reset"] = function() return false end
M["level"] = 1
"#;

    let items = parse_and_extract(source);

    let stop = find_by_name(&items, "stop");
    assert_eq!(stop.kind, SymbolKind::Method);
    assert_eq!(stop.metadata.owner_name.as_deref(), Some("M"));
    assert!(stop.metadata.is_static_member);
    assert!(
        stop.metadata
            .attributes
            .iter()
            .any(|a| a == "member_access:dot")
    );

    let run = find_by_name(&items, "run");
    assert_eq!(run.kind, SymbolKind::Method);
    assert!(!run.metadata.is_static_member);
    assert!(
        run.metadata
            .attributes
            .iter()
            .any(|a| a == "member_access:colon")
    );

    let reset = find_by_name(&items, "reset");
    assert_eq!(reset.kind, SymbolKind::Method);
    assert!(
        reset
            .metadata
            .attributes
            .iter()
            .any(|a| a == "member_access:bracket")
    );

    let level = find_by_name(&items, "level");
    assert_eq!(level.kind, SymbolKind::Field);
    assert!(
        level
            .metadata
            .attributes
            .iter()
            .any(|a| a == "member_access:bracket")
    );
}
