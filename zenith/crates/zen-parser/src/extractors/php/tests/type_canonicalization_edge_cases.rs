use super::*;

#[test]
fn canonicalizes_union_order_and_dedupes() {
    let source = r"
<?php
function a(string|int|int $x): string|int|int { return $x; }
";

    let items = parse_and_extract(source);
    let a = find_by_name(&items, "a");
    assert_eq!(a.metadata.return_type.as_deref(), Some("int|string"));
    assert!(
        a.metadata
            .parameters
            .iter()
            .any(|p| p.contains("x: int|string"))
    );
}

#[test]
fn canonicalizes_intersection_order() {
    let source = r"
<?php
function a(B&A $x): B&A { return $x; }
function b(A&B $x): A&B { return $x; }
";

    let items = parse_and_extract(source);
    let a = find_by_name(&items, "a");
    let b = find_by_name(&items, "b");
    assert_eq!(a.metadata.return_type, b.metadata.return_type);
    assert_eq!(a.metadata.parameters, b.metadata.parameters);
}

#[test]
fn canonicalizes_optional_union_type_order() {
    let source = r"
<?php
function a(?Foo|Bar $x): ?Foo|Bar { return $x; }
function b(?Bar|Foo $x): ?Bar|Foo { return $x; }
";

    let items = parse_and_extract(source);
    let a = find_by_name(&items, "a");
    let b = find_by_name(&items, "b");
    assert_eq!(a.metadata.return_type, b.metadata.return_type);
}

#[test]
fn removes_extraneous_whitespace_in_type_text() {
    let source = r"
<?php
function a( string | int $x ) : string | int { return $x; }
";

    let items = parse_and_extract(source);
    let a = find_by_name(&items, "a");
    assert_eq!(a.metadata.return_type.as_deref(), Some("int|string"));
}
