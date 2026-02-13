use super::common::extract_md;

#[test]
fn extracts_inline_link_and_code_from_paragraph() {
    let src = "# Intro\n\nParagraph with [Zen](https://zen.dev) and `let x = 1`.\n";
    let items = extract_md(src);

    let link = items
        .iter()
        .find(|i| {
            i.metadata
                .attributes
                .iter()
                .any(|a| a == "md:kind:inline_link")
        })
        .expect("should extract inline link");
    assert_eq!(link.name, "Zen");
    assert_eq!(link.start_line, 3);
    assert!(link
        .metadata
        .attributes
        .iter()
        .any(|a| a == "md:url:https://zen.dev"));

    let code = items
        .iter()
        .find(|i| {
            i.metadata
                .attributes
                .iter()
                .any(|a| a == "md:kind:inline_code")
        })
        .expect("should extract inline code");
    assert_eq!(code.signature, "let x = 1");
    assert_eq!(code.start_line, 3);
}

#[test]
fn extracts_inline_image_and_autolink_without_image_link_duplication() {
    let src = "# Intro\n\nImage ![Logo](https://cdn.example/logo.png) and <https://example.dev>.\n";
    let items = extract_md(src);

    let image = items
        .iter()
        .find(|i| {
            i.metadata
                .attributes
                .iter()
                .any(|a| a == "md:kind:inline_image")
        })
        .expect("should extract inline image");
    assert_eq!(image.name, "Logo");
    assert!(image
        .metadata
        .attributes
        .iter()
        .any(|a| a == "md:src:https://cdn.example/logo.png"));

    let autolink = items
        .iter()
        .find(|i| {
            i.metadata
                .attributes
                .iter()
                .any(|a| a == "md:kind:autolink")
        })
        .expect("should extract autolink");
    assert_eq!(autolink.signature, "<https://example.dev>");

    let duplicate_link_from_image = items.iter().find(|i| {
        i.metadata
            .attributes
            .iter()
            .any(|a| a == "md:kind:inline_link")
            && i.signature.contains("logo.png")
    });
    assert!(
        duplicate_link_from_image.is_none(),
        "image should not also be extracted as inline link"
    );
}

#[test]
fn extracts_reference_links_and_bare_urls_without_link_target_duplication() {
    let src = "# Intro\n\nUse [guide][docs] and [docs][]. Also visit https://example.org/docs and [Site](https://example.org/docs).\n";
    let items = extract_md(src);

    let ref_link = items
        .iter()
        .find(|i| {
            i.metadata
                .attributes
                .iter()
                .any(|a| a == "md:kind:inline_ref_link")
        })
        .expect("should extract reference-style link");
    assert!(ref_link
        .metadata
        .attributes
        .iter()
        .any(|a| a == "md:ref:docs"));

    let bare = items
        .iter()
        .find(|i| {
            i.metadata
                .attributes
                .iter()
                .any(|a| a == "md:kind:bare_url")
        })
        .expect("should extract bare url");
    assert_eq!(bare.signature, "https://example.org/docs");

    let bare_dupes = items
        .iter()
        .filter(|i| {
            i.metadata
                .attributes
                .iter()
                .any(|a| a == "md:kind:bare_url")
                && i.signature == "https://example.org/docs"
        })
        .count();
    assert_eq!(bare_dupes, 1);
}

#[test]
fn assigns_heading_hierarchy_and_owner_paths() {
    let src = "# A\n\n## B\n\n- item\n";
    let items = extract_md(src);

    let heading_a = items
        .iter()
        .find(|i| i.name == "A" && i.metadata.attributes.iter().any(|a| a == "md:kind:heading"))
        .expect("heading A should exist");
    assert!(heading_a
        .metadata
        .attributes
        .iter()
        .any(|a| a == "md:path:A"));

    let heading_b = items
        .iter()
        .find(|i| i.name == "B" && i.metadata.attributes.iter().any(|a| a == "md:kind:heading"))
        .expect("heading B should exist");
    assert_eq!(heading_b.metadata.owner_name.as_deref(), Some("A"));
    assert!(heading_b
        .metadata
        .attributes
        .iter()
        .any(|a| a == "md:path:A/B"));

    let list = items
        .iter()
        .find(|i| i.name.starts_with("list-"))
        .expect("list item should exist");
    assert_eq!(list.metadata.owner_name.as_deref(), Some("A/B"));
    assert!(list
        .metadata
        .attributes
        .iter()
        .any(|a| a == "md:owner_path:A/B"));
}
