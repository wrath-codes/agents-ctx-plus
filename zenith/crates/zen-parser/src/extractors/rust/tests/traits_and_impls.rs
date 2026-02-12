use super::*;

#[test]
fn trait_methods_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.rs");
    let items = parse_and_extract(source);
    let handler = find_by_name(&items, "Handler");
    assert_eq!(handler.kind, SymbolKind::Trait);
    assert!(
        handler.metadata.methods.contains(&"handle".to_string()),
        "methods: {:?}",
        handler.metadata.methods
    );
}

#[test]
fn trait_associated_types_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.rs");
    let items = parse_and_extract(source);
    let handler = find_by_name(&items, "Handler");
    assert!(
        handler
            .metadata
            .associated_types
            .contains(&"Output".to_string()),
        "associated_types: {:?}",
        handler.metadata.associated_types
    );
}

#[test]
fn impl_methods_as_separate_items() {
    let source = include_str!("../../../../tests/fixtures/sample.rs");
    let items = parse_and_extract(source);
    let methods: Vec<&ParsedItem> = items
        .iter()
        .filter(|i| i.kind == SymbolKind::Method || i.kind == SymbolKind::Constructor)
        .collect();
    let method_names: Vec<&str> = methods.iter().map(|m| m.name.as_str()).collect();
    assert!(
        method_names.contains(&"new"),
        "should have 'new' method: {method_names:?}"
    );
    assert!(
        method_names.contains(&"handle"),
        "should have 'handle' method: {method_names:?}"
    );
}

#[test]
fn trait_impl_methods_have_trait_name() {
    let source = include_str!("../../../../tests/fixtures/sample.rs");
    let items = parse_and_extract(source);
    let handle = items
        .iter()
        .find(|i| i.kind == SymbolKind::Method && i.name == "handle")
        .expect("should find handle method");
    assert!(
        handle.metadata.trait_name.is_some(),
        "trait impl method should have trait_name"
    );
    assert_eq!(
        handle.metadata.for_type.as_deref(),
        Some("Config"),
        "for_type should be Config"
    );
}

#[test]
fn inherent_impl_methods_have_for_type_only() {
    let source = include_str!("../../../../tests/fixtures/sample.rs");
    let items = parse_and_extract(source);
    let new_method = items
        .iter()
        .find(|i| {
            (i.kind == SymbolKind::Method || i.kind == SymbolKind::Constructor) && i.name == "new"
        })
        .expect("should find new method");
    assert!(
        new_method.metadata.trait_name.is_none(),
        "inherent impl should have no trait_name"
    );
    assert_eq!(
        new_method.metadata.for_type.as_deref(),
        Some("Config"),
        "for_type should be Config"
    );
}

#[test]
fn from_impl_detected() {
    let source = include_str!("../../../../tests/fixtures/sample.rs");
    let items = parse_and_extract(source);
    let from_methods: Vec<&ParsedItem> = items
        .iter()
        .filter(|i| {
            i.kind == SymbolKind::Method
                && i.metadata
                    .trait_name
                    .as_deref()
                    .is_some_and(|t| t.contains("From"))
        })
        .collect();
    assert!(
        from_methods.len() >= 2,
        "should find at least 2 From impls, found {}",
        from_methods.len()
    );
    let for_types: Vec<&str> = from_methods
        .iter()
        .filter_map(|m| m.metadata.for_type.as_deref())
        .collect();
    assert!(
        for_types.iter().all(|t| *t == "MyError"),
        "all From impls should be for MyError: {for_types:?}"
    );
}

#[test]
fn from_impl_has_source_type_in_trait_name() {
    let source = include_str!("../../../../tests/fixtures/sample.rs");
    let items = parse_and_extract(source);
    let from_io: Option<&ParsedItem> = items.iter().find(|i| {
        i.kind == SymbolKind::Method
            && i.metadata
                .trait_name
                .as_deref()
                .is_some_and(|t| t.contains("io::Error") || t.contains("io :: Error"))
    });
    assert!(from_io.is_some(), "should find From<std::io::Error> impl");
}

#[test]
fn unsafe_trait_detected() {
    let source = include_str!("../../../../tests/fixtures/sample.rs");
    let items = parse_and_extract(source);
    let t = find_by_name(&items, "ThreadSafe");
    assert_eq!(t.kind, SymbolKind::Trait);
    assert!(t.metadata.is_unsafe, "trait should be unsafe");
}

#[test]
fn unsafe_impl_marks_methods() {
    let source = include_str!("../../../../tests/fixtures/sample.rs");
    let items = parse_and_extract(source);
    let verify_methods: Vec<&ParsedItem> = items
        .iter()
        .filter(|i| {
            i.kind == SymbolKind::Method
                && i.name == "verify"
                && i.metadata
                    .trait_name
                    .as_deref()
                    .is_some_and(|t| t == "ThreadSafe")
        })
        .collect();
    assert!(
        !verify_methods.is_empty(),
        "should find verify method from unsafe impl"
    );
    assert!(
        verify_methods[0].metadata.is_unsafe,
        "method in unsafe impl should be marked unsafe"
    );
}

#[test]
fn supertraits_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.rs");
    let items = parse_and_extract(source);
    let validator = find_by_name(&items, "Validator");
    assert_eq!(validator.kind, SymbolKind::Trait);
    assert!(
        validator
            .metadata
            .base_classes
            .contains(&"Clone".to_string()),
        "supertraits: {:?}",
        validator.metadata.base_classes
    );
    assert!(
        validator
            .metadata
            .base_classes
            .contains(&"Send".to_string()),
        "supertraits: {:?}",
        validator.metadata.base_classes
    );
}

#[test]
fn trait_constants_in_methods() {
    let source = include_str!("../../../../tests/fixtures/sample.rs");
    let items = parse_and_extract(source);
    let configurable = find_by_name(&items, "Configurable");
    assert_eq!(configurable.kind, SymbolKind::Trait);
    assert!(
        configurable
            .metadata
            .methods
            .iter()
            .any(|m| m.contains("MAX_ITEMS")),
        "methods should include const MAX_ITEMS: {:?}",
        configurable.metadata.methods
    );
}

#[test]
fn trait_has_default_method() {
    let source = include_str!("../../../../tests/fixtures/sample.rs");
    let items = parse_and_extract(source);
    let configurable = find_by_name(&items, "Configurable");
    assert!(
        configurable.metadata.methods.contains(&"name".to_string()),
        "methods should include default method 'name': {:?}",
        configurable.metadata.methods
    );
}

#[test]
fn gat_associated_type_has_params() {
    let source = include_str!("../../../../tests/fixtures/sample.rs");
    let items = parse_and_extract(source);
    let lending = find_by_name(&items, "Lending");
    assert_eq!(lending.kind, SymbolKind::Trait);
    assert!(
        lending
            .metadata
            .associated_types
            .iter()
            .any(|a| a.contains("Item") && a.contains('<')),
        "GAT should include type params: {:?}",
        lending.metadata.associated_types
    );
}

#[test]
fn impl_assoc_consts_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.rs");
    let items = parse_and_extract(source);
    let dc = items
        .iter()
        .find(|i| i.name == "DEFAULT_COUNT" && i.kind == SymbolKind::Const)
        .expect("should find DEFAULT_COUNT");
    assert_eq!(
        dc.metadata.for_type.as_deref(),
        Some("Config"),
        "for_type should be Config"
    );
    assert!(
        dc.metadata.return_type.is_some(),
        "should have a type: {:?}",
        dc.metadata.return_type
    );
}

#[test]
fn impl_assoc_const_version() {
    let source = include_str!("../../../../tests/fixtures/sample.rs");
    let items = parse_and_extract(source);
    let v = items
        .iter()
        .find(|i| i.name == "VERSION" && i.kind == SymbolKind::Const)
        .expect("should find VERSION");
    assert_eq!(v.metadata.for_type.as_deref(), Some("Config"));
}

#[test]
fn negative_impl_detected() {
    let source = include_str!("../../../../tests/fixtures/sample.rs");
    let items = parse_and_extract(source);
    let neg = items
        .iter()
        .find(|i| i.name.contains("!Send"))
        .expect("should find negative impl marker");
    assert!(
        neg.metadata
            .attributes
            .contains(&"negative_impl".to_string()),
        "attrs: {:?}",
        neg.metadata.attributes
    );
    assert_eq!(neg.metadata.for_type.as_deref(), Some("RawValue"));
}

#[test]
fn receiver_forms_detected() {
    let source = include_str!("../../../../tests/fixtures/sample.rs");
    let items = parse_and_extract(source);
    let take = items
        .iter()
        .find(|i| {
            i.kind == SymbolKind::Method
                && i.name == "take"
                && i.metadata.for_type.as_deref() == Some("Receiver")
        })
        .expect("should find take method");
    assert!(
        take.metadata.parameters.iter().any(|p| p == "self"),
        "params: {:?}",
        take.metadata.parameters
    );

    let borrow = items
        .iter()
        .find(|i| {
            i.kind == SymbolKind::Method
                && i.name == "borrow"
                && i.metadata.for_type.as_deref() == Some("Receiver")
        })
        .expect("should find borrow method");
    assert!(
        borrow.metadata.parameters.iter().any(|p| p == "&self"),
        "params: {:?}",
        borrow.metadata.parameters
    );

    let mutate = items
        .iter()
        .find(|i| {
            i.kind == SymbolKind::Method
                && i.name == "mutate"
                && i.metadata.for_type.as_deref() == Some("Receiver")
        })
        .expect("should find mutate method");
    assert!(
        mutate.metadata.parameters.iter().any(|p| p == "&mut self"),
        "params: {:?}",
        mutate.metadata.parameters
    );
}

#[test]
fn impl_assoc_type_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.rs");
    let items = parse_and_extract(source);
    let assoc = items
        .iter()
        .find(|i| {
            i.kind == SymbolKind::TypeAlias
                && i.name == "Item"
                && i.metadata.for_type.as_deref() == Some("Receiver")
        })
        .expect("should find associated type Item for Receiver");
    assert_eq!(assoc.metadata.trait_name.as_deref(), Some("Configurable"));
}

#[test]
fn impl_for_reference_type() {
    let source = include_str!("../../../../tests/fixtures/sample.rs");
    let items = parse_and_extract(source);
    let fmt = items
        .iter()
        .find(|i| {
            i.kind == SymbolKind::Method
                && i.name == "fmt"
                && i.metadata
                    .for_type
                    .as_deref()
                    .is_some_and(|t| t.contains('&') && t.contains("RawValue"))
        })
        .expect("should find fmt for &RawValue");
    assert!(fmt.metadata.trait_name.is_some(), "should have trait_name");
}

#[test]
fn no_duplicate_free_functions_from_impls() {
    let source = include_str!("../../../../tests/fixtures/sample.rs");
    let items = parse_and_extract(source);
    let free_fns: Vec<&ParsedItem> = items
        .iter()
        .filter(|i| i.kind == SymbolKind::Function && i.name == "handle")
        .collect();
    assert!(
        free_fns.is_empty(),
        "impl/trait methods should not appear as free functions: {:?}",
        free_fns.iter().map(|i| &i.name).collect::<Vec<_>>()
    );
}

#[test]
fn no_duplicate_new_as_free_function() {
    let source = include_str!("../../../../tests/fixtures/sample.rs");
    let items = parse_and_extract(source);
    assert!(
        !items
            .iter()
            .any(|i| i.kind == SymbolKind::Function && i.name == "new"),
        "impl method 'new' should not appear as free function"
    );
}

#[test]
fn configurable_trait_assoc_type() {
    let source = include_str!("../../../../tests/fixtures/sample.rs");
    let items = parse_and_extract(source);
    let c = find_by_name(&items, "Configurable");
    assert!(
        c.metadata.associated_types.iter().any(|t| t == "Item"),
        "assoc types: {:?}",
        c.metadata.associated_types
    );
}
