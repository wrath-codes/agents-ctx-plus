---
name: rust
description: Use when working on projects involving rust code.
---

## References

- [Rust Design Patterns (Unofficial)](https://rust-unofficial.github.io/patterns/rust-design-patterns.pdf)
- [The Rust Programming Language (The Book)](https://doc.rust-lang.org/book/)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Clippy Lints Documentation](https://rust-lang.github.io/rust-clippy/)

---

## Code Quality Checks

All code must pass the following checks before being considered complete:

```bash
# 1. Format code according to Rust style guidelines
cargo fmt

# 2. Check for compilation errors
cargo check

# 3. Lint for common mistakes and non-idiomatic code
cargo clippy -- -D warnings

# 4. Run all tests and verify they pass
cargo test
```

Run them in this order. Never submit code that fails any of these checks. **Code must successfully compile and all tests must pass.**

---

## Project Structure

Follow standard Rust project layout conventions:

```
project-root/
├── Cargo.toml
├── Cargo.lock
├── src/
│   ├── main.rs              # Binary entry point (if applicable)
│   ├── lib.rs               # Library root (if applicable)
│   └── module_name/
│       ├── mod.rs
│       └── submodule.rs
├── tests/                   # Integration & unit tests
│   └── module_name.rs
├── benches/                 # Benchmarks (if applicable)
└── examples/                # Example usage (if applicable)
```

---

## Testing

### Requirements

- **All code must successfully compile** before testing.
- **All implemented functionality must have a corresponding unit test** in the `tests/` directory.
- Tests must be meaningful. They should validate behavior, not just assert `true`.
- Use descriptive test names that explain what is being tested.
- Test both the happy path and error/edge cases.
- **All tests must pass.** Do not submit code with failing tests.

### Running Tests

```bash
cargo test                  # Run all tests
cargo test test_name        # Run a specific test
cargo test -- --nocapture   # Run tests with output
```

---

## Naming Conventions

| Item                | Convention        | Example               |
| ------------------- | ----------------- | --------------------- |
| Crates              | `snake_case`      | `my_crate`            |
| Modules             | `snake_case`      | `my_module`           |
| Types / Traits      | `PascalCase`      | `MyStruct`, `MyTrait` |
| Functions / Methods | `snake_case`      | `do_something()`      |
| Constants           | `SCREAMING_SNAKE` | `MAX_RETRIES`         |
| Local variables     | `snake_case`      | `item_count`          |
| Type parameters     | Single uppercase  | `T`, `E`, `K`, `V`    |
| Lifetimes           | Short lowercase   | `'a`, `'de`           |

---

## Idioms

These are the community-agreed conventions for writing idiomatic Rust. Break them only with good reason.

### Use borrowed types for arguments

- Prefer `&str` over `&String`, `&[T]` over `&Vec<T>`, and `&T` over `&Box<T>` in function parameters.
- This avoids unnecessary layers of indirection and allows the function to accept more input types through deref coercion.

### Concatenating strings with `format!`

- Prefer `format!("Hello {name}!")` over manual `push`/`push_str` chains for readability.
- For performance-critical paths where the string can be pre-allocated, manual push operations may be faster.

### Constructors

- Rust has no language-level constructors. Use an associated function called `new` to create objects.
- If the type has a sensible zero/empty state, also implement the `Default` trait.
- It is common and expected to implement both `Default` and `new`. Provide `new` even if it is functionally identical to `default`.

### The `Default` Trait

- Implement or derive `Default` for structs whose fields all support it.
- Use `Default` for partial initialization: `MyStruct { field: value, ..Default::default() }`.
- `Default` enables usage with `or_default` functions throughout the standard library.

### Collections are smart pointers

- Implement the `Deref` trait for owning collections to provide a borrowed view (e.g., `Vec<T>` derefs to `&[T]`, `String` derefs to `&str`).
- Implement methods on the borrowed view (slice) rather than the owning type where possible.

### Finalisation in destructors

- Use `Drop` implementations as a replacement for `finally` blocks to ensure cleanup code runs on all exit paths (early returns, `?`, panics).
- Assign the guard object to a named variable (not just `_`) to prevent immediate destruction.
- Destructors are not guaranteed to run in all cases (infinite loops, double panics), so do not rely on them for absolutely critical finalisation.

### Use `mem::take` and `mem::replace` to keep owned values in changed enums

- When transforming an enum variant in place, use `mem::take(name)` to move values out without cloning.
- This avoids the "Clone to satisfy the borrow checker" anti-pattern.
- For `Option` fields, prefer `Option::take()` as a more idiomatic alternative.

### On-stack dynamic dispatch

- When you need dynamic dispatch over multiple types but want to avoid heap allocation, use `&mut dyn Trait` with temporary values.
- Since Rust 1.79.0, the compiler automatically extends lifetimes of temporaries in `&` or `&mut`, simplifying this pattern.

### Iterating over an `Option`

- `Option` implements `IntoIterator`, so it can be used with `.extend()`, `.chain()`, and `for` loops.
- Use `std::iter::once` as a more readable alternative to `Some(foo).into_iter()` when the value is always present.

### Pass variables to closure

- Use a separate scope block before the closure to prepare variables (clone, borrow, move) rather than creating separate named variables like `num2_cloned`.
- This groups the closure's captured state together with its definition.

### Use `#[non_exhaustive]` for extensibility

- Apply `#[non_exhaustive]` to public structs and enums that may gain fields or variants in the future, to maintain backwards compatibility across crate boundaries.
- Within a crate, a private field (e.g., `_b: ()`) achieves a similar effect.
- Use deliberately and with caution. Incrementing the major version when adding fields or variants is often a better option.

### Easy doc initialization

- When doc examples require complex setup, use a helper function that takes the complex type as a parameter to avoid repeating boilerplate.

### Temporary mutability

- When data must be prepared mutably but then used immutably, use a nested block or variable rebinding (`let data = data;`) to enforce immutability after preparation.

### Return consumed argument on error

- If a fallible function takes ownership of an argument, include that argument in the error type so the caller can recover it and retry.
- Example from std: `String::from_utf8` returns the original `Vec<u8>` inside `FromUtf8Error`.

---

## Design Patterns

### Behavioural Patterns

#### Command

- Separate actions into their own objects and pass them as parameters.
- Three approaches in Rust: trait objects (for complex commands with state), function pointers (for simple stateless commands), and `Fn` trait objects (closures).
- Use trait objects when commands are whole structs with multiple functions and state. Use function pointers or closures when commands are simple and stateless.

#### Interpreter

- Express recurring problem instances in a domain-specific language and implement an interpreter to solve them.
- Rust's `macro_rules!` can serve as a lightweight interpreter for simple DSLs at compile time.

#### Newtype

- Use a tuple struct with a single field to create a distinct type (e.g., `struct Password(String)`).
- Provides type safety, encapsulation, and the ability to implement custom traits on existing types.
- Zero-cost abstraction with no runtime overhead.
- Downside: no special language support, so pass-through methods and trait impls create boilerplate. Consider the `derive_more` crate.

#### RAII with guards

- Tie resource acquisition to object creation and resource release to object destruction (`Drop`).
- Use guard objects to mediate access to resources. The borrow checker ensures references to the resource cannot outlive the guard.
- Classic example: `MutexGuard`, which locks on creation and unlocks on drop.

#### Strategy (aka Policy)

- Define an abstract algorithm skeleton and let specific implementations be swapped via traits or closures.
- In Rust, traits naturally implement the strategy pattern. Closures provide a lightweight alternative for simple cases.
- Serde is an excellent real-world example: `Serialize`/`Deserialize` traits allow swapping serialization formats (JSON, CBOR, etc.) transparently.

#### Visitor

- Encapsulate an algorithm that operates over a heterogeneous collection of objects without modifying the data types.
- Define `visit_*` methods on a `Visitor` trait for each data type. Provide `walk_*` helper functions to factor out traversal logic.
- The visitor can be stateful, communicating information between nodes.

### Creational Patterns

#### Builder

- Construct complex objects step by step using a separate builder type.
- Provide a `builder()` method on the target type so users can discover it.
- Return the builder by value from each setter to enable method chaining: `FooBuilder::new().name("x").build()`.
- Alternatively, take and return `&mut self` for a two-phase style.
- Useful when a type has many optional fields or when construction has side effects.
- Consider the `derive_builder` crate to reduce boilerplate.

#### Fold

- Transform a data structure by running an algorithm over each node, producing a new structure.
- Provide default `fold_*` methods that recurse into children, allowing implementors to override only the nodes they care about.
- Related to the visitor pattern, but produces a new data structure rather than just observing the old one.

### Structural Patterns

#### Struct decomposition for independent borrowing

- When the borrow checker prevents simultaneous borrows of different fields in a large struct, decompose it into smaller structs.
- Compose the smaller structs back into the original. Each can then be borrowed independently.
- Often leads to better design by revealing smaller units of functionality.

#### Prefer small crates

- Build small, focused crates that do one thing well.
- Small crates are easier to understand, encourage modular code, and allow parallel compilation.
- Be mindful of dependency hell and crate quality. Not all crates on crates.io are well-maintained.

#### Contain unsafety in small modules

- Isolate `unsafe` code in the smallest possible module that upholds the needed invariants.
- Build a safe interface on top of that module. Embed it into a larger module with only safe code.
- This restricts the surface area that must be audited for safety.

#### Use custom traits to avoid complex type bounds

- When trait bounds become unwieldy (especially with `Fn` traits and specific output types), introduce a new trait with a generic `impl` for all types satisfying the original bound.
- This reduces verbosity, eliminates type parameters, and increases expressiveness.

---

## Anti-Patterns

These are common but counterproductive solutions. **Avoid them.**

### Clone to satisfy the borrow checker

- Do not resolve borrow checker errors by cloning variables without understanding the consequences.
- Cloning creates independent copies. Changes to one are not reflected in the other.
- If the borrow checker complains, first understand the ownership issue. Use `mem::take`, restructure borrows, or redesign the data flow.
- **Exception:** `Rc` and `Arc` are designed for shared ownership via clone. Cloning them is cheap and correct.
- Deliberate cloning is fine when ownership semantics require it, or for prototypes and non-performance-critical code.

### `#![deny(warnings)]`

- Do not use `#![deny(warnings)]` in crate roots. New compiler versions may introduce new warnings, breaking builds unexpectedly.
- Instead, deny specific named lints explicitly, or use `RUSTFLAGS="-D warnings"` in CI.
- This preserves Rust's stability guarantees while still enforcing lint discipline.

### Deref polymorphism

- Do not misuse the `Deref` trait to emulate struct inheritance.
- `Deref` is designed for smart pointers (`pointer-to-T` to `T`), not for converting between arbitrary types.
- It does not introduce subtyping. Traits on the inner type are not automatically available on the outer type. It interacts badly with generics and bounds checking.
- Instead, use composition with explicit delegation methods, or use traits for shared behavior.

---

## Functional Patterns

Rust supports many functional programming paradigms alongside its imperative core.

- Prefer declarative iterator chains (`.fold()`, `.map()`, `.filter()`) over imperative loops when they improve clarity.
- Use generics as type classes. Rust's generic type parameters create type class constraints. Different filled-in parameters create different types with potentially different `impl` blocks and available methods.
- Apply the **YAGNI** principle (You Aren't Going to Need It). Many traditional OO patterns are unnecessary in Rust due to traits, enums, and the type system.

---

## Error Handling

- Prefer `Result<T, E>` over `panic!()` for recoverable errors.
- Use `thiserror` for library error types and `anyhow` for application-level errors.
- Avoid `.unwrap()` and `.expect()` in production code. Use proper error propagation with `?`.
- Define custom error types when the module has more than one failure mode.
- When a fallible function consumes an argument, return the argument inside the error type so callers can recover it.

---

## Ownership and Borrowing

- Prefer borrowing (`&T`, `&mut T`) over transferring ownership when the caller does not need to give up the value.
- Use `Clone` sparingly, only when necessary. Do not clone just to satisfy the borrow checker.
- Prefer `&str` over `String` in function parameters when ownership is not needed.
- Use `Cow<'_, str>` when a function may or may not need to allocate.
- Use `mem::take` or `mem::replace` to move values out of mutable references without cloning.

---

## Structs and Enums

- Derive common traits where appropriate: `Debug`, `Clone`, `PartialEq`, `Eq`, `Hash`, `Default`.
- Use the builder pattern for structs with many optional fields.
- Prefer enums over boolean flags for state representation.
- Use `#[non_exhaustive]` on public structs and enums that may grow over time.
- Use the newtype pattern for type safety wrappers around primitives (zero-cost abstraction).
- Decompose large structs into smaller ones when the borrow checker prevents independent field access.

---

## Documentation

- All public items (`pub`) **must** have doc comments.
- Include examples in doc comments for public functions.
- Use module-level documentation at the top of files.
- Use helper functions in doc examples to avoid repeating complex setup boilerplate.

---

## Dependencies

- Prefer well-maintained, widely-used crates from [crates.io](https://crates.io).
- Pin dependency versions in `Cargo.toml` (e.g., `serde = "1.0"`, not `serde = "*"`).
- Minimize the dependency tree. Avoid adding crates for trivial functionality.
- Prefer small, focused crates that do one thing well.
- Use `cargo audit` to check for known vulnerabilities.

---

## Performance Considerations

- Avoid unnecessary heap allocations. Prefer stack allocation and slices.
- Use `&[T]` instead of `&Vec<T>` in function signatures.
- Prefer `String` only when ownership is required. Use `&str` otherwise.
- Profile before optimizing. Use `cargo bench` and tools like `criterion`.
- Use on-stack dynamic dispatch (`&dyn Trait`) to avoid heap allocation when dynamic dispatch is needed.
- Stay lazy with iterators. Avoid `.collect()`-ing unnecessarily.

---

## Design Principles

- **KISS:** Keep it simple. Simplicity should be a key goal in design.
- **YAGNI:** You Aren't Going to Need It. Do not add functionality until it is necessary.
- **DRY:** Don't Repeat Yourself. Every piece of knowledge should have a single, authoritative representation.
- **SOLID:** Single Responsibility, Open/Closed, Liskov Substitution, Interface Segregation, Dependency Inversion.
- **Composition over inheritance:** Favor polymorphic behavior and code reuse through composition, not inheritance.
- **Law of Demeter:** An object should assume as little as possible about the structure of other objects.
- **Command-Query Separation:** Functions should either return data or produce side effects, not both.
- **Principle of Least Astonishment:** Components should behave the way most users expect.

---

## Checklist Before Completion

- [ ] Code compiles: `cargo check`
- [ ] Code is formatted: `cargo fmt`
- [ ] Code is linted: `cargo clippy -- -D warnings`
- [ ] All tests pass: `cargo test`
- [ ] All public items are documented with doc comments
- [ ] Every new function or feature has a corresponding test in `tests/`
- [ ] No `.unwrap()` or `.expect()` in production code paths
- [ ] Error types are properly defined and propagated
- [ ] Borrowed types are used for function arguments where ownership is not needed
- [ ] No Clone anti-pattern usage to work around the borrow checker
- [ ] No Deref polymorphism to emulate inheritance
- [ ] No `#![deny(warnings)]` in crate roots
