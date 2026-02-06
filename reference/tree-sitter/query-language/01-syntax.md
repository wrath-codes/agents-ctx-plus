# Query Syntax

A tree-sitter query consists of one or more **patterns**, each written as an S-expression that describes a subtree to match against a syntax tree.

## Basic Patterns

A pattern names a node type and optionally specifies children:

```scheme
(binary_expression (number_literal) (number_literal))
```

This matches any `binary_expression` node that has two `number_literal` children.

## Omitting Children

Children listed in a pattern act as **constraints**, not an exhaustive list. A node can have additional children beyond those specified:

```scheme
(binary_expression (string_literal))
```

This matches any `binary_expression` where **at least one** child is a `string_literal`, regardless of other children present.

## Fields

Prefix a child pattern with `field_name:` to constrain which field the child occupies:

```scheme
(assignment_expression
  left: (identifier)
  right: (function))
```

This only matches when `identifier` is the `left` field and `function` is the `right` field.

## Negated Fields

Use `!field_name` to assert that a field is **not present**:

```scheme
(class_declaration
  name: (identifier)
  !type_parameters)
```

This matches class declarations that do not have type parameters.

## Anonymous Nodes

String literals in double quotes match **anonymous** (unnamed) nodes — operators, keywords, punctuation:

```scheme
"!="
"null"
"return"
```

## Wildcard

| Pattern | Matches |
|---------|---------|
| `(_)` | Any **named** node |
| `_` | Any node (named or anonymous) |

```scheme
(binary_expression (_) (_))
```

Matches a `binary_expression` with any two named children.

## ERROR Node

Match syntax error nodes explicitly:

```scheme
(ERROR)
```

This matches any node that the parser flagged as a syntax error.

## MISSING Node

Match nodes inserted by error recovery (expected tokens that were not found in the source):

```scheme
(MISSING)
```

You can specify the expected type:

```scheme
(MISSING identifier)
(MISSING ";")
```

## Supertype Nodes

Supertypes are abstract grammar categories (e.g., `expression`, `statement`). A supertype pattern matches **any** of its subtypes:

```scheme
(expression)
```

To match a **specific** subtype through a supertype, use the `/` syntax:

```scheme
(expression/binary_expression)
```

This matches only `binary_expression` nodes that are classified under the `expression` supertype.
