# Async/Await in Rust

Async/await is Rust's mechanism for writing concurrent code that looks sequential. Unlike threading, async code **suspends** operations rather than **blocking** threads, allowing a single thread to multiplex many tasks.

---

## What Is Async Programming?

In synchronous code, a blocking call (reading a file, waiting on a network socket) stalls the entire thread until it completes. Async programming decouples "waiting" from "occupying a thread" — when an operation isn't ready, the task **yields** control so other tasks can run on the same thread.

```
Synchronous (1 thread, 2 requests):

Thread ─── [Request A: send]──[wait]──[recv] ──[Request B: send]──[wait]──[recv]──►

Asynchronous (1 thread, 2 requests):

Thread ─── [A: send]──[B: send]──[A: recv]──[B: recv]──►
                      ▲                     ▲
                 A is waiting,          B is waiting,
                 so work on B           so work on A
```

---

## `async fn` Returns a Future

An `async fn` does **not** execute its body when called. Instead, it returns an `impl Future<Output = T>` — a lazy value that represents a computation that hasn't started yet.

```rust
async fn fetch_data() -> String {
    // This body does NOT run when fetch_data() is called.
    // It runs only when the returned Future is .await-ed.
    "data".to_string()
}

// Calling the function produces a Future, nothing more:
let future = fetch_data(); // no work happens here

// Driving the future to completion:
let result = future.await; // NOW the body executes
```

This is fundamentally different from JavaScript or C# where `async` functions begin executing immediately up to the first `await` point. In Rust, **futures are lazy** — nothing happens until you `.await` them or pass them to a runtime.

---

## `.await` Yields Control

The `.await` keyword does two things:

1. **Drives** the future toward completion by polling it.
2. **Yields** control back to the runtime scheduler if the future is not yet ready (returns `Poll::Pending`).

When a future yields, the runtime is free to schedule other tasks on the same thread. When the underlying I/O becomes ready, the runtime re-polls the future and execution resumes right where it left off.

```rust
async fn process() {
    let data = read_from_network().await;  // yields if data isn't ready
    let parsed = parse(data);              // runs once read completes
    write_to_disk(parsed).await;           // yields again if disk is busy
}
```

---

## Compile-Time State Machines

The Rust compiler transforms every `async fn` into a **state machine** at compile time. Each `.await` point becomes a state transition. This means:

- **Zero-cost abstraction** — no heap allocation for the future itself (unless boxed).
- **No runtime overhead** — the state machine is a plain `enum` with variants for each suspension point.
- **Deterministic memory** — the size of the future is known at compile time.

Conceptually, an async function like:

```rust
async fn example() {
    let a = step_one().await;
    let b = step_two(a).await;
    step_three(b).await;
}
```

Compiles into something resembling:

```rust
enum ExampleFuture {
    StepOne { fut: StepOneFuture },
    StepTwo { a: TypeA, fut: StepTwoFuture },
    StepThree { fut: StepThreeFuture },
    Done,
}
```

Each time `poll()` is called, the state machine advances through its variants.

---

## No Implicit Runtime

Rust's standard library defines the `Future` trait but provides **no runtime** to execute futures. You must choose a runtime explicitly. Tokio is the most widely used:

| Runtime | Focus |
|---------|-------|
| **Tokio** | Full-featured async runtime for I/O, timers, networking |
| **async-std** | Standard-library-like API with async equivalents |
| **smol** | Minimal, lightweight runtime |

Without a runtime, futures do nothing — there's no background thread polling them.

---

## Async Blocks

Besides `async fn`, you can create futures inline with **async blocks**:

```rust
// Basic async block — captures references by default
let future = async {
    let data = fetch().await;
    process(data)
};

// async move — takes ownership of captured variables
let url = String::from("https://example.com");
let future = async move {
    // `url` is moved into this block
    download(&url).await
};
```

Async blocks are useful for:
- Creating futures without defining a separate `async fn`.
- Controlling ownership with `async move` when spawning tasks (spawned tasks require `'static` lifetimes).

---

## When Async Helps

Async shines when your program spends most of its time **waiting on I/O**:

- **Network servers** handling thousands of concurrent connections
- **HTTP clients** making many parallel requests
- **Database access** with concurrent queries
- **File I/O** multiplexed across many files
- **WebSocket** connections with many simultaneous clients

```rust
// Handling 10,000 connections with async — one thread does the work
async fn handle_connections(listener: TcpListener) {
    loop {
        let (socket, _) = listener.accept().await.unwrap();
        tokio::spawn(async move {
            process(socket).await;
        });
    }
}
```

## When Async Does NOT Help

- **CPU-bound work** — async doesn't parallelize computation. Use `rayon` or `tokio::task::spawn_blocking` for heavy CPU tasks.
- **Single sequential operations** — if you only do one thing at a time, async adds complexity without benefit.
- **Simple scripts** — the overhead of a runtime isn't justified for short-lived programs.

```rust
// CPU-bound work: use spawn_blocking to avoid starving the async runtime
let hash = tokio::task::spawn_blocking(move || {
    compute_expensive_hash(&data)
}).await.unwrap();
```

---

## The `#[tokio::main]` Macro

Rust's `main()` function cannot be `async` by default. The `#[tokio::main]` attribute macro sets up the Tokio runtime and blocks on your async main:

```rust
#[tokio::main]
async fn main() {
    println!("Hello from async main!");
    let result = do_work().await;
    println!("Result: {result}");
}
```

This expands to roughly:

```rust
fn main() {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            println!("Hello from async main!");
            let result = do_work().await;
            println!("Result: {result}");
        })
}
```

You can configure the runtime flavor:

```rust
// Single-threaded runtime (current_thread scheduler)
#[tokio::main(flavor = "current_thread")]
async fn main() { /* ... */ }

// Multi-threaded with specific worker count
#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() { /* ... */ }
```

---

## Synchronous vs Asynchronous Comparison

```rust
// ── Synchronous ──────────────────────────
use std::io::Read;
use std::net::TcpStream;

fn fetch_sync(addr: &str) -> Vec<u8> {
    let mut stream = TcpStream::connect(addr).unwrap();  // blocks thread
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).unwrap();                // blocks thread
    buf
}

// ── Asynchronous ─────────────────────────
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;

async fn fetch_async(addr: &str) -> Vec<u8> {
    let mut stream = TcpStream::connect(addr).await.unwrap();  // yields
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).await.unwrap();               // yields
    buf
}
```

The async version looks almost identical but can handle thousands of concurrent connections on a single thread because each `.await` yields rather than blocking.

---

## See Also

- [02-futures-in-depth.md](02-futures-in-depth.md) — How the `Future` trait, polling, and wakers work under the hood.
- [03-cancellation.md](03-cancellation.md) — Cancellation, cleanup, and graceful shutdown patterns.
