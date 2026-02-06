# tree-sitter — Sub-Index

> Incremental parser generator for syntax analysis (16 files)

### Root

|file|description|
|---|---|
|[README.md](README.md)|Getting started guide|
| |↳ [Key Features](README.md#key-features) · [Quick Start (Rust)](README.md#quick-start-rust) · [Architecture](README.md#architecture) · [Essential Rust Types](README.md#essential-rust-types) · [Documentation Map](README.md#documentation-map) · [Quick Links](README.md#quick-links) · [External Resources](README.md#external-resources)|

### [concepts](concepts/)

|file|description|
|---|---|
|[01-basic-parsing.md](concepts/01-basic-parsing.md)|Basic parsing — parse(), Tree, concrete syntax trees|
| |↳ [The Four Core Objects](concepts/01-basic-parsing.md#the-four-core-objects) · [Complete Example](concepts/01-basic-parsing.md#complete-example) · [Providing Source Code](concepts/01-basic-parsing.md#providing-source-code)|
|[02-syntax-nodes.md](concepts/02-syntax-nodes.md)|Syntax nodes — Node, named/anonymous, traversal|
| |↳ [Named vs Anonymous Nodes](concepts/02-syntax-nodes.md#named-vs-anonymous-nodes) · [Node Positions](concepts/02-syntax-nodes.md#node-positions) · [Node Fields](concepts/02-syntax-nodes.md#node-fields) · [Extracting Text](concepts/02-syntax-nodes.md#extracting-text) · [S-Expressions](concepts/02-syntax-nodes.md#s-expressions) · [Error Nodes](concepts/02-syntax-nodes.md#error-nodes)|
|[03-incremental-parsing.md](concepts/03-incremental-parsing.md)|Incremental — edit + reparse, O(log n) updates|
| |↳ [How It Works](concepts/03-incremental-parsing.md#how-it-works) · [InputEdit](concepts/03-incremental-parsing.md#inputedit) · [Complete Example](concepts/03-incremental-parsing.md#complete-example) · [Changed Ranges](concepts/03-incremental-parsing.md#changed-ranges) · [Editing Retained Node References](concepts/03-incremental-parsing.md#editing-retained-node-references) · [Concurrency](concepts/03-incremental-parsing.md#concurrency)|
|[04-multi-language.md](concepts/04-multi-language.md)|Multi-language — injection, ranges|
| |↳ [Approach](concepts/04-multi-language.md#approach) · [Setting Included Ranges](concepts/04-multi-language.md#setting-included-ranges) · [Example](concepts/04-multi-language.md#example) · [Key Details](concepts/04-multi-language.md#key-details) · [Next Steps](concepts/04-multi-language.md#next-steps)|

### [grammar-authoring](grammar-authoring/)

|file|description|
|---|---|
|[01-getting-started.md](grammar-authoring/01-getting-started.md)|Grammar setup — tree-sitter init, grammar.js|
| |↳ [Dependencies](grammar-authoring/01-getting-started.md#dependencies) · [Install the CLI](grammar-authoring/01-getting-started.md#install-the-cli) · [Project Setup](grammar-authoring/01-getting-started.md#project-setup) · [Naming Convention](grammar-authoring/01-getting-started.md#naming-convention) · [Generate the Parser](grammar-authoring/01-getting-started.md#generate-the-parser) · [Test the Parser](grammar-authoring/01-getting-started.md#test-the-parser) · [Next Steps](grammar-authoring/01-getting-started.md#next-steps)|
|[02-grammar-dsl.md](grammar-authoring/02-grammar-dsl.md)|Grammar DSL — rules, seq, choice, repeat, prec|
| |↳ [Rule Functions](grammar-authoring/02-grammar-dsl.md#rule-functions) · [Precedence Functions](grammar-authoring/02-grammar-dsl.md#precedence-functions) · [Grammar Configuration Fields](grammar-authoring/02-grammar-dsl.md#grammar-configuration-fields) · [Example Grammar](grammar-authoring/02-grammar-dsl.md#example-grammar)|

### [query-language](query-language/)

|file|description|
|---|---|
|[01-syntax.md](query-language/01-syntax.md)|Query syntax — S-expression patterns|
| |↳ [Basic Patterns](query-language/01-syntax.md#basic-patterns) · [Omitting Children](query-language/01-syntax.md#omitting-children) · [Fields](query-language/01-syntax.md#fields) · [Negated Fields](query-language/01-syntax.md#negated-fields) · [Anonymous Nodes](query-language/01-syntax.md#anonymous-nodes) · [Wildcard](query-language/01-syntax.md#wildcard) · [ERROR Node](query-language/01-syntax.md#error-node) · [MISSING Node](query-language/01-syntax.md#missing-node) · +1 more|
|[02-operators.md](query-language/02-operators.md)|Operators — capture, field, wildcard, anchor|
| |↳ [Captures](query-language/02-operators.md#captures) · [Quantifiers](query-language/02-operators.md#quantifiers) · [Grouping](query-language/02-operators.md#grouping) · [Alternations](query-language/02-operators.md#alternations) · [Anchors](query-language/02-operators.md#anchors)|
|[03-predicates.md](query-language/03-predicates.md)|Predicates — #eq?, #match?, #any-of?|
| |↳ [#eq? / #not-eq?](query-language/03-predicates.md#eq-not-eq) · [#match? / #not-match?](query-language/03-predicates.md#match-not-match) · [#any-of?](query-language/03-predicates.md#any-of) · [#is? / #is-not?](query-language/03-predicates.md#is-is-not) · [Directives](query-language/03-predicates.md#directives)|

### [rust-api](rust-api/)

|file|description|
|---|---|
|[01-parser.md](rust-api/01-parser.md)|Parser — Parser::new(), set_language(), parse()|
| |↳ [API Reference](rust-api/01-parser.md#api-reference) · [Examples](rust-api/01-parser.md#examples) · [Thread Safety](rust-api/01-parser.md#thread-safety)|
|[02-tree-and-nodes.md](rust-api/02-tree-and-nodes.md)|Tree/Nodes — Tree, Node, walk(), children()|
| |↳ [Tree](rust-api/02-tree-and-nodes.md#tree) · [Node](rust-api/02-tree-and-nodes.md#node) · [Named vs Anonymous Nodes](rust-api/02-tree-and-nodes.md#named-vs-anonymous-nodes) · [Examples](rust-api/02-tree-and-nodes.md#examples)|
|[03-tree-cursor.md](rust-api/03-tree-cursor.md)|TreeCursor — efficient traversal|
| |↳ [API Reference](rust-api/03-tree-cursor.md#api-reference) · [Key Concepts](rust-api/03-tree-cursor.md#key-concepts) · [Examples](rust-api/03-tree-cursor.md#examples)|
|[04-queries.md](rust-api/04-queries.md)|Queries — Query::new(), QueryCursor, captures|
| |↳ [Query](rust-api/04-queries.md#query) · [QueryCursor](rust-api/04-queries.md#querycursor) · [QueryMatch and QueryCapture](rust-api/04-queries.md#querymatch-and-querycapture) · [Text Provider](rust-api/04-queries.md#text-provider) · [Examples](rust-api/04-queries.md#examples) · [Query Pattern Syntax (Quick Reference)](rust-api/04-queries.md#query-pattern-syntax-quick-reference)|
|[05-types.md](rust-api/05-types.md)|Types — Point, Range, InputEdit|
| |↳ [Position Types](rust-api/05-types.md#position-types) · [InputEdit](rust-api/05-types.md#inputedit) · [Language Types](rust-api/05-types.md#language-types) · [Query Error Types](rust-api/05-types.md#query-error-types) · [Query Predicate Types](rust-api/05-types.md#query-predicate-types) · [Logging](rust-api/05-types.md#logging) · [C-to-Rust Type Mapping](rust-api/05-types.md#c-to-rust-type-mapping) · [Thread Safety Summary](rust-api/05-types.md#thread-safety-summary)|

### [available-parsers](available-parsers/)

|file|description|
|---|---|
|[01-language-list.md](available-parsers/01-language-list.md)|Languages — 200+ supported parsers|
| |↳ [Official Parsers](available-parsers/01-language-list.md#official-parsers) · [WASM Parsers](available-parsers/01-language-list.md#wasm-parsers) · [Community Parsers](available-parsers/01-language-list.md#community-parsers) · [Using a Parser in Rust](available-parsers/01-language-list.md#using-a-parser-in-rust) · [Next Steps](available-parsers/01-language-list.md#next-steps)|

### Key Patterns
```rust
let mut parser = Parser::new();
parser.set_language(&tree_sitter_rust::LANGUAGE.into())?;
let tree = parser.parse(source, None).unwrap();
let query = Query::new(&tree_sitter_rust::LANGUAGE.into(), "(function_item) @fn")?;
```

---
*16 files*
