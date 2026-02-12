use super::*;

#[test]
fn extracts_constructor_methods_fields_and_consts() {
    let items = fixture_items();

    let ctor = items
        .iter()
        .find(|item| {
            item.kind == SymbolKind::Constructor
                && item.name == "Widget"
                && item.metadata.owner_name.as_deref() == Some("Widget")
        })
        .expect("should find Widget constructor");
    assert_eq!(ctor.metadata.owner_kind, Some(SymbolKind::Class));

    let render = items
        .iter()
        .find(|item| item.name == "render" && item.metadata.owner_name.as_deref() == Some("Widget"))
        .expect("should find Widget.render");
    assert_eq!(render.kind, SymbolKind::Method);
    assert_eq!(render.metadata.owner_name.as_deref(), Some("Widget"));

    let count = find_by_name(&items, "count");
    assert_eq!(count.kind, SymbolKind::Field);
    assert_eq!(count.metadata.owner_name.as_deref(), Some("Widget"));

    let seed = find_by_name(&items, "seed");
    assert_eq!(seed.kind, SymbolKind::Field);
    assert_eq!(seed.metadata.owner_name.as_deref(), Some("Widget"));

    let max_size = find_by_name(&items, "MAX_SIZE");
    assert_eq!(max_size.kind, SymbolKind::Const);
    assert_eq!(max_size.metadata.owner_name.as_deref(), Some("Widget"));
    assert!(max_size.metadata.is_static_member);

    let version = find_by_name(&items, "VERSION");
    assert_eq!(version.kind, SymbolKind::Const);
    assert_eq!(version.metadata.owner_name.as_deref(), Some("Renderer"));

    let label_value = find_by_name(&items, "value");
    assert_eq!(label_value.kind, SymbolKind::Method);
    assert_eq!(label_value.metadata.owner_name.as_deref(), Some("Label"));
}

#[test]
fn package_private_members_are_public_crate_visibility() {
    let items = fixture_items();

    let seed = find_by_name(&items, "seed");
    assert_eq!(seed.visibility, Visibility::PublicCrate);

    let package_name = find_by_name(&items, "packageName");
    assert_eq!(package_name.visibility, Visibility::PublicCrate);
}

#[test]
fn captures_method_and_constructor_extended_metadata() {
    let source = r#"
class Meta {
    @Deprecated
    <T> Meta(T value) throws IllegalArgumentException {}

    @Override
    public <T> T id(T value) throws RuntimeException { return value; }
}

@interface Flag {
    String value() default "on";
}
"#;

    let items = parse_and_extract(source);

    let ctor = items
        .iter()
        .find(|item| item.kind == SymbolKind::Constructor && item.name == "Meta")
        .expect("expected constructor");
    assert_eq!(ctor.metadata.type_parameters.as_deref(), Some("<T>"));
    assert!(
        ctor.metadata
            .attributes
            .iter()
            .any(|attr| attr == "@Deprecated")
    );
    assert!(
        ctor.metadata
            .attributes
            .iter()
            .any(|attr| attr.contains("throws IllegalArgumentException"))
    );

    let method = find_by_name(&items, "id");
    assert_eq!(method.metadata.type_parameters.as_deref(), Some("<T>"));
    assert!(
        method
            .metadata
            .attributes
            .iter()
            .any(|attr| attr == "@Override")
    );
    assert!(
        method
            .metadata
            .attributes
            .iter()
            .any(|attr| attr.contains("throws RuntimeException"))
    );

    let annotation_member = find_by_name(&items, "value");
    assert!(
        annotation_member
            .metadata
            .attributes
            .iter()
            .any(|attr| attr.contains("default \"on\""))
    );
}
