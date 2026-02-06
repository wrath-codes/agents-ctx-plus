# Grammar DSL Reference

The `grammar.js` file uses a JavaScript-based DSL to define parsing rules. All functions below are available as globals inside the `grammar({...})` call.

## Rule Functions

| Function | Description | EBNF Equivalent |
|----------|-------------|-----------------|
| `seq(rule1, rule2, ...)` | Sequence — match rules in order | `rule1 rule2` |
| `choice(rule1, rule2, ...)` | Alternatives — match one of the rules | `rule1 \| rule2` |
| `repeat(rule)` | Zero or more | `{rule}` |
| `repeat1(rule)` | One or more | `rule {rule}` |
| `optional(rule)` | Zero or one | `[rule]` |
| `field(name, rule)` | Assign a field name to a child node | — |
| `token(rule)` | Treat a complex rule as a single token (no child nodes) | — |
| `token.immediate(rule)` | Like `token`, but requires no preceding whitespace | — |
| `alias(rule, name)` | Give a rule an alternative name in the syntax tree | — |
| `reserved(wordset, rule)` | Override the reserved word set for this rule | — |

## Precedence Functions

| Function | Description |
|----------|-------------|
| `prec(number, rule)` | Numerical precedence (higher number wins) |
| `prec.left([number], rule)` | Left-associative (default precedence 0 if number omitted) |
| `prec.right([number], rule)` | Right-associative (default precedence 0 if number omitted) |
| `prec.dynamic(number, rule)` | Runtime precedence for resolving GLR ambiguities |

## Grammar Configuration Fields

Beyond `name` and `rules`, the grammar object accepts these fields:

| Field | Description |
|-------|-------------|
| `extras` | Tokens that may appear anywhere between rules (default: whitespace). Typically whitespace and comments. |
| `inline` | Array of rule names to inline — these rules won't produce syntax tree nodes. Useful for reducing parser states. |
| `conflicts` | Array of rule-name tuples representing intended LR(1) conflicts. Enables GLR parsing for those ambiguities. |
| `externals` | Tokens produced by an external scanner (custom C code in `src/scanner.c`). |
| `precedences` | Array of arrays defining named precedence levels in descending order. |
| `word` | The rule used for keyword extraction optimization. Typically `identifier`. |
| `supertypes` | Rules that act as abstract categories (e.g., `expression`, `statement`). Each must be a `choice` of other named rules. |
| `reserved` | Named sets of reserved words for contextual keyword handling. |

## Example Grammar

```javascript
export default grammar({
  name: 'calculator',

  rules: {
    expression: $ => choice(
      $.number,
      $.binary_expression,
      seq('(', $.expression, ')')
    ),

    binary_expression: $ => choice(
      prec.left(1, seq($.expression, choice('+', '-'), $.expression)),
      prec.left(2, seq($.expression, choice('*', '/'), $.expression)),
    ),

    number: $ => /\d+/,
  }
});
```

This grammar:
- Defines `expression` as a number, a binary operation, or a parenthesized expression
- Uses `prec.left` to make operators left-associative
- Gives `*` and `/` higher precedence (2) than `+` and `-` (1)
- Uses a regex `/\d+/` for the `number` token
