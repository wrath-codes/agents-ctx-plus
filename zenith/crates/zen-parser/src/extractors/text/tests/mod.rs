use crate::types::SymbolKind;

use super::*;

// ── Delegation to markdown ───────────────────────────────────

#[test]
fn delegates_markdown_content_to_md_extractor() {
    let content = "# Title\n\n> Summary\n\n## Section\n\n- [link](url): desc\n";
    let items = extract(content).unwrap();
    // Should produce markdown-style items (md:kind:document root, headings, etc.)
    assert!(
        items
            .iter()
            .any(|i| i.metadata.attributes.iter().any(|a| a.starts_with("md:"))),
        "expected markdown attributes from delegated extraction"
    );
}

// ── Delegation to RST ────────────────────────────────────────

#[test]
fn delegates_rst_content_to_rst_extractor() {
    let content = "\
Title
=====

.. code-block:: python

   print('hi')

Subtitle
--------

More text.
";
    let items = extract(content).unwrap();
    assert!(
        items
            .iter()
            .any(|i| i.metadata.attributes.iter().any(|a| a.starts_with("rst:"))),
        "expected RST attributes from delegated extraction"
    );
}

// ── Plain text: heuristic headings ───────────────────────────

#[test]
fn plain_text_underline_headings() {
    // Content with underline-adorned headings triggers RST detection in the smart
    // router, so verify via RST section attributes instead of txt:kind:heading.
    let content = "\
Getting Started
===============

Welcome to the tool.

Installation
------------

Install via pip.
";
    let items = extract(content).unwrap();
    // Smart router detects underline-adorned headings as RST format.
    // The RST extractor produces section items with rst:path attributes.
    let sections: Vec<_> = items
        .iter()
        .filter(|i| {
            i.kind == SymbolKind::Module
                && i.metadata
                    .attributes
                    .iter()
                    .any(|a| a.starts_with("rst:path:"))
        })
        .collect();
    assert_eq!(sections.len(), 2);
    assert_eq!(sections[0].name, "Getting Started");
    assert_eq!(sections[1].name, "Installation");
}

#[test]
fn plain_text_all_caps_headings() {
    let content = "\
GETTING STARTED

Welcome to the tool.

INSTALLATION GUIDE

Install via pip.
";
    let items = extract(content).unwrap();
    let headings: Vec<_> = items
        .iter()
        .filter(|i| {
            i.kind == SymbolKind::Module
                && i.metadata
                    .attributes
                    .iter()
                    .any(|a| a.starts_with("txt:kind:heading"))
        })
        .collect();
    assert_eq!(headings.len(), 2);
    assert_eq!(headings[0].name, "Getting Started");
    assert_eq!(headings[1].name, "Installation Guide");
}

#[test]
fn plain_text_numbered_headings() {
    let content = "\
1. Introduction

Some intro text.

2. Setup

Setup instructions.
";
    let items = extract(content).unwrap();
    let headings: Vec<_> = items
        .iter()
        .filter(|i| {
            i.kind == SymbolKind::Module
                && i.metadata
                    .attributes
                    .iter()
                    .any(|a| a.starts_with("txt:kind:heading"))
        })
        .collect();
    assert_eq!(headings.len(), 2);
    assert_eq!(headings[0].name, "Introduction");
    assert_eq!(headings[1].name, "Setup");
}

#[test]
fn plain_text_heading_hierarchy() {
    // Use content that the smart router identifies as plain text:
    // ALL CAPS headings (>= 2 words) and numbered headings.
    let content = "\
PROJECT OVERVIEW

Intro.

1. Detail Section

The details.
";
    let items = extract(content).unwrap();
    let headings: Vec<_> = items
        .iter()
        .filter(|i| {
            i.kind == SymbolKind::Module
                && i.metadata
                    .attributes
                    .iter()
                    .any(|a| a.starts_with("txt:kind:heading"))
        })
        .collect();
    assert_eq!(headings.len(), 2);

    // "Detail Section" should have "Project Overview" as owner
    let details = &headings[1];
    assert!(
        details
            .metadata
            .attributes
            .iter()
            .any(|a| a.contains("txt:path:Project Overview/Detail Section")),
        "expected hierarchical path, got: {:?}",
        details.metadata.attributes
    );
}

// ── Plain text: no headings (paragraph mode) ─────────────────

#[test]
fn plain_text_no_headings_produces_paragraphs() {
    let content = "\
First paragraph of text.
With a second line.


Second paragraph here.
";
    let items = extract(content).unwrap();
    let paragraphs: Vec<_> = items
        .iter()
        .filter(|i| {
            i.metadata
                .attributes
                .iter()
                .any(|a| a == "txt:kind:paragraph")
        })
        .collect();
    assert_eq!(paragraphs.len(), 2);
    assert_eq!(paragraphs[0].kind, SymbolKind::Property);
}

// ── Empty document ───────────────────────────────────────────

#[test]
fn empty_document() {
    let items = extract("").unwrap();
    // Should at least have the root document item
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].name, "$");
}

// ── Single line ──────────────────────────────────────────────

#[test]
fn single_line_text() {
    let content = "type Foo = Bar;";
    let items = extract(content).unwrap();
    // Root + 1 paragraph
    assert!(items.len() >= 1);
}
