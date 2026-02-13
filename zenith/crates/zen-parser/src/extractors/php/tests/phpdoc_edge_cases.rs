use super::*;

#[test]
fn phpdoc_param_types_apply_to_untyped_parameters() {
    let source = r"
<?php
/**
 * @param int $x
 */
function f($x) {}
";

    let items = parse_and_extract(source);
    let f = find_by_name(&items, "f");
    assert!(f.metadata.parameters.iter().any(|p| p == "x: int"));
}

#[test]
fn ast_return_type_wins_over_phpdoc_return() {
    let source = r"
<?php
/** @return int */
function g(): string { return ''; }
";

    let items = parse_and_extract(source);
    let g = find_by_name(&items, "g");
    assert_eq!(g.metadata.return_type.as_deref(), Some("string"));
}

#[test]
fn phpdoc_var_falls_back_to_return_type_when_missing() {
    let source = r"
<?php
/** @var array<string,int> */
function h($x) { return $x; }
";

    let items = parse_and_extract(source);
    let h = find_by_name(&items, "h");
    assert_eq!(h.metadata.return_type.as_deref(), Some("array<string,int>"));
}

#[test]
fn phpdoc_extends_and_implements_enrich_base_classes() {
    let source = r"
<?php
/**
 * @extends BaseRepo<User>
 * @implements Repo<User>
 */
class RepoImpl {}
";

    let items = parse_and_extract(source);
    let repo = find_by_name(&items, "RepoImpl");
    assert!(repo
        .metadata
        .base_classes
        .iter()
        .any(|b| b == "BaseRepo<User>"));
    assert!(repo.metadata.base_classes.iter().any(|b| b == "Repo<User>"));
}
