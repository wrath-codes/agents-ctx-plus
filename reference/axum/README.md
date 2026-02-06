# Axum - Quick Introduction

> **Ergonomic and modular web framework built on Tower and Hyper**

Axum is a web application framework built on top of Tower and Hyper by the Tokio project. It provides a routing system, request extraction, response building, and middleware composition — all built on the Tower `Service` trait. Every Axum router is a Tower service, making the entire Tower middleware ecosystem available. Axum focuses on ergonomics and modularity without sacrificing performance.

## Key Features

| Feature | Description |
|---------|-------------|
| **Router** | Type-safe routing with path parameters, nesting, merging, and fallbacks |
| **Extractors** | Declarative request parsing — Path, Query, Json, State, Headers, and more |
| **Responses** | IntoResponse trait makes returning responses natural and flexible |
| **Middleware** | Full Tower compatibility plus `from_fn` for quick middleware |
| **Type-Safe** | Compile-time errors for handler signature mismatches |
| **WebSocket** | Built-in WebSocket upgrade support |
| **SSE** | Server-Sent Events for real-time streaming |
| **No Macros Required** | Routing and handlers work without procedural macros |

## Quick Start

### Cargo.toml

```toml
[dependencies]
axum = "0.8"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
```

### Hello World

```rust
use axum::{routing::get, Router};

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

## Architecture

```
                    ┌──────────────────┐
                    │  Client Request  │
                    └────────┬─────────┘
                             │
              ┌──────────────▼──────────────┐
              │      Tokio TcpListener      │
              └──────────────┬──────────────┘
                             │
              ┌──────────────▼──────────────┐
              │       Hyper HTTP/1+2        │
              └──────────────┬──────────────┘
                             │
              ┌──────────────▼──────────────┐
              │   Tower Middleware Layers    │
              │  (trace, cors, compression) │
              └──────────────┬──────────────┘
                             │
              ┌──────────────▼──────────────┐
              │        Axum Router          │
              │                             │
              │  route("/users", get|post)  │
              │  route("/items/{id}", get)  │
              │  nest("/api", api_router)   │
              └──────────────┬──────────────┘
                             │
              ┌──────────────▼──────────────┐
              │         Handler             │
              │                             │
              │  Extractors → Business →    │
              │          → IntoResponse     │
              └─────────────────────────────┘
```

## Essential Rust Types

| Type | Purpose |
|------|---------|
| `Router` | Routes requests to handlers and services |
| `Handler` | Async function that processes requests |
| `Path<T>` | Extracts URL path parameters |
| `Query<T>` | Extracts query string parameters |
| `Json<T>` | Extracts/returns JSON bodies |
| `State<S>` | Extracts shared application state |
| `Form<T>` | Extracts URL-encoded form data |
| `IntoResponse` | Trait for types that can be returned as responses |
| `Redirect` | HTTP redirect response |
| `StatusCode` | HTTP status code (re-exported from `http`) |

## Documentation Map

```
reference/axum/
├── index.md                    # Comprehensive reference and navigation
├── README.md                   # This file - quick introduction
├── core/                       # Core framework
│   ├── 01-router.md
│   ├── 02-handlers.md
│   ├── 03-extractors.md
│   └── 04-responses.md
├── middleware/                  # Middleware
│   ├── 01-from-fn.md
│   └── 02-tower-integration.md
├── advanced/                    # Advanced features
│   ├── 01-websockets.md
│   ├── 02-sse.md
│   ├── 03-state-management.md
│   └── 04-error-handling.md
└── extras/                      # Extensions and testing
    ├── 01-axum-extra.md
    └── 02-testing.md
```

## Quick Links

- **[Complete Reference](index.md)** - Comprehensive documentation and navigation
- **[Core](core/)** - Router, handlers, extractors, responses
- **[Middleware](middleware/)** - from_fn, Tower integration
- **[Advanced](advanced/)** - WebSocket, SSE, state, error handling
- **[Extras](extras/)** - axum-extra, testing

## Related References

- **[Tower](../tower/)** - The service abstraction Axum builds on
- **[Tokio](../tokio/)** - The async runtime
- **[Tonic](../tonic/)** - gRPC framework (sibling project)

## External Resources

- **[Crates.io](https://crates.io/crates/axum)** - Axum crate
- **[API Docs](https://docs.rs/axum)** - docs.rs reference
- **[GitHub Repository](https://github.com/tokio-rs/axum)** - Source code and issues
- **[axum-extra Docs](https://docs.rs/axum-extra)** - Extra utilities
- **[Examples](https://github.com/tokio-rs/axum/tree/main/examples)** - Official examples

---

**Axum - Ergonomic web framework for Rust, powered by Tower and Tokio.**
