# Creating Parsers — Getting Started

## Dependencies

- **Node.js** — required to interpret `grammar.js`
- **C compiler** — required to compile the generated parser (gcc, clang, or MSVC)

## Install the CLI

```bash
cargo install tree-sitter-cli --locked
```

## Project Setup

```bash
mkdir tree-sitter-mylang
cd tree-sitter-mylang
tree-sitter init
```

This creates a `grammar.js` file with a minimal grammar:

```javascript
export default grammar({
  name: 'mylang',
  rules: {
    source_file: $ => 'hello'
  }
});
```

## Naming Convention

Repositories should be named `tree-sitter-{language}` (e.g., `tree-sitter-rust`, `tree-sitter-python`). The CLI and ecosystem tooling rely on this convention.

## Generate the Parser

```bash
tree-sitter generate
```

This reads `grammar.js` and produces C source files (`src/parser.c`, `src/tree_sitter/parser.h`, etc.) that implement the parser.

## Test the Parser

Create a sample file and parse it:

```bash
echo 'hello' > example-file
tree-sitter parse example-file
```

Expected output:

```
(source_file [0, 0] - [1, 0])
```

The output shows the syntax tree as an S-expression with byte ranges. A clean parse with no `ERROR` or `MISSING` nodes means the grammar matched the input correctly.

## Next Steps

- [Grammar DSL Reference](02-grammar-dsl.md) — all available DSL functions for defining grammars
- [Query Language](../query-language/01-syntax.md) — writing queries against parsed syntax trees
