use super::*;

#[test]
fn extracts_xml_doc_signature_and_lines() {
    let source = r"
public class Docs {
    /// <summary>Compute answer.</summary>
    public int Compute(int x) { return x + 1; }
}
";

    let items = parse_and_extract(source);
    let compute = find_by_name(&items, "Compute");

    assert!(
        compute
            .doc_comment
            .contains("<summary>Compute answer.</summary>")
    );
    assert!(compute.signature.contains("Compute(int x)"));
    assert!(compute.start_line >= 3);
    assert!(compute.end_line >= compute.start_line);
}
