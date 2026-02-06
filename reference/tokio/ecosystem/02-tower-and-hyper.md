# Tower Service Abstraction and Hyper HTTP

## Overview

Tower provides a generic `Service` trait that models request/response interactions. Hyper implements HTTP/1 and HTTP/2 on top of Tokio. Together with Axum, they form the dominant web stack in the Tokio ecosystem.

```
┌─────────────────────────────────────────────┐
│                  Your App                    │
│              (Axum handlers)                 │
├─────────────────────────────────────────────┤
│             tower-http middleware            │
│        (cors, compression, tracing)          │
├─────────────────────────────────────────────┤
│              Tower Service + Layers          │
│     (timeout, rate-limit, retry, buffer)     │
├─────────────────────────────────────────────┤
│              Hyper (HTTP/1 + HTTP/2)         │
├─────────────────────────────────────────────┤
│              Tokio (async runtime)           │
└─────────────────────────────────────────────┘
```

---

## Tower

| | |
|---|---|
| **Crate** | [`tower`](https://crates.io/crates/tower) |
| **Core** | [`tower-service`](https://crates.io/crates/tower-service) (just the trait) |
| **Layer** | [`tower-layer`](https://crates.io/crates/tower-layer) (just the Layer trait) |
| **HTTP** | [`tower-http`](https://crates.io/crates/tower-http) |
| **Docs** | [docs.rs/tower](https://docs.rs/tower) |
| **Repository** | [tower-rs/tower](https://github.com/tower-rs/tower) |

### The Service Trait

The foundational abstraction — a function from `Request` to `Future<Response>` with backpressure:

```rust
pub trait Service<Request> {
    type Response;
    type Error;
    type Future: Future<Output = Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>>;
    fn call(&mut self, req: Request) -> Self::Future;
}
```

| Method | Purpose |
|--------|---------|
| `poll_ready()` | Backpressure — signals the service can accept a new request |
| `call()` | Process a request, returning a future that resolves to a response |

### The Layer Trait

Wraps a `Service` to add behavior (the middleware pattern):

```rust
pub trait Layer<S> {
    type Service;
    fn layer(&self, inner: S) -> Self::Service;
}
```

A `Layer` takes an inner service `S` and returns a new service that wraps it.

### Built-in Middleware

| Layer | Description |
|-------|-------------|
| `TimeoutLayer` | Fail requests that take longer than a duration |
| `RateLimitLayer` | Limit requests per time interval |
| `RetryLayer` | Retry failed requests with a policy |
| `ConcurrencyLimitLayer` | Limit concurrent in-flight requests |
| `BufferLayer` | Clone-able handle to a service via an internal channel |
| `LoadShedLayer` | Reject requests when the service is not ready |
| `FilterLayer` | Reject requests based on a predicate |

### ServiceBuilder

Compose layers in a readable chain:

```rust
use tower::ServiceBuilder;
use std::time::Duration;

let service = ServiceBuilder::new()
    .timeout(Duration::from_secs(30))
    .rate_limit(100, Duration::from_secs(1))
    .concurrency_limit(50)
    .buffer(100)
    .service(my_service);
```

Layers are applied bottom-up: requests flow through `buffer → concurrency_limit → rate_limit → timeout → my_service`, and responses flow back out in reverse.

### Implementing a Custom Layer

```rust
use tower::{Layer, Service};
use std::task::{Context, Poll};
use std::future::Future;
use std::pin::Pin;

#[derive(Clone)]
struct LogLayer;

impl<S> Layer<S> for LogLayer {
    type Service = LogService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        LogService { inner }
    }
}

#[derive(Clone)]
struct LogService<S> {
    inner: S,
}

impl<S, Request> Service<Request> for LogService<S>
where
    S: Service<Request>,
    Request: std::fmt::Debug,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        println!("request: {:?}", req);
        self.inner.call(req)
    }
}
```

---

## tower-http

| | |
|---|---|
| **Crate** | [`tower-http`](https://crates.io/crates/tower-http) |
| **Docs** | [docs.rs/tower-http](https://docs.rs/tower-http) |
| **Repository** | [tower-rs/tower-http](https://github.com/tower-rs/tower-http) |

HTTP-specific middleware layers built on Tower. Works with any `Service<http::Request<B>>`.

### Available Middleware

| Middleware | Description |
|------------|-------------|
| `CorsLayer` | Cross-Origin Resource Sharing headers |
| `CompressionLayer` | Response body compression (gzip, br, deflate, zstd) |
| `DecompressionLayer` | Request body decompression |
| `TraceLayer` | Request/response tracing with `tracing` |
| `AuthLayer` / `RequireAuthorizationLayer` | Authentication/authorization |
| `SetRequestHeaderLayer` | Add/override request headers |
| `SetResponseHeaderLayer` | Add/override response headers |
| `PropagateHeaderLayer` | Copy headers from request to response |
| `AddExtensionLayer` | Inject data into request extensions |
| `RequestBodyLimitLayer` | Limit request body size |
| `TimeoutLayer` | HTTP-aware request timeout |
| `CatchPanicLayer` | Convert panics into 500 responses |
| `ServeDir` / `ServeFile` | Static file serving |
| `FollowRedirectLayer` | Follow HTTP redirects (client-side) |
| `MapRequestBodyLayer` / `MapResponseBodyLayer` | Transform bodies |
| `ValidateRequestHeaderLayer` | Validate incoming request headers |

### Example: Composing tower-http Middleware

```rust
use tower_http::{
    cors::CorsLayer,
    compression::CompressionLayer,
    trace::TraceLayer,
    timeout::TimeoutLayer,
};
use tower::ServiceBuilder;
use std::time::Duration;

let middleware = ServiceBuilder::new()
    .layer(TraceLayer::new_for_http())
    .layer(CorsLayer::permissive())
    .layer(CompressionLayer::new())
    .layer(TimeoutLayer::new(Duration::from_secs(30)));
```

---

## Hyper

| | |
|---|---|
| **Crate** | [`hyper`](https://crates.io/crates/hyper) |
| **Utilities** | [`hyper-util`](https://crates.io/crates/hyper-util) |
| **Docs** | [docs.rs/hyper](https://docs.rs/hyper) |
| **Repository** | [hyperium/hyper](https://github.com/hyperium/hyper) |
| **Version** | 1.x |

Low-level, correct, fast HTTP/1 and HTTP/2 implementation built on Tokio.

### Architecture (Hyper 1.x)

Hyper 1.x is deliberately low-level. Higher-level conveniences live in `hyper-util`.

| Crate | Purpose |
|-------|---------|
| `hyper` | Core HTTP types, connection handling, body trait |
| `hyper-util` | `Client`, `Server` builders, connection pools, service adapters |
| `http` | Standard `Request`, `Response`, `StatusCode`, `HeaderMap` types |
| `http-body` | `Body` trait for streaming HTTP bodies |
| `http-body-util` | `Full`, `Empty`, `BodyExt`, combinators |

### Key Types

```rust
// From the `http` crate (re-exported by hyper)
http::Request<B>
http::Response<B>
http::StatusCode
http::Method
http::HeaderMap
http::Uri

// Body types (http-body-util)
http_body_util::Full<Bytes>     // single-chunk body
http_body_util::Empty<Bytes>    // no body
http_body_util::BodyExt         // combinators (collect, frame, etc.)
```

### Server Example

```rust
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response};
use hyper_util::rt::TokioIo;
use http_body_util::Full;
use bytes::Bytes;
use tokio::net::TcpListener;

async fn hello(_req: Request<hyper::body::Incoming>) -> Result<Response<Full<Bytes>>, hyper::Error> {
    Ok(Response::new(Full::new(Bytes::from("Hello, World!"))))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:3000").await?;

    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);

        tokio::spawn(async move {
            if let Err(err) = http1::Builder::new()
                .serve_connection(io, service_fn(hello))
                .await
            {
                eprintln!("Error serving connection: {}", err);
            }
        });
    }
}
```

### Client Example

```rust
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use http_body_util::BodyExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::builder(TokioExecutor::new()).build_http();

    let uri = "http://httpbin.org/ip".parse()?;
    let resp = client.get(uri).await?;

    let body = resp.into_body().collect().await?.to_bytes();
    println!("{}", String::from_utf8(body.to_vec())?);

    Ok(())
}
```

---

## Axum

| | |
|---|---|
| **Crate** | [`axum`](https://crates.io/crates/axum) |
| **Extras** | [`axum-extra`](https://crates.io/crates/axum-extra), [`axum-macros`](https://crates.io/crates/axum-macros) |
| **Docs** | [docs.rs/axum](https://docs.rs/axum) |
| **Repository** | [tokio-rs/axum](https://github.com/tokio-rs/axum) |

The most popular Tokio web framework. Built directly on Tower and Hyper — every Axum router is a `tower::Service`.

### Core Concepts

| Concept | Description |
|---------|-------------|
| **Router** | Maps paths to handlers; composable with `nest()` and `merge()` |
| **Handler** | Async function that takes extractors and returns a response |
| **Extractor** | Pulls data from requests (`Path`, `Query`, `Json`, `State`, `Headers`, etc.) |
| **Middleware** | Any Tower `Layer` — use `ServiceBuilder` or `.layer()` on the router |
| **State** | Shared application state injected via `State` extractor |

### Basic Example

```rust
use axum::{
    extract::{Path, State, Json},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
struct AppState {
    db: Arc<RwLock<Vec<User>>>,
}

#[derive(Serialize, Deserialize, Clone)]
struct User {
    id: u64,
    name: String,
}

async fn get_user(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Json<Option<User>> {
    let db = state.db.read().await;
    Json(db.iter().find(|u| u.id == id).cloned())
}

async fn create_user(
    State(state): State<AppState>,
    Json(user): Json<User>,
) -> Json<User> {
    state.db.write().await.push(user.clone());
    Json(user)
}

#[tokio::main]
async fn main() {
    let state = AppState {
        db: Arc::new(RwLock::new(vec![])),
    };

    let app = Router::new()
        .route("/users/{id}", get(get_user))
        .route("/users", post(create_user))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

### Axum with Tower Middleware

```rust
use axum::Router;
use tower::ServiceBuilder;
use tower_http::{
    cors::CorsLayer,
    compression::CompressionLayer,
    trace::TraceLayer,
};
use std::time::Duration;

let app = Router::new()
    .route("/", get(root))
    .layer(
        ServiceBuilder::new()
            .layer(TraceLayer::new_for_http())
            .layer(CorsLayer::permissive())
            .layer(CompressionLayer::new())
    );
```

### Common Extractors

| Extractor | Source | Type |
|-----------|--------|------|
| `Path<T>` | URL path parameters | `T: Deserialize` |
| `Query<T>` | URL query string | `T: Deserialize` |
| `Json<T>` | JSON request body | `T: Deserialize` |
| `State<S>` | Shared application state | `S: Clone` |
| `Headers` | Request headers | `HeaderMap` |
| `Extension<T>` | Request extensions | `T: Clone + Send + Sync` |
| `Form<T>` | URL-encoded form body | `T: Deserialize` |
| `Multipart` | Multipart form data | streaming |
| `ConnectInfo<T>` | Connection info (e.g., remote address) | `T` |

---

## How They Fit Together

```
Request arrives
    │
    ▼
┌─────────────┐
│  TcpListener │  (tokio::net)
└──────┬──────┘
       │
       ▼
┌─────────────┐
│    Hyper     │  Parses HTTP, manages connections
└──────┬──────┘
       │
       ▼
┌─────────────┐
│ Tower Layers │  timeout → rate_limit → cors → trace → ...
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  Axum Router │  Routes request to handler
└──────┬──────┘
       │
       ▼
┌─────────────┐
│   Handler   │  Extracts data, runs business logic, returns response
└─────────────┘
```

| Layer | Crate | Responsibility |
|-------|-------|----------------|
| Transport | `tokio` | TCP/TLS, async I/O |
| Protocol | `hyper` | HTTP parsing, connection management |
| Middleware | `tower` + `tower-http` | Cross-cutting concerns (auth, tracing, limits) |
| Application | `axum` | Routing, extraction, response building |

## Next Steps

- **[Workspace Crates](01-workspace-crates.md)** — Core Tokio crates (runtime, streams, utilities)
- **[Related Projects](03-related-projects.md)** — gRPC, tracing, TLS, WebSockets, testing
