# Incremental Parsing

In editors, re-parsing the entire file on every keystroke is wasteful. Tree-sitter solves this by reusing unchanged parts of the previous syntax tree, making re-parses proportional to the size of the change rather than the size of the file.

---

## How It Works

Incremental parsing is a two-step process:

1. **Edit the old tree** — call `tree.edit(&input_edit)` to update the tree's byte/position metadata so it knows what changed.
2. **Re-parse with the old tree** — call `parser.parse(new_source, Some(&old_tree))` to produce a new tree. Tree-sitter reuses nodes from the old tree that fall outside the edited region.

---

## InputEdit

The `InputEdit` struct describes a single edit to the source text:

```rust
use tree_sitter::{InputEdit, Point};

InputEdit {
    start_byte: usize,         // where the edit begins
    old_end_byte: usize,       // where the old text ended (before edit)
    new_end_byte: usize,       // where the new text ends (after edit)
    start_position: Point,     // row/column of start
    old_end_position: Point,   // row/column of old end
    new_end_position: Point,   // row/column of new end
}
```

- For an **insertion**, `start_byte == old_end_byte` (zero-length old range).
- For a **deletion**, `start_byte == new_end_byte` (zero-length new range).
- For a **replacement**, all three differ.

---

## Complete Example

```rust
use tree_sitter::{Parser, InputEdit, Point};

let mut parser = Parser::new();
parser.set_language(&tree_sitter_rust::LANGUAGE.into()).unwrap();

// Initial parse
let old_source = b"fn main() {}";
let mut old_tree = parser.parse(old_source, None).unwrap();

// User inserts " return;" inside the braces
let new_source = b"fn main() { return; }";
let edit = InputEdit {
    start_byte: 11,
    old_end_byte: 11,
    new_end_byte: 20,
    start_position: Point { row: 0, column: 11 },
    old_end_position: Point { row: 0, column: 11 },
    new_end_position: Point { row: 0, column: 20 },
};

old_tree.edit(&edit);
let new_tree = parser.parse(new_source, Some(&old_tree)).unwrap();

// Find changed ranges
for range in old_tree.changed_ranges(&new_tree) {
    println!("Changed: bytes {}..{}", range.start_byte, range.end_byte);
}
```

---

## Changed Ranges

After an incremental re-parse, `old_tree.changed_ranges(&new_tree)` returns the ranges of the source where the syntax tree structure actually differs. This is useful for:

- Limiting syntax highlighting updates to changed regions
- Minimizing re-analysis in language servers

---

## Editing Retained Node References

If you hold `Node` references from before the edit, their byte offsets and positions become stale. Call `node.edit(&input_edit)` on any retained node to update its positional metadata to reflect the edit. This does **not** re-parse — it only adjusts the stored offsets.

---

## Concurrency

Trees are cheap to clone — cloning performs an atomic reference count increment on the underlying data. You can clone a tree and send it to another thread for read-only access.

However, individual `Tree` instances are **not** thread-safe. Do not share a single `Tree` across threads without synchronization. Clone it first:

```rust
let tree_clone = tree.clone(); // cheap atomic refcount bump
std::thread::spawn(move || {
    let root = tree_clone.root_node();
    // safe to read on this thread
});
```
