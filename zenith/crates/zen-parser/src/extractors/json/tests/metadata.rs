use super::*;

#[test]
fn annotates_value_types_in_metadata() {
    let items = fixture_items();

    let feature_enabled = find_by_name(&items, "app.features.auth");
    assert_eq!(
        feature_enabled.metadata.return_type.as_deref(),
        Some("boolean")
    );

    let timeout = find_by_name(&items, "app.db.timeout_ms");
    assert_eq!(timeout.metadata.return_type.as_deref(), Some("number"));

    let note = find_by_name(&items, "app.notes");
    assert_eq!(note.metadata.return_type.as_deref(), Some("null"));

    let routes = find_by_name(&items, "routes");
    assert_eq!(routes.metadata.return_type.as_deref(), Some("array"));
    assert!(
        routes
            .metadata
            .attributes
            .iter()
            .any(|attribute| attribute == "json:key:routes")
    );
    assert!(
        routes
            .metadata
            .attributes
            .iter()
            .any(|attribute| attribute == "json:array_elements:object")
    );
    assert!(
        routes
            .metadata
            .attributes
            .iter()
            .any(|attribute| attribute == "json:array_count:2")
    );

    let app = find_by_name(&items, "app");
    assert!(
        app.metadata
            .attributes
            .iter()
            .any(|attribute| attribute == "json:object_keys:4")
    );
}

#[test]
fn array_shape_metadata_includes_mixed_and_nullable_flags() {
    let items = parse_and_extract(r#"{"values":[1,null,"x"]}"#);
    let values = find_by_name(&items, "values");

    assert!(
        values
            .metadata
            .attributes
            .iter()
            .any(|attribute| attribute == "json:array_count:3")
    );
    assert!(
        values
            .metadata
            .attributes
            .iter()
            .any(|attribute| attribute == "json:array_nullable")
    );
    assert!(
        values
            .metadata
            .attributes
            .iter()
            .any(|attribute| attribute == "json:array_mixed")
    );
}
