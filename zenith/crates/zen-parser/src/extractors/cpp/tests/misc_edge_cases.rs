use super::*;

// ════════════════════════════════════════════════════════════════
// 21. Misc edge cases
// ════════════════════════════════════════════════════════════════

#[test]
fn operator_km_literal_extracted() {
    let items = fixture_items();
    assert!(
        items
            .iter()
            .any(|i| i.kind == SymbolKind::Function && i.name.contains("operator")),
        "user-defined literal operator should be extracted as a Function"
    );
}

#[test]
fn operator_km_literal_has_operator_attr() {
    let items = fixture_items();
    let op = items
        .iter()
        .find(|i| i.kind == SymbolKind::Function && i.name.contains("_km"))
        .expect("operator\"\"_km should exist");
    assert!(
        op.metadata.attributes.contains(&"operator".to_string()),
        "operator\"\"_km should have 'operator' attribute, got {:?}",
        op.metadata.attributes
    );
}

#[test]
fn empty_source_returns_empty() {
    let items = parse_and_extract("");
    assert!(items.is_empty(), "empty source should produce no items");
}

#[test]
fn minimal_class() {
    let items = parse_and_extract("class Foo {};");
    let f = find_by_name(&items, "Foo");
    assert_eq!(f.kind, SymbolKind::Class);
}

#[test]
fn minimal_namespace() {
    let items = parse_and_extract("namespace bar {}");
    let b = find_by_name(&items, "bar");
    assert_eq!(b.kind, SymbolKind::Module);
    assert!(b.metadata.attributes.contains(&"namespace".to_string()));
}

#[test]
fn minimal_template_class() {
    let items = parse_and_extract("template<typename T> class Box {};");
    let b = find_by_name(&items, "Box");
    assert_eq!(b.kind, SymbolKind::Class);
    assert!(b.metadata.attributes.contains(&"template".to_string()));
    assert!(b.metadata.generics.is_some());
}

#[test]
fn minimal_concept() {
    let items =
        parse_and_extract("template<typename T> concept Hashable = requires(T a) { a.hash(); };");
    let h = find_by_name(&items, "Hashable");
    assert_eq!(h.kind, SymbolKind::Trait);
    assert!(h.metadata.attributes.contains(&"concept".to_string()));
}

#[test]
fn minimal_using_alias() {
    let items = parse_and_extract("using MyInt = int;");
    let mi = find_by_name(&items, "MyInt");
    assert_eq!(mi.kind, SymbolKind::TypeAlias);
    assert!(mi.metadata.attributes.contains(&"using".to_string()));
}

#[test]
fn minimal_static_assert() {
    let items = parse_and_extract("static_assert(true, \"ok\");");
    let sa = find_by_name(&items, "static_assert");
    assert_eq!(sa.kind, SymbolKind::Macro);
}

#[test]
fn minimal_extern_c() {
    let items = parse_and_extract("extern \"C\" { void foo(); }");
    assert!(
        items.iter().any(|i| {
            i.metadata
                .attributes
                .contains(&"linkage_specification".to_string())
        }),
        "extern C block should be extracted"
    );
}

#[test]
fn minimal_enum_class() {
    let items = parse_and_extract("enum class Direction { Up, Down, Left, Right };");
    let d = find_by_name(&items, "Direction");
    assert_eq!(d.kind, SymbolKind::Enum);
    assert!(
        d.metadata.attributes.contains(&"scoped_enum".to_string()),
        "enum class should be scoped"
    );
    assert!(d.metadata.variants.contains(&"Up".to_string()));
}

#[test]
fn minimal_final_class() {
    let items = parse_and_extract("class Sealed final {};");
    let s = find_by_name(&items, "Sealed");
    assert_eq!(s.kind, SymbolKind::Class);
    assert!(s.metadata.attributes.contains(&"final".to_string()));
}

#[test]
fn minimal_class_inheritance() {
    let items = parse_and_extract("class A {}; class B : public A {};");
    let b = find_by_name(&items, "B");
    assert!(
        b.metadata.base_classes.contains(&"A".to_string()),
        "B should inherit from A"
    );
}

#[test]
fn shared_value_extern() {
    let items = fixture_items();
    let sv = find_by_name(&items, "shared_value");
    assert!(
        sv.kind == SymbolKind::Static || sv.kind == SymbolKind::Const,
        "shared_value should be Static or Const"
    );
}

#[test]
fn class_document_has_methods() {
    let items = fixture_items();
    let d = find_by_name(&items, "Document");
    assert!(
        d.metadata.methods.contains(&"serialize".to_string()),
        "Document should have serialize method"
    );
}

#[test]
fn fixture_no_duplicate_classes() {
    let items = fixture_items();
    let classes = find_all_by_kind(&items, SymbolKind::Class);
    let mut names: Vec<_> = classes.iter().map(|c| &c.name).collect();
    let total = names.len();
    names.sort();
    names.dedup();
    // Allow some duplicates from specializations but not excessive
    assert!(
        names.len() >= total / 2,
        "too many duplicate class names: {total} total, {} unique",
        names.len()
    );
}

#[test]
fn class_shape_class_attribute() {
    let items = fixture_items();
    let shape = find_by_name(&items, "Shape");
    assert!(
        shape.metadata.attributes.contains(&"class".to_string()),
        "Shape should have 'class' attribute"
    );
}

#[test]
fn class_square_not_abstract() {
    let items = fixture_items();
    let sq = find_by_name(&items, "Square");
    assert!(
        !sq.metadata.attributes.contains(&"abstract".to_string()),
        "Square should NOT be abstract"
    );
}

#[test]
fn minimal_abstract_class() {
    let items = parse_and_extract(
        "class IFoo {\npublic:\n    virtual void bar() = 0;\n    virtual ~IFoo() = default;\n};",
    );
    let f = find_by_name(&items, "IFoo");
    assert!(f.metadata.attributes.contains(&"abstract".to_string()));
}

#[test]
fn minimal_multiple_inheritance() {
    let items = parse_and_extract("class X {}; class Y {}; class Z : public X, public Y {};");
    let z = find_by_name(&items, "Z");
    assert!(z.metadata.base_classes.len() >= 2);
}

#[test]
fn class_document_has_title_method() {
    let items = fixture_items();
    let d = find_by_name(&items, "Document");
    assert!(
        d.metadata.methods.contains(&"title".to_string()),
        "Document should have title method, got {:?}",
        d.metadata.methods
    );
}

#[test]
fn namespace_signature_format() {
    let items = fixture_items();
    let m = find_by_name(&items, "math");
    assert!(
        m.signature.contains("namespace"),
        "namespace signature should contain 'namespace', got {:?}",
        m.signature
    );
}

#[test]
fn constexpr_pi_has_return_type() {
    let items = fixture_items();
    let pi = find_by_name(&items, "PI");
    assert!(
        pi.metadata
            .return_type
            .as_deref()
            .unwrap_or("")
            .contains("double"),
        "PI should have double return_type, got {:?}",
        pi.metadata.return_type
    );
}
