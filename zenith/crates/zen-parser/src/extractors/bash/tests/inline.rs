use super::*;

#[test]
fn empty_script() {
    let items = parse_and_extract("");
    assert!(items.is_empty());
}

#[test]
fn comment_only_script() {
    let items = parse_and_extract("# just a comment");
    assert!(items.is_empty());
}

#[test]
fn inline_function() {
    let items = parse_and_extract("hello() { echo 'world'; }");
    let f = find_by_name(&items, "hello");
    assert_eq!(f.kind, SymbolKind::Function);
}

#[test]
fn inline_export() {
    let items = parse_and_extract("export MY_VAR=\"hello\"");
    let v = find_by_name(&items, "MY_VAR");
    assert_eq!(v.kind, SymbolKind::Const);
    assert_eq!(v.visibility, Visibility::Export);
}

#[test]
fn inline_readonly() {
    let items = parse_and_extract("readonly MY_CONST=42");
    let v = find_by_name(&items, "MY_CONST");
    assert_eq!(v.kind, SymbolKind::Const);
}

#[test]
fn inline_alias() {
    let items = parse_and_extract("alias k='kubectl'");
    let a = find_by_name(&items, "k");
    assert_eq!(a.kind, SymbolKind::Static);
}

#[test]
fn inline_pipeline() {
    let items = parse_and_extract("cat file.txt | grep error | wc -l");
    let pipe = items
        .iter()
        .find(|i| i.metadata.attributes.contains(&"pipeline".to_string()))
        .expect("should find pipeline");
    assert_eq!(pipe.metadata.parameters.len(), 3);
}

#[test]
fn inline_heredoc() {
    let items = parse_and_extract("cat <<MARKER\nhello\nMARKER");
    let hd = find_by_name(&items, "heredoc MARKER");
    assert_eq!(hd.kind, SymbolKind::Const);
}

#[test]
fn inline_trap() {
    let items = parse_and_extract("trap 'exit 1' SIGTERM");
    let t = items
        .iter()
        .find(|i| i.name.starts_with("trap "))
        .expect("should find trap");
    assert_eq!(t.kind, SymbolKind::Function);
}

#[test]
fn inline_source() {
    let items = parse_and_extract("source /etc/profile");
    let s = find_by_name(&items, "/etc/profile");
    assert_eq!(s.kind, SymbolKind::Module);
}

#[test]
fn inline_dot() {
    let items = parse_and_extract(". ~/.bashrc");
    let d = find_by_name(&items, "~/.bashrc");
    assert_eq!(d.kind, SymbolKind::Module);
}

#[test]
fn inline_case() {
    let items = parse_and_extract("case \"$1\" in\n  yes) echo ok ;;\n  no) echo fail ;;\nesac");
    let cs = find_by_name_prefix(&items, "case ");
    assert_eq!(cs.kind, SymbolKind::Enum);
    assert!(
        cs.metadata.variants.len() >= 2,
        "should have at least 2 patterns: {:?}",
        cs.metadata.variants
    );
}

#[test]
fn inline_for() {
    let items = parse_and_extract("for x in a b c; do echo $x; done");
    let f = find_by_name_prefix(&items, "for ");
    assert_eq!(f.kind, SymbolKind::Macro);
}

#[test]
fn inline_while() {
    let items = parse_and_extract("while true; do sleep 1; done");
    let w = find_by_name_prefix(&items, "while ");
    assert_eq!(w.kind, SymbolKind::Macro);
}

#[test]
fn inline_until() {
    let items = parse_and_extract("until false; do sleep 1; done");
    let u = find_by_name_prefix(&items, "until ");
    assert_eq!(u.kind, SymbolKind::Macro);
}

#[test]
fn inline_subshell() {
    let items = parse_and_extract("(echo hello; echo world)");
    let sub = find_by_name_prefix(&items, "subshell ");
    assert_eq!(sub.kind, SymbolKind::Macro);
}

#[test]
fn inline_command_group() {
    let items = parse_and_extract("{ echo hello; echo world; }");
    let cg = find_by_name_prefix(&items, "command_group ");
    assert_eq!(cg.kind, SymbolKind::Macro);
}

#[test]
fn inline_select() {
    let items = parse_and_extract("select x in a b c; do echo $x; done");
    let s = find_by_name_prefix(&items, "select ");
    assert_eq!(s.kind, SymbolKind::Enum);
}

#[test]
fn inline_if() {
    let items = parse_and_extract("if true; then echo ok; fi");
    let i = find_by_name_prefix(&items, "if ");
    assert_eq!(i.kind, SymbolKind::Enum);
}

#[test]
fn inline_declare_array() {
    let items = parse_and_extract("declare -a ARR=(1 2 3)");
    let v = find_by_name(&items, "ARR");
    assert!(
        v.metadata
            .attributes
            .iter()
            .any(|a| a.contains("indexed_array")),
        "should have indexed_array: {:?}",
        v.metadata.attributes
    );
}

#[test]
fn inline_declare_assoc() {
    let items = parse_and_extract("declare -A MAP=([a]=1 [b]=2)");
    let v = find_by_name(&items, "MAP");
    assert!(
        v.metadata
            .attributes
            .iter()
            .any(|a| a.contains("associative")),
        "should have associative_array: {:?}",
        v.metadata.attributes
    );
}

#[test]
fn here_string_not_crash() {
    // Here strings are part of commands, not top-level nodes
    // Ensure we don't crash on them
    let items = parse_and_extract("grep pattern <<< \"hello world\"");
    // The command itself may or may not produce an item (it's a plain grep command)
    // Just verify no crash
    assert!(items.is_empty() || !items.is_empty());
}

#[test]
fn inline_c_style_for() {
    let items = parse_and_extract("for ((x=0; x<3; x++)); do echo $x; done");
    let f = items
        .iter()
        .find(|i| i.metadata.attributes.contains(&"c_style".to_string()))
        .expect("should find c-style for");
    assert_eq!(f.kind, SymbolKind::Macro);
    assert!(f.name.starts_with("for "));
}

#[test]
fn inline_negated() {
    let items = parse_and_extract("! false");
    let n = items
        .iter()
        .find(|i| i.metadata.attributes.contains(&"negated".to_string()))
        .expect("should find negated command");
    assert_eq!(n.kind, SymbolKind::Macro);
}

#[test]
fn inline_test_bracket() {
    let items = parse_and_extract("[[ -d /tmp ]]");
    let t = items
        .iter()
        .find(|i| i.metadata.attributes.contains(&"test".to_string()))
        .expect("should find test command");
    assert_eq!(t.kind, SymbolKind::Macro);
}

#[test]
fn inline_test_single_bracket() {
    let items = parse_and_extract("[ -f /etc/passwd ]");
    let t = items
        .iter()
        .find(|i| i.metadata.attributes.contains(&"test".to_string()))
        .expect("should find single-bracket test");
    assert_eq!(t.kind, SymbolKind::Macro);
}

#[test]
fn inline_unset_var() {
    let items = parse_and_extract("unset FOO");
    let u = find_by_name(&items, "unset FOO");
    assert_eq!(u.kind, SymbolKind::Static);
}

#[test]
fn inline_unset_func() {
    let items = parse_and_extract("unset -f bar");
    let u = find_by_name(&items, "unset bar");
    assert_eq!(u.kind, SymbolKind::Function);
}
