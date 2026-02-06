# Tree-sitter - Quick Introduction

> **A parser generator tool and an incremental parsing library**

Tree-sitter is a parser generator tool and an incremental parsing library. It builds a concrete syntax tree for source files and efficiently updates the syntax tree as the source code is edited. Tree-sitter aims to be general enough to parse any programming language, fast enough to parse on every keystroke, robust enough to provide useful results even in the presence of syntax errors, and dependency-free so the runtime library has no third-party dependencies.

## Key Features

| Feature | Description |
|---------|-------------|
| **General** | Parse any programming language with a unified API |
| **Fast** | Designed for every-keystroke parsing in text editors |
| **Robust** | Produces useful trees even with syntax errors present |
| **Dependency-free** | Pure C11 runtime with zero third-party dependencies |
| **Incremental** | Re-parses only what changed, not the entire file |
| **Thread-safe** | Safe for concurrent use across multiple threads |

## Quick Start (Rust)

### Cargo.toml

```toml
[dependencies]
tree-sitter = "0.26"
tree-sitter-rust = "0.23"
```

### Basic Parsing

```rust
use tree_sitter::Parser;

fn main() {
    let mut parser = Parser::new();
    let language = tree_sitter_rust::LANGUAGE;
    parser.set_language(&language.into()).expect("Error loading Rust grammar");

    let source_code = "fn main() { println!(\"Hello\"); }";
    let tree = parser.parse(source_code, None).unwrap();
    let root_node = tree.root_node();

    println!("Root: {}", root_node.kind());
    println!("S-expression: {}", root_node.to_sexp());
}
```

## Architecture

```
                    ┌──────────────┐
                    │ Source Code   │
                    │  (string)    │
                    └──────┬───────┘
                           │
                    ┌──────▼───────┐     ┌────────────┐
                    │   Parser     │◄────│  Language   │
                    │              │     │  (grammar)  │
                    └──────┬───────┘     └────────────┘
                           │
                    ┌──────▼───────┐
                    │    Tree      │
                    │  (syntax)    │
                    └──────┬───────┘
                           │
              ┌────────────┼────────────┐
              │            │            │
       ┌──────▼───┐ ┌─────▼────┐ ┌─────▼──────┐
       │  Nodes   │ │  Cursor  │ │  Queries   │
       │ (access) │ │ (walk)   │ │ (pattern)  │
       └──────────┘ └──────────┘ └────────────┘
```

## Essential Rust Types

| Type | Purpose |
|------|---------|
| `Parser` | Parses source code into a syntax tree |
| `Language` | Defines the grammar rules for a specific language |
| `Tree` | Immutable syntax tree produced by the parser |
| `Node` | A single node within the syntax tree |
| `TreeCursor` | Efficient stateful cursor for walking the tree |
| `Query` | Compiled S-expression pattern for matching nodes |
| `QueryCursor` | Executes a query against a tree, producing matches |
| `QueryMatch` | A single match result from a query execution |
| `QueryCapture` | A captured node within a query match |

## Documentation Map

```
reference/tree-sitter/
├── index.md                    # Comprehensive reference and navigation
├── README.md                   # This file - quick introduction
├── rust-api/                   # Rust API reference
│   ├── 01-parser.md
│   ├── 02-tree-and-nodes.md
│   ├── 03-tree-cursor.md
│   ├── 04-queries.md
│   └── 05-types.md
├── concepts/                   # Core concepts and theory
│   ├── 01-basic-parsing.md
│   ├── 02-syntax-nodes.md
│   ├── 03-incremental-parsing.md
│   └── 04-multi-language.md
├── query-language/             # Query pattern language
│   ├── 01-syntax.md
│   ├── 02-operators.md
│   └── 03-predicates.md
├── grammar-authoring/          # Writing custom grammars
│   ├── 01-getting-started.md
│   └── 02-grammar-dsl.md
└── available-parsers/          # Official language parsers
    └── 01-language-list.md
```

## Quick Links

- **[Complete Reference](index.md)** - Comprehensive documentation and navigation
- **[Rust API](rust-api/)** - Parser, Tree, Node, Query reference
- **[Concepts](concepts/)** - Basic parsing, syntax nodes, incremental parsing
- **[Query Language](query-language/)** - Pattern syntax, operators, predicates
- **[Grammar Authoring](grammar-authoring/)** - Writing custom grammars
- **[Available Parsers](available-parsers/)** - Official language parser list

## External Resources

- **[Official Documentation](https://tree-sitter.github.io/tree-sitter/)** - Tree-sitter docs
- **[Rust API Docs](https://docs.rs/tree-sitter)** - docs.rs reference
- **[GitHub Repository](https://github.com/tree-sitter/tree-sitter)** - Source code and issues
- **[Crates.io](https://crates.io/crates/tree-sitter)** - Rust crate

---

**Tree-sitter - Fast, robust, incremental parsing for every language.**
