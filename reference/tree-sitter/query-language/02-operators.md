# Query Operators

## Captures

Append `@name` after a node pattern to **capture** that node. Captures are how you extract matched nodes from query results:

```scheme
(function_item
  name: (identifier) @fn_name)
```

Multiple captures can appear in a single pattern:

```scheme
(assignment_expression
  left: (identifier) @lhs
  right: (expression) @rhs)
```

## Quantifiers

Quantifiers control how many times a pattern element must appear:

| Quantifier | Meaning | Example |
|------------|---------|---------|
| `+` | One or more | `(comment)+` |
| `*` | Zero or more | `(decorator)* @decorator` |
| `?` | Zero or one (optional) | `(arguments (string)? @string_arg)` |

## Grouping

Parentheses create a **group** of sibling nodes that must appear consecutively:

```scheme
((comment) (function_declaration))
```

This matches a `comment` node immediately followed by a `function_declaration` as siblings.

Quantifiers apply to groups:

```scheme
((number) ("," (number))*)
```

This matches a `number` followed by zero or more comma-separated `number` nodes.

## Alternations

Square brackets define **alternatives** — the pattern matches if any one branch matches:

```scheme
(call_expression
  function: [
    (identifier) @function
    (member_expression
      property: (property_identifier) @method)
  ])
```

Alternations work with anonymous nodes for keyword matching:

```scheme
["break" "delete" "else" "for" "if" "return" "try" "while"] @keyword
```

## Anchors

The `.` anchor constrains a pattern to match at a specific **position** among siblings.

**First child** — `.` before the first pattern element:

```scheme
(array . (identifier) @first)
```

`@first` only captures an `identifier` that is the **first** named child of `array`.

**Last child** — `.` after the last pattern element:

```scheme
(block (_) @last .)
```

`@last` only captures the **last** named child of `block`.

**Immediate siblings** — `.` between two pattern elements:

```scheme
(dotted_name
  (identifier) @prev . (identifier) @next)
```

`@prev` and `@next` only match `identifier` nodes that are **immediately adjacent** siblings with no nodes in between.
