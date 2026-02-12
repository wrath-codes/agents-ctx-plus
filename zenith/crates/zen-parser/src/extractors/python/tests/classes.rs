use super::*;

#[test]
fn class_docstring_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    let base = find_by_name(&items, "BaseProcessor");
    assert_eq!(base.kind, SymbolKind::Class);
    assert!(
        base.doc_comment.contains("base processor"),
        "doc: {:?}",
        base.doc_comment
    );
}

#[test]
fn class_methods_listed() {
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    let base = find_by_name(&items, "BaseProcessor");
    assert!(
        base.metadata.methods.contains(&"process".to_string()),
        "methods: {:?}",
        base.metadata.methods
    );
    assert!(
        base.metadata.methods.contains(&"helper".to_string()),
        "methods: {:?}",
        base.metadata.methods
    );
}

#[test]
fn dataclass_detected() {
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    let config = find_by_name(&items, "Config");
    assert_eq!(config.kind, SymbolKind::Class);
    assert!(config.metadata.is_dataclass);
    assert!(
        config.metadata.decorators.iter().any(|d| d == "dataclass"),
        "decorators: {:?}",
        config.metadata.decorators
    );
}

#[test]
fn protocol_detected() {
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    let validator = find_by_name(&items, "Validator");
    assert!(validator.metadata.is_protocol);
    assert!(
        validator
            .metadata
            .base_classes
            .contains(&"Protocol".to_string()),
        "base_classes: {:?}",
        validator.metadata.base_classes
    );
}

#[test]
fn pydantic_model_detected() {
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    let settings = find_by_name(&items, "UserSettings");
    assert_eq!(settings.kind, SymbolKind::Class);
    assert!(settings.metadata.is_pydantic);
    assert!(
        settings
            .metadata
            .base_classes
            .contains(&"BaseModel".to_string()),
        "base_classes: {:?}",
        settings.metadata.base_classes
    );
}

#[test]
fn enum_class_detected() {
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    let status = find_by_name(&items, "Status");
    assert_eq!(status.kind, SymbolKind::Enum);
    assert!(status.metadata.is_enum);
    assert!(
        status.metadata.base_classes.contains(&"Enum".to_string()),
        "base_classes: {:?}",
        status.metadata.base_classes
    );
}

#[test]
fn int_enum_detected() {
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    let priority = find_by_name(&items, "Priority");
    assert!(priority.metadata.is_enum);
}

#[test]
fn enum_docstring_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    let status = find_by_name(&items, "Status");
    assert!(
        status.doc_comment.contains("enumeration"),
        "doc: {:?}",
        status.doc_comment
    );
}

#[test]
fn pydantic_docstring_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    let settings = find_by_name(&items, "UserSettings");
    assert!(
        settings.doc_comment.contains("user settings"),
        "doc: {:?}",
        settings.doc_comment
    );
}

// ── Critical bug fix tests ─────────────────────────────────────

#[test]
fn protocol_is_interface_kind() {
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    let validator = find_by_name(&items, "Validator");
    assert_eq!(validator.kind, SymbolKind::Interface);
    assert!(validator.metadata.is_protocol);
}

#[test]
fn enum_is_enum_kind() {
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    let status = find_by_name(&items, "Status");
    assert_eq!(status.kind, SymbolKind::Enum);
    let priority = find_by_name(&items, "Priority");
    assert_eq!(priority.kind, SymbolKind::Enum);
}

#[test]
fn strenum_detected() {
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    let http = find_by_name(&items, "HttpMethod");
    assert_eq!(http.kind, SymbolKind::Enum);
    assert!(http.metadata.is_enum);
}

#[test]
fn namedtuple_detected() {
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    let point = find_by_name(&items, "Point");
    assert_eq!(point.kind, SymbolKind::Class);
    assert!(
        point
            .metadata
            .attributes
            .contains(&"namedtuple".to_string()),
        "attrs: {:?}",
        point.metadata.attributes
    );
    assert!(
        point
            .metadata
            .base_classes
            .contains(&"NamedTuple".to_string()),
        "base: {:?}",
        point.metadata.base_classes
    );
}

#[test]
fn typed_dict_detected() {
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    let profile = find_by_name(&items, "UserProfile");
    assert_eq!(profile.kind, SymbolKind::Class);
    assert!(
        profile
            .metadata
            .attributes
            .contains(&"typed_dict".to_string()),
        "attrs: {:?}",
        profile.metadata.attributes
    );
}

#[test]
fn metaclass_filtered_from_base_classes() {
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    let handler = find_by_name(&items, "AbstractHandler");
    assert!(
        !handler
            .metadata
            .base_classes
            .iter()
            .any(|b| b.contains("metaclass")),
        "base_classes should not contain metaclass: {:?}",
        handler.metadata.base_classes
    );
}

#[test]
fn multiple_inheritance_base_classes() {
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    let multi = find_by_name(&items, "MultiBase");
    assert!(
        multi
            .metadata
            .base_classes
            .contains(&"BaseProcessor".to_string()),
        "base: {:?}",
        multi.metadata.base_classes
    );
    assert!(
        multi
            .metadata
            .base_classes
            .contains(&"AbstractHandler".to_string()),
        "base: {:?}",
        multi.metadata.base_classes
    );
}

#[test]
fn generic_class_detected() {
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    let stack = find_by_name(&items, "Stack");
    assert!(
        stack.metadata.attributes.contains(&"generic".to_string()),
        "attrs: {:?}",
        stack.metadata.attributes
    );
    assert!(
        stack.metadata.generics.is_some(),
        "should have generics: {:?}",
        stack.metadata.generics
    );
}

#[test]
fn instance_vars_from_init() {
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    let slotted = find_by_name(&items, "SlottedClass");
    // __init__ sets self.x, self.y, self.name
    assert!(
        slotted.metadata.fields.contains(&"x".to_string()),
        "fields: {:?}",
        slotted.metadata.fields
    );
    assert!(
        slotted.metadata.fields.contains(&"y".to_string()),
        "fields: {:?}",
        slotted.metadata.fields
    );
}

#[test]
fn enum_variants_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    let status = find_by_name(&items, "Status");
    assert!(
        !status.metadata.variants.is_empty(),
        "enum should have variants"
    );
    assert!(
        status.metadata.variants.contains(&"ACTIVE".to_string()),
        "variants: {:?}",
        status.metadata.variants
    );
}

#[test]
fn enum_with_methods() {
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    let direction = find_by_name(&items, "Direction");
    assert_eq!(direction.kind, SymbolKind::Enum);
    assert!(
        direction.metadata.methods.contains(&"opposite".to_string()),
        "methods: {:?}",
        direction.metadata.methods
    );
}

#[test]
fn dataclass_with_decorators() {
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    let immutable = find_by_name(&items, "ImmutableConfig");
    assert!(immutable.metadata.is_dataclass);
    assert!(!immutable.metadata.fields.is_empty(), "should have fields");
}

#[test]
fn outer_class_has_methods() {
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    let outer = find_by_name(&items, "Outer");
    assert!(
        outer.metadata.methods.contains(&"outer_method".to_string()),
        "methods: {:?}",
        outer.metadata.methods
    );
}

#[test]
fn container_dunder_methods() {
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    let container = find_by_name(&items, "Container");
    assert!(container.metadata.methods.contains(&"__init__".to_string()));
    assert!(container.metadata.methods.contains(&"__len__".to_string()));
    assert!(
        container
            .metadata
            .methods
            .contains(&"__getitem__".to_string())
    );
    assert!(container.metadata.methods.contains(&"__iter__".to_string()));
    assert!(container.metadata.methods.contains(&"__repr__".to_string()));
    assert!(
        container
            .metadata
            .methods
            .contains(&"__enter__".to_string())
    );
    assert!(container.metadata.methods.contains(&"__exit__".to_string()));
}

#[test]
fn container_instance_vars() {
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    let container = find_by_name(&items, "Container");
    assert!(
        container.metadata.fields.contains(&"items".to_string()),
        "fields: {:?}",
        container.metadata.fields
    );
}

// ── Function feature tests ─────────────────────────────────────

#[test]
fn cached_property_detected() {
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    let cached = find_by_name(&items, "CachedExample");
    assert!(
        cached.metadata.methods.contains(&"expensive".to_string()),
        "methods: {:?}",
        cached.metadata.methods
    );
}

#[test]
fn property_example_methods() {
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    let prop = find_by_name(&items, "PropertyExample");
    assert!(
        prop.metadata.methods.contains(&"__init__".to_string()),
        "methods: {:?}",
        prop.metadata.methods
    );
    // value appears as property getter (and setter/deleter with same name)
    assert!(
        prop.metadata.methods.contains(&"value".to_string()),
        "methods: {:?}",
        prop.metadata.methods
    );
}

// ── Async resource class ───────────────────────────────────────

#[test]
fn async_resource_methods() {
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    let resource = find_by_name(&items, "AsyncResource");
    assert!(
        resource
            .metadata
            .methods
            .contains(&"__aenter__".to_string())
    );
    assert!(resource.metadata.methods.contains(&"__aexit__".to_string()));
    assert!(resource.metadata.methods.contains(&"fetch".to_string()));
}

// ── Visibility example class ───────────────────────────────────

#[test]
fn visibility_example_class() {
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    let vis = find_by_name(&items, "VisibilityExample");
    assert!(
        vis.metadata.methods.contains(&"public_method".to_string()),
        "methods: {:?}",
        vis.metadata.methods
    );
    assert!(
        vis.metadata
            .methods
            .contains(&"_protected_method".to_string()),
        "methods: {:?}",
        vis.metadata.methods
    );
    assert!(
        vis.metadata
            .methods
            .contains(&"__private_method".to_string()),
        "methods: {:?}",
        vis.metadata.methods
    );
    assert!(
        vis.metadata
            .methods
            .contains(&"__dunder_method__".to_string()),
        "methods: {:?}",
        vis.metadata.methods
    );
}

// ── Unit tests for python_visibility ────────────────────────────
