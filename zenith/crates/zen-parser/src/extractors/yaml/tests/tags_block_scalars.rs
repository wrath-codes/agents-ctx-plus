use super::*;

#[test]
fn tags_and_block_scalar_styles_are_recorded() {
    let source = r"
message: !str >-
  hello
  world
literal: |
  alpha
  beta
";
    let items = parse_and_extract(source);

    let message = find_by_name(&items, "message");
    assert!(
        message
            .metadata
            .attributes
            .iter()
            .any(|attr| attr == "yaml:tag:str")
    );
    assert!(
        message
            .metadata
            .attributes
            .iter()
            .any(|attr| attr == "yaml:block_style:folded")
    );
    assert!(
        message
            .metadata
            .attributes
            .iter()
            .any(|attr| attr == "yaml:block_header:>-")
    );

    let literal = find_by_name(&items, "literal");
    assert!(
        literal
            .metadata
            .attributes
            .iter()
            .any(|attr| attr == "yaml:block_style:literal")
    );
    assert!(
        literal
            .metadata
            .attributes
            .iter()
            .any(|attr| attr == "yaml:block_header:|")
    );
}
