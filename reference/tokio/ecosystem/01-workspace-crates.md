# Tokio Workspace Crates

## Overview

The [tokio-rs/tokio](https://github.com/tokio-rs/tokio) repository is a Cargo workspace containing the core runtime and several companion crates. Two closely related crates — `bytes` and `mio` — live in separate repositories but are foundational to the ecosystem.

---

## tokio

| | |
|---|---|
| **Crate** | [`tokio`](https://crates.io/crates/tokio) |
| **Docs** | [docs.rs/tokio](https://docs.rs/tokio) |
| **Version** | 1.x (latest ~1.49) |
| **Repository** | [tokio-rs/tokio](https://github.com/tokio-rs/tokio) |

The main crate. Provides the async runtime, I/O driver, networking primitives, timers, synchronization, filesystem operations, process management, and signal handling.

### Feature Flags

| Feature | Description |
|---------|-------------|
| `rt` | Core runtime (current-thread scheduler) |
| `rt-multi-thread` | Multi-threaded work-stealing scheduler |
| `io-util` | `AsyncReadExt`, `AsyncWriteExt` helpers |
| `io-std` | Async `stdin` / `stdout` / `stderr` |
| `net` | TCP, UDP, Unix sockets |
| `time` | `sleep`, `interval`, `timeout` |
| `process` | Async child processes |
| `signal` | Async signal handling |
| `sync` | Channels, `Mutex`, `RwLock`, `Semaphore`, etc. |
| `fs` | Async filesystem operations |
| `macros` | `#[tokio::main]`, `#[tokio::test]`, `select!`, `join!` |
| `test-util` | Time mocking for tests |
| `full` | Enables all stable features |

### Key Types & Traits

```rust
// Runtime
tokio::runtime::Runtime
tokio::runtime::Builder

// I/O
tokio::io::AsyncRead
tokio::io::AsyncWrite
tokio::io::AsyncReadExt   // (io-util)
tokio::io::AsyncWriteExt  // (io-util)

// Networking
tokio::net::TcpListener
tokio::net::TcpStream
tokio::net::UdpSocket

// Time
tokio::time::sleep(duration)
tokio::time::interval(period)
tokio::time::timeout(duration, future)

// Sync
tokio::sync::mpsc        // multi-producer, single-consumer
tokio::sync::oneshot      // single-value channel
tokio::sync::broadcast    // multi-producer, multi-consumer
tokio::sync::watch        // single-value, latest-wins
tokio::sync::Mutex
tokio::sync::RwLock
tokio::sync::Semaphore
tokio::sync::Notify
tokio::sync::Barrier

// Tasks
tokio::task::spawn(future)
tokio::task::spawn_blocking(closure)
tokio::task::JoinHandle<T>
tokio::task::JoinSet<T>
```

### Minimal Example

```rust
use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8080").await?;

    loop {
        let (mut socket, _addr) = listener.accept().await?;
        tokio::spawn(async move {
            let mut buf = [0u8; 1024];
            let n = socket.read(&mut buf).await.unwrap();
            socket.write_all(&buf[..n]).await.unwrap();
        });
    }
}
```

---

## tokio-macros

| | |
|---|---|
| **Crate** | [`tokio-macros`](https://crates.io/crates/tokio-macros) |
| **Docs** | [docs.rs/tokio-macros](https://docs.rs/tokio-macros) |
| **Version** | 2.x |
| **Repository** | [tokio-rs/tokio](https://github.com/tokio-rs/tokio/tree/master/tokio-macros) |

Procedural macros for Tokio. Usually pulled in transitively via the `macros` feature flag on the `tokio` crate.

### Provided Macros

| Macro | Description |
|-------|-------------|
| `#[tokio::main]` | Turns an `async fn main()` into a synchronous entry point that starts the runtime |
| `#[tokio::test]` | Turns an `async fn` test into a synchronous test that runs on the Tokio runtime |

### Configuration Options

```rust
// Default: current-thread runtime
#[tokio::main]
async fn main() {}

// Multi-threaded runtime
#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {}

// Test with time pausing enabled
#[tokio::test(start_paused = true)]
async fn my_test() {}
```

---

## tokio-stream

| | |
|---|---|
| **Crate** | [`tokio-stream`](https://crates.io/crates/tokio-stream) |
| **Docs** | [docs.rs/tokio-stream](https://docs.rs/tokio-stream) |
| **Version** | 0.1.x |
| **Repository** | [tokio-rs/tokio](https://github.com/tokio-rs/tokio/tree/master/tokio-stream) |

Stream utilities for Tokio. Re-exports `futures_core::Stream` and provides the `StreamExt` trait with a rich set of adapters.

### StreamExt Adapters

| Adapter | Description |
|---------|-------------|
| `next()` | Yield the next item |
| `map()` | Transform each item |
| `filter()` | Keep items matching a predicate |
| `filter_map()` | Filter and transform in one step |
| `take(n)` | Yield at most `n` items |
| `skip(n)` | Skip the first `n` items |
| `merge()` | Interleave two streams |
| `chain()` | Concatenate two streams |
| `throttle(duration)` | Rate-limit items |
| `timeout(duration)` | Error if no item within duration |
| `chunks(n)` | Batch items into vectors |
| `fuse()` | Ensure stream returns `None` forever after first `None` |
| `peekable()` | Peek at the next item without consuming |
| `all()` / `any()` | Boolean aggregation |
| `fold()` | Accumulate into a single value |
| `collect()` | Collect into a container |

### StreamMap

Keyed multiplexing of streams. Each entry has a key, and items are yielded as `(K, V)` pairs. Streams are removed automatically when they complete.

```rust
use tokio_stream::{StreamMap, StreamExt};
use tokio::time::{interval, Duration};
use tokio_stream::wrappers::IntervalStream;

let mut map = StreamMap::new();
map.insert("fast", IntervalStream::new(interval(Duration::from_millis(100))));
map.insert("slow", IntervalStream::new(interval(Duration::from_secs(1))));

while let Some((key, _instant)) = map.next().await {
    println!("tick from: {}", key);
}
```

### Wrappers Module

Convert Tokio types into `Stream` implementations:

| Wrapper | Source Type |
|---------|-------------|
| `IntervalStream` | `tokio::time::Interval` |
| `ReceiverStream` | `tokio::sync::mpsc::Receiver` |
| `UnboundedReceiverStream` | `tokio::sync::mpsc::UnboundedReceiver` |
| `BroadcastStream` | `tokio::sync::broadcast::Receiver` |
| `WatchStream` | `tokio::sync::watch::Receiver` |
| `ReadDirStream` | `tokio::fs::ReadDir` |
| `SignalStream` | `tokio::signal::unix::Signal` |
| `TcpListenerStream` | `tokio::net::TcpListener` |
| `UnixListenerStream` | `tokio::net::UnixListener` |

---

## tokio-util

| | |
|---|---|
| **Crate** | [`tokio-util`](https://crates.io/crates/tokio-util) |
| **Docs** | [docs.rs/tokio-util](https://docs.rs/tokio-util) |
| **Version** | 0.7.x |
| **Repository** | [tokio-rs/tokio](https://github.com/tokio-rs/tokio/tree/master/tokio-util) |

Additional utilities built on top of Tokio. Covers codecs, cancellation, task tracking, I/O bridging, and more.

### Modules

| Module | Description | Key Types |
|--------|-------------|-----------|
| `codec` | Frame encoding/decoding for byte streams | `Encoder`, `Decoder`, `Framed`, `FramedRead`, `FramedWrite`, `BytesCodec`, `LinesCodec`, `LengthDelimitedCodec`, `AnyDelimiterCodec` |
| `compat` | Compatibility between `tokio::io` and `futures-io` | `TokioAsyncReadCompatExt`, `FuturesAsyncReadCompatExt` |
| `sync` | Cancellation and synchronization | `CancellationToken`, `WaitGroup`, `PollSender` |
| `task` | Task group management | `TaskTracker`, `JoinMap`, `LocalPoolHandle` |
| `io` | I/O bridging utilities | `ReaderStream`, `StreamReader`, `SyncIoBridge` |
| `time` | Time-based data structures | `DelayQueue` |
| `either` | Sum type | `Either<A, B>` |
| `net` | Network helpers | `TcpListenerStream`, `UdpFramed` |

### CancellationToken (Graceful Shutdown)

```rust
use tokio_util::sync::CancellationToken;

let token = CancellationToken::new();
let child_token = token.child_token();

let handle = tokio::spawn(async move {
    tokio::select! {
        _ = child_token.cancelled() => {
            println!("shutting down");
        }
        _ = do_work() => {}
    }
});

// Trigger shutdown
token.cancel();
handle.await.unwrap();
```

### TaskTracker (Wait for Task Groups)

```rust
use tokio_util::task::TaskTracker;

let tracker = TaskTracker::new();

for i in 0..10 {
    tracker.spawn(async move {
        do_work(i).await;
    });
}

tracker.close();
tracker.wait().await; // waits for all tracked tasks to complete
```

### Codec Example

```rust
use tokio::net::TcpStream;
use tokio_util::codec::{Framed, LinesCodec};
use futures::{SinkExt, StreamExt};

let stream = TcpStream::connect("127.0.0.1:8080").await?;
let mut framed = Framed::new(stream, LinesCodec::new());

// Send a line
framed.send("hello".to_string()).await?;

// Receive a line
if let Some(Ok(line)) = framed.next().await {
    println!("got: {}", line);
}
```

---

## tokio-test

| | |
|---|---|
| **Crate** | [`tokio-test`](https://crates.io/crates/tokio-test) |
| **Docs** | [docs.rs/tokio-test](https://docs.rs/tokio-test) |
| **Version** | 0.4.x |
| **Repository** | [tokio-rs/tokio](https://github.com/tokio-rs/tokio/tree/master/tokio-test) |

Testing utilities for code built on Tokio.

### Assertion Macros

| Macro | Description |
|-------|-------------|
| `assert_pending!(future)` | Assert the future is not yet ready |
| `assert_ready!(future)` | Assert the future is ready, return value |
| `assert_ready_ok!(future)` | Assert ready with `Ok`, return inner value |
| `assert_ready_err!(future)` | Assert ready with `Err`, return error |

### Testing Futures in Isolation

```rust
use tokio_test::task;

let mut task = task::spawn(async {
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    42
});

// Future is not ready yet
assert_pending!(task.poll());
```

### Mocking AsyncRead + AsyncWrite

```rust
use tokio_test::io::Builder;
use tokio::io::AsyncReadExt;

let mut mock = Builder::new()
    .read(b"hello ")
    .read(b"world")
    .build();

let mut buf = vec![0u8; 11];
mock.read_exact(&mut buf).await.unwrap();
assert_eq!(&buf, b"hello world");
```

---

## bytes

| | |
|---|---|
| **Crate** | [`bytes`](https://crates.io/crates/bytes) |
| **Docs** | [docs.rs/bytes](https://docs.rs/bytes) |
| **Version** | 1.x |
| **Repository** | [tokio-rs/bytes](https://github.com/tokio-rs/bytes) |

Efficient byte buffer library. Essential for network programming — avoids unnecessary copies through reference-counted slicing.

### Key Types

| Type | Description |
|------|-------------|
| `Bytes` | Immutable, reference-counted byte slice. `clone()` is cheap (Arc-based, no data copy). |
| `BytesMut` | Mutable byte buffer with internal cursor tracking. Grows as needed. |

### Key Traits

| Trait | Direction | Key Methods |
|-------|-----------|-------------|
| `Buf` | Read from buffers | `get_u8()`, `get_u16()`, `get_u32()`, `get_u64()`, `advance(n)`, `remaining()`, `chunk()` |
| `BufMut` | Write into buffers | `put_u8()`, `put_u16()`, `put_u32()`, `put_u64()`, `put_slice()`, `remaining_mut()` |

### Usage

```rust
use bytes::{Bytes, BytesMut, Buf, BufMut};

// Immutable bytes — cheap clone
let data = Bytes::from("hello world");
let slice = data.slice(0..5); // "hello", shares underlying memory
let clone = data.clone();     // no copy, just Arc increment

// Mutable buffer — building up data
let mut buf = BytesMut::with_capacity(1024);
buf.put_u32(42);
buf.put_slice(b"hello");
buf.put_u8(b'\n');

// Freeze into immutable Bytes when done
let frozen: Bytes = buf.freeze();

// Reading from a Buf
let mut reader = &frozen[..];
let num = reader.get_u32();
```

---

## mio

| | |
|---|---|
| **Crate** | [`mio`](https://crates.io/crates/mio) |
| **Docs** | [docs.rs/mio](https://docs.rs/mio) |
| **Version** | 1.x |
| **Repository** | [tokio-rs/mio](https://github.com/tokio-rs/mio) |

Low-level, portable, non-blocking I/O abstraction. Wraps OS-specific event notification APIs.

### Platform Backends

| OS | API |
|----|-----|
| Linux | `epoll` |
| macOS / BSD | `kqueue` |
| Windows | IOCP (I/O Completion Ports) |

### Key Types

| Type | Description |
|------|-------------|
| `Poll` | Event loop — registers interest and polls for readiness |
| `Events` | Collection of readiness events |
| `Token` | Identifies a registered I/O source |
| `Interest` | `READABLE`, `WRITABLE`, or both |
| `Registry` | Register/deregister I/O sources with a `Poll` |

Most users never interact with `mio` directly — Tokio's reactor wraps it internally.

---

## Summary Table

| Crate | Version | Description | Docs |
|-------|---------|-------------|------|
| `tokio` | 1.x | Async runtime, I/O, networking, time, sync | [docs.rs/tokio](https://docs.rs/tokio) |
| `tokio-macros` | 2.x | `#[tokio::main]` and `#[tokio::test]` proc macros | [docs.rs/tokio-macros](https://docs.rs/tokio-macros) |
| `tokio-stream` | 0.1.x | Stream trait + adapters + wrappers | [docs.rs/tokio-stream](https://docs.rs/tokio-stream) |
| `tokio-util` | 0.7.x | Codecs, CancellationToken, TaskTracker, I/O bridge | [docs.rs/tokio-util](https://docs.rs/tokio-util) |
| `tokio-test` | 0.4.x | Assertion macros, future testing, I/O mocking | [docs.rs/tokio-test](https://docs.rs/tokio-test) |
| `bytes` | 1.x | Efficient byte buffers (Bytes, BytesMut, Buf, BufMut) | [docs.rs/bytes](https://docs.rs/bytes) |
| `mio` | 1.x | Low-level portable I/O (epoll/kqueue/IOCP) | [docs.rs/mio](https://docs.rs/mio) |

## Next Steps

- **[Tower and Hyper](02-tower-and-hyper.md)** — Service abstraction and HTTP
- **[Related Projects](03-related-projects.md)** — gRPC, tracing, TLS, testing tools
