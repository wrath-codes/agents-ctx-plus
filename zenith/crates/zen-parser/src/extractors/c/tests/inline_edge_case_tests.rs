use super::*;

// ── Inline / edge case tests ──────────────────────────────────

#[test]
fn inline_empty_source() {
    let items = parse_and_extract("");
    assert!(items.is_empty(), "empty source should yield no items");
}

#[test]
fn inline_single_function() {
    let items = parse_and_extract("int main(void) { return 0; }");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].kind, SymbolKind::Function);
    assert_eq!(items[0].name, "main");
}

#[test]
fn inline_single_struct() {
    let items = parse_and_extract("struct Foo { int bar; };");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].kind, SymbolKind::Struct);
    assert_eq!(items[0].name, "Foo");
    assert!(items[0].metadata.fields.contains(&"bar".to_string()));
}

#[test]
fn inline_single_enum() {
    let items = parse_and_extract("enum Dir { NORTH, SOUTH, EAST, WEST };");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].kind, SymbolKind::Enum);
    assert_eq!(items[0].metadata.variants.len(), 4);
}

#[test]
fn inline_single_typedef() {
    let items = parse_and_extract("typedef int MyInt;");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].kind, SymbolKind::TypeAlias);
}

#[test]
fn inline_global_variable() {
    let items = parse_and_extract("int x = 42;");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].kind, SymbolKind::Static);
    assert_eq!(items[0].name, "x");
}

#[test]
fn inline_const_variable() {
    let items = parse_and_extract("const int Y = 100;");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].kind, SymbolKind::Const);
}

#[test]
fn inline_static_variable() {
    let items = parse_and_extract("static int hidden = 0;");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].visibility, Visibility::Private);
}

#[test]
fn inline_extern_variable() {
    let items = parse_and_extract("extern int external;");
    assert_eq!(items.len(), 1);
    assert!(items[0].metadata.attributes.contains(&"extern".to_string()));
}

#[test]
fn inline_include_system() {
    let items = parse_and_extract("#include <math.h>\n");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].kind, SymbolKind::Module);
    assert!(items[0].metadata.attributes.contains(&"system".to_string()));
}

#[test]
fn inline_include_local() {
    let items = parse_and_extract("#include \"header.h\"\n");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].kind, SymbolKind::Module);
    assert!(items[0].metadata.attributes.contains(&"local".to_string()));
}

#[test]
fn inline_define_object() {
    let items = parse_and_extract("#define FOO 42\n");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].kind, SymbolKind::Const);
    assert_eq!(items[0].name, "FOO");
}

#[test]
fn inline_define_function() {
    let items = parse_and_extract("#define ADD(a,b) ((a)+(b))\n");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].kind, SymbolKind::Macro);
    assert_eq!(items[0].name, "ADD");
    assert!(items[0].metadata.parameters.contains(&"a".to_string()));
    assert!(items[0].metadata.parameters.contains(&"b".to_string()));
}

#[test]
fn inline_union() {
    let items = parse_and_extract("union U { int a; float b; };");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].kind, SymbolKind::Union);
    assert_eq!(items[0].metadata.fields.len(), 2);
}

#[test]
fn inline_forward_declaration() {
    let items = parse_and_extract("struct Forward;");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].kind, SymbolKind::Struct);
    assert!(
        items[0]
            .metadata
            .attributes
            .contains(&"forward_declaration".to_string())
    );
}

#[test]
fn inline_function_pointer_typedef() {
    let items = parse_and_extract("typedef void (*Handler)(int);");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].kind, SymbolKind::TypeAlias);
    assert!(
        items[0]
            .metadata
            .attributes
            .contains(&"function_pointer".to_string())
    );
}

#[test]
fn inline_prototype() {
    let items = parse_and_extract("int foo(int x);");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].kind, SymbolKind::Function);
    assert!(
        items[0]
            .metadata
            .attributes
            .contains(&"prototype".to_string())
    );
}

#[test]
fn inline_static_assert() {
    let items = parse_and_extract("_Static_assert(1, \"always true\");");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].name, "_Static_assert");
}

#[test]
fn inline_array_declaration() {
    let items = parse_and_extract("int data[100];");
    assert_eq!(items.len(), 1);
    assert!(items[0].metadata.attributes.contains(&"array".to_string()));
}

#[test]
fn inline_typedef_struct() {
    let items = parse_and_extract("typedef struct { int x; } Wrapper;");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].kind, SymbolKind::Struct);
    assert_eq!(items[0].name, "Wrapper");
    assert!(items[0].metadata.fields.contains(&"x".to_string()));
}

#[test]
fn inline_typedef_enum() {
    let items = parse_and_extract("typedef enum { A, B, C } Letters;");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].kind, SymbolKind::Enum);
    assert_eq!(items[0].name, "Letters");
    assert_eq!(items[0].metadata.variants.len(), 3);
}

#[test]
fn inline_doc_comment_above_function() {
    let items = parse_and_extract("/* My function */\nint f(void) { return 0; }");
    assert_eq!(items.len(), 1);
    assert!(
        items[0].doc_comment.contains("My function"),
        "doc comment: {:?}",
        items[0].doc_comment
    );
}

#[test]
fn inline_multiple_items() {
    let source = "int x = 1;\nint y = 2;\nint z = 3;";
    let items = parse_and_extract(source);
    assert_eq!(items.len(), 3);
}

#[test]
fn inline_variadic_prototype() {
    let items = parse_and_extract("int printf(const char *fmt, ...);");
    assert_eq!(items.len(), 1);
    assert!(
        items[0]
            .metadata
            .attributes
            .contains(&"variadic".to_string()),
        "should detect variadic: {:?}",
        items[0].metadata.attributes
    );
}

#[test]
fn inline_function_pointer_variable() {
    let items = parse_and_extract("void (*handler)(int) = 0;");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].name, "handler");
    assert!(
        items[0]
            .metadata
            .attributes
            .contains(&"function_pointer".to_string())
    );
}

#[test]
fn visibility_static_is_private() {
    let items = parse_and_extract("static void internal(void) {}");
    assert_eq!(items[0].visibility, Visibility::Private);
}

#[test]
fn visibility_extern_is_public() {
    let items = parse_and_extract("extern int api_func(void);");
    assert_eq!(items[0].visibility, Visibility::Public);
}

#[test]
fn visibility_default_is_public() {
    let items = parse_and_extract("int regular(void) { return 0; }");
    assert_eq!(items[0].visibility, Visibility::Public);
}
