use super::*;

#[test]
fn adds_scalar_and_shape_metadata() {
    let items = fixture_items();

    let auth = find_by_name(&items, "app.features.auth");
    assert_eq!(auth.metadata.return_type.as_deref(), Some("boolean"));

    let timeout = find_by_name(&items, "app.db.timeout_ms");
    assert_eq!(timeout.metadata.return_type.as_deref(), Some("number"));

    let note = find_by_name(&items, "app.note");
    assert_eq!(note.metadata.return_type.as_deref(), Some("null"));

    let routes = find_by_name(&items, "routes");
    assert!(routes
        .metadata
        .attributes
        .iter()
        .any(|attr| attr == "yaml:array_count:2"));

    let app = find_by_name(&items, "app");
    assert!(app
        .metadata
        .attributes
        .iter()
        .any(|attr| attr == "yaml:object_keys:5"));
}
