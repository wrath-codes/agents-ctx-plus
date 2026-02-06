# Related Ecosystem Projects

## Overview

The Tokio ecosystem extends well beyond the core runtime and web stack. This document covers major related projects for networking, observability, TLS, WebSockets, testing, and serialization.

---

## Networking

### tonic — gRPC over HTTP/2

| | |
|---|---|
| **Crate** | [`tonic`](https://crates.io/crates/tonic) |
| **Docs** | [docs.rs/tonic](https://docs.rs/tonic) |
| **Repository** | [hyperium/tonic](https://github.com/hyperium/tonic) |

Full-featured gRPC implementation built on Hyper and Tower. Includes code generation from `.proto` files via `tonic-build`.

```rust
// Server-side: implement a generated trait
#[tonic::async_trait]
impl Greeter for MyGreeter {
    async fn say_hello(
        &self,
        request: Request<HelloRequest>,
    ) -> Result<Response<HelloReply>, Status> {
        let reply = HelloReply {
            message: format!("Hello {}!", request.into_inner().name),
        };
        Ok(Response::new(reply))
    }
}

// Client-side
let mut client = GreeterClient::connect("http://[::1]:50051").await?;
let request = tonic::Request::new(HelloRequest {
    name: "World".into(),
});
let response = client.say_hello(request).await?;
```

Key features:
- Unary, server-streaming, client-streaming, and bidirectional streaming RPCs
- Interceptors (middleware) via Tower layers
- TLS support, health checking, reflection
- `tonic-build`: compile `.proto` files at build time
- `tonic-reflection`: gRPC server reflection
- `tonic-health`: gRPC health checking protocol

---

### warp — Composable Web Framework

| | |
|---|---|
| **Crate** | [`warp`](https://crates.io/crates/warp) |
| **Docs** | [docs.rs/warp](https://docs.rs/warp) |
| **Repository** | [seanmonstar/warp](https://github.com/seanmonstar/warp) |

Filter-based web framework built on Hyper. Composes request handling through combinators.

```rust
use warp::Filter;

let hello = warp::path!("hello" / String)
    .map(|name| format!("Hello, {}!", name));

let health = warp::path("health")
    .map(|| "ok");

let routes = hello.or(health);

warp::serve(routes).run(([127, 0, 0, 1], 3000)).await;
```

> **Note**: Axum has largely superseded warp for new projects due to its deeper Tower integration and more flexible API.

---

### reqwest — Ergonomic HTTP Client

| | |
|---|---|
| **Crate** | [`reqwest`](https://crates.io/crates/reqwest) |
| **Docs** | [docs.rs/reqwest](https://docs.rs/reqwest) |
| **Repository** | [seanmonstar/reqwest](https://github.com/seanmonstar/reqwest) |

High-level HTTP client built on Hyper. The go-to crate for making HTTP requests.

```rust
use reqwest::Client;
use serde::Deserialize;

#[derive(Deserialize)]
struct ApiResponse {
    origin: String,
}

let client = Client::new();

// Simple GET
let body = client.get("https://httpbin.org/ip")
    .send()
    .await?
    .text()
    .await?;

// JSON deserialization
let resp: ApiResponse = client.get("https://httpbin.org/ip")
    .send()
    .await?
    .json()
    .await?;

// POST with JSON body
let resp = client.post("https://httpbin.org/post")
    .json(&serde_json::json!({ "key": "value" }))
    .header("Authorization", "Bearer token")
    .timeout(std::time::Duration::from_secs(10))
    .send()
    .await?;
```

Key features:
- Connection pooling, keep-alive
- TLS via `rustls` or `native-tls`
- Cookie store, redirect following
- Multipart form uploads
- Streaming request/response bodies
- Proxy support
- Blocking client (feature-gated) for non-async contexts

---

### h2 — HTTP/2 Implementation

| | |
|---|---|
| **Crate** | [`h2`](https://crates.io/crates/h2) |
| **Docs** | [docs.rs/h2](https://docs.rs/h2) |
| **Repository** | [hyperium/h2](https://github.com/hyperium/h2) |

Low-level HTTP/2 frame layer. Used internally by Hyper for HTTP/2 support. Most users interact with it through Hyper or Tonic rather than directly.

---

## Observability

### tracing — Structured Diagnostics

| | |
|---|---|
| **Crate** | [`tracing`](https://crates.io/crates/tracing) |
| **Docs** | [docs.rs/tracing](https://docs.rs/tracing) |
| **Repository** | [tokio-rs/tracing](https://github.com/tokio-rs/tracing) |

The structured, async-aware diagnostics framework for Rust. Designed to work with async runtimes where traditional logging (line-based, thread-local context) breaks down.

#### Core Concepts

| Concept | Description |
|---------|-------------|
| **Span** | A period of time with structured fields — enters/exits as tasks are polled |
| **Event** | A single point-in-time occurrence (like a log line) |
| **Subscriber** | Collects and processes spans and events |
| **Layer** | Composable subscriber behavior (similar to Tower's Layer) |

#### Usage

```rust
use tracing::{info, warn, error, debug, trace, instrument, span, Level};

#[tracing::instrument]
async fn handle_request(id: u64, path: String) {
    info!("processing request");

    let result = do_work(id).await;

    match result {
        Ok(val) => info!(value = %val, "request completed"),
        Err(e) => error!(error = %e, "request failed"),
    }
}

fn manual_span() {
    let span = span!(Level::INFO, "my_operation", key = "value");
    let _guard = span.enter();
    info!("inside span");
}
```

#### Companion Crates

| Crate | Description |
|-------|-------------|
| [`tracing-subscriber`](https://crates.io/crates/tracing-subscriber) | Subscriber implementations: `fmt` (human-readable), `json`, `EnvFilter` for `RUST_LOG`-style filtering, layer composition |
| [`tracing-futures`](https://crates.io/crates/tracing-futures) | `.instrument(span)` on futures and streams |
| [`tracing-opentelemetry`](https://crates.io/crates/tracing-opentelemetry) | Export spans to OpenTelemetry collectors (Jaeger, Zipkin, OTLP) |
| [`tracing-appender`](https://crates.io/crates/tracing-appender) | Non-blocking log appending, file rotation |
| [`tracing-log`](https://crates.io/crates/tracing-log) | Bridge between `log` crate and `tracing` |
| [`tracing-error`](https://crates.io/crates/tracing-error) | Enrich error types with span context (SpanTrace) |

#### Subscriber Setup

```rust
use tracing_subscriber::{fmt, EnvFilter, prelude::*};

tracing_subscriber::registry()
    .with(fmt::layer().with_target(true).with_thread_ids(true))
    .with(EnvFilter::from_default_env()) // respects RUST_LOG
    .init();
```

---

### console — Tokio Runtime Debugger

| | |
|---|---|
| **Crate** | [`console-subscriber`](https://crates.io/crates/console-subscriber) |
| **CLI** | [`tokio-console`](https://crates.io/crates/tokio-console) |
| **Docs** | [docs.rs/console-subscriber](https://docs.rs/console-subscriber) |
| **Repository** | [tokio-rs/console](https://github.com/tokio-rs/console) |

Interactive debugger for async Rust programs. Shows live task states, poll durations, waker counts, and resource usage.

```rust
// In your application
console_subscriber::init(); // replaces tracing_subscriber::init()

// Then run the CLI tool
// $ tokio-console
```

Requires the `tokio` crate built with `--cfg tokio_unstable`.

---

## TLS

### tokio-rustls

| | |
|---|---|
| **Crate** | [`tokio-rustls`](https://crates.io/crates/tokio-rustls) |
| **Docs** | [docs.rs/tokio-rustls](https://docs.rs/tokio-rustls) |
| **Repository** | [rustls/tokio-rustls](https://github.com/rustls/tokio-rustls) |

Async TLS using [rustls](https://github.com/rustls/rustls) (pure Rust, no OpenSSL dependency).

```rust
use tokio_rustls::TlsConnector;
use rustls::ClientConfig;
use std::sync::Arc;

let config = ClientConfig::builder()
    .with_native_roots()?
    .with_no_client_auth();

let connector = TlsConnector::from(Arc::new(config));
let stream = tokio::net::TcpStream::connect("example.com:443").await?;
let tls_stream = connector.connect("example.com".try_into()?, stream).await?;
```

### tokio-native-tls

| | |
|---|---|
| **Crate** | [`tokio-native-tls`](https://crates.io/crates/tokio-native-tls) |
| **Docs** | [docs.rs/tokio-native-tls](https://docs.rs/tokio-native-tls) |

Async TLS using the platform's native TLS library (SChannel on Windows, Secure Transport on macOS, OpenSSL on Linux).

---

## WebSockets

### tokio-tungstenite

| | |
|---|---|
| **Crate** | [`tokio-tungstenite`](https://crates.io/crates/tokio-tungstenite) |
| **Docs** | [docs.rs/tokio-tungstenite](https://docs.rs/tokio-tungstenite) |
| **Repository** | [snapview/tokio-tungstenite](https://github.com/snapview/tokio-tungstenite) |

Async WebSocket implementation for Tokio.

```rust
use tokio_tungstenite::connect_async;
use futures::{SinkExt, StreamExt};

let (mut ws_stream, _response) = connect_async("ws://echo.websocket.org").await?;

ws_stream.send(tungstenite::Message::Text("hello".into())).await?;

if let Some(Ok(msg)) = ws_stream.next().await {
    println!("received: {}", msg);
}
```

---

## Utilities

### async-stream

| | |
|---|---|
| **Crate** | [`async-stream`](https://crates.io/crates/async-stream) |
| **Docs** | [docs.rs/async-stream](https://docs.rs/async-stream) |

Provides `stream!` and `try_stream!` macros for creating `Stream` implementations using `async`/`await` syntax:

```rust
use async_stream::stream;
use tokio_stream::StreamExt;

let s = stream! {
    for i in 0..3 {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        yield i;
    }
};

tokio::pin!(s);
while let Some(val) = s.next().await {
    println!("{}", val);
}
```

### tokio-retry

| | |
|---|---|
| **Crate** | [`tokio-retry`](https://crates.io/crates/tokio-retry) |
| **Docs** | [docs.rs/tokio-retry](https://docs.rs/tokio-retry) |

Retry logic for async operations with configurable strategies:

```rust
use tokio_retry::strategy::{ExponentialBackoff, jitter};
use tokio_retry::Retry;

let strategy = ExponentialBackoff::from_millis(100)
    .max_delay(std::time::Duration::from_secs(10))
    .map(jitter)
    .take(5);

let result = Retry::spawn(strategy, || async {
    do_fallible_work().await
}).await?;
```

### tokio-cron-scheduler

| | |
|---|---|
| **Crate** | [`tokio-cron-scheduler`](https://crates.io/crates/tokio-cron-scheduler) |
| **Docs** | [docs.rs/tokio-cron-scheduler](https://docs.rs/tokio-cron-scheduler) |

Cron-based task scheduling for Tokio:

```rust
use tokio_cron_scheduler::{JobScheduler, Job};

let sched = JobScheduler::new().await?;

sched.add(Job::new("0 */5 * * * *", |_uuid, _lock| {
    println!("runs every 5 minutes");
})?).await?;

sched.start().await?;
```

---

## Testing

### loom — Concurrency Testing

| | |
|---|---|
| **Crate** | [`loom`](https://crates.io/crates/loom) |
| **Docs** | [docs.rs/loom](https://docs.rs/loom) |
| **Repository** | [tokio-rs/loom](https://github.com/tokio-rs/loom) |

Permutation-based concurrency testing tool. Explores all possible thread interleavings to find data races, deadlocks, and other concurrency bugs.

```rust
use loom::sync::Arc;
use loom::sync::atomic::{AtomicUsize, Ordering};
use loom::thread;

#[test]
fn test_concurrent_increment() {
    loom::model(|| {
        let counter = Arc::new(AtomicUsize::new(0));
        let c1 = counter.clone();
        let c2 = counter.clone();

        let t1 = thread::spawn(move || { c1.fetch_add(1, Ordering::SeqCst); });
        let t2 = thread::spawn(move || { c2.fetch_add(1, Ordering::SeqCst); });

        t1.join().unwrap();
        t2.join().unwrap();

        assert_eq!(counter.load(Ordering::SeqCst), 2);
    });
}
```

Replaces `std::sync`, `std::thread`, etc. with `loom::sync`, `loom::thread` to control scheduling.

### turmoil — Network Simulation Testing

| | |
|---|---|
| **Crate** | [`turmoil`](https://crates.io/crates/turmoil) |
| **Docs** | [docs.rs/turmoil](https://docs.rs/turmoil) |
| **Repository** | [tokio-rs/turmoil](https://github.com/tokio-rs/turmoil) |

Deterministic network simulation for testing distributed systems. Simulates network partitions, latency, and packet loss without real sockets.

```rust
use turmoil::Builder;

let mut sim = Builder::new().build();

sim.host("server", || async {
    let listener = turmoil::net::TcpListener::bind("0.0.0.0:8080").await?;
    let (mut conn, _) = listener.accept().await?;
    // handle connection...
    Ok(())
});

sim.client("client", async {
    let mut conn = turmoil::net::TcpStream::connect("server:8080").await?;
    // send data...
    Ok(())
});

sim.run().unwrap();
```

---

## Serialization

### tokio-serde

| | |
|---|---|
| **Crate** | [`tokio-serde`](https://crates.io/crates/tokio-serde) |
| **Docs** | [docs.rs/tokio-serde](https://docs.rs/tokio-serde) |

Frame-level serialization/deserialization for Tokio transport streams. Bridges `tokio-util` codecs with `serde`.

| Feature | Codec |
|---------|-------|
| `json` | JSON via `serde_json` |
| `bincode` | Bincode binary format |
| `cbor` | CBOR via `serde_cbor` |
| `messagepack` | MessagePack via `rmp-serde` |

```rust
use tokio_serde::formats::Json;
use tokio_util::codec::{Framed, LengthDelimitedCodec};

let transport = Framed::new(tcp_stream, LengthDelimitedCodec::new());
let mut serialized = tokio_serde::Framed::new(
    transport,
    Json::<MyMessage, MyMessage>::default(),
);
```

---

## Summary Table

| Category | Crate | Description | Links |
|----------|-------|-------------|-------|
| **gRPC** | `tonic` | gRPC client/server over HTTP/2 | [crates.io](https://crates.io/crates/tonic) · [docs](https://docs.rs/tonic) |
| **Web** | `warp` | Filter-based web framework | [crates.io](https://crates.io/crates/warp) · [docs](https://docs.rs/warp) |
| **HTTP Client** | `reqwest` | Ergonomic HTTP client | [crates.io](https://crates.io/crates/reqwest) · [docs](https://docs.rs/reqwest) |
| **HTTP/2** | `h2` | Low-level HTTP/2 frames | [crates.io](https://crates.io/crates/h2) · [docs](https://docs.rs/h2) |
| **Diagnostics** | `tracing` | Structured async-aware logging | [crates.io](https://crates.io/crates/tracing) · [docs](https://docs.rs/tracing) |
| **Diagnostics** | `tracing-subscriber` | Subscriber implementations | [crates.io](https://crates.io/crates/tracing-subscriber) · [docs](https://docs.rs/tracing-subscriber) |
| **Diagnostics** | `tracing-opentelemetry` | OpenTelemetry export | [crates.io](https://crates.io/crates/tracing-opentelemetry) · [docs](https://docs.rs/tracing-opentelemetry) |
| **Debugger** | `tokio-console` | Runtime task debugger | [crates.io](https://crates.io/crates/tokio-console) · [docs](https://docs.rs/console-subscriber) |
| **TLS** | `tokio-rustls` | Async TLS (pure Rust) | [crates.io](https://crates.io/crates/tokio-rustls) · [docs](https://docs.rs/tokio-rustls) |
| **TLS** | `tokio-native-tls` | Async TLS (platform native) | [crates.io](https://crates.io/crates/tokio-native-tls) · [docs](https://docs.rs/tokio-native-tls) |
| **WebSocket** | `tokio-tungstenite` | Async WebSocket client/server | [crates.io](https://crates.io/crates/tokio-tungstenite) · [docs](https://docs.rs/tokio-tungstenite) |
| **Streams** | `async-stream` | `stream!` macro for easy streams | [crates.io](https://crates.io/crates/async-stream) · [docs](https://docs.rs/async-stream) |
| **Retry** | `tokio-retry` | Retry strategies for futures | [crates.io](https://crates.io/crates/tokio-retry) · [docs](https://docs.rs/tokio-retry) |
| **Scheduling** | `tokio-cron-scheduler` | Cron-based task scheduling | [crates.io](https://crates.io/crates/tokio-cron-scheduler) · [docs](https://docs.rs/tokio-cron-scheduler) |
| **Testing** | `loom` | Concurrency permutation testing | [crates.io](https://crates.io/crates/loom) · [docs](https://docs.rs/loom) |
| **Testing** | `turmoil` | Network simulation testing | [crates.io](https://crates.io/crates/turmoil) · [docs](https://docs.rs/turmoil) |
| **Serialization** | `tokio-serde` | Serde codecs for Tokio streams | [crates.io](https://crates.io/crates/tokio-serde) · [docs](https://docs.rs/tokio-serde) |

## Next Steps

- **[Workspace Crates](01-workspace-crates.md)** — Core Tokio runtime and companion crates
- **[Tower and Hyper](02-tower-and-hyper.md)** — Service abstraction and HTTP stack
