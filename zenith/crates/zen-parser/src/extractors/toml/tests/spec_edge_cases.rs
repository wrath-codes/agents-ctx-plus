use super::*;

#[test]
fn quoted_dotted_keys_preserve_segments() {
    let items = edge_fixture_items();

    let quoted = find_by_name(&items, "a.b");
    assert_eq!(quoted.kind, SymbolKind::Property);

    let nested = find_by_name(&items, "outer.inner.part.leaf");
    assert_eq!(nested.kind, SymbolKind::Property);
}

#[test]
fn marks_duplicate_and_conflicting_tables() {
    let items = edge_fixture_items();
    let dup_tables = find_all_by_name(&items, "dup");
    assert_eq!(dup_tables.len(), 2);
    assert!(
        dup_tables[1]
            .metadata
            .attributes
            .iter()
            .any(|a| a == "toml:duplicate_table:dup")
    );

    let mix_table = find_by_name(&items, "mix");
    assert!(
        mix_table
            .metadata
            .attributes
            .iter()
            .any(|a| a == "toml:table_kind_conflict:mix")
    );
}

#[test]
fn marks_duplicate_keys_and_mixed_arrays() {
    let items = edge_fixture_items();

    let dup_v = find_all_by_name(&items, "dup.v");
    assert_eq!(dup_v.len(), 2);
    assert!(
        dup_v[1]
            .metadata
            .attributes
            .iter()
            .any(|a| a == "toml:duplicate_key:dup.v")
    );

    let mixed = find_by_name(&items, "mix.mixed");
    assert!(
        mixed
            .metadata
            .attributes
            .iter()
            .any(|a| a == "toml:array_mixed")
    );
}

#[test]
fn table_array_has_context_attributes() {
    let items = edge_fixture_items();
    let array_table = find_by_name(&items, "mix[0]");
    assert!(
        array_table
            .metadata
            .attributes
            .iter()
            .any(|a| a == "toml:table_array:path:mix")
    );
    assert!(
        array_table
            .metadata
            .attributes
            .iter()
            .any(|a| a == "toml:table_array:index:0")
    );
}

#[test]
fn attaches_line_comments_to_items() {
    let items = edge_fixture_items();
    let title = find_by_name(&items, "title");
    assert!(title.doc_comment.contains("title comment"));

    let note = find_by_name(&items, "mix.note");
    assert!(!note.doc_comment.contains("not comment"));
}

#[test]
fn detects_key_hierarchy_conflicts() {
    let items = edge_fixture_items();

    let child = find_by_name(&items, "mix.key_parent.child");
    assert!(
        child
            .metadata
            .attributes
            .iter()
            .any(|a| a == "toml:key_parent_conflict:mix.key_parent")
    );

    let parent = find_by_name(&items, "mix.key_child");
    assert!(
        parent
            .metadata
            .attributes
            .iter()
            .any(|a| a == "toml:key_child_conflict:mix.key_child")
    );

    let root_scalar_child = find_by_name(&items, "root_scalar.child");
    assert!(
        root_scalar_child
            .metadata
            .attributes
            .iter()
            .any(|a| a == "toml:table_key_conflict:root_scalar")
    );
}

#[test]
fn normalizes_spans_without_trailing_blank_lines() {
    let items = edge_fixture_items();
    let title = find_by_name(&items, "title");
    assert_eq!(title.start_line, title.end_line);
}
