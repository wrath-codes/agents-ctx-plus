use ast_grep_core::Node;

/// Collect leading `#` comments above a node as doc comments.
///
/// Walks backward through siblings from `idx`, collecting contiguous
/// `comment` nodes (but not shebangs). Stops at any non-comment node
/// or a blank-line gap (detected by non-consecutive lines).
pub(super) fn collect_doc_comments<D: ast_grep_core::Doc>(
    siblings: &[Node<D>],
    idx: usize,
    _source: &str,
) -> String {
    let mut comments = Vec::new();
    let target_line = siblings[idx].start_pos().line();

    let mut i = idx;
    while i > 0 {
        i -= 1;
        let sibling = &siblings[i];
        if sibling.kind().as_ref() != "comment" {
            break;
        }
        let text = sibling.text().to_string();
        // Skip shebangs
        if text.starts_with("#!") {
            break;
        }
        // Check for line gap — comments must be contiguous
        let comment_end = sibling.end_pos().line();
        let next_start = if i + 1 < idx {
            siblings[i + 1].start_pos().line()
        } else {
            target_line
        };
        if next_start > comment_end + 1 {
            break;
        }
        let stripped = text.trim_start_matches('#').trim().to_string();
        comments.push(stripped);
    }

    comments.reverse();
    comments.join("\n")
}
