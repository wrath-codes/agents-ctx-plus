use super::*;

// ════════════════════════════════════════════════════════════════
// 27. Friend declaration tests
// ════════════════════════════════════════════════════════════════

#[test]
fn class_secret_holder_has_friends() {
    let items = fixture_items();
    let sh = find_by_name(&items, "SecretHolder");
    let has_friend = sh
        .metadata
        .attributes
        .iter()
        .any(|a| a.starts_with("friend:"));
    assert!(
        has_friend,
        "SecretHolder should have friend attributes, got {:?}",
        sh.metadata.attributes
    );
}

#[test]
fn class_friend_demo_has_friends() {
    let items = fixture_items();
    let fd = find_by_name(&items, "FriendDemo");
    let has_friend = fd
        .metadata
        .attributes
        .iter()
        .any(|a| a.starts_with("friend:"));
    assert!(
        has_friend,
        "FriendDemo should have friend attributes, got {:?}",
        fd.metadata.attributes
    );
}

#[test]
fn minimal_friend_class() {
    let items = parse_and_extract("class A { friend class B; };");
    let a = find_by_name(&items, "A");
    assert!(
        a.metadata.attributes.iter().any(|a| a.contains("friend")),
        "A should have friend attribute"
    );
}
