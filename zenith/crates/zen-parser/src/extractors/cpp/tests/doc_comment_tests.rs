use super::*;

// ════════════════════════════════════════════════════════════════
// 16. Doc comment tests
// ════════════════════════════════════════════════════════════════

#[test]
fn doc_comment_shape() {
    let items = fixture_items();
    let s = find_by_name(&items, "Shape");
    assert!(
        s.doc_comment.contains("Abstract") || s.doc_comment.contains("shape"),
        "Shape should have doc comment about abstract/shape, got {:?}",
        s.doc_comment
    );
}

#[test]
fn doc_comment_circle() {
    let items = fixture_items();
    let c = find_by_name(&items, "Circle");
    assert!(
        !c.doc_comment.is_empty(),
        "Circle should have a doc comment"
    );
}

#[test]
fn doc_comment_container() {
    let items = fixture_items();
    let c = find_by_name(&items, "Container");
    assert!(
        !c.doc_comment.is_empty(),
        "Container should have a doc comment"
    );
}

#[test]
fn doc_comment_math_namespace() {
    let items = fixture_items();
    let m = find_by_name(&items, "math");
    assert!(
        !m.doc_comment.is_empty(),
        "math namespace should have a doc comment"
    );
}

#[test]
fn doc_comment_safe_divide() {
    let items = fixture_items();
    let sd = find_by_name(&items, "safe_divide");
    assert!(
        !sd.doc_comment.is_empty(),
        "safe_divide should have a doc comment"
    );
}
