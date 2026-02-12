use super::*;

#[test]
fn line_numbers_are_one_based() {
    let source = include_str!("../../../../tests/fixtures/sample.ex");
    let items = parse_and_extract(source);
    let m = find_by_name(&items, "Sample.Processor");
    assert!(m.start_line >= 1, "start_line should be 1-based");
    assert!(
        m.end_line > m.start_line,
        "end_line {} should be > start_line {}",
        m.end_line,
        m.start_line
    );
}

#[test]
fn function_signature_format() {
    let source = include_str!("../../../../tests/fixtures/sample.ex");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "process");
    assert!(
        f.signature.starts_with("def process"),
        "sig: {:?}",
        f.signature
    );
}

#[test]
fn private_function_signature() {
    let source = include_str!("../../../../tests/fixtures/sample.ex");
    let items = parse_and_extract(source);
    let transforms: Vec<_> = items
        .iter()
        .filter(|i| i.name == "transform" && i.visibility == Visibility::Private)
        .collect();
    assert!(!transforms.is_empty());
    assert!(
        transforms[0].signature.starts_with("defp transform"),
        "sig: {:?}",
        transforms[0].signature
    );
}

#[test]
fn macro_signature_format() {
    let source = include_str!("../../../../tests/fixtures/sample.ex");
    let items = parse_and_extract(source);
    let m = find_by_name(&items, "define_handler");
    assert!(
        m.signature.starts_with("defmacro define_handler"),
        "sig: {:?}",
        m.signature
    );
}
