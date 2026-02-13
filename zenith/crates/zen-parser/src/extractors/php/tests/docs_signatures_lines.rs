use super::*;

#[test]
fn extracts_phpdoc_signature_and_lines() {
    let source = r"
<?php
/**
 * Build score.
 * @param int $x
 * @return int
 */
function score(int $x): int {
    return $x + 1;
}
";

    let items = parse_and_extract(source);
    let score = find_by_name(&items, "score");

    assert!(score.doc_comment.contains("Build score."));
    assert!(score.doc_comment.contains("@param int $x"));
    assert!(score.signature.contains("function score(int $x): int"));
    assert_eq!(score.metadata.return_type.as_deref(), Some("int"));
    assert!(score.start_line >= 7);
    assert!(score.end_line >= score.start_line);
}
