use super::*;

fn has_attr(item: &ParsedItem, expected: &str) -> bool {
    item.metadata.attributes.iter().any(|a| a == expected)
}

fn has_attr_prefix(item: &ParsedItem, prefix: &str) -> bool {
    item.metadata
        .attributes
        .iter()
        .any(|a| a.starts_with(prefix))
}

#[test]
fn cargo_dependency_entries_are_normalized() {
    let items = dependency_fixture_items();

    let serde = find_by_name(&items, "dependencies.serde");
    assert!(has_attr(serde, "toml:dependency"));
    assert!(has_attr(serde, "toml:dep_scope:cargo:dependencies"));
    assert!(has_attr(serde, "toml:dep_name:serde"));
    assert!(has_attr(serde, "toml:dep_req:1.0"));
    assert!(has_attr(serde, "toml:value_normalized:1.0"));

    let tokio = find_by_name(&items, "dependencies.tokio");
    assert!(has_attr(tokio, "toml:dep_scope:cargo:dependencies"));
    assert!(has_attr(tokio, "toml:dep_name:tokio"));
    assert!(has_attr_prefix(tokio, "toml:dep_req:"));
    assert!(has_attr(tokio, "toml:dep_optional"));

    let local = find_by_name(&items, "dependencies.my-local");
    assert!(has_attr(local, "toml:dep_source:path"));
    assert!(has_attr(local, "toml:dep_package:my-local-lib"));

    let insta = find_by_name(&items, "dev-dependencies.insta");
    assert!(has_attr(insta, "toml:dep_scope:cargo:dev-dependencies"));
    assert!(has_attr(insta, "toml:dep_source:registry"));
}

#[test]
fn poetry_and_pep621_dependencies_are_detected() {
    let items = dependency_fixture_items();

    let requests = find_by_name(&items, "tool.poetry.dependencies.requests");
    assert!(has_attr(requests, "toml:dependency"));
    assert!(has_attr(requests, "toml:dep_scope:poetry:dependencies"));
    assert!(has_attr(requests, "toml:dep_name:requests"));

    let dep0 = find_by_name(&items, "project.dependencies[0]");
    assert!(has_attr(dep0, "toml:dependency"));
    assert!(has_attr(dep0, "toml:dep_scope:pep621:dependencies"));
    assert!(has_attr(dep0, "toml:dep_name:httpx"));
    assert!(has_attr(dep0, "toml:dep_req:>=0.28"));

    let dev0 = find_by_name(&items, "project.optional-dependencies.dev[0]");
    assert!(has_attr(dev0, "toml:dependency"));
    assert!(has_attr(dev0, "toml:dep_scope:pep621:optional:dev"));
    assert!(has_attr(dev0, "toml:dep_name:pytest"));
}

#[test]
fn scalar_values_are_normalized_for_package_fields() {
    let items = dependency_fixture_items();

    let python = find_by_name(&items, "tool.poetry.dependencies.python");
    assert!(has_attr_prefix(python, "toml:value_normalized:"));

    let dep1 = find_by_name(&items, "project.dependencies[1]");
    assert!(has_attr_prefix(dep1, "toml:value_normalized:"));
}
