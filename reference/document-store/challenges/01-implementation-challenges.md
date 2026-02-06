# Implementation Challenges

## Overview

Building a custom document store in Rust presented several challenges, primarily related to the maturity of Rust's web ecosystem and the difficulties of porting from a dynamically-typed language.

## Limited Rust Web Support

At the time of development, Rust had limited support on the web for building custom database drivers and query engines. Specifically:

- **Custom database drivers** - Few established patterns or libraries for building database server interfaces in Rust
- **Query engine tooling** - Limited frameworks for implementing query parsing and execution in Rust compared to languages like Java or C++
- **Web server ecosystem** - While frameworks like Actix-web and Rocket existed, integrating them with custom storage engines required significant boilerplate
- **GraphQL libraries** - The Rust GraphQL ecosystem (e.g., Juniper) was less mature than equivalents in JavaScript (Apollo) or Python (Graphene)

This meant that many components had to be built from scratch rather than relying on existing libraries, increasing development time and complexity.

## Strongly vs Loosely Typed Language Porting

The document store was originally prototyped in PHP (a loosely-typed language) and then ported to Rust (a strictly-typed language). This transition introduced significant integration challenges:

### Type System Differences

```text
PHP (loosely typed):
  $value = "hello";    // string
  $value = 42;         // now an integer - no error
  $value = [1, "two"]; // mixed array - no error

Rust (strictly typed):
  let value: String = "hello".to_string();
  // let value: i32 = 42;  // ERROR: cannot reassign with different type
  // Mixed arrays require enums or trait objects
```

### Specific Challenges

| Challenge | PHP | Rust |
|-----------|-----|------|
| Variable types | Dynamic, change at runtime | Fixed at compile time |
| Null handling | `null` for any type | `Option<T>` wrapper required |
| Array types | Mixed types allowed | Homogeneous `Vec<T>` |
| Error handling | Exceptions, warnings | `Result<T, E>` types |
| Memory management | Garbage collected | Ownership and borrowing |
| String handling | Mutable, auto-coercion | `String` vs `&str`, explicit conversion |
| Byte manipulation | Loose pack/unpack | Strict byte array operations |

### Impact on Development

- **Data serialization** - PHP's flexible `pack()`/`unpack()` functions had to be replaced with explicit byte-level operations in Rust
- **Error propagation** - PHP's permissive error handling had to be replaced with Rust's strict `Result` and `Option` types throughout the codebase
- **Memory layout** - PHP's automatic memory management had to be replaced with explicit ownership patterns, particularly for page buffers and record data
- **Type conversions** - Implicit type coercions in PHP had to become explicit conversions in Rust, adding verbosity but improving safety

## Trade-offs

Despite these challenges, the Rust implementation delivered significant benefits:

| Aspect | PHP | Rust |
|--------|-----|------|
| Performance | Interpreted, slower | Compiled, ~2.3x faster than JS |
| Memory safety | Runtime errors | Compile-time guarantees |
| Concurrency | Limited | Fearless concurrency |
| Deployment | Requires PHP runtime | Single binary |
| Memory usage | Higher (GC overhead) | Lower (no GC) |

The strict type system, while harder to port to, caught many bugs at compile time that would have been runtime errors in PHP.

## Next Steps

- **[System Overview](../architecture/01-system-overview.md)** - Architecture that resulted from these design decisions
- **[Performance Results](../experiments/01-performance-results.md)** - Performance benefits of the Rust implementation
- **[Planned Improvements](../future-work/02-improvements.md)** - Future work addressing current limitations
