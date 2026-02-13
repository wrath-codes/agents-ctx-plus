use super::*;

#[test]
fn luadoc_enriches_declared_parameters() {
    let source = r"
---@param x number
---@param y string
---@return boolean
function User.check(x, y)
  return x > 0 and #y > 0
end
";

    let items = parse_and_extract(source);
    let check = find_by_name(&items, "check");

    assert!(check.metadata.parameters.iter().any(|p| p == "x: number"));
    assert!(check.metadata.parameters.iter().any(|p| p == "y: string"));
    assert_eq!(check.metadata.return_type.as_deref(), Some("boolean"));
}
