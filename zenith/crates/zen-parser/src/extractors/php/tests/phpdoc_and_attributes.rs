use super::*;

#[test]
fn parses_deep_phpdoc_tags() {
    let source = r"
<?php
/**
 * @template T
 * @param T $item
 * @var array<string, T>
 * @throws RuntimeException
 * @return T
 * @psalm-return T
 */
function head($item) {
    return $item;
}

/**
 * @template TModel
 * @extends BaseRepo<TModel>
 * @implements Repo<TModel>
 */
class UserRepo {}
";

    let items = parse_and_extract(source);
    let head = find_by_name(&items, "head");

    assert_eq!(head.metadata.return_type.as_deref(), Some("T"));
    assert!(
        head.metadata
            .attributes
            .iter()
            .any(|a| a == "phpdoc:template:T")
    );
    assert!(
        head.metadata
            .attributes
            .iter()
            .any(|a| a.starts_with("phpdoc:var:"))
    );
    assert!(
        head.metadata
            .attributes
            .iter()
            .any(|a| a == "phpdoc:throws:RuntimeException")
    );
    assert!(
        head.metadata
            .attributes
            .iter()
            .any(|a| a.starts_with("phpdoc:psalm:"))
    );

    let repo = find_by_name(&items, "UserRepo");
    assert_eq!(repo.metadata.type_parameters.as_deref(), Some("TModel"));
    assert!(
        repo.metadata
            .base_classes
            .iter()
            .any(|b| b == "BaseRepo<TModel>")
    );
    assert!(
        repo.metadata
            .base_classes
            .iter()
            .any(|b| b == "Repo<TModel>")
    );
}

#[test]
fn parses_php_attributes() {
    let source = r"
<?php
#[Route('/ok')]
function endpoint(): void {}
";

    let items = parse_and_extract(source);
    let endpoint = find_by_name(&items, "endpoint");
    assert!(
        endpoint
            .metadata
            .attributes
            .iter()
            .any(|a| a == "attr:name:Route")
    );
    assert!(
        endpoint
            .metadata
            .attributes
            .iter()
            .any(|a| a == "attr:args:('/ok')")
    );
}
