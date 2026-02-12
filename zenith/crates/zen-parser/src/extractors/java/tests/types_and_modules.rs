use super::*;

#[test]
fn extracts_java_modules_and_types() {
    let items = fixture_items();

    assert_eq!(
        find_by_name(&items, "zenith.sample").kind,
        SymbolKind::Module
    );
    assert_eq!(
        find_by_name(&items, "java.util.List").kind,
        SymbolKind::Module
    );
    assert_eq!(
        find_by_name(&items, "static java.util.Collections.emptyList").kind,
        SymbolKind::Module
    );

    assert_eq!(find_by_name(&items, "Widget").kind, SymbolKind::Class);
    assert_eq!(find_by_name(&items, "Renderer").kind, SymbolKind::Interface);
    assert_eq!(find_by_name(&items, "Status").kind, SymbolKind::Enum);
    assert_eq!(find_by_name(&items, "Point").kind, SymbolKind::Struct);
    assert_eq!(find_by_name(&items, "Label").kind, SymbolKind::Interface);
}

#[test]
fn extracts_enum_variants() {
    let items = fixture_items();
    let status = find_by_name(&items, "Status");

    assert!(status.metadata.variants.iter().any(|v| v == "NEW"));
    assert!(status.metadata.variants.iter().any(|v| v == "READY"));
    assert!(status.metadata.variants.iter().any(|v| v == "DONE"));
}

#[test]
fn extracts_module_declaration_from_module_info_fixture() {
    let source = include_str!("../../../../tests/fixtures/module-info.java");
    let items = parse_and_extract(source);

    assert_eq!(
        find_by_name(&items, "zenith.sample.module").kind,
        SymbolKind::Module
    );
    let exports = find_by_name(&items, "exports zenith.sample");
    assert_eq!(exports.kind, SymbolKind::Module);
    assert!(
        exports
            .metadata
            .attributes
            .iter()
            .any(|attr| attr == "module_directive:exports")
    );
    assert_eq!(
        exports.metadata.return_type.as_deref(),
        Some("zenith.sample")
    );

    let opens = find_by_name(&items, "opens zenith.sample.internal");
    assert_eq!(opens.kind, SymbolKind::Module);
    assert!(
        opens
            .metadata
            .attributes
            .iter()
            .any(|attr| attr == "module_directive:opens")
    );
    assert_eq!(
        opens.metadata.return_type.as_deref(),
        Some("zenith.sample.internal")
    );

    let requires_base = find_by_name(&items, "requires java.base");
    assert_eq!(requires_base.kind, SymbolKind::Module);
    assert!(
        requires_base
            .metadata
            .attributes
            .iter()
            .any(|attr| attr == "module_directive:requires")
    );
    assert_eq!(
        requires_base.metadata.return_type.as_deref(),
        Some("java.base")
    );

    let requires_static = find_by_name(&items, "requires static java.sql");
    assert_eq!(requires_static.kind, SymbolKind::Module);
    assert!(
        requires_static
            .metadata
            .attributes
            .iter()
            .any(|attr| attr == "module_directive:requires")
    );
    assert!(
        requires_static
            .metadata
            .attributes
            .iter()
            .any(|attr| attr == "module_modifier:static")
    );
    assert_eq!(
        requires_static.metadata.return_type.as_deref(),
        Some("java.sql")
    );

    let uses = find_by_name(&items, "uses zenith.sample.spi.Renderer");
    assert_eq!(uses.kind, SymbolKind::Module);
    assert!(
        uses.metadata
            .attributes
            .iter()
            .any(|attr| attr == "module_directive:uses")
    );
    assert_eq!(
        uses.metadata.return_type.as_deref(),
        Some("zenith.sample.spi.Renderer")
    );

    let provides = find_by_name(
        &items,
        "provides zenith.sample.spi.Renderer with zenith.sample.impl.DefaultRenderer",
    );
    assert_eq!(provides.kind, SymbolKind::Module);
    assert!(
        provides
            .metadata
            .attributes
            .iter()
            .any(|attr| attr == "module_directive:provides")
    );
    assert_eq!(
        provides.metadata.return_type.as_deref(),
        Some("zenith.sample.spi.Renderer")
    );
    assert!(
        provides
            .metadata
            .parameters
            .iter()
            .any(|p| p == "zenith.sample.impl.DefaultRenderer")
    );
}

#[test]
fn extracts_record_components_as_fields() {
    let items = fixture_items();

    let x = items
        .iter()
        .find(|item| item.kind == SymbolKind::Field && item.name == "x")
        .expect("expected record component field x");
    assert_eq!(x.metadata.owner_name.as_deref(), Some("Point"));
    assert_eq!(x.metadata.owner_kind, Some(SymbolKind::Struct));

    let y = items
        .iter()
        .find(|item| item.kind == SymbolKind::Field && item.name == "y")
        .expect("expected record component field y");
    assert_eq!(y.metadata.owner_name.as_deref(), Some("Point"));
    assert_eq!(y.metadata.owner_kind, Some(SymbolKind::Struct));
}
