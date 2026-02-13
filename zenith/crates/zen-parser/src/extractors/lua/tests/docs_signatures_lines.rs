use super::*;

#[test]
fn extracts_docs_signature_and_lines() {
    let source = r"
-- Compute score.
function score(x, y)
    return x + y
end
";

    let items = parse_and_extract(source);
    let score = find_by_name(&items, "score");

    assert!(score.doc_comment.contains("Compute score."));
    assert!(score.signature.contains("function score(x, y)"));
    assert!(score.start_line >= 2);
    assert!(score.end_line >= score.start_line);
}

#[test]
fn preserves_luadoc_annotations() {
    let source = r"
---@param x number
---@param y number
---@return number
function sum(x, y)
    return x + y
end
";

    let items = parse_and_extract(source);
    let sum = find_by_name(&items, "sum");

    assert!(sum.doc_comment.contains("@param x number"));
    assert!(sum.doc_comment.contains("@param y number"));
    assert!(sum.doc_comment.contains("@return number"));
    assert!(sum.metadata.parameters.iter().any(|p| p == "x: number"));
    assert!(sum.metadata.parameters.iter().any(|p| p == "y: number"));
    assert_eq!(sum.metadata.return_type.as_deref(), Some("number"));
    assert!(
        sum.metadata
            .attributes
            .iter()
            .any(|a| a == "luadoc:param:x:number")
    );
    assert!(
        sum.metadata
            .attributes
            .iter()
            .any(|a| a == "luadoc:return:number")
    );
}
