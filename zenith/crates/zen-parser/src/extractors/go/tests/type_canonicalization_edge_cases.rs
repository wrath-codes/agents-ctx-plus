use super::*;

#[test]
fn canonicalizes_return_and_parameter_whitespace() {
    let source = r"package demo
func Divide(a, b float64) ( result float64 , err error ) { return 0, nil }
";

    let items = parse_and_extract(source);
    let divide = find_by_name(&items, "Divide");

    assert_eq!(
        divide.metadata.return_type.as_deref(),
        Some("(resultfloat64,errerror)")
    );
    assert!(divide.metadata.parameters.iter().any(|p| p == "a,bfloat64"));
}

#[test]
fn marks_variadic_functions_with_tag() {
    let source = r"package demo
func Printf(format string, args ...interface{}) {}
";

    let items = parse_and_extract(source);
    let printf = find_by_name(&items, "Printf");
    assert!(
        printf
            .metadata
            .attributes
            .iter()
            .any(|a| a == "go:variadic")
    );
}
