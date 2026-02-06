# Syntax Nodes

Nodes are the building blocks of a tree-sitter syntax tree. Every node carries positional information, type metadata, and relationships to other nodes.

---

## Named vs Anonymous Nodes

Tree-sitter distinguishes between two categories of nodes:

- **Named nodes** come from rules with explicit names in the grammar (e.g. `function_definition`, `identifier`, `block`). These represent meaningful syntactic constructs.
- **Anonymous nodes** represent literal string tokens defined inline in the grammar (e.g. `"if"`, `"("`, `";"`, `"->"`, `"fn"`). These are punctuation, keywords, and operators.

Use `is_named()` to distinguish them:

```rust
// Given: if (x) { y; }
// if_statement has children:
//   "if"     — anonymous (is_named() == false)
//   "("      — anonymous
//   identifier "x"  — named (is_named() == true)
//   ")"      — anonymous
//   block    — named
```

When traversing children, you can filter to only named children using `named_child()` and `named_child_count()` to skip over punctuation and keywords.

---

## Node Positions

Every node tracks its location in the source as both byte offsets and row/column points:

| Method             | Returns  | Description                          |
|--------------------|----------|--------------------------------------|
| `start_byte()`     | `usize`  | Byte offset of the node's first byte |
| `end_byte()`       | `usize`  | Byte offset past the node's last byte|
| `start_position()` | `Point`  | Row and column of the start          |
| `end_position()`   | `Point`  | Row and column of the end            |

`Point` has `row` and `column` fields. Both are **zero-based**.

```rust
let node = root.child(0).unwrap();
let start = node.start_position();
let end = node.end_position();
println!("{}:{} → {}:{}", start.row, start.column, end.row, end.column);
```

---

## Node Fields

Many grammars assign **field names** to specific children of a rule. Fields provide stable, named access to children regardless of optional siblings or ordering changes.

```rust
// For a function_item node in Rust:
let name = func.child_by_field_name("name").unwrap();
let params = func.child_by_field_name("parameters").unwrap();
let body = func.child_by_field_name("body").unwrap();
```

Not all children have field names. Use `child_by_field_name()` for named access and `child()` / `named_child()` for positional access.

---

## Extracting Text

Use `utf8_text()` to get the source text a node covers. You must pass the original source bytes:

```rust
let source = b"fn add(a: i32) -> i32 { a }";
let tree = parser.parse(source, None).unwrap();
let func = tree.root_node().child(0).unwrap();
let name = func.child_by_field_name("name").unwrap();

assert_eq!(name.utf8_text(source).unwrap(), "add");
```

---

## S-Expressions

`to_sexp()` returns a Lisp-like string representation of a node's subtree, useful for debugging and inspecting tree structure:

```rust
let root = tree.root_node();
println!("{}", root.to_sexp());
// (source_file (function_item name: (identifier) parameters: (parameters ...) ...))
```

---

## Error Nodes

Tree-sitter performs error recovery and produces a tree even for invalid syntax. Error-related nodes are marked with special types:

| Method          | Description                                              |
|-----------------|----------------------------------------------------------|
| `is_error()`    | `true` for `ERROR` nodes — unrecognized syntax           |
| `is_missing()`  | `true` for `MISSING` nodes — tokens inserted by recovery |
| `has_error()`   | `true` if this node or any descendant contains errors    |

```rust
let tree = parser.parse("fn main( {}", None).unwrap();
let root = tree.root_node();

assert!(root.has_error());

// Walk children to find error nodes
for i in 0..root.named_child_count() {
    let child = root.named_child(i).unwrap();
    if child.is_error() {
        println!("ERROR at byte {}..{}", child.start_byte(), child.end_byte());
    }
}
```
