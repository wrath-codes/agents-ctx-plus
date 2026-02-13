use super::*;

#[test]
fn luadoc_param_and_return_tags_are_normalized() {
    let source = r"
---@param x number
---@param y string
---@return boolean
local function check(x, y)
  return x > 0 and #y > 0
end
";

    let items = parse_and_extract(source);
    let check = find_by_name(&items, "check");

    assert!(
        check
            .metadata
            .attributes
            .iter()
            .any(|a| a == "luadoc:param:x:number")
    );
    assert!(
        check
            .metadata
            .attributes
            .iter()
            .any(|a| a == "luadoc:param:y:string")
    );
    assert!(
        check
            .metadata
            .attributes
            .iter()
            .any(|a| a == "luadoc:return:boolean")
    );
    assert_eq!(check.metadata.return_type.as_deref(), Some("boolean"));
}

#[test]
fn luadoc_class_field_and_type_tags_are_captured() {
    let source = r"
---@class User
---@field id number
local User = {}

---@type table<string,number>
local scores = {}
";

    let items = parse_and_extract(source);
    let user = find_by_name(&items, "User");
    assert!(
        user.metadata
            .attributes
            .iter()
            .any(|a| a == "luadoc:class:User")
    );
    assert!(
        user.metadata
            .attributes
            .iter()
            .any(|a| a == "luadoc:field:id:number")
    );

    let scores = find_by_name(&items, "scores");
    assert!(
        scores
            .metadata
            .attributes
            .iter()
            .any(|a| a == "luadoc:type:table<string,number>")
    );
}
