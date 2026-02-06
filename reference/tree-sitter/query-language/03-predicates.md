# Predicates and Directives

Predicates filter matches based on node text or properties. They appear inside `#(...)` syntax within a pattern.

> **Important:** The tree-sitter C library does **not** evaluate predicates directly. It exposes them in structured form. Higher-level bindings (the Rust crate, WASM binding) implement the common predicates listed here. In Rust, use `query.general_predicates(pattern_index)` to access predicates and implement custom filtering.

## #eq? / #not-eq?

Exact text comparison — compare a capture's text against a string or another capture:

```scheme
((identifier) @var
  (#eq? @var "self"))
```

```scheme
((pair
  key: (property_identifier) @key
  value: (identifier) @val)
  (#eq? @key @val))
```

**Quantified variants:** `#any-eq?` and `#any-not-eq?` — succeed if **any** node in a quantified capture matches.

## #match? / #not-match?

Regex comparison — test a capture's text against a regular expression:

```scheme
((identifier) @constant
  (#match? @constant "^[A-Z][A-Z_]+"))
```

```scheme
((comment)+ @doc
  (#match? @doc "^///\\s+.*"))
```

**Quantified variants:** `#any-match?` and `#any-not-match?`

## #any-of?

Test a capture's text against **multiple** string literals (more efficient than chaining `#eq?` with alternations):

```scheme
((identifier) @builtin
  (#any-of? @builtin
    "arguments" "module" "console" "window" "document"))
```

## #is? / #is-not?

Property assertions — test or assert boolean properties on captures:

```scheme
((identifier) @var
  (#is-not? @var local))
```

Properties are implementation-defined by the host application or binding.

## Directives

Directives **modify** matches rather than filtering them. They end with `!` instead of `?`.

### #set!

Associate key-value metadata with a pattern match:

```scheme
((comment) @injection.content
  (#set! injection.language "python"))
```

Common uses include setting injection languages, highlight groups, and scope metadata.

### #select-adjacent!

Filter a quantified capture to only include nodes that are adjacent to each other:

```scheme
((comment)+ @doc
  (#select-adjacent! @doc))
```

### #strip!

Remove text matching a regex from a capture's content:

```scheme
((comment)+ @doc
  (#strip! @doc "^///\\s*"))
```
