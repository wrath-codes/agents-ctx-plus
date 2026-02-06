# Available Language Parsers

## Official Parsers

These parsers are maintained in the [tree-sitter GitHub organization](https://github.com/tree-sitter):

| Language | Rust Crate | Repository |
|----------|-----------|------------|
| Bash | `tree-sitter-bash` | [github.com/tree-sitter/tree-sitter-bash](https://github.com/tree-sitter/tree-sitter-bash) |
| C | `tree-sitter-c` | [github.com/tree-sitter/tree-sitter-c](https://github.com/tree-sitter/tree-sitter-c) |
| C++ | `tree-sitter-cpp` | [github.com/tree-sitter/tree-sitter-cpp](https://github.com/tree-sitter/tree-sitter-cpp) |
| C# | `tree-sitter-c-sharp` | [github.com/tree-sitter/tree-sitter-c-sharp](https://github.com/tree-sitter/tree-sitter-c-sharp) |
| CSS | `tree-sitter-css` | [github.com/tree-sitter/tree-sitter-css](https://github.com/tree-sitter/tree-sitter-css) |
| Go | `tree-sitter-go` | [github.com/tree-sitter/tree-sitter-go](https://github.com/tree-sitter/tree-sitter-go) |
| Haskell | `tree-sitter-haskell` | [github.com/tree-sitter/tree-sitter-haskell](https://github.com/tree-sitter/tree-sitter-haskell) |
| HTML | `tree-sitter-html` | [github.com/tree-sitter/tree-sitter-html](https://github.com/tree-sitter/tree-sitter-html) |
| Java | `tree-sitter-java` | [github.com/tree-sitter/tree-sitter-java](https://github.com/tree-sitter/tree-sitter-java) |
| JavaScript | `tree-sitter-javascript` | [github.com/tree-sitter/tree-sitter-javascript](https://github.com/tree-sitter/tree-sitter-javascript) |
| JSON | `tree-sitter-json` | [github.com/tree-sitter/tree-sitter-json](https://github.com/tree-sitter/tree-sitter-json) |
| Julia | `tree-sitter-julia` | [github.com/tree-sitter/tree-sitter-julia](https://github.com/tree-sitter/tree-sitter-julia) |
| OCaml | `tree-sitter-ocaml` | [github.com/tree-sitter/tree-sitter-ocaml](https://github.com/tree-sitter/tree-sitter-ocaml) |
| PHP | `tree-sitter-php` | [github.com/tree-sitter/tree-sitter-php](https://github.com/tree-sitter/tree-sitter-php) |
| Python | `tree-sitter-python` | [github.com/tree-sitter/tree-sitter-python](https://github.com/tree-sitter/tree-sitter-python) |
| Regex | `tree-sitter-regex` | [github.com/tree-sitter/tree-sitter-regex](https://github.com/tree-sitter/tree-sitter-regex) |
| Ruby | `tree-sitter-ruby` | [github.com/tree-sitter/tree-sitter-ruby](https://github.com/tree-sitter/tree-sitter-ruby) |
| Rust | `tree-sitter-rust` | [github.com/tree-sitter/tree-sitter-rust](https://github.com/tree-sitter/tree-sitter-rust) |
| Scala | `tree-sitter-scala` | [github.com/tree-sitter/tree-sitter-scala](https://github.com/tree-sitter/tree-sitter-scala) |
| TypeScript | `tree-sitter-typescript` | [github.com/tree-sitter/tree-sitter-typescript](https://github.com/tree-sitter/tree-sitter-typescript) |

## WASM Parsers

Pre-built `.wasm` parser files are available from [`tree-sitter-wasms@0.1.13`](https://www.npmjs.com/package/tree-sitter-wasms). These can be loaded at runtime in WASM-compatible environments.

Available languages (36 parsers):

`bash`, `c`, `c_sharp`, `cpp`, `css`, `dart`, `elisp`, `elixir`, `elm`, `erlang`, `go`, `html`, `java`, `javascript`, `json`, `kotlin`, `lua`, `markdown`, `objc`, `ocaml`, `perl`, `php`, `python`, `ql`, `rescript`, `ruby`, `rust`, `scala`, `solidity`, `swift`, `systemrdl`, `toml`, `tsx`, `typescript`, `verilog`, `yaml`

## Community Parsers

A comprehensive list of community-maintained parsers is available at:
[github.com/tree-sitter/tree-sitter/wiki/List-of-parsers](https://github.com/tree-sitter/tree-sitter/wiki/List-of-parsers)

## Using a Parser in Rust

Add dependencies to `Cargo.toml`:

```toml
[dependencies]
tree-sitter = "0.26"
tree-sitter-rust = "0.23"
```

Initialize a parser with the language:

```rust
use tree_sitter::Parser;

let mut parser = Parser::new();
parser.set_language(&tree_sitter_rust::LANGUAGE.into()).unwrap();
```

## Next Steps

- [Rust API — Core Concepts](../concepts/) — `Parser`, `Tree`, `Node`, and `Query` types
- [Query Language](../query-language/01-syntax.md) — writing queries to extract information from syntax trees
- [Creating Parsers](../grammar-authoring/01-getting-started.md) — authoring your own grammar
