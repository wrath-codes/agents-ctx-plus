# Tokio - Quick Introduction

> **An asynchronous runtime for the Rust programming language**

Tokio is an event-driven, non-blocking I/O platform for writing asynchronous applications with Rust. It provides a multi-threaded, work-stealing task scheduler, a reactor backed by the OS event queue (epoll, kqueue, IOCP), asynchronous TCP/UDP sockets, async file I/O, timers, synchronization primitives, and more. Used by 551k+ projects including AWS, Discord, Meta, Dropbox, and Azure.

## Key Features

| Feature | Description |
|---------|-------------|
| **Fast** | Zero-cost abstractions and compile-time optimizations deliver bare-metal performance |
| **Reliable** | Rust's ownership model and type system prevent data races, null pointers, and common concurrency bugs |
| **Scalable** | Minimal per-task footprint with natural backpressure and efficient resource usage |
| **Flexible** | Multi-thread (work-stealing) and single-thread (current-thread) runtime flavors |
| **Modular** | Feature flags allow compile-time selection of only the components you need |
| **Ecosystem** | Hyper (HTTP), Tonic (gRPC), Axum (web), Tower (middleware), Tracing (diagnostics), Mio (low-level I/O), Bytes (byte buffers) |

## Quick Start

### Cargo.toml

```toml
[dependencies]
tokio = { version = "1", features = ["full"] }
```

### TCP Echo Server

```rust
use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:8080").await?;

    loop {
        let (mut socket, _) = listener.accept().await?;

        tokio::spawn(async move {
            let mut buf = [0; 1024];
            loop {
                let n = match socket.read(&mut buf).await {
                    Ok(0) => return,
                    Ok(n) => n,
                    Err(_) => return,
                };
                if socket.write_all(&buf[..n]).await.is_err() {
                    return;
                }
            }
        });
    }
}
```

## Architecture

```
                    ┌──────────────────┐
                    │   Source Code     │
                    │  (async fn main) │
                    └────────┬─────────┘
                             │
              ┌──────────────▼──────────────┐
              │          Runtime             │
              │                              │
              │  ┌──────────┐ ┌──────────┐   │
              │  │Scheduler │ │ Reactor  │   │     ┌──────────────┐
              │  │(work-    │ │(epoll/   │   │     │    Sync      │
              │  │ stealing)│ │ kqueue/  │   │◄───►│  Primitives  │
              │  │          │ │ IOCP)    │   │     │(Mutex,Semaphore│
              │  └──────────┘ └──────────┘   │     │ mpsc,oneshot) │
              │  ┌──────────┐                │     └──────────────┘
              │  │  Timer   │                │
              │  │(time     │                │     ┌──────────────┐
              │  │ wheel)   │                │     │  Ecosystem   │
              │  └──────────┘                │◄───►│  Crates      │
              └──────────────┬───────────────┘     │(Hyper,Axum,  │
                             │                     │ Tonic,Tower) │
                      ┌──────▼──────┐              └──────────────┘
                      │   Tasks     │
                      │ (futures)   │
                      └──────┬──────┘
                             │
              ┌──────────────┼──────────────┐
              │              │              │
       ┌──────▼───┐   ┌─────▼────┐   ┌─────▼──────┐
       │   TCP    │   │   UDP    │   │   File     │
       │  Socket  │   │  Socket  │   │   I/O      │
       └──────────┘   └──────────┘   └────────────┘
```

## Essential Rust Types

| Type | Purpose |
|------|---------|
| `Runtime` | The async runtime that drives futures to completion |
| `Builder` | Configures and constructs a `Runtime` instance |
| `Handle` | Reference-counted handle to a running `Runtime` |
| `JoinHandle<T>` | Handle to a spawned task, used to await its result |
| `JoinSet<T>` | Collection of spawned tasks, awaited as a group |
| `TcpListener` | Listens for inbound TCP connections |
| `TcpStream` | Async TCP connection for reading and writing |
| `UdpSocket` | Async UDP socket for sending and receiving datagrams |
| `Mutex<T>` | Async-aware mutual exclusion lock |
| `RwLock<T>` | Async-aware reader-writer lock |
| `Semaphore` | Limits concurrent access to a resource |
| `mpsc` | Multi-producer, single-consumer async channel |
| `oneshot` | Single-use channel for sending one value |
| `broadcast` | Multi-producer, multi-consumer broadcast channel |
| `watch` | Single-producer, multi-consumer channel for latest value |
| `Notify` | Async notification primitive (like a condition variable) |
| `Interval` | Yields at a fixed time interval |
| `Sleep` | Future that completes after a duration |
| `Timeout` | Wraps a future with a deadline |

## Documentation Map

```
reference/tokio/
├── index.md                    # Comprehensive reference and navigation
├── README.md                   # This file - quick introduction
├── rust-api/                   # Rust API reference
│   ├── 01-runtime.md
│   ├── 02-tasks.md
│   ├── 03-io.md
│   ├── 04-networking.md
│   ├── 05-time.md
│   ├── 06-sync.md
│   ├── 07-fs.md
│   └── 08-macros.md
├── concepts/                   # Core concepts and theory
│   ├── 01-async-await.md
│   ├── 02-futures-in-depth.md
│   └── 03-cancellation.md
├── tutorial/                   # Step-by-step tutorial
│   ├── 01-setup-and-hello.md
│   ├── 02-spawning.md
│   ├── 03-shared-state.md
│   ├── 04-channels.md
│   ├── 05-io.md
│   ├── 06-framing.md
│   ├── 07-select.md
│   └── 08-streams.md
├── topics/                     # Advanced topics
│   ├── 01-bridging-sync-code.md
│   ├── 02-graceful-shutdown.md
│   └── 03-tracing.md
└── ecosystem/                  # Ecosystem and related crates
    ├── 01-workspace-crates.md
    ├── 02-tower-and-hyper.md
    └── 03-related-projects.md
```

## Quick Links

- **[Complete Reference](index.md)** - Comprehensive documentation and navigation
- **[Rust API](rust-api/)** - Runtime, Tasks, I/O, Networking, Time, Sync, FS, Macros
- **[Concepts](concepts/)** - Async/await, futures, cancellation
- **[Tutorial](tutorial/)** - Step-by-step guide from setup to streams
- **[Topics](topics/)** - Bridging sync code, graceful shutdown, tracing
- **[Ecosystem](ecosystem/)** - Workspace crates, Tower, Hyper, related projects

## External Resources

- **[Official Site](https://tokio.rs)** - Tokio project home
- **[API Docs](https://docs.rs/tokio)** - docs.rs reference for the `tokio` crate
- **[GitHub Repository](https://github.com/tokio-rs/tokio)** - Source code, issues, discussions
- **[Crates.io](https://crates.io/crates/tokio)** - Rust crate page
- **[Discord](https://discord.gg/tokio)** - Community chat

---

**Tokio - Fast, reliable, and scalable asynchronous runtime for Rust.**
