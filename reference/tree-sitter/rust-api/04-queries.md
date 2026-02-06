# Queries

The query system is tree-sitter's pattern-matching engine. You write S-expression patterns (similar to `to_sexp()` output), compile them into a `Query`, then execute them against a tree with a `QueryCursor` to find all matching nodes.

`Query` is immutable after creation and is `Send + Sync` — compile once, share across threads. `QueryCursor` is stateful and must not be shared.

---

## Query

```rust
impl Query {
    pub fn new(language: &Language, source: &str) -> Result<Self, QueryError>
    pub fn pattern_count(&self) -> usize
    pub fn capture_names(&self) -> &[&str]
    pub fn capture_index_for_name(&self, name: &str) -> Option<u32>
    pub fn start_byte_for_pattern(&self, pattern_index: usize) -> usize
    pub fn property_predicates(&self, index: usize) -> &[(QueryProperty, bool)]
    pub fn property_settings(&self, index: usize) -> &[QueryProperty]
    pub fn general_predicates(&self, index: usize) -> &[QueryPredicate]
    pub fn disable_capture(&mut self, name: &str)
    pub fn disable_pattern(&mut self, index: usize)
}
```

### `Query::new(language, source)`

Compiles a query pattern string for the given language. Returns `Err(QueryError)` if the pattern has syntax errors, references unknown node types, or uses invalid fields.

### `capture_names()`

Returns the list of capture names used in the query (e.g., `["fn_name", "params"]` for captures `@fn_name` and `@params`). Capture indices correspond to positions in this array.

### `capture_index_for_name(name)`

Returns the numeric index for a named capture, or `None` if the name is not used in the query.

### `pattern_count()`

Returns the number of top-level patterns in the query. A query string can contain multiple patterns separated by whitespace.

### `disable_capture(name)` / `disable_pattern(index)`

Disable specific captures or patterns at runtime. Disabled captures are excluded from match results. Disabled patterns are skipped during matching. Useful for selectively toggling parts of a large query.

### `general_predicates(pattern_index)`

Returns predicates for a pattern that tree-sitter does not evaluate automatically (e.g., `#match?`, `#eq?`). Your code is responsible for checking these predicates against match results.

### Thread Safety

| Property | Value |
|----------|-------|
| `Send`   | Yes   |
| `Sync`   | Yes   |

---

## QueryCursor

```rust
impl QueryCursor {
    pub fn new() -> Self
    pub fn matches<'query, 'cursor, 'tree, T: TextProvider<I>, I: AsRef<[u8]>>(
        &'cursor mut self,
        query: &'query Query,
        node: Node<'tree>,
        text_provider: T,
    ) -> QueryMatches<'query, 'tree, T, I>
    pub fn captures<'query, 'cursor, 'tree, T: TextProvider<I>, I: AsRef<[u8]>>(
        &'cursor mut self,
        query: &'query Query,
        node: Node<'tree>,
        text_provider: T,
    ) -> QueryCaptures<'query, 'tree, T, I>
    pub fn set_byte_range(&mut self, range: std::ops::Range<usize>) -> &mut Self
    pub fn set_point_range(&mut self, range: std::ops::Range<Point>) -> &mut Self
    pub fn set_max_start_depth(&mut self, max_start_depth: Option<u32>) -> &mut Self
    pub fn match_limit(&self) -> u32
    pub fn set_match_limit(&mut self, limit: u32)
}
```

### `matches(query, node, text_provider)`

Returns an iterator over all `QueryMatch` values within the subtree rooted at `node`. Each match contains all captures for one occurrence of a pattern. The `text_provider` supplies source text for predicate evaluation — typically `source.as_slice()` for a `&[u8]` buffer.

### `captures(query, node, text_provider)`

Returns an iterator over `(QueryMatch, usize)` tuples where the `usize` is the index of the capture within the match. Unlike `matches()`, this yields one item per capture rather than one per match, which can be more convenient when you only care about specific captures.

### `set_byte_range(range)` / `set_point_range(range)`

Restricts the cursor to only report matches within the given byte or point range. Useful for limiting queries to the visible region of an editor.

### `set_max_start_depth(depth)`

Limits how deep into the tree the cursor will look for pattern starts. `Some(0)` matches only the root node itself. `None` (default) has no limit.

### `set_match_limit(limit)` / `match_limit()`

Controls the maximum number of in-progress matches the cursor tracks simultaneously. If exceeded, some matches may be dropped. The default is 32. Increase for queries that produce many overlapping matches.

### Thread Safety

| Property | Value |
|----------|-------|
| `Send`   | Yes   |
| `Sync`   | No    |

---

## QueryMatch and QueryCapture

```rust
pub struct QueryMatch<'cursor, 'tree> {
    pub pattern_index: usize,
    pub captures: &'cursor [QueryCapture<'tree>],
}

impl<'cursor, 'tree> QueryMatch<'cursor, 'tree> {
    pub fn nodes_for_capture_index(
        &self, capture_index: u32,
    ) -> impl Iterator<Item = Node<'tree>> + '_
}

pub struct QueryCapture<'tree> {
    pub node: Node<'tree>,
    pub index: u32,
}
```

- `pattern_index` — which pattern in the query produced this match (0-based).
- `captures` — the captured nodes. Each `QueryCapture` has a `node` and an `index` into `query.capture_names()`.
- `nodes_for_capture_index()` — convenience method to get all nodes for a given capture index within a single match (useful when a capture appears multiple times via quantifiers).

---

## Text Provider

Both `matches()` and `captures()` require a `TextProvider` — anything that can supply source text chunks for predicate evaluation. The simplest provider is a byte slice:

```rust
// &[u8] implements TextProvider
let source = b"fn main() {}";
cursor.matches(&query, root_node, source.as_slice());
```

For rope-based editors, implement the `TextProvider` trait to supply text chunks by byte range without materializing the entire source.

---

## Examples

### Finding All Function Definitions

```rust
use tree_sitter::{Parser, Query, QueryCursor};

let mut parser = Parser::new();
parser
    .set_language(&tree_sitter_rust::LANGUAGE.into())
    .unwrap();

let source = b"fn hello() {} fn world() {} struct Foo;";
let tree = parser.parse(source, None).unwrap();

let query = Query::new(
    &tree_sitter_rust::LANGUAGE.into(),
    "(function_item name: (identifier) @fn_name) @fn_def",
)
.unwrap();

let mut cursor = QueryCursor::new();
let matches = cursor.matches(&query, tree.root_node(), source.as_slice());

for m in matches {
    for capture in m.captures {
        let name = &query.capture_names()[capture.index as usize];
        let text = capture.node.utf8_text(source).unwrap();
        println!("@{}: {}", name, text);
    }
}
// Output:
//   @fn_name: hello
//   @fn_def: fn hello() {}
//   @fn_name: world
//   @fn_def: fn world() {}
```

### Extracting Function Names and Parameters

```rust
use tree_sitter::{Parser, Query, QueryCursor};

let mut parser = Parser::new();
parser
    .set_language(&tree_sitter_rust::LANGUAGE.into())
    .unwrap();

let source = b"fn add(a: i32, b: i32) -> i32 { a + b } fn noop() {}";
let tree = parser.parse(source, None).unwrap();

let query = Query::new(
    &tree_sitter_rust::LANGUAGE.into(),
    r#"
    (function_item
      name: (identifier) @name
      parameters: (parameters) @params
      return_type: (_)? @ret)
    "#,
)
.unwrap();

let name_idx = query.capture_index_for_name("name").unwrap();
let params_idx = query.capture_index_for_name("params").unwrap();
let ret_idx = query.capture_index_for_name("ret");

let mut cursor = QueryCursor::new();
for m in cursor.matches(&query, tree.root_node(), source.as_slice()) {
    let name = m
        .nodes_for_capture_index(name_idx)
        .next()
        .unwrap()
        .utf8_text(source)
        .unwrap();

    let params = m
        .nodes_for_capture_index(params_idx)
        .next()
        .unwrap()
        .utf8_text(source)
        .unwrap();

    let ret = ret_idx.and_then(|idx| {
        m.nodes_for_capture_index(idx)
            .next()
            .map(|n| n.utf8_text(source).unwrap())
    });

    println!("fn {}{}  -> {:?}", name, params, ret);
}
// Output:
//   fn add(a: i32, b: i32)  -> Some("i32")
//   fn noop()  -> None
```

### Using Captures for Specific Nodes

The `captures()` method yields one result per capture occurrence, which is convenient when you only care about one capture name.

```rust
use tree_sitter::{Parser, Query, QueryCursor};

let mut parser = Parser::new();
parser
    .set_language(&tree_sitter_rust::LANGUAGE.into())
    .unwrap();

let source = b"let x = 1; let y = 2; let z = 3;";
let tree = parser.parse(source, None).unwrap();

let query = Query::new(
    &tree_sitter_rust::LANGUAGE.into(),
    "(let_declaration pattern: (identifier) @var_name value: (_) @var_value)",
)
.unwrap();

let var_name_idx = query.capture_index_for_name("var_name").unwrap();

let mut cursor = QueryCursor::new();
for (m, capture_idx) in cursor.captures(&query, tree.root_node(), source.as_slice()) {
    let capture = &m.captures[capture_idx];
    let name = &query.capture_names()[capture.index as usize];
    let text = capture.node.utf8_text(source).unwrap();
    println!("@{} = {}", name, text);
}
// Output:
//   @var_name = x
//   @var_value = 1
//   @var_name = y
//   @var_value = 2
//   @var_name = z
//   @var_value = 3
```

### Limiting Query Scope with Byte Range

```rust
use tree_sitter::{Parser, Query, QueryCursor};

let mut parser = Parser::new();
parser
    .set_language(&tree_sitter_rust::LANGUAGE.into())
    .unwrap();

let source = b"fn a() {} fn b() {} fn c() {}";
let tree = parser.parse(source, None).unwrap();

let query = Query::new(
    &tree_sitter_rust::LANGUAGE.into(),
    "(function_item name: (identifier) @name)",
)
.unwrap();

let mut cursor = QueryCursor::new();

// Only match functions in bytes 10..20 (covers "fn b() {}")
cursor.set_byte_range(10..20);

let matches: Vec<_> = cursor
    .matches(&query, tree.root_node(), source.as_slice())
    .collect();

assert_eq!(matches.len(), 1);
let name = matches[0].captures[0].node.utf8_text(source).unwrap();
assert_eq!(name, "b");
```

### Multiple Patterns in a Single Query

A query string can contain multiple patterns. Each match's `pattern_index` tells you which pattern produced it.

```rust
use tree_sitter::{Parser, Query, QueryCursor};

let mut parser = Parser::new();
parser
    .set_language(&tree_sitter_rust::LANGUAGE.into())
    .unwrap();

let source = b"fn foo() {} struct Bar; fn baz() {}";
let tree = parser.parse(source, None).unwrap();

let query = Query::new(
    &tree_sitter_rust::LANGUAGE.into(),
    r#"
    (function_item name: (identifier) @fn_name)
    (struct_item name: (type_identifier) @struct_name)
    "#,
)
.unwrap();

assert_eq!(query.pattern_count(), 2);

let mut cursor = QueryCursor::new();
for m in cursor.matches(&query, tree.root_node(), source.as_slice()) {
    let capture = &m.captures[0];
    let text = capture.node.utf8_text(source).unwrap();
    match m.pattern_index {
        0 => println!("Function: {}", text),
        1 => println!("Struct: {}", text),
        _ => unreachable!(),
    }
}
// Output:
//   Function: foo
//   Struct: Bar
//   Function: baz
```

---

## Query Pattern Syntax (Quick Reference)

| Pattern | Meaning |
|---------|---------|
| `(node_type)` | Match a node of this type |
| `(node_type field: (child_type))` | Match with a specific field |
| `(node_type (child_type))` | Match with an anonymous (non-field) child |
| `(node_type field: (child_type) @cap)` | Capture the child as `@cap` |
| `(node_type) @cap` | Capture the whole node |
| `(node_type (_))` | Match with any child type (wildcard) |
| `(node_type field: (_)? @cap)` | Optional child (zero or one) |
| `(node_type (child_type)+ @cap)` | One or more children |
| `(node_type (child_type)* @cap)` | Zero or more children |
| `"keyword"` | Match an anonymous (literal) node |
| `(#eq? @cap "text")` | Predicate: capture text equals literal |
| `(#match? @cap "regex")` | Predicate: capture text matches regex |

---

## See Also

- [Parser](01-parser.md) — producing trees to query
- [Trees and Nodes](02-tree-and-nodes.md) — understanding the nodes returned by queries
- [TreeCursor](03-tree-cursor.md) — manual traversal as an alternative to queries
- [Supporting Types](05-types.md) — `QueryError`, `QueryPredicate`, `QueryProperty`
