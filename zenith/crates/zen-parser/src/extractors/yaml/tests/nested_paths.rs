use super::*;

#[test]
fn tracks_nested_paths_and_sequences() {
    let items = fixture_items();

    let max = find_by_name(&items, "app.db.pool.max");
    assert_eq!(max.metadata.owner_name.as_deref(), Some("app.db.pool"));

    let route_path = find_by_name(&items, "routes[0].path");
    assert_eq!(route_path.metadata.owner_name.as_deref(), Some("routes[0]"));

    let retries = find_by_name(&items, "app.retries[1]");
    assert_eq!(retries.metadata.return_type.as_deref(), Some("number"));
}
