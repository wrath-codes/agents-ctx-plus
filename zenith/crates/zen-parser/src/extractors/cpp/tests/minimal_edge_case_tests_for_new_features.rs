use super::*;

// ════════════════════════════════════════════════════════════════
// 43. Minimal edge-case tests for new features
// ════════════════════════════════════════════════════════════════

#[test]
fn minimal_friend_function() {
    let items = parse_and_extract("class C { friend void f(); public: void g() {} };");
    let c = find_by_name(&items, "C");
    let has_friend = c.metadata.attributes.iter().any(|a| a.contains("friend"));
    assert!(has_friend, "class with friend function should track it");
}

#[test]
fn minimal_nested_enum() {
    let items = parse_and_extract("class Outer { public: enum class E { A, B }; };");
    let e = items
        .iter()
        .find(|i| i.kind == SymbolKind::Enum && i.name == "E");
    assert!(e.is_some(), "nested enum should be extracted");
}

#[test]
fn minimal_nested_struct() {
    let items = parse_and_extract("class Outer { public: struct S { int x; }; };");
    let s = items
        .iter()
        .find(|i| i.kind == SymbolKind::Struct && i.name == "S");
    assert!(s.is_some(), "nested struct should be extracted");
}

#[test]
fn minimal_override_method() {
    let items = parse_and_extract(
        "class Base { public: virtual void f() {} };\nclass D : public Base { public: void f() override {} };",
    );
    let d = find_by_name(&items, "D");
    assert!(
        d.metadata.attributes.contains(&"has_override".to_string()),
        "D should have has_override attribute"
    );
}

#[test]
fn minimal_deleted_constructor() {
    let items = parse_and_extract(
        "class NoCopy { public: NoCopy() = default; NoCopy(const NoCopy&) = delete; };",
    );
    let nc = find_by_name(&items, "NoCopy");
    assert!(
        nc.metadata
            .attributes
            .contains(&"has_deleted_members".to_string()),
        "NoCopy should have has_deleted_members, got {:?}",
        nc.metadata.attributes
    );
}

#[test]
fn minimal_union_in_namespace() {
    let items = parse_and_extract("namespace ns { union U { int x; double y; }; }");
    let u = items
        .iter()
        .find(|i| i.kind == SymbolKind::Union && i.name == "U");
    assert!(u.is_some(), "union in namespace should be extracted");
}

#[test]
fn minimal_template_instantiation() {
    let items = parse_and_extract("template<typename T> class V {}; template class V<int>;");
    let inst = items.iter().find(|i| {
        i.metadata
            .attributes
            .contains(&"explicit_instantiation".to_string())
    });
    assert!(
        inst.is_some(),
        "explicit template instantiation should be extracted"
    );
}

#[test]
fn minimal_requires_clause() {
    let items = parse_and_extract("template<typename T> requires true T ident(T x) { return x; }");
    let f = find_by_name(&items, "ident");
    let has_requires = f
        .metadata
        .attributes
        .iter()
        .any(|a| a.starts_with("requires:"));
    assert!(
        has_requires,
        "function with requires clause should have requires attr, got {:?}",
        f.metadata.attributes
    );
}
