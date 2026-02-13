use super::*;

#[test]
fn tracks_nested_paths_and_array_elements() {
    let items = fixture_items();

    let pool_max = find_by_name(&items, "app.db.pool.max");
    assert_eq!(pool_max.metadata.owner_name.as_deref(), Some("app.db.pool"));

    let route_path = find_by_name(&items, "routes[0].path");
    assert_eq!(route_path.metadata.owner_name.as_deref(), Some("routes[0]"));

    let route_method = find_by_name(&items, "routes[1].method");
    assert_eq!(
        route_method.metadata.owner_name.as_deref(),
        Some("routes[1]")
    );
}
