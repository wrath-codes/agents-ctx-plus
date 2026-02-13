use super::*;

#[test]
fn extracts_constructor_methods_fields_consts_and_promoted_properties() {
    let items = fixture_items();

    let ctor = items
        .iter()
        .find(|i| i.kind == SymbolKind::Constructor && i.name == "__construct")
        .expect("expected constructor");
    assert_eq!(ctor.metadata.owner_name.as_deref(), Some("Service"));

    let render = items
        .iter()
        .find(|i| i.name == "render" && i.metadata.owner_name.as_deref() == Some("Service"))
        .expect("expected Service::render");
    assert_eq!(render.kind, SymbolKind::Method);
    assert_eq!(render.metadata.owner_name.as_deref(), Some("Service"));

    let name_field = find_by_name(&items, "name");
    assert_eq!(name_field.kind, SymbolKind::Property);
    assert_eq!(name_field.metadata.owner_name.as_deref(), Some("Service"));
    assert_eq!(name_field.visibility, Visibility::Protected);

    let id_field = find_by_name(&items, "id");
    assert_eq!(id_field.kind, SymbolKind::Field);
    assert_eq!(id_field.metadata.owner_name.as_deref(), Some("Service"));

    let version = find_by_name(&items, "VERSION");
    assert_eq!(version.kind, SymbolKind::Const);
    assert_eq!(version.metadata.owner_name.as_deref(), Some("Service"));

    let top_const = find_by_name(&items, "TOP_LIMIT");
    assert_eq!(top_const.kind, SymbolKind::Const);
    assert_eq!(top_const.visibility, Visibility::Public);

    let enum_case = find_by_name(&items, "Ready");
    assert_eq!(enum_case.kind, SymbolKind::Const);
    assert_eq!(enum_case.metadata.owner_name.as_deref(), Some("Status"));

    let trait_use = items
        .iter()
        .find(|i| i.kind == SymbolKind::Module && i.name == "UsesHelpers")
        .expect("expected trait use symbol");
    assert!(
        trait_use
            .metadata
            .attributes
            .iter()
            .any(|a| a == "trait_use")
    );

    let global_counter = find_by_name(&items, "globalCounter");
    assert_eq!(global_counter.kind, SymbolKind::Static);
    assert_eq!(
        global_counter.metadata.owner_name.as_deref(),
        Some("globalState")
    );

    let memo = find_by_name(&items, "memo");
    assert_eq!(memo.kind, SymbolKind::Static);
    assert!(
        memo.metadata
            .attributes
            .iter()
            .any(|a| a == "function_static")
    );
}

#[test]
fn extracts_trait_adaptation_metadata() {
    let source = r"
<?php
trait A { public function ping() {} }
trait B { public function ping() {} }
class C {
    use A, B {
        A::ping insteadof B;
        B::ping as private pingB;
    }
}
";

    let items = parse_and_extract(source);
    let insteadof = items
        .iter()
        .find(|i| i.kind == SymbolKind::Module && i.name.contains("insteadof"))
        .expect("expected insteadof adaptation");
    assert!(
        insteadof
            .metadata
            .attributes
            .iter()
            .any(|a| a == "trait_use:mode=insteadof")
    );
    assert!(
        insteadof
            .metadata
            .attributes
            .iter()
            .any(|a| a.starts_with("trait_use:target="))
    );

    let as_clause = items
        .iter()
        .find(|i| i.kind == SymbolKind::Module && i.name.contains(" as private "))
        .expect("expected as adaptation");
    assert!(
        as_clause
            .metadata
            .attributes
            .iter()
            .any(|a| a == "trait_use:mode=as")
    );
    assert!(
        as_clause
            .metadata
            .attributes
            .iter()
            .any(|a| a.starts_with("trait_use:alias="))
    );
}
