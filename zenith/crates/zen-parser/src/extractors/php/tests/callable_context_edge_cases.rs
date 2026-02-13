use super::*;

#[test]
fn closure_assignment_sets_callable_assignment_context() {
    let source = r"
<?php
$cb = function () { return 1; };
";

    let items = parse_and_extract(source);
    let closure = items
        .iter()
        .find(|i| i.kind == SymbolKind::Function && i.name.starts_with("<closure@"))
        .expect("expected closure symbol");

    assert!(
        closure
            .metadata
            .attributes
            .iter()
            .any(|a| a == "callable_origin:assignment")
    );
    assert!(
        closure
            .metadata
            .attributes
            .iter()
            .any(|a| a.starts_with("callable_alias:"))
    );
}

#[test]
fn closure_in_array_pair_sets_array_pair_context() {
    let source = r"
<?php
$x = ['k' => function () { return 1; }];
";

    let items = parse_and_extract(source);
    let closure = items
        .iter()
        .find(|i| i.kind == SymbolKind::Function && i.name.starts_with("<closure@"))
        .expect("expected closure symbol");

    assert!(
        closure
            .metadata
            .attributes
            .iter()
            .any(|a| a == "callable_origin:array_pair" || a == "callable_origin:assignment")
    );
}

#[test]
fn arrow_in_return_sets_return_context() {
    let source = r"
<?php
function mk() {
    return fn (int $x): int => $x + 1;
}
";

    let items = parse_and_extract(source);
    let arrow = items
        .iter()
        .find(|i| i.kind == SymbolKind::Function && i.name.starts_with("<arrow@"))
        .expect("expected arrow symbol");

    assert!(
        arrow
            .metadata
            .attributes
            .iter()
            .any(|a| a == "callable_origin:return")
    );
}

#[test]
fn closure_in_argument_sets_argument_context() {
    let source = r"
<?php
consume(function () { return 1; });
";

    let items = parse_and_extract(source);
    let closure = items
        .iter()
        .find(|i| i.kind == SymbolKind::Function && i.name.starts_with("<closure@"))
        .expect("expected closure symbol");

    assert!(
        closure
            .metadata
            .attributes
            .iter()
            .any(|a| a == "callable_origin:argument")
    );
}
