use super::*;

// ════════════════════════════════════════════════════════════════
// 11. Extern "C" tests
// ════════════════════════════════════════════════════════════════

#[test]
fn extern_c_block_extracted() {
    let items = fixture_items();
    let ext = items.iter().find(|i| {
        i.name.contains("extern")
            && i.metadata
                .attributes
                .contains(&"linkage_specification".to_string())
    });
    assert!(ext.is_some(), "extern \"C\" block should be extracted");
}

#[test]
fn extern_c_block_has_extern_c_attr() {
    let items = fixture_items();
    let ext = items
        .iter()
        .find(|i| {
            i.metadata
                .attributes
                .contains(&"linkage_specification".to_string())
        })
        .expect("extern C block should exist");
    assert!(
        ext.metadata.attributes.contains(&"extern_c".to_string()),
        "extern C should have extern_c attribute"
    );
}

#[test]
fn extern_c_c_init_prototype() {
    let items = fixture_items();
    let ci = items
        .iter()
        .find(|i| i.name == "c_init" && i.kind == SymbolKind::Function);
    assert!(ci.is_some(), "c_init prototype should be extracted");
}

#[test]
fn extern_c_c_process_prototype() {
    let items = fixture_items();
    let cp = items
        .iter()
        .find(|i| i.name == "c_process" && i.kind == SymbolKind::Function);
    assert!(cp.is_some(), "c_process prototype should be extracted");
}
