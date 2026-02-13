use super::*;

#[test]
fn duplicate_keys_are_emitted_without_deduping() {
    let items = parse_and_extract(r#"{"a":1,"a":2}"#);
    let props = find_all_by_name(&items, "a");
    assert_eq!(props.len(), 2);
    assert!(props.iter().any(|item| {
        item.metadata
            .attributes
            .iter()
            .any(|attribute| attribute == "json:duplicate_key:a")
    }));
}

#[test]
fn empty_object_and_array_emit_shape_metadata() {
    let items = parse_and_extract(r#"{"obj":{},"arr":[]}"#);

    let obj = find_by_name(&items, "obj");
    assert!(
        obj.metadata
            .attributes
            .iter()
            .any(|attribute| attribute == "json:object_keys:0")
    );

    let arr = find_by_name(&items, "arr");
    assert!(
        arr.metadata
            .attributes
            .iter()
            .any(|attribute| attribute == "json:array_count:0")
    );
    assert!(
        arr.metadata
            .attributes
            .iter()
            .any(|attribute| attribute == "json:array_elements:empty")
    );
}
