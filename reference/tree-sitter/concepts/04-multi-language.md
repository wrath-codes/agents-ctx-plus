# Multi-Language Documents

Many document formats embed one language inside another — HTML with JavaScript, ERB templates with Ruby, Markdown with fenced code blocks. Tree-sitter supports this through **included ranges**, which restrict a parser to specific byte regions of the source.

---

## Approach

Tree-sitter does **not** handle language composition automatically. You write application logic to:

1. Parse the **outer language** first.
2. Query or traverse the outer tree to find the byte ranges belonging to the **inner language**.
3. Call `parser.set_included_ranges()` to restrict the parser to those ranges.
4. Parse the **inner language** within those ranges.

---

## Setting Included Ranges

`Parser::set_included_ranges()` accepts a slice of `Range` values. Each `Range` specifies a byte span and point span:

```rust
use tree_sitter::{Range, Point};

Range {
    start_byte: usize,
    end_byte: usize,
    start_point: Point { row: usize, column: usize },
    end_point: Point { row: usize, column: usize },
}
```

The parser will only consider source text within these ranges, treating everything outside as whitespace.

---

## Example

```rust
use tree_sitter::{Parser, Range, Point};

let mut parser = Parser::new();

// First pass: parse as the outer template language
// ... get ranges for embedded code ...

// Second pass: parse embedded language in specific ranges
let ranges = vec![
    Range {
        start_byte: 10,
        end_byte: 50,
        start_point: Point { row: 1, column: 0 },
        end_point: Point { row: 3, column: 0 },
    },
];

parser.set_language(&tree_sitter_javascript::LANGUAGE.into()).unwrap();
parser.set_included_ranges(&ranges).unwrap();
let js_tree = parser.parse(full_source, None).unwrap();
```

---

## Key Details

- Ranges must be **non-overlapping** and **sorted** in ascending byte order.
- To reset the parser to parse the full document again, call `set_included_ranges(&[])`.
- Each language needs its own parse pass. If a document embeds three languages, you run three parse passes (one per language).
- Combine with incremental parsing: when the document changes, re-parse only the languages whose ranges were affected.

---

## Next Steps

- [Query Language](../queries/) — use tree-sitter queries to locate embedded language regions and extract their ranges programmatically.
