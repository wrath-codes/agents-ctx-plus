# Setup and Hello Tokio

## Overview

This tutorial walks through building a mini-redis client and server using Tokio. We start by setting up the project and writing a first async program that connects to a Redis server.

## Prerequisites

- Rust toolchain installed (rustup recommended)
- Basic Rust knowledge (ownership, borrowing, modules)

## Project Setup

```bash
cargo new my-redis
cd my-redis
```

Add dependencies to `Cargo.toml`:

```toml
[dependencies]
tokio = { version = "1", features = ["full"] }
mini-redis = "0.4"
```

## First Async Program

```rust
// src/main.rs
use mini_redis::{client, Result};

#[tokio::main]
async fn main() -> Result<()> {
    // Open a connection to the mini-redis server
    let mut client = client::connect("127.0.0.1:6379").await?;

    // Set the key "hello" with value "world"
    client.set("hello", "world".into()).await?;

    // Get key "hello"
    let result = client.get("hello").await?;

    println!("got value from the server; result={:?}", result);

    Ok(())
}
```

Run the mini-redis server first:

```bash
# Install and start the server
cargo install mini-redis
mini-redis-server
```

Then run the client:

```bash
cargo run
# got value from the server; result=Some(b"world")
```

## Breaking It Down

### Async Functions and .await

`client::connect()` is an async function. Calling it doesn't execute anything immediately — it returns a `Future`. The `.await` operator yields control to the runtime until the future completes:

```rust
// This returns a Future, no work is done yet
let future = client::connect("127.0.0.1:6379");

// .await drives the future to completion
let mut client = future.await?;
```

### Futures Are Lazy

Unlike other languages, Rust futures do nothing until polled. Calling an async function without `.await` produces a compiler warning and performs no work:

```rust
async fn say_hello() {
    println!("hello");
}

#[tokio::main]
async fn main() {
    // Warning: future is never polled — "hello" is NOT printed
    say_hello();
}
```

### async fn Returns a Future

Every `async fn` is syntactic sugar. The compiler transforms it into a function that returns `impl Future<Output = T>`:

```rust
// This:
async fn hello() -> String {
    "hello".to_string()
}

// Is equivalent to:
fn hello() -> impl Future<Output = String> {
    async {
        "hello".to_string()
    }
}
```

### The #[tokio::main] Macro

`#[tokio::main]` transforms `async fn main()` into a synchronous `fn main()` that starts the Tokio runtime:

```rust
// What you write:
#[tokio::main]
async fn main() {
    println!("hello");
}

// What the macro expands to:
fn main() {
    let mut rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        println!("hello");
    })
}
```

`block_on` enters the runtime context and drives the top-level future to completion. Only one `block_on` call is needed — the runtime handles all subsequent async work.

## Feature Flags

The `"full"` feature flag enables all Tokio components. For production, you can be selective to reduce compile times:

| Feature | Description |
|---------|-------------|
| `full` | Enables everything below |
| `rt` | Runtime (required) |
| `rt-multi-thread` | Multi-threaded runtime |
| `io-util` | I/O helpers (AsyncReadExt, etc.) |
| `io-std` | Async stdin/stdout |
| `net` | TCP, UDP, Unix sockets |
| `time` | Sleep, interval, timeout |
| `process` | Async child processes |
| `sync` | Channels, Mutex, Semaphore |
| `signal` | Signal handling |
| `macros` | `#[tokio::main]`, `#[tokio::test]` |
| `fs` | Async filesystem operations |

Example with selective features:

```toml
[dependencies]
tokio = { version = "1", features = ["rt-multi-thread", "net", "macros"] }
```

## Next Steps

- **[Spawning](02-spawning.md)** - Spawn concurrent tasks to handle multiple connections
