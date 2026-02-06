# TreeCursor

`TreeCursor<'tree>` provides efficient, stateful traversal of a syntax tree. It is more performant than repeated `node.child()` calls for large traversals because it maintains internal state that avoids re-traversing from the root on each step.

---

## API Reference

```rust
impl<'tree> TreeCursor<'tree> {
    pub fn node(&self) -> Node<'tree>
    pub fn field_name(&self) -> Option<&'static str>
    pub fn field_id(&self) -> Option<FieldId>
    pub fn depth(&self) -> u32
    pub fn descendant_index(&self) -> usize
    pub fn goto_first_child(&mut self) -> bool
    pub fn goto_last_child(&mut self) -> bool
    pub fn goto_parent(&mut self) -> bool
    pub fn goto_next_sibling(&mut self) -> bool
    pub fn goto_previous_sibling(&mut self) -> bool
    pub fn goto_descendant(&mut self, descendant_index: usize)
    pub fn goto_first_child_for_byte(&mut self, index: usize) -> Option<usize>
    pub fn goto_first_child_for_point(&mut self, point: Point) -> Option<usize>
    pub fn reset(&mut self, node: Node<'tree>)
    pub fn reset_to(&mut self, cursor: &Self)
}
```

### `node()`

Returns the `Node` the cursor is currently pointing at.

### `field_name()` / `field_id()`

Returns the field name (or numeric field ID) of the current node relative to its parent, if the grammar assigns one. For example, a cursor on the `name` child of a `function_item` returns `Some("name")`.

### `depth()`

Returns the depth of the current node relative to the cursor's starting node (which has depth 0).

### `descendant_index()`

Returns the index of the current node in a pre-order traversal of the tree.

### Navigation Methods

| Method | Behavior | Returns |
|--------|----------|---------|
| `goto_first_child()` | Move to the first child | `true` if the node has children |
| `goto_last_child()` | Move to the last child | `true` if the node has children |
| `goto_parent()` | Move to the parent | `false` at the cursor's root boundary |
| `goto_next_sibling()` | Move to the next sibling | `false` if no next sibling |
| `goto_previous_sibling()` | Move to the previous sibling | `false` if no previous sibling |
| `goto_descendant(index)` | Jump to the descendant at the given pre-order index | (void) |
| `goto_first_child_for_byte(index)` | Move to the first child that extends beyond the byte | `Some(child_index)` or `None` |
| `goto_first_child_for_point(point)` | Move to the first child that extends beyond the point | `Some(child_index)` or `None` |

### `reset(node)`

Resets the cursor to start at the given node. The node becomes the new root boundary.

### `reset_to(cursor)`

Copies the state of another cursor into this one, avoiding reallocation.

---

## Key Concepts

### Root Boundary

A `TreeCursor` is created from a specific node (via `Node::walk()` or `Tree::walk()`). That node is the cursor's **root** — `goto_parent()` and `goto_next_sibling()` will return `false` when the cursor is at this root, preventing traversal outside the subtree.

### Performance

For iterating over all children of a node, a cursor-based loop is more efficient than calling `node.child(0)`, `node.child(1)`, etc., because the latter re-traverses from the parent on each call. With a cursor, each `goto_next_sibling()` is O(1) amortized.

### Thread Safety

| Property | Value |
|----------|-------|
| `Send`   | Yes   |
| `Sync`   | No    |

A `TreeCursor` is stateful and must not be shared across threads. Create a separate cursor per thread.

---

## Examples

### Iterating Over Children with a Cursor

```rust
use tree_sitter::Parser;

let mut parser = Parser::new();
parser
    .set_language(&tree_sitter_rust::LANGUAGE.into())
    .unwrap();

let source = b"fn main() { let a = 1; let b = 2; }";
let tree = parser.parse(source, None).unwrap();
let root = tree.root_node();

let func = root.child(0).unwrap();
let body = func.child_by_field_name("body").unwrap();

let mut cursor = body.walk();
if cursor.goto_first_child() {
    loop {
        let node = cursor.node();
        if node.is_named() {
            println!(
                "{}: {} (field: {:?})",
                node.kind(),
                node.utf8_text(source).unwrap(),
                cursor.field_name(),
            );
        }
        if !cursor.goto_next_sibling() {
            break;
        }
    }
}
// Output:
//   let_declaration: let a = 1; (field: None)
//   let_declaration: let b = 2; (field: None)
```

### Recursive Tree Traversal

A depth-first traversal using a cursor, printing every named node with its depth and byte range.

```rust
use tree_sitter::Node;

fn visit_tree(node: Node, source: &[u8], depth: usize) {
    let indent = " ".repeat(depth * 2);
    if node.is_named() {
        println!(
            "{}{} [{}-{}]",
            indent,
            node.kind(),
            node.start_byte(),
            node.end_byte(),
        );
    }

    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            visit_tree(cursor.node(), source, depth + 1);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
}

// Usage:
use tree_sitter::Parser;

let mut parser = Parser::new();
parser
    .set_language(&tree_sitter_rust::LANGUAGE.into())
    .unwrap();

let source = b"fn add(a: i32, b: i32) -> i32 { a + b }";
let tree = parser.parse(source, None).unwrap();
visit_tree(tree.root_node(), source, 0);
```

### Iterative Depth-First Traversal (No Recursion)

For very deep trees, an iterative approach avoids stack overflow.

```rust
use tree_sitter::{Parser, TreeCursor};

let mut parser = Parser::new();
parser
    .set_language(&tree_sitter_rust::LANGUAGE.into())
    .unwrap();

let source = b"fn main() { if true { return 1; } }";
let tree = parser.parse(source, None).unwrap();

let mut cursor = tree.walk();
let mut depth = 0u32;
let mut reached_root = false;

while !reached_root {
    let node = cursor.node();
    if node.is_named() {
        let indent = " ".repeat(depth as usize * 2);
        println!("{}{}", indent, node.kind());
    }

    if cursor.goto_first_child() {
        depth += 1;
        continue;
    }

    if cursor.goto_next_sibling() {
        continue;
    }

    loop {
        if !cursor.goto_parent() {
            reached_root = true;
            break;
        }
        depth -= 1;
        if cursor.goto_next_sibling() {
            break;
        }
    }
}
```

### Jumping to a Byte Position

`goto_first_child_for_byte` moves the cursor to the first child that contains or follows a given byte offset. This is useful for quickly navigating to a position in the document without scanning all children.

```rust
use tree_sitter::Parser;

let mut parser = Parser::new();
parser
    .set_language(&tree_sitter_rust::LANGUAGE.into())
    .unwrap();

let source = b"fn a() {} fn b() {} fn c() {}";
let tree = parser.parse(source, None).unwrap();

let mut cursor = tree.walk();

// Jump to the child that covers byte 15 (somewhere inside "fn b()").
if let Some(child_index) = cursor.goto_first_child_for_byte(15) {
    let node = cursor.node();
    println!(
        "Child index {}: {} = '{}'",
        child_index,
        node.kind(),
        node.utf8_text(source).unwrap(),
    );
    // -> Child index 1: function_item = 'fn b() {}'
}
```

### Using `field_name()` During Traversal

```rust
use tree_sitter::Parser;

let mut parser = Parser::new();
parser
    .set_language(&tree_sitter_rust::LANGUAGE.into())
    .unwrap();

let source = b"fn example(x: u32) -> bool { true }";
let tree = parser.parse(source, None).unwrap();
let func = tree.root_node().child(0).unwrap();

let mut cursor = func.walk();
if cursor.goto_first_child() {
    loop {
        let node = cursor.node();
        match cursor.field_name() {
            Some(field) => println!(
                "field '{}': {} = '{}'",
                field,
                node.kind(),
                node.utf8_text(source).unwrap(),
            ),
            None => println!(
                "(anonymous): {} = '{}'",
                node.kind(),
                node.utf8_text(source).unwrap(),
            ),
        }
        if !cursor.goto_next_sibling() {
            break;
        }
    }
}
// Output:
//   (anonymous): fn = 'fn'
//   field 'name': identifier = 'example'
//   field 'parameters': parameters = '(x: u32)'
//   (anonymous): -> = '->'
//   field 'return_type': primitive_type = 'bool'
//   field 'body': block = '{ true }'
```

---

## See Also

- [Trees and Nodes](02-tree-and-nodes.md) — `Node` methods for non-cursor traversal
- [Queries](04-queries.md) — pattern matching as an alternative to manual traversal
- [Parser](01-parser.md) — producing trees
- [Supporting Types](05-types.md) — `Point`, `Range`, `FieldId`
