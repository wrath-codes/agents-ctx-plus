use ast_grep_language::{LanguageExt, SupportLang};

use crate::types::SymbolKind;

#[test]
fn constructor_normalization_across_languages() {
    let js_source = "class User { constructor(name) { this.name = name; } }";
    let js_root = ast_grep_language::SupportLang::JavaScript.ast_grep(js_source);
    let js_items = super::javascript::extract(&js_root).expect("js extraction");
    assert!(js_items
        .iter()
        .any(|i| i.kind == SymbolKind::Constructor
            && i.metadata.owner_name.as_deref() == Some("User")));

    let ts_source = "class User { constructor(public name: string) {} }";
    let ts_root = SupportLang::TypeScript.ast_grep(ts_source);
    let ts_items =
        super::typescript::extract(&ts_root, SupportLang::TypeScript).expect("ts extraction");
    assert!(ts_items
        .iter()
        .any(|i| i.kind == SymbolKind::Constructor
            && i.metadata.owner_name.as_deref() == Some("User")));

    let py_source = "class User:\n    def __init__(self, name):\n        self.name = name\n";
    let py_root = ast_grep_language::SupportLang::Python.ast_grep(py_source);
    let py_items = super::python::extract(&py_root).expect("python extraction");
    assert!(py_items
        .iter()
        .any(|i| i.kind == SymbolKind::Constructor
            && i.metadata.owner_name.as_deref() == Some("User")));

    let rust_source = "struct User; impl User { fn new() -> Self { Self } }";
    let rust_root = SupportLang::Rust.ast_grep(rust_source);
    let rust_items = super::rust::extract(&rust_root, rust_source).expect("rust extraction");
    assert!(rust_items
        .iter()
        .any(|i| i.kind == SymbolKind::Constructor && i.name == "new"));

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

    let php_source = "<?php class User { public function __construct(string $name) {} }";
    let php_root = SupportLang::Php.ast_grep(php_source);
    let php_items = super::php::extract(&php_root).expect("php extraction");
    assert!(php_items.iter().any(|i| {
        i.kind == SymbolKind::Constructor && i.metadata.owner_name.as_deref() == Some("User")
    }));

    let ruby_source = "class User; def initialize(name); @name = name; end; end";
    let ruby_root = SupportLang::Ruby.ast_grep(ruby_source);
    let ruby_items = super::ruby::extract(&ruby_root).expect("ruby extraction");
    assert!(ruby_items.iter().any(|i| {
        i.kind == SymbolKind::Constructor && i.metadata.owner_name.as_deref() == Some("User")
    }));
}

#[test]
fn property_and_field_members_have_owner_metadata() {
    assert_js_member_ownership();
    assert_ts_member_ownership();
    assert_java_member_ownership();
    assert_lua_member_ownership();
    assert_php_member_ownership();
    assert_go_member_ownership();
    assert_ruby_member_ownership();
    assert_json_member_ownership();
    assert_yaml_member_ownership();
}

fn assert_js_member_ownership() {
    let js_source = "class Card { get title() { return 'x'; } set title(v) {} id = 1; }";
    let js_root = ast_grep_language::SupportLang::JavaScript.ast_grep(js_source);
    let js_items = super::javascript::extract(&js_root).expect("js extraction");

    let title = js_items
        .iter()
        .find(|i| i.kind == SymbolKind::Property && i.name == "title")
        .expect("expected property member item");
    assert_eq!(title.metadata.owner_name.as_deref(), Some("Card"));
    assert_eq!(title.metadata.owner_kind, Some(SymbolKind::Class));
}

fn assert_ts_member_ownership() {
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
}

fn assert_java_member_ownership() {
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

fn assert_lua_member_ownership() {
    let lua_source = "local M = { make = function(v) return v end, mode = 'x' }; function M.add(a, b) return a + b end; function M:greet(name) return name end; M.version = '1.0'; M['alias'] = function(v) return v end";
    let lua_root = SupportLang::Lua.ast_grep(lua_source);
    let lua_items = super::lua::extract(&lua_root).expect("lua extraction");

    let lua_add = lua_items
        .iter()
        .find(|i| i.kind == SymbolKind::Method && i.name == "add")
        .expect("expected lua method member item");
    assert_eq!(lua_add.metadata.owner_name.as_deref(), Some("M"));
    assert_eq!(lua_add.metadata.owner_kind, Some(SymbolKind::Module));
    assert!(lua_add.metadata.is_static_member);

    let lua_greet = lua_items
        .iter()
        .find(|i| i.kind == SymbolKind::Method && i.name == "greet")
        .expect("expected lua colon-method member item");
    assert_eq!(lua_greet.metadata.owner_name.as_deref(), Some("M"));
    assert_eq!(lua_greet.metadata.owner_kind, Some(SymbolKind::Module));
    assert!(!lua_greet.metadata.is_static_member);

    let lua_version = lua_items
        .iter()
        .find(|i| i.kind == SymbolKind::Field && i.name == "version")
        .expect("expected lua field member item");
    assert_eq!(lua_version.metadata.owner_name.as_deref(), Some("M"));
    assert_eq!(lua_version.metadata.owner_kind, Some(SymbolKind::Module));

    let lua_alias = lua_items
        .iter()
        .find(|i| i.kind == SymbolKind::Method && i.name == "alias")
        .expect("expected lua bracket member method item");
    assert_eq!(lua_alias.metadata.owner_name.as_deref(), Some("M"));
    assert_eq!(lua_alias.metadata.owner_kind, Some(SymbolKind::Module));

    let lua_make = lua_items
        .iter()
        .find(|i| i.kind == SymbolKind::Method && i.name == "make")
        .expect("expected lua table-constructor member method item");
    assert_eq!(lua_make.metadata.owner_name.as_deref(), Some("M"));
}

fn assert_php_member_ownership() {
    let php_source = "<?php class Card { public const MAX = 1; public string $id; }";
    let php_root = SupportLang::Php.ast_grep(php_source);
    let php_items = super::php::extract(&php_root).expect("php extraction");

    let php_max = php_items
        .iter()
        .find(|i| i.kind == SymbolKind::Const && i.name == "MAX")
        .expect("expected php const member item");
    assert_eq!(php_max.metadata.owner_name.as_deref(), Some("Card"));
    assert_eq!(php_max.metadata.owner_kind, Some(SymbolKind::Class));

    let php_id = php_items
        .iter()
        .find(|i| i.kind == SymbolKind::Property && i.name == "id")
        .expect("expected php property member item");
    assert_eq!(php_id.metadata.owner_name.as_deref(), Some("Card"));
    assert_eq!(php_id.metadata.owner_kind, Some(SymbolKind::Class));
}

fn assert_go_member_ownership() {
    let go_source =
        "package demo; type Card struct { id string }; func (c *Card) Set(v string) { c.id = v }";
    let go_root = SupportLang::Go.ast_grep(go_source);
    let go_items = super::go::extract(&go_root).expect("go extraction");

    let go_field = go_items
        .iter()
        .find(|i| i.kind == SymbolKind::Field && i.name == "Card::id")
        .expect("expected go field member item");
    assert_eq!(go_field.metadata.owner_name.as_deref(), Some("Card"));
    assert_eq!(go_field.metadata.owner_kind, Some(SymbolKind::Struct));

    let go_method = go_items
        .iter()
        .find(|i| i.kind == SymbolKind::Method && i.name == "Set")
        .expect("expected go method member item");
    assert_eq!(go_method.metadata.owner_name.as_deref(), Some("Card"));
    assert_eq!(go_method.metadata.owner_kind, Some(SymbolKind::Struct));
    assert!(!go_method.metadata.is_static_member);
}

fn assert_ruby_member_ownership() {
    let ruby_source =
        "class Card; attr_reader :id; def initialize(id); @id = id; end; def total; id; end; end";
    let ruby_root = SupportLang::Ruby.ast_grep(ruby_source);
    let ruby_items = super::ruby::extract(&ruby_root).expect("ruby extraction");

    let property = ruby_items
        .iter()
        .find(|i| i.kind == SymbolKind::Property && i.name == "id")
        .expect("expected ruby property member item");
    assert_eq!(property.metadata.owner_name.as_deref(), Some("Card"));
    assert_eq!(property.metadata.owner_kind, Some(SymbolKind::Class));

    let method = ruby_items
        .iter()
        .find(|i| i.kind == SymbolKind::Method && i.name == "total")
        .expect("expected ruby method member item");
    assert_eq!(method.metadata.owner_name.as_deref(), Some("Card"));
    assert_eq!(method.metadata.owner_kind, Some(SymbolKind::Class));
    assert!(!method.metadata.is_static_member);
}

fn assert_json_member_ownership() {
    let json_source = "{\"app\":{\"name\":\"zenith\"},\"routes\":[{\"path\":\"/health\"}]}";
    let json_root = SupportLang::Json.ast_grep(json_source);
    let json_items = super::json::extract(&json_root).expect("json extraction");

    let app_name = json_items
        .iter()
        .find(|i| i.kind == SymbolKind::Property && i.name == "app.name")
        .expect("expected json property member item");
    assert_eq!(app_name.metadata.owner_name.as_deref(), Some("app"));
    assert_eq!(app_name.metadata.owner_kind, Some(SymbolKind::Module));

    let route_path = json_items
        .iter()
        .find(|i| i.kind == SymbolKind::Property && i.name == "routes[0].path")
        .expect("expected json array-nested property member item");
    assert_eq!(route_path.metadata.owner_name.as_deref(), Some("routes[0]"));
    assert_eq!(route_path.metadata.owner_kind, Some(SymbolKind::Module));
}

fn assert_yaml_member_ownership() {
    let yaml_source = "app:\n  name: zenith\nroutes:\n  - path: /health\n";
    let yaml_root = SupportLang::Yaml.ast_grep(yaml_source);
    let yaml_items = super::yaml::extract(&yaml_root).expect("yaml extraction");

    let app_name = yaml_items
        .iter()
        .find(|i| i.kind == SymbolKind::Property && i.name == "app.name")
        .expect("expected yaml property member item");
    assert_eq!(app_name.metadata.owner_name.as_deref(), Some("app"));
    assert_eq!(app_name.metadata.owner_kind, Some(SymbolKind::Module));

    let route_path = yaml_items
        .iter()
        .find(|i| i.kind == SymbolKind::Property && i.name == "routes[0].path")
        .expect("expected yaml array-nested property member item");
    assert_eq!(route_path.metadata.owner_name.as_deref(), Some("routes[0]"));
    assert_eq!(route_path.metadata.owner_kind, Some(SymbolKind::Module));
}
