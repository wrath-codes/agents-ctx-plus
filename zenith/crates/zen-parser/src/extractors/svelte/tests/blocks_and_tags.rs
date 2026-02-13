use super::*;

#[test]
fn extracts_svelte_control_blocks() {
    let items = fixture_items();

    assert!(items
        .iter()
        .any(|i| has_attr(i, "svelte:kind:if_statement")));
    assert!(items
        .iter()
        .any(|i| has_attr(i, "svelte:kind:each_statement")));
    assert!(items
        .iter()
        .any(|i| has_attr(i, "svelte:kind:await_statement")));
    assert!(items
        .iter()
        .any(|i| has_attr(i, "svelte:kind:key_statement")));
    assert!(items
        .iter()
        .any(|i| has_attr(i, "svelte:kind:snippet_statement")));
}

#[test]
fn extracts_special_tags() {
    let items = fixture_items();

    assert!(items.iter().any(|i| has_attr(i, "svelte:kind:render_tag")));
    assert!(items.iter().any(|i| has_attr(i, "svelte:kind:html_tag")));
    assert!(items.iter().any(|i| has_attr(i, "svelte:kind:const_tag")));
    assert!(items.iter().any(|i| has_attr(i, "svelte:kind:debug_tag")));
    assert!(items
        .iter()
        .any(|i| has_attr(i, "svelte:kind:expression_tag")));
}

#[test]
fn extracts_script_api_and_events_and_directives() {
    let items = fixture_items();

    assert!(items.iter().any(|i| i.name == "script_api:count"));
    assert!(items.iter().any(|i| i.name == "script_api:mode"));
    assert!(items.iter().any(|i| i.name == "script_api:inc"));
    assert!(items
        .iter()
        .any(|i| has_attr(i, "svelte:kind:event_dispatcher")));
    let submit_event = find_by_name(&items, "event:submit");
    assert!(has_attr(submit_event, "svelte:event:submit"));

    assert!(items
        .iter()
        .any(|i| has_attr(i, "svelte:kind:directive_attribute")
            && has_attr(i, "svelte:directive_type:on")));
    assert!(items
        .iter()
        .any(|i| has_attr(i, "svelte:kind:directive_attribute")
            && has_attr(i, "svelte:directive_type:bind")));
    assert!(items
        .iter()
        .any(|i| has_attr(i, "svelte:kind:directive_attribute")
            && has_attr(i, "svelte:directive_type:use")));
}

#[test]
fn links_snippet_render_references_and_marks_broken() {
    let items = fixture_items();
    let resolved = items
        .iter()
        .find(|i| {
            has_attr(i, "svelte:kind:render_tag")
                && i.metadata
                    .attributes
                    .iter()
                    .any(|a| a.starts_with("svelte:ref_target:snippet-"))
        })
        .expect("resolved snippet render ref should exist");
    assert!(resolved
        .metadata
        .attributes
        .iter()
        .any(|a| a.starts_with("svelte:render_call:row(")));

    let broken = items
        .iter()
        .find(|i| {
            has_attr(i, "svelte:kind:render_tag")
                && has_attr(i, "svelte:broken_snippet_ref:missingRow")
        })
        .expect("broken snippet render should be flagged");
    assert!(has_attr(broken, "svelte:broken_snippet_ref:missingRow"));
}
