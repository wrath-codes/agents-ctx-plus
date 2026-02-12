use ast_grep_language::{LanguageExt, SupportLang};

use crate::types::SymbolKind;

#[test]
fn constructor_normalization_across_languages() {
    let js_source = "class User { constructor(name) { this.name = name; } }";
    let js_root = ast_grep_language::SupportLang::JavaScript.ast_grep(js_source);
    let js_items = super::javascript::extract(&js_root).expect("js extraction");
    assert!(
        js_items.iter().any(|i| i.kind == SymbolKind::Constructor
            && i.metadata.owner_name.as_deref() == Some("User"))
    );

    let ts_source = "class User { constructor(public name: string) {} }";
    let ts_root = SupportLang::TypeScript.ast_grep(ts_source);
    let ts_items =
        super::typescript::extract(&ts_root, SupportLang::TypeScript).expect("ts extraction");
    assert!(
        ts_items.iter().any(|i| i.kind == SymbolKind::Constructor
            && i.metadata.owner_name.as_deref() == Some("User"))
    );

    let py_source = "class User:\n    def __init__(self, name):\n        self.name = name\n";
    let py_root = ast_grep_language::SupportLang::Python.ast_grep(py_source);
    let py_items = super::python::extract(&py_root).expect("python extraction");
    assert!(
        py_items.iter().any(|i| i.kind == SymbolKind::Constructor
            && i.metadata.owner_name.as_deref() == Some("User"))
    );

    let rust_source = "struct User; impl User { fn new() -> Self { Self } }";
    let rust_root = SupportLang::Rust.ast_grep(rust_source);
    let rust_items = super::rust::extract(&rust_root, rust_source).expect("rust extraction");
    assert!(
        rust_items
            .iter()
            .any(|i| i.kind == SymbolKind::Constructor && i.name == "new")
    );

    let csharp_source = "class User { public User(string name) {} }";
    let csharp_root = SupportLang::CSharp.ast_grep(csharp_source);
    let csharp_items = super::csharp::extract(&csharp_root).expect("csharp extraction");
    assert!(csharp_items.iter().any(|i| {
        i.kind == SymbolKind::Constructor && i.metadata.owner_name.as_deref() == Some("User")
    }));

    let java_source = "class User { User(String name) {} }";
    let java_root = SupportLang::Java.ast_grep(java_source);
    let java_items = super::java::extract(&java_root).expect("java extraction");
    assert!(java_items.iter().any(|i| {
        i.kind == SymbolKind::Constructor && i.metadata.owner_name.as_deref() == Some("User")
    }));
}

#[test]
fn property_and_field_members_have_owner_metadata() {
    let js_source = "class Card { get title() { return 'x'; } set title(v) {} id = 1; }";
    let js_root = ast_grep_language::SupportLang::JavaScript.ast_grep(js_source);
    let js_items = super::javascript::extract(&js_root).expect("js extraction");

    let title = js_items
        .iter()
        .find(|i| i.kind == SymbolKind::Property && i.name == "title")
        .expect("expected property member item");
    assert_eq!(title.metadata.owner_name.as_deref(), Some("Card"));
    assert_eq!(title.metadata.owner_kind, Some(SymbolKind::Class));

    let ts_source = "class Card { id: number = 1; }";
    let ts_root = SupportLang::TypeScript.ast_grep(ts_source);
    let ts_items =
        super::typescript::extract(&ts_root, SupportLang::TypeScript).expect("ts extraction");

    let id_field = ts_items
        .iter()
        .find(|i| i.kind == SymbolKind::Field && i.metadata.owner_name.as_deref() == Some("Card"))
        .expect("expected field member item");
    assert_eq!(id_field.metadata.owner_name.as_deref(), Some("Card"));
    assert_eq!(id_field.metadata.owner_kind, Some(SymbolKind::Class));

    let java_source = "class Card { static final int MAX = 1; int id; }";
    let java_root = SupportLang::Java.ast_grep(java_source);
    let java_items = super::java::extract(&java_root).expect("java extraction");

    let java_max = java_items
        .iter()
        .find(|i| i.kind == SymbolKind::Const && i.name == "MAX")
        .expect("expected const member item");
    assert_eq!(java_max.metadata.owner_name.as_deref(), Some("Card"));
    assert_eq!(java_max.metadata.owner_kind, Some(SymbolKind::Class));
    assert!(java_max.metadata.is_static_member);

    let java_id = java_items
        .iter()
        .find(|i| i.kind == SymbolKind::Field && i.name == "id")
        .expect("expected field member item");
    assert_eq!(java_id.metadata.owner_name.as_deref(), Some("Card"));
    assert_eq!(java_id.metadata.owner_kind, Some(SymbolKind::Class));
}
