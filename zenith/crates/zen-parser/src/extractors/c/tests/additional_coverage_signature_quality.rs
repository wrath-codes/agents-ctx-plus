use super::*;

// ── Additional coverage: signature quality ────────────────────

#[test]
fn function_signature_normalized() {
    let items = parse_and_extract("int   spaced_func(  int   x,  int   y  ) { return x+y; }");
    let f = find_by_name(&items, "spaced_func");
    // Signature should be whitespace-normalized
    assert!(
        !f.signature.contains("  "),
        "signature should not have double spaces: {:?}",
        f.signature
    );
}

#[test]
fn prototype_signature_no_body() {
    let items = parse_and_extract("int proto_func(int x);");
    assert!(
        !items[0].signature.contains('{'),
        "prototype signature should not contain braces"
    );
}
