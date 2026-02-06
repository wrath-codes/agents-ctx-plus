# Trees and Nodes

A `Tree` is the output of a successful parse — a full concrete syntax tree over the source text. A `Node<'tree>` is an immutable, lightweight handle into a tree, borrowing from it for its lifetime. Nodes provide rich traversal, position, and type information.

---

## Tree

```rust
impl Tree {
    pub fn root_node(&self) -> Node
    pub fn root_node_with_offset(&self, offset_bytes: usize, offset_extent: Point) -> Node
    pub fn language(&self) -> LanguageRef
    pub fn edit(&mut self, edit: &InputEdit)
    pub fn walk(&self) -> TreeCursor
    pub fn changed_ranges(&self, other: &Self) -> impl ExactSizeIterator<Item = Range>
    pub fn included_ranges(&self) -> Vec<Range>
}
```

### `Tree::root_node()`

Returns the root node of the syntax tree. This is always a node of the grammar's top-level rule (e.g., `source_file` for most languages).

### `Tree::root_node_with_offset(offset_bytes, offset_extent)`

Returns the root node with all positions shifted by the given byte and point offsets. Useful when parsing embedded language regions that start at a non-zero position in the host document.

### `Tree::edit(&mut self, edit: &InputEdit)`

Inform the tree about an edit to the source text. This does **not** re-parse — it adjusts internal byte ranges so the tree remains consistent for a subsequent incremental re-parse via `parser.parse(new_source, Some(&edited_tree))`.

### `Tree::walk()`

Creates a `TreeCursor` starting at the root node.

### `Tree::changed_ranges(&self, other: &Self)`

Computes the byte ranges that differ between two versions of a tree. Both trees must have been parsed from the same language. Useful for efficiently determining which regions of the source changed.

### `Tree::included_ranges()`

Returns the ranges the parser was restricted to (set via `Parser::set_included_ranges`), or a single range covering the whole document if none were set.

### Clone and Thread Safety

`Tree` implements `Clone`. Cloning is **cheap** — it increments an atomic reference count on the underlying data. This makes it practical to clone a tree and send the clone to another thread.

| Property | Value |
|----------|-------|
| `Send`   | Yes   |
| `Sync`   | No    |

Individual `Tree` instances should not be shared across threads. Clone the tree and give each thread its own copy.

---

## Node

`Node<'tree>` is a lightweight, `Copy`-able handle into a `Tree`. Its lifetime `'tree` is tied to the tree it came from — the tree must outlive all of its nodes.

```rust
impl<'tree> Node<'tree> {
    // Type info
    pub fn kind(&self) -> &'static str
    pub fn kind_id(&self) -> u16
    pub fn is_named(&self) -> bool
    pub fn is_extra(&self) -> bool
    pub fn is_error(&self) -> bool
    pub fn is_missing(&self) -> bool
    pub fn has_error(&self) -> bool

    // Position
    pub fn start_byte(&self) -> usize
    pub fn end_byte(&self) -> usize
    pub fn byte_range(&self) -> std::ops::Range<usize>
    pub fn start_position(&self) -> Point
    pub fn end_position(&self) -> Point
    pub fn range(&self) -> Range

    // Children
    pub fn child(&self, i: u32) -> Option<Self>
    pub fn child_count(&self) -> usize
    pub fn named_child(&self, i: u32) -> Option<Self>
    pub fn named_child_count(&self) -> usize
    pub fn children<'cursor>(
        &self, cursor: &'cursor mut TreeCursor<'tree>,
    ) -> impl ExactSizeIterator<Item = Node<'tree>>
    pub fn named_children<'cursor>(
        &self, cursor: &'cursor mut TreeCursor<'tree>,
    ) -> impl ExactSizeIterator<Item = Node<'tree>>

    // Field-based access
    pub fn child_by_field_name(&self, field_name: impl AsRef<[u8]>) -> Option<Self>
    pub fn child_by_field_id(&self, field_id: u16) -> Option<Self>
    pub fn children_by_field_name<'cursor>(
        &self, field_name: &str, cursor: &'cursor mut TreeCursor<'tree>,
    ) -> impl Iterator<Item = Node<'tree>>

    // Navigation
    pub fn parent(&self) -> Option<Self>
    pub fn next_sibling(&self) -> Option<Self>
    pub fn prev_sibling(&self) -> Option<Self>
    pub fn next_named_sibling(&self) -> Option<Self>
    pub fn prev_named_sibling(&self) -> Option<Self>

    // Descendant search
    pub fn descendant_for_byte_range(&self, start: usize, end: usize) -> Option<Self>
    pub fn named_descendant_for_byte_range(&self, start: usize, end: usize) -> Option<Self>

    // Text
    pub fn utf8_text<'a>(&self, source: &'a [u8]) -> Result<&'a str, Utf8Error>
    pub fn to_sexp(&self) -> String

    // Walking
    pub fn walk(&self) -> TreeCursor<'tree>

    // Editing
    pub fn edit(&mut self, edit: &InputEdit)
}
```

### Thread Safety

| Property | Value |
|----------|-------|
| `Send`   | Yes   |
| `Sync`   | Yes   |

Nodes are immutable views into tree data and are safe to share across threads (as long as the tree they borrow from outlives them).

---

## Named vs Anonymous Nodes

Tree-sitter produces a **concrete** syntax tree that includes every token in the source. Nodes fall into two categories:

- **Named nodes** — correspond to named rules in the grammar (e.g., `function_item`, `identifier`, `block`). These form the abstract structure of the code.
- **Anonymous nodes** — correspond to literal string tokens in the grammar (e.g., `fn`, `(`, `)`, `{`, `}`). These are punctuation, keywords, and operators.

Use `node.is_named()` to distinguish between them.

For AST-like traversal that skips punctuation and keywords:
- `named_child(i)` — get the i-th named child
- `named_child_count()` — count of named children only
- `named_children(cursor)` — iterate named children only
- `next_named_sibling()` / `prev_named_sibling()` — skip anonymous siblings

```rust
use tree_sitter::Parser;

let mut parser = Parser::new();
parser
    .set_language(&tree_sitter_rust::LANGUAGE.into())
    .unwrap();

let source = b"fn add(a: i32) -> i32 { a }";
let tree = parser.parse(source, None).unwrap();
let root = tree.root_node();
let func = root.child(0).unwrap(); // function_item

// All children include anonymous nodes like "fn", "(", ")", "->", "{", "}"
println!("Total children: {}", func.child_count());

// Named children are the structural parts: name, parameters, return_type, body
println!("Named children: {}", func.named_child_count());

let mut cursor = func.walk();
for child in func.children(&mut cursor) {
    println!(
        "{:>12} | named={} | kind={}",
        child.utf8_text(source).unwrap(),
        child.is_named(),
        child.kind(),
    );
}
// Output includes both anonymous ("fn", "(", etc.) and named ("identifier", "parameters", etc.)
```

---

## Examples

### Getting the Root Node and Traversing Children

```rust
use tree_sitter::Parser;

let mut parser = Parser::new();
parser
    .set_language(&tree_sitter_rust::LANGUAGE.into())
    .unwrap();

let source = b"use std::io;\nfn main() {}";
let tree = parser.parse(source, None).unwrap();
let root = tree.root_node();

println!("Root: {} [{} children]", root.kind(), root.child_count());

let mut cursor = root.walk();
for (i, child) in root.named_children(&mut cursor).enumerate() {
    println!(
        "  child {}: {} ({}-{})",
        i,
        child.kind(),
        child.start_byte(),
        child.end_byte(),
    );
}
// Output:
//   Root: source_file [2 children]
//   child 0: use_declaration (0-12)
//   child 1: function_item (13-25)
```

### Field-Based Access

Grammar rules often assign **field names** to their children. Fields provide stable, semantic access to specific parts of a node regardless of optional children or ordering changes.

```rust
use tree_sitter::Parser;

let mut parser = Parser::new();
parser
    .set_language(&tree_sitter_rust::LANGUAGE.into())
    .unwrap();

let source = b"fn greet(name: &str) -> String { format!(\"Hello, {name}\") }";
let tree = parser.parse(source, None).unwrap();
let func = tree.root_node().child(0).unwrap();

// Access children by field name
let name_node = func.child_by_field_name("name").unwrap();
println!("Function name: {}", name_node.utf8_text(source).unwrap());
// -> "greet"

let params_node = func.child_by_field_name("parameters").unwrap();
println!("Parameters: {}", params_node.utf8_text(source).unwrap());
// -> "(name: &str)"

let return_type = func.child_by_field_name("return_type").unwrap();
println!("Return type: {}", return_type.utf8_text(source).unwrap());
// -> "String"

let body_node = func.child_by_field_name("body").unwrap();
println!("Body: {}", body_node.utf8_text(source).unwrap());
// -> "{ format!(\"Hello, {name}\") }"
```

### Getting Text Content with `utf8_text`

`utf8_text` returns a `&str` slice from the original source bytes, using the node's byte range. The returned reference borrows from the source, not from the tree.

```rust
use tree_sitter::Parser;

let mut parser = Parser::new();
parser
    .set_language(&tree_sitter_rust::LANGUAGE.into())
    .unwrap();

let source = b"let x = 42;";
let tree = parser.parse(source, None).unwrap();
let root = tree.root_node();

let let_decl = root.child(0).unwrap();
let value = let_decl.child_by_field_name("value").unwrap();

let text = value.utf8_text(source).unwrap();
assert_eq!(text, "42");

// Equivalent manual extraction:
let manual = std::str::from_utf8(&source[value.start_byte()..value.end_byte()]).unwrap();
assert_eq!(text, manual);
```

### Printing the S-Expression

`to_sexp()` returns a Lisp-like s-expression representation of the subtree, useful for debugging and understanding tree structure.

```rust
use tree_sitter::Parser;

let mut parser = Parser::new();
parser
    .set_language(&tree_sitter_rust::LANGUAGE.into())
    .unwrap();

let source = b"let x = 1 + 2;";
let tree = parser.parse(source, None).unwrap();

println!("{}", tree.root_node().to_sexp());
// (source_file
//   (let_declaration
//     pattern: (identifier)
//     value: (binary_expression
//       left: (integer_literal)
//       right: (integer_literal))))
```

### Finding a Node at a Byte Position

```rust
use tree_sitter::Parser;

let mut parser = Parser::new();
parser
    .set_language(&tree_sitter_rust::LANGUAGE.into())
    .unwrap();

let source = b"fn main() { let x = 42; }";
let tree = parser.parse(source, None).unwrap();

// Find the smallest named node covering byte offset 20 (the "42").
let node = tree
    .root_node()
    .named_descendant_for_byte_range(20, 22)
    .unwrap();

assert_eq!(node.kind(), "integer_literal");
assert_eq!(node.utf8_text(source).unwrap(), "42");
```

---

## See Also

- [Parser](01-parser.md) — creating trees from source text
- [TreeCursor](03-tree-cursor.md) — efficient cursor-based traversal
- [Queries](04-queries.md) — pattern matching on nodes
- [Supporting Types](05-types.md) — `Point`, `Range`, `InputEdit`
