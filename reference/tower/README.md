# Tower - Quick Introduction

> **A library of modular and reusable components for building robust networking clients and servers**

Tower is a framework for composing asynchronous request/response services in Rust. At its core is the `Service` trait — an asynchronous function from a `Request` to a `Response` with backpressure support. Tower provides a collection of middleware (rate limiting, timeouts, retries, load balancing) and utilities for building reliable, production-grade network services. Used extensively by Hyper, Axum, Tonic, and the broader Tokio ecosystem.

## Key Features

| Feature | Description |
|---------|-------------|
| **Service Trait** | Universal abstraction for async request/response — the foundation of the Tokio web stack |
| **Layer Trait** | Composable middleware pattern for wrapping services with cross-cutting concerns |
| **ServiceBuilder** | Declarative, chainable API for composing multiple layers |
| **Backpressure** | Built-in `poll_ready` mechanism prevents overloading services |
| **Middleware** | Timeout, retry, rate limit, concurrency limit, load shed, buffer, filter, balance |
| **HTTP Support** | `tower-http` provides CORS, compression, tracing, auth, and more for HTTP services |
| **Ecosystem** | Powers Axum, Hyper, Tonic, Warp — the dominant Rust web/gRPC stack |

## Quick Start

### Cargo.toml

```toml
[dependencies]
tower = { version = "0.5", features = ["full"] }
```

### Basic Service

```rust
use tower::{Service, ServiceBuilder, ServiceExt};
use tower::timeout::TimeoutLayer;
use std::time::Duration;
use std::convert::Infallible;

// Using service_fn to create a service from a closure
let service = tower::service_fn(|req: String| async move {
    Ok::<_, Infallible>(format!("Hello, {}!", req))
});

// Wrap with middleware
let service = ServiceBuilder::new()
    .timeout(Duration::from_secs(30))
    .service(service);
```

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                   Your Application                   │
│                                                      │
│   ServiceBuilder::new()                              │
│       .layer(TimeoutLayer::new(...))                 │
│       .layer(RateLimitLayer::new(...))               │
│       .layer(ConcurrencyLimitLayer::new(...))        │
│       .service(my_service)                           │
│                                                      │
├──────────────────────────────────────────────────────┤
│                                                      │
│   Request ──► Layer N ──► ... ──► Layer 1 ──► Service│
│                                                      │
│   Response ◄── Layer N ◄── ... ◄── Layer 1 ◄── ─────│
│                                                      │
├──────────────────────────────────────────────────────┤
│                                                      │
│   ┌──────────────────┐    ┌──────────────────┐       │
│   │  tower-service   │    │   tower-layer    │       │
│   │  (Service trait) │    │  (Layer trait)    │       │
│   └──────────────────┘    └──────────────────┘       │
│                                                      │
│   ┌──────────────────┐    ┌──────────────────┐       │
│   │     tower        │    │   tower-http     │       │
│   │  (middleware +   │    │ (HTTP-specific   │       │
│   │   utilities)     │    │  middleware)     │       │
│   └──────────────────┘    └──────────────────┘       │
│                                                      │
└──────────────────────────────────────────────────────┘
```

## Essential Rust Types

| Type | Purpose |
|------|---------|
| `Service<Request>` | Core trait: async function from Request to Response with backpressure |
| `Layer<S>` | Wraps a service to add behavior (middleware pattern) |
| `ServiceBuilder` | Declarative composition of layers into a service stack |
| `ServiceExt<Request>` | Extension methods on any `Service` (ready, oneshot, map, etc.) |
| `ServiceFn` | Service created from an async closure via `service_fn` |
| `BoxService` | Type-erased, heap-allocated service |
| `BoxCloneService` | Type-erased, cloneable service |

## Documentation Map

```
reference/tower/
├── index.md                    # Comprehensive reference and navigation
├── README.md                   # This file - quick introduction
├── core/                       # Core traits and builders
│   ├── 01-service-trait.md
│   ├── 02-layer-trait.md
│   └── 03-service-builder.md
├── middleware/                  # Built-in middleware
│   ├── 01-timeout.md
│   ├── 02-rate-limit.md
│   ├── 03-retry.md
│   ├── 04-concurrency.md
│   └── 05-other.md
├── tower-http/                  # HTTP-specific middleware
│   ├── 01-overview.md
│   ├── 02-request-response.md
│   └── 03-observability.md
└── patterns/                    # Usage patterns
    ├── 01-composition.md
    └── 02-custom-middleware.md
```

## Quick Links

- **[Complete Reference](index.md)** - Comprehensive documentation and navigation
- **[Core](core/)** - Service trait, Layer trait, ServiceBuilder
- **[Middleware](middleware/)** - Timeout, retry, rate limit, concurrency, buffer
- **[tower-http](tower-http/)** - CORS, compression, tracing, auth, static files
- **[Patterns](patterns/)** - Composition patterns, custom middleware

## Related References

- **[Tokio Runtime](../tokio/)** - The async runtime Tower services run on
- **[Axum](../axum/)** - Web framework built on Tower
- **[Tonic](../tonic/)** - gRPC framework built on Tower

## External Resources

- **[Crates.io](https://crates.io/crates/tower)** - Tower crate
- **[API Docs](https://docs.rs/tower)** - docs.rs reference
- **[GitHub Repository](https://github.com/tower-rs/tower)** - Source code and issues
- **[tower-http Docs](https://docs.rs/tower-http)** - HTTP middleware docs
- **[tower-http GitHub](https://github.com/tower-rs/tower-http)** - HTTP middleware source

---

**Tower - Modular, composable middleware for async Rust services.**
