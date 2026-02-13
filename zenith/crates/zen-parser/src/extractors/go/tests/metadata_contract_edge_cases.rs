use super::*;

#[test]
fn method_and_type_params_emit_go_tags() {
    let source = r"package demo
type Pair[T any] struct { First T }
func Map[T any](items []T, fn func(T) T) []T { return items }
";

    let items = parse_and_extract(source);

    let pair = find_by_name(&items, "Pair");
    assert!(
        pair.metadata
            .attributes
            .iter()
            .any(|a| a.starts_with("go:type_param:"))
    );

    let map = find_by_name(&items, "Map");
    assert!(
        map.metadata
            .attributes
            .iter()
            .any(|a| a.starts_with("go:type_param:"))
    );
}
