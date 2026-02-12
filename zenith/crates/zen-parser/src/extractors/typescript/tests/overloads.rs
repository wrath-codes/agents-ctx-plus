use super::*;

#[test]
fn function_overload_signatures_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.ts");
    let items = parse_and_extract(source);
    let greets: Vec<&ParsedItem> = items.iter().filter(|i| i.name == "greet").collect();
    assert!(
        greets.len() >= 3,
        "should find at least 3 greet items (2 overloads + 1 impl), found {}",
        greets.len()
    );
}

#[test]
fn function_overload_has_jsdoc() {
    let source = include_str!("../../../../tests/fixtures/sample.ts");
    let items = parse_and_extract(source);
    let greets: Vec<&ParsedItem> = items.iter().filter(|i| i.name == "greet").collect();
    let has_doc = greets
        .iter()
        .any(|g| g.doc_comment.contains("Greet a person"));
    assert!(has_doc, "at least one greet should have JSDoc");
}
