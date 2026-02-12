use super::*;

#[test]
fn extracts_javadoc_signature_and_lines() {
    let source = r"
public class Docs {
    /** Computes answer. */
    public int compute(int x) { return x + 1; }
}
";

    let items = parse_and_extract(source);
    let compute = find_by_name(&items, "compute");

    assert!(compute.doc_comment.contains("Computes answer."));
    assert!(compute.signature.contains("compute(int x)"));
    assert!(compute.start_line >= 3);
    assert!(compute.end_line >= compute.start_line);
}
