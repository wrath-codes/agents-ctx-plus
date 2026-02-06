# tree-sitter — Sub-Index

> Incremental parser generator for syntax analysis (17 files)

### Root

|file|description|
|---|---|
|[README.md](README.md)|Getting started guide|

### [concepts](concepts/)

|file|description|
|---|---|
|[01-basic-parsing.md](concepts/01-basic-parsing.md)|Basic parsing — parse(), Tree, concrete syntax trees|
|[02-syntax-nodes.md](concepts/02-syntax-nodes.md)|Syntax nodes — Node, named/anonymous, traversal|
|[03-incremental-parsing.md](concepts/03-incremental-parsing.md)|Incremental — edit + reparse, O(log n) updates|
|[04-multi-language.md](concepts/04-multi-language.md)|Multi-language — injection, ranges|

### [grammar-authoring](grammar-authoring/)

|file|description|
|---|---|
|[01-getting-started.md](grammar-authoring/01-getting-started.md)|Grammar setup — tree-sitter init, grammar.js|
|[02-grammar-dsl.md](grammar-authoring/02-grammar-dsl.md)|Grammar DSL — rules, seq, choice, repeat, prec|

### [query-language](query-language/)

|file|description|
|---|---|
|[01-syntax.md](query-language/01-syntax.md)|Query syntax — S-expression patterns|
|[02-operators.md](query-language/02-operators.md)|Operators — capture, field, wildcard, anchor|
|[03-predicates.md](query-language/03-predicates.md)|Predicates — #eq?, #match?, #any-of?|

### [rust-api](rust-api/)

|file|description|
|---|---|
|[01-parser.md](rust-api/01-parser.md)|Parser — Parser::new(), set_language(), parse()|
|[02-tree-and-nodes.md](rust-api/02-tree-and-nodes.md)|Tree/Nodes — Tree, Node, walk(), children()|
|[03-tree-cursor.md](rust-api/03-tree-cursor.md)|TreeCursor — efficient traversal|
|[04-queries.md](rust-api/04-queries.md)|Queries — Query::new(), QueryCursor, captures|
|[05-types.md](rust-api/05-types.md)|Types — Point, Range, InputEdit|

### [available-parsers](available-parsers/)

|file|description|
|---|---|
|[01-language-list.md](available-parsers/01-language-list.md)|Languages — 200+ supported parsers|

### Key Patterns
```rust
let mut parser = Parser::new();
parser.set_language(&tree_sitter_rust::LANGUAGE.into())?;
let tree = parser.parse(source, None).unwrap();
let query = Query::new(&tree_sitter_rust::LANGUAGE.into(), "(function_item) @fn")?;
```

---
*17 files*
